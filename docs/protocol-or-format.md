# Contract and artifact format v1

All formats use UTF-8 JSON. Contract `schema_version`, report `schema_version`,
and canonical IR `semantic_version` are separately checked and currently `1`.
Unknown fields, duplicate fields, unknown enum variants, and unsupported
versions are rejected. Labels contain 1–64 ASCII letters, digits, `_`, or `-`.
They identify model concepts and must not contain secrets.

## Contract

Start with [receipt.json](../examples/contracts/receipt.json). The required
top-level fields are:

| Field | Meaning |
| --- | --- |
| `schema_version` | Integer `1`. |
| `name` | Diagnostic contract label; excluded from semantic hashing. |
| `variables` | Array of `{name, values}`; each domain is an array of distinct labels. |
| `initial` | Predicate selecting the nonempty initial belief. |
| `safe` | Predicate that must hold initially and after every decision. |
| `goal` | Predicate that must hold in every possible world when terminating. |
| `actions` | Array of `{id, enabled, outcomes}`. |

Predicates have exactly one of these forms:

```json
{"op":"true"}
{"op":"false"}
{"op":"eq","var":"effect","value":"done"}
{"op":"all","args":[]}
{"op":"any","args":[]}
{"op":"not","arg":{"op":"eq","var":"effect","value":"duplicate"}}
```

These are separate JSON examples, not one JSON document. `all` and `any`
accept predicate arrays; empty `all` is true and empty `any` is false.
References must resolve even in unreachable branches.

Each outcome has `{id, when, set, observe}`. `when` reads the pre-state;
`set` is an array of `{var, value}` assignments applied simultaneously.
Unassigned variables retain their values. Assigning one variable twice is
invalid. `observe` is the visible result label. Multiple applicable outcomes
are nondeterministic alternatives, including alternatives with the same
observation and different effects.

An action with false `enabled` has no outcomes in that world. If it is enabled,
at least one outcome must apply. Compilation checks this over every Cartesian
world, including unreachable and unsafe worlds. There is no implicit failure
outcome and no automatic environment progress between decisions.

## Canonical model binding

`lower` emits `{schema_version, model_sha256, ir, origins}`. It is a diagnostic
export, not an accepted contract input. `origins` maps lowered edges to source
action/outcome labels; it is excluded from the hash.

The SHA-256 input is compact JSON serialization of the canonical IR with this
field order:

```text
semantic_version, variables, states, initial, safe, goal, actions
```

Variables have `name, values` fields and sort by name; domain labels sort
lexicographically. Worlds are lexicographically enumerated arrays of domain
indices. Initial, safe, and goal sets are sorted state-index arrays. Actions
have `id, edges` fields and sort by ID. Edges have `from, to, observation`
fields and sort/deduplicate by that tuple. Strings are ASCII; index numbers
are unsigned decimal integers. There are no floats or unordered maps.

The digest is 64 lowercase hexadecimal characters. Contract title, source
outcome IDs, and duplicate identical transitions do not affect it. Labels
that remain in the canonical IR do. This is Contingram's canonical encoding,
not an implementation of JSON Canonicalization Scheme. A hash binds the
finite model; it does not authenticate its author or validate its real-world
accuracy.

## Report

The fields are `schema_version`, `model_sha256`, `horizon`, `status`,
`stats`, `certificate`, and `limit`. Optional certificate/limit values serialize
as `null` when absent. `stats` contains `nodes` and `work` and
is informational; verification computes its own counts.

| Status | Certificate | Meaning |
| --- | --- | --- |
| `policy_found` | Positive | Safe completion exists within this horizon. |
| `no_policy_within_horizon` | Negative | Every controller fails the stated bounded property. |
| `unknown` | `null` | Resource limits prevented a complete result; no mathematical verdict. |

Complete reports have a matching certificate and `limit: null`. An unknown
report carries a limit object with `stage`, `cap`, and `consumed`. Input
validation and compilation errors are typed errors, not bounded refutations.

