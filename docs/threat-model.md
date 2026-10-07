# Threat model

The assets are correctness of finite-model verdicts, availability of the
analyzing process, confidentiality of caller inputs, and integrity of releases.
Contracts, reports, and observation prefixes may be attacker-controlled.
There is no server, live tool executor, plugin host, or credential store.

## Trust boundaries

| Boundary | Untrusted input | Trusted component / obligation |
| --- | --- | --- |
| File to parser | Caller-selected file bytes | Bounded regular-file reader; typed JSON parser; version and field checks. |
| Contract to model | Predicates, domains, outcomes | Structural validation and finite lowering preserve all effects and observations. |
| Model to search | Adversarial modeled outcomes | Uniform actions over beliefs; universal successor coverage; bounded work. |
| Report to checker | Entire artifact, including status and hash | Reconstruct beliefs; verify binding, rank, polarity, coverage, and every relevant branch. |
| Prefix to follower | Observation labels | Recheck positive evidence; reject impossible or post-goal prefixes. |
| Core to telemetry | Closed coarse event values | No raw-input properties; no collection or network transport in v0.1. |
| Source to release | Dependencies, workflows, build artifacts | Locked inputs, automated checks, tagged source and release evidence. |

Authors are responsible for modeling relevant real behavior. The parser and
compiler are shared trust assumptions; verifier independence applies to
semantic evidence traversal, not the complete software stack. The compiler,
dependencies, operating system, and toolchain remain trusted.

## Attack paths and controls

**False positive evidence.** A forged goal, missing observation, disabled
action, unsafe successor, or incompatible shared proof node must fail checking.
The checker starts from the model's initial belief and derives each posterior;
it never takes an artifact's assertion about hidden state on trust.

**False negative evidence.** A refutation must cover every model action.
For each action it must establish disabledness, a possible unsafe successor,
or a recursively losing observation. One bad execution does not prove that
every controller loses. Safety is checked before completion, including at
horizon zero.

**Resource exhaustion.** JSON has byte and recursion limits. Domains, products,
predicate trees, actions, outcomes, edges, horizon, memo entries, evidence,
and serialized reports have explicit ceilings. The checker charges its own
work and contexts. See [all limits](protocol-or-format.md#limits). Caps bound
algorithmic work; they are not wall-clock deadlines or a process-wide memory
quota. Concurrent callers multiply resource use and should impose their own
admission limits.

**Malformed or ambiguous serialization.** Unknown and duplicate fields,
invalid labels, unresolved references, duplicate IDs, and assignments to the
same variable are rejected. Backward node references, bounded node count,
and complete reachability prevent cycles and irrelevant appended evidence.
Unsupported versions fail explicitly.

**File or command injection.** Only explicit input paths are opened, and the
opened handle must be a regular file. Reads enforce a byte cap independently
of reported file length. Contracts cannot name files to load or commands to
execute. The library spawns no processes. A caller controls stdout redirection
and must avoid overwriting important files; the CLI does not manage output
paths or temporary files.

The caller chooses the filesystem namespace. On Unix, nonblocking open also
prevents a raced FIFO replacement from blocking at open. Filesystem drivers,
network mounts, and operating-system I/O remain trusted; deterministic work
caps are not I/O deadlines or a sandbox for hostile filesystem behavior.

**Input disclosure.** Diagnostics use stable categories and bounded labels
rather than raw JSON excerpts. Reports and `lower` intentionally expose
modeled labels and transitions to their caller; they are not redacted copies
of a confidential model. Do not put actual secrets in labels or fixtures.

**Telemetry collection.** v0.1 can represent a closed event containing version,
coarse platform, command category, outcome category, and duration bucket.
It has no endpoint, network transport, queue, disk spool, or identifier.
`CONTINGRAM_TELEMETRY=0` disables intent, and nonempty `DO_NOT_TRACK`/`DNT`
values other than `0` veto it. Even explicit opt-in cannot send data in this
release. There is no retained telemetry. Tests cover settings and the
payload allowlist. See the full [privacy disclosure](telemetry.md).

**Supply-chain substitution.** Hashes detect asset changes but do not establish
who built them. Release provenance, when attached, ties artifacts to a GitHub
workflow and source revision; it does not prove functional correctness.
Consumers should inspect the release's checksums, provenance, and source tag
and verify them using the documented release procedure.

## Residual risks and excluded guarantees

The tool cannot prove that a finite contract matches a service, that a model
author is trustworthy, or that a later runtime follows its policy. Cancellation
is not assumed to roll back work; fairness and spontaneous progress are not
implicit. The goal does not include safe abstention unless the author models
it that way. Content-based prompt injection, production authorization, and
concurrent execution require separate controls.

No automated scan, synthetic corpus, or agent review is described as a formal
proof or independent human audit. Bugs implementing the stated finite
semantics, bounds, or privacy boundary remain reportable under
[SECURITY.md](../SECURITY.md).

## Optional platform boundary

The statements above concern the offline Rust core. The separate
platform reference (`platform/README.md` in the Git checkout) adds a network service, issuer trust,
database and event transport with its own documented threats and limitations.
