# Contributing

Start with the [authoring guide](docs/authoring.md) and
[architecture](docs/architecture.md). Small synthetic contracts that expose a
missing case, clearer examples, and independently checkable semantic fixes are
especially useful. For a large change, open an issue describing the problem,
the proposed contract, and how it will be verified before implementing it.

## Local checks

Install Rust 1.85 or newer with rustfmt and Clippy. Use the committed lockfile.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --release
cargo doc --locked --no-deps
```

Tests and the core demo require no API keys or live tools. Run the publication
and security checks documented in CI when changing packaging or releases.
Do not globally upgrade unrelated dependencies or regenerate fixtures without
explaining the semantic change.

[Verification](docs/verification.md) documents fuzzing, sanitizer scope, and
the isolated package-consumer check. [Fuzzing instructions](fuzz/README.md)
pin the additional development toolchain.

## Correctness changes

Add a failing regression before changing semantics. Exercise both policy and
refutation paths where relevant, and include resource-limit behavior. Keep
the checker independent of the solver's traversal and intermediate results.
Shared parsing or lowering changes need direct tests because a checker using
the same wrong model cannot expose them.

For new corpus cases, include provenance, expected result and horizon,
reasoning independent of search, and whether the fixture informed the
implementation. Synthetic examples must not be represented as real provider
guarantees. Describe any changed schema or error-code compatibility explicitly.

Use minimal dependencies, safe Rust, bounded inputs, and deterministic work
accounting. Do not add live tool execution or telemetry transport as an
incidental feature. The [threat model](docs/threat-model.md) and
[privacy disclosure](docs/telemetry.md) must stay aligned with code.

## Pull requests and conduct

Explain the concrete problem, resulting behavior, tests run, and remaining
limits. Include source links for external factual claims. AI-assisted
contributions are welcome under the same review and evidence requirements;
do not submit private prompts, credentials, or confidential code.

By intentionally submitting a contribution for inclusion, you submit it under
the project's [Apache-2.0 license](LICENSE), unless explicitly stated otherwise
and agreed in advance. Preserve applicable notices. No contributor license
agreement is required. Follow the [code of conduct](CODE_OF_CONDUCT.md), and
use [private reporting](SECURITY.md) for vulnerabilities.
