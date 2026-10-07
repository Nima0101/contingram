# Reproducible local evaluation

Run from the repository root with Rust and locked dependencies available:

```sh
cargo run --locked --release --example evaluate > evaluation.json
```

The generator executes three warmups and 21 measured samples per case. Worlds range over 4, 8, 16, 32, 64, 128 and 256. All cases have two hidden modes, an initial incomplete effect and irrelevant finite noise variables. A mode-specific finishing action is disabled in the other mode.

- `ambiguity`: sensing distinguishes the two modes; two calls suffice.
- `branching`: sensing additionally reveals the noise variables, producing up to 128 observation branches; two calls suffice.
- `unresolved`: sensing merges both modes, so no policy exists at horizon four.
- `cutoff`: the one-work-unit limit deliberately returns UNKNOWN, never a refutation.

Three practical manual-policy contracts are measured at horizon three as well. Every complete result is independently verified. Every repeated report must be byte-identical. A failed expectation, checker rejection or nondeterministic report aborts the experiment. The generator does not compare against an unexecuted competitor.

Timing excludes parsing, compilation, file IO and artifact serialization. The `synthesis` field times `analyze`, including its mandatory internal verification; `verification` separately times a second independent verifier invocation. `combined` measures both API calls, so it intentionally includes that second check. Counts include model worlds, initial uncertainty, actions, edges, solver/checker work and contexts, serialized artifact bytes, and UNKNOWN samples/reasons. p95 is the nearest-rank 20th ordered value of 21 samples. Minimum, median and maximum are included. UNKNOWN has no verified proof and therefore no checker measurement.

`results.json` is the recorded local release-profile run. The host was Darwin 25.6.0, aarch64, Rust 1.98.1 (`48a229cea`, 2026-09-01), Cargo 1.98.1. Exact CPU identification was unavailable in the measurement environment. Other concurrent processes were not controlled; these are scoped local observations, not portable latency guarantees. Compiler and dependency versions are pinned or recorded by the package and lockfile; rerunning on another host will produce different timings.

A separate Darwin `getrusage(RUSAGE_CHILDREN)` run measured peak resident memory of **6,668,288 bytes** (about 6.36 MiB) for the entire evaluation executable, including model compilation, all warmups/samples, and output serialization. This is a whole-process peak, not per-model memory. The `/usr/bin/time -l` route was unavailable because its `sysctl kern.clockrate` call was denied; Python’s standard-library process resource accounting was used instead.
