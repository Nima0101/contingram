# Evaluation and evidence boundaries

Contingram is evaluated against finite synthetic contracts. These results establish behavior within those contracts, not production reliability, real API conformance or novel planning theory.

## Correctness comparisons

The concrete-history oracle in `tests/history_oracle.rs` enumerates complete decision tables over all observation histories shorter than the horizon. Each entry is STOP or an action. For each table it executes every concrete initial world and every transition choice, requiring safety on every visited world and a goal at STOP. Decisions depend only on history. It never builds beliefs, calls the production solver recursively, or uses verifier transition helpers.

The exhaustive slice covers 81 disabled/deterministic two-action transition relations, all three nonempty initial sets, all four safe sets, all four goal sets and horizons zero through two: **11,664 comparisons**. A separately seeded nondeterministic three-world, two-action, two-observation slice adds **512 comparisons** at horizons zero through three. Every complete production report is independently verified. The oracle explicitly declines tables exceeding 20,000 candidates; an out-of-envelope model is never classified as losing. A dedicated test covers this boundary.

The older bottom-up belief oracle remains a complementary implementation. It is not counted as the structurally independent history-table oracle. Direct lowering tests separately address errors shared by any consumer of the normalized relation.

## Independent held-out cases

An independent author froze nine contracts, **54 horizon verdicts**, reasoning and relation counts before inspecting or executing the solver. The files and expectations are preserved in `examples/heldout/`; the generator contains no synthesis. The held-out test checks those expectations and independent counts, then verifies all complete evidence.

| Capability change | Without stronger capability | First winning horizon with it |
| --- | --- | --- |
| Refresh stale receipt to current | No policy through tested horizon 8 | 3 |
| Settle/exclude pending work with cancellation fence | No policy through tested horizon 8 | 2 |
| Observe completion stage instead of object existence | No policy through tested horizon 8 | 3 |
| Keep identity through retry | Expiry without ledger: no policy through horizon 8 | 1 |
| Consult authoritative ledger after identity expiry | No policy through tested horizon 8 | 2 |

These are paired semantic changes, not parameter tuning from solver output. The contract author supplies guards, effects and observations; the synthesizer derives a contingent policy or a complete bounded refutation. That distinction is useful when changing a capability invalidates an old recovery table. Results beyond a tested horizon are not inferred from the table alone.

## Manual and naive baselines

Three recovery tables in `examples/manual/` were authored directly from the contract semantics. The test binds each table to a model digest and verifies it without invoking `analyze`. A separate concrete-path runner evaluates the same policy and the naive baselines.

| Contract | Source bytes / lines | Worlds / actions / edges | Manual DAG nodes | Horizon |
| --- | --- | --- | --- | --- |
| stale-receipt-refresh | 3817 / 201 | 6 / 3 / 18 | 4 | 3 |
| identity-expired-ledger | 4105 / 214 | 6 / 2 / 13 | 3 | 2 |
| partial-stage-status | 6060 / 300 | 8 / 3 / 24 | 4 | 3 |

For each of the three cases:

| Policy | Universally safe | Universally complete | Verifier accepts success claim |
| --- | --- | --- | --- |
| Manually authored conditional recovery | Yes | Yes | Yes |
| Stop immediately | Yes | No | No |
| Retry/notify immediately | No | No | No |

The review steps are: enumerate initial ambiguity; identify an unsafe duplicate/orphan path; identify an observation that separates the required decisions; write the history table; check every outgoing observation; count the longest action path; independently verify and concretely execute it. These are small synthetic authoring exercises, not a controlled usability study. JSON line counts reflect explicit formatting and are not a claim of reduced authoring time.

## Performance experiment

The exact generator is `examples/evaluate.rs`; procedure, measurement scope and raw summarized samples are in `benchmarks/README.md` and `benchmarks/results.json`. It uses three warmups and 21 samples over seven world counts, varying ambiguity, observation branching, losing instances and deterministic resource exhaustion, plus the three practical cases. Complete reports are checked every time; repeated bytes must agree. UNKNOWN samples have no certificate and no verifier timing.

Local measurements and limitations are reported with the recorded results. The release-profile target for shipped small demos is median analysis plus verification below 100 ms and artifacts below 100 KiB. This is an engineering acceptance target, not a service-level claim.


The three manual tables are also rebound to weaker paired models (sticky receipts, missing ledger, object-only status), deliberately updating the digest so that the test cannot pass by detecting a hash mismatch alone. The independent verifier rejects all three. The held-out synthesis tests independently establish the replacement result: no policy at each tested bound. This demonstrates the concrete value of separating capability facts from a hand-maintained table: a capability regression can invalidate a plausible recovery procedure, and complete bounded search can distinguish a bad particular table from absence of any admissible table. These cases remain small enough to reason about by hand; they support authoring viability, not measured productivity or adoption claims.

The recorded release run produced 24 complete cases and seven intentionally exhausted cases. Across 21 measured samples each, all **504 complete reports** passed the additional independent check and all **147 cutoff results** were UNKNOWN. The largest branching instance (256 worlds, 128 initial worlds) had median combined API time **4.301 ms** and an **18,191-byte** artifact. The three practical cases had combined medians **0.024–0.054 ms**, with artifacts of **530, 521 and 661 bytes** respectively, below the declared small-demo targets. Analysis includes its internal certificate check; the combined measurement adds another standalone check. See the raw per-case minimum, maximum and p95 in the recorded results. A separate whole-evaluation-process measurement recorded peak RSS of **6,668,288 bytes** on Darwin; this is not a per-model allocation bound.
