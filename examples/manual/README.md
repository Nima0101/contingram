# Manually authored recovery policies

These three policy DAGs were written directly from the held-out contracts, without calling the synthesizer. Each contains a horizon and portable certificate nodes. To build a complete report, supply schema version 1, the compiled contract's semantic digest, status `policy_found`, zero advisory statistics, and no resource limit. `tests/manual.rs` performs that binding and passes the result directly to the independent verifier.

| Contract | Manual decisions | Longest path |
| --- | --- | --- |
| Stale receipt with refresh | Refresh; read status; send only if absent; stop if done | 3 |
| Expiring identity with ledger | Read ledger; resend only at zero; stop at one | 2 |
| Partial completion with stage status | Inspect; empty: reserve then notify; reserved: notify; complete: stop | 3 |

The concrete execution baseline checks every initial world and every possible transition, separately reporting safety and completion. For all three contracts the manual policy is safe and complete. Always stopping is safe but incomplete. Immediately sending, resending, or notifying respectively is unsafe and incomplete. The verifier rejects both naive policies as claimed successful artifacts.

The JSON DAGs share their terminal goal node. The partial-completion policy also shares the notify node across two histories at different remaining horizons. This exercises context-dependent verification independently of synthesis's preferred output shape.
