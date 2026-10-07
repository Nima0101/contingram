# Authoring a recovery contract

A useful model records what a controller can know after a tool response, not
what the model author knows happened. Start with a small failure mode whose
effects, observations, safety condition, and completion requirement you can
audit directly. The shipped contracts are original synthetic examples, not
provider specifications.

## Receipt after a lost acknowledgement

Read [receipt.json](../examples/contracts/receipt.json). Its hidden `effect`
variable is `absent`, `done`, or `duplicate`. The initial belief permits
`absent` and `done`; it represents an earlier ambiguous call. Safety excludes
`duplicate`, and completion requires `done`.

`send` reaches `done` from `absent`, but reaches `duplicate` from `done`.
Both outcomes emit `ack`. `status` leaves the effect unchanged and emits a
final, operation-bound observation. At horizon two, an explicitly authored table is:

| Observation history | Decision |
| --- | --- |
| Empty | `status` |
| `committed` | Goal |
| `absent-final` | `send` |
| `absent-final`, `ack` | Goal |

Build once, then analyze and walk that interface:

```sh
cargo build --release --locked
cargo run --release --locked -- analyze examples/contracts/receipt.json --horizon 2 > receipt-report.json
cargo run --release --locked -- verify examples/contracts/receipt.json receipt-report.json
cargo run --release --locked -- explain examples/contracts/receipt.json receipt-report.json
cargo run --release --locked -- follow examples/contracts/receipt.json receipt-report.json
cargo run --release --locked -- follow examples/contracts/receipt.json receipt-report.json absent-final
cargo run --release --locked -- follow examples/contracts/receipt.json receipt-report.json absent-final ack
```

The final three commands produce the abstract decisions `status`, `send`,
and goal. No tool executes. Supply only the observation prefix actually
being explored; an unmodeled label is an error.

## Change the interface, then recheck

| Change | Case | Consequence under the supplied finite model |
| --- | --- | --- |
| Status may be stale | [stale-read](../examples/contracts/stale-read.json) | Repeated labels do not separate absent from done; retry can duplicate. |
| Deduplication remains valid | [idempotent-retry](../examples/contracts/idempotent-retry.json) | One retry can safely complete without sensing. |
| Deduplication might have expired | [idempotent-expiry](../examples/contracts/idempotent-expiry.json) | The same retry may duplicate a completed operation. |
| Operation has two components | [partial-completion](../examples/contracts/partial-completion.json) | Inspect first, then complete only the missing components. |
| A negative read can precede a late commit | [late-pending](../examples/contracts/late-pending.json) | A negative result is insufficient evidence for retry. |
| An explicit settle step gives a final result | [late-pending-fenced](../examples/contracts/late-pending-fenced.json) | The stronger observation permits recovery within two decisions. |

For partial completion, the manual table selects `inspect`; `both` stops,
`left` selects `complete-right`, `right` selects `complete-left`, and `none`
selects `complete-left` followed by `complete-right`. Each completion emits
`ok`. The maximum path consumes three decisions.

These are changes to assumptions, not recommendations to trust a particular
service. Removing a modeled failure without evidence can manufacture a
positive result. Run `lower` and inspect every edge when editing a contract.

## Constructing a manual policy artifact

Manual policies use the same [certificate format](protocol-or-format.md) as
generated ones. Build goal leaves first, then `choose` nodes whose children
point backward. Include every possible observation. Set the root to the last
node, bind the report to `lower`'s `model_sha256`, set a sufficient horizon,
and use `policy_found`, `stats: {"nodes":0,"work":0}`, and `limit: null`.
The stats are informational, so manual tables need not fabricate search work.

For receipt, one shared goal node, one send node with an `ack` branch to goal,
and one status node with `committed` and `absent-final` branches express the
table above. The checker derives the beliefs; authors never need to supply
hidden-state assertions.

A model change should first reject the old report's hash binding. To assess
whether the same manual strategy still works, deliberately update only the
binding (and any explicitly reviewed label correspondence) and re-run
`verify`. Do not use search to fill missing branches in a claimed manual
baseline. Rebinding is an experiment setup step, not successful verification.

## Audit questions

1. Does the initial set include every relevant outcome of the ambiguous call?
2. Can a response describe another operation, stale state, or partial work?
3. Could pending work commit during a later call? Represent that in outcomes.
4. Do unsafe events remain visible in state even if a later operation repairs
   the final state?
5. Is each action enabled in every world where its policy might choose it?
6. Does every allowed outcome have a branch, including repeated failures?
7. Is the goal actual completion, or was unresolved stopping accidentally
   counted as success?

If the answer depends on unmodeled concurrency, timing, identity, or fairness,
refine the finite abstraction before treating the verdict as useful evidence.
An explicit resource `unknown` warrants more budget or a smaller model; it is
not a reason to remove adverse behavior.
