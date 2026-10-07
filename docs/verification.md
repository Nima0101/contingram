# Verification and reproducibility

Verification has several separate layers: correctness of the finite semantics,
hostile-input behavior, artifact checking, authoring experiments, packaging,
platform builds, and release provenance. Passing one layer does not imply the
others passed.

## Run the local checks

From the repository root with Rust 1.85 or newer, rustfmt, Clippy, and Python 3:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --release
cargo doc --locked --no-deps
python3 scripts/public_gate.py
python3 scripts/package_consumer.py
```

The public gate checks the source inventory, private-path and credential
signatures, and local Markdown links. The package-consumer check builds the
distributable crate, scans its archive, and uses a separate temporary crate
and installed CLI. These checks complement dependency advisory/license and
secret-scanning tools; matching no pattern is not proof that secrets cannot
be present.

## Correctness evidence

| Surface | Executable evidence |
| --- | --- |
| Parser and lowering | Hostile contracts, predicate truth tables, simultaneous updates, direct reference lowering, and canonical-byte/hash fixtures. |
| Search recurrence | Positive/negative cases, horizon transitions, universal applicability, no implicit fairness, and complete round-trip verification. |
| Reference comparisons | A tiny bottom-up exhaustive belief oracle over original transition tables, plus an observation-history-table oracle that enumerates concrete executions. |
| Metamorphic relations | Increasing horizon, bijective observation renaming, observation refinement, weakened safety, adversarial added outcomes, and declaration permutation. Each relation has explicit side conditions in its test. |
| Hostile artifacts | Missing/extra branches and actions, false goals/reasons, version/hash/rank changes, unreachable nodes, duplicate fields, and incompatible shared-node contexts. |
| Resources | Boundary budgets, cutoff-to-UNKNOWN behavior, explicit checker-limit errors, malformed/oversized inputs, and bounded serialization. |
| CLI and privacy | Exit codes, file handling, following observations, discoverable telemetry status, consent/kill-switch behavior, and a closed event schema. |
| Interface changes | Synthetic corpus, separately authored held-out models, and manual policies checked independently of synthesis. |

The history-table oracle does not implement the solver's belief recurrence:
it enumerates observation-dependent action tables, then checks concrete paths.
It still consumes the compiled IR, so direct lowering-reference tests cover
that shared assumption. The bottom-up oracle starts from separately generated
transition tables. None of these tests constitutes a proof about all possible
Rust executions.

The held-out split describes when examples were authored relative to the
solver. It is not a statistical generalization result. Tests compare exact
model-relative expectations, not real-world false-positive or false-negative
rates. See [evaluation](evaluation.md) for corpus provenance and measurements.

## Fuzzing and sanitizers

Normal tests include deterministic byte mutations that reach parsing and
semantic paths. The [cargo-fuzz targets](../fuzz/README.md) provide coverage-guided
contract and report inputs with AddressSanitizer. Use the pinned toolchain and
commands there; keep any discovered crash as a regression before resuming.
Bounded fuzz smoke is useful evidence, not an exhaustive campaign or a claim
that all malformed input is safe.

Project sources forbid unsafe Rust. That does not eliminate dependency,
compiler, resource, or semantic defects. Dependency advisories and license
checks are separate from sanitizer and correctness tests.

## Release and platform claims

The hosted matrix exercises the declared release targets. A local successful
build does not establish another operating system's support. Inspect the
Actions runs for the exact revision and the release's assets rather than
inferring success from a workflow file.

Release verification should confirm the source tag, checksums, SBOM, and
available build provenance, then download and execute the relevant archive.
After publication, a fresh unauthenticated clone must run the exact README
quickstart and test command. The archive and public-clone leak scans are
separate checks. A locally staged or tested tree is not evidence that a
public release or hosted check exists.

For a published `v0.1.0`, download the assets into a new directory:

```sh
gh release download v0.1.0 --repo Nima0101/contingram --dir release-assets
cd release-assets
```

On Linux, use `sha256sum -c SHA256SUMS`; on macOS, use
`shasum -a 256 -c SHA256SUMS`. On Windows, compare the SHA-256 value from
`Get-FileHash -Algorithm SHA256 ASSET` against the corresponding line. Validate
every downloaded archive and `contingram.spdx.json`. Then verify provenance for
each asset, including the checksum file:

```sh
gh attestation verify SHA256SUMS --repo Nima0101/contingram
gh attestation verify contingram.spdx.json --repo Nima0101/contingram
```

Repeat the second command with the actual native archive filename. These are
post-publication instructions, not a statement that assets already exist.
The official [download](https://cli.github.com/manual/gh_release_download) and
[attestation verification](https://cli.github.com/manual/gh_attestation_verify)
references explain the GitHub CLI requirements. The
[release workflow](../.github/workflows/release.yml) defines the asset inventory
and source/check gates. Provenance establishes the attested build origin, not
model fidelity or bug freedom.

## Interpreting a verdict

`policy_found` and `no_policy_within_horizon` are accepted only with complete
matching evidence. `unknown` has no certificate and establishes neither.
`verify` can itself reach a resource limit; this is an error with its own code,
not a successful verdict. A result never proves that an authored abstraction
matches a real API. Read the [finite semantics](engineering-deep-dive.md) and
[threat model](threat-model.md) alongside any result.