## Certificate

A certificate is `{root, nodes}`. Child indices are strictly lower than the
parent index. The root is the last node; every node must be reachable from it.

| Node `kind` | Fields and obligation |
| --- | --- |
| `goal` | Positive leaf; derived belief is entirely safe and goal. |
| `unsafe` | Negative leaf; some derived world is unsafe. |
| `horizon` | Negative leaf; remaining horizon is zero, belief safe but not entirely goal. |
| `choose` | `action`, `branches`; each branch has `observation, next`. Action must apply in every possible world, have only safe successors, and include every possible observation exactly once. |
| `refute` | `actions`; one entry `{action, reason}` for every model action. |

A refutation reason is `{"kind":"disabled"}`, `{"kind":"unsafe"}`, or
`{"kind":"branch","observation":"label","next":0}`. The checker must
establish the stated obstruction; the string alone has no authority. A
branch refutation needs one losing observation, while a positive choice needs
all observations. Negative internal nodes apply only to safe non-goal beliefs
with remaining horizon greater than zero.

The certificate supplies no trusted belief annotations. `verify` reconstructs
the initial belief and every posterior from the compiled contract. An unknown
report has no certificate to verify. Use [authoring](authoring.md) for manual
policy construction and changed-contract rechecking.

## CLI contract

| Command | Output / exit behavior |
| --- | --- |
| `analyze FILE [--horizon N] [--max-nodes N] [--max-work N]` | JSON report; `0` policy, `1` bounded no-policy, `3` unknown, `2` error. |
| `verify MODEL REPORT` | JSON checked verdict; `0` valid complete evidence, `2` error or verification limit. |
| `lower FILE` | Diagnostic normalized-model JSON; `0` success, `2` error. |
| `explain MODEL REPORT` | Text from checked complete evidence; `0` success, `2` error. |
| `follow MODEL REPORT [OBSERVATION ...]` | JSON action or goal from a rechecked positive policy; `0` success, `2` error. |
| `demo` | Offline checked demonstration summary. |
| `telemetry status`, `telemetry explain` | Telemetry state and disclosure. |
| `--help`, `--version` | Command reference and version. |

Unknown flags and repeated options are errors. Machine-readable data goes to
stdout; bounded diagnostics go to stderr. The CLI does not write a report
path itself. Shell redirection is caller-owned and can truncate an existing
file before the command starts: choose a separate output file, inspect the
exit status, then verify the report before using it.

An unexpected observation or an observation after goal is `model_mismatch`.
An invalid artifact is `invalid_certificate`; a checker budget exhaustion is
`verification_limit`. Neither licenses another action. Exact human-readable
error messages are not a compatibility surface; use the error code.

## Limits

| Resource | Limit |
| --- | --- |
| Contract / report bytes | 1 MiB / 8 MiB |
| Variables / values per variable | 12 / 16; at least one of each |
| Cartesian worlds | 256 |
| Actions / total outcomes | 32 / 512 |
| Expanded outcome edges, before deduplication | 16,384 |
| Predicate depth / total predicate nodes | 16 / 4,096 |
| Compilation predicate work | 1,000,000 |
| Horizon | Default 8; maximum 32 |
| Search memo/evidence nodes | Default 10,000; maximum 50,000 |
| Search work | Default 1,000,000; maximum 10,000,000 |
| Checker contexts / work | Default and maximum 50,000 / 10,000,000 |

The Rust API may lower checker caps. Analysis with a zero work or node budget
returns `unknown`, including a trivial goal. Hard-ceiling violations are
invalid limits. Compiler limits return typed errors before search. Complete
analysis also passes fixed bounded self-verification and the report byte cap.
Wall-clock performance is separate from deterministic work accounting.

Checker work includes structural node/branch/action preflight as well as
semantic traversal. Search statistics and checker statistics measure different
operations and must not be compared as if their units were interchangeable.
