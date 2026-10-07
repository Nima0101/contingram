# Research landscape and claim boundary

Contingram applies established contingent-planning ideas to a narrow developer
workflow: an explicit finite agent-tool contract becomes a checked recovery
policy or a checked obstruction within a decision horizon. The contribution
is the contract, evidence, authoring, and verification workflow; belief states,
partial-observation reasoning, and strong planning are prior art.

## Closest technical neighbors

| Work | Relevant overlap | Contingram's scoped distinction |
| --- | --- | --- |
| [BCP](https://github.com/kevinmcareavey/bcp) | Strong contingent planning through belief-space search with bounded branching. | A tool-interface contract and independently checked positive/negative evidence; a decision-horizon bound. Neither the bound nor tool terminology establishes algorithmic novelty. |
| [CPOR / POPRP integration](https://github.com/aiplan4eu/up-cpor) | Contingent planning and online replanning in a general planning ecosystem. | A small offline JSON workflow with explicit artifact checking. General planners can encode closely related problems. |
| [PRISM](https://www.prismmodelchecker.org/) | Broad formal modeling, including probabilistic systems and partially observable models. | A restricted finite adversarial model requiring no probabilities; this is a narrower engineering choice, not a stronger verification framework. |
| [AFT-Bench: Callability Is Not Operability](https://arxiv.org/abs/2608.23628) | Interface-level recovery under operational uncertainty and controlled interface interventions. | Synthesized policies or bounded obstructions for an authored finite model, rather than empirical agent scores. The underlying ambiguity and interface-design problem are shared. |
| [Verified Tool Calls](https://arxiv.org/abs/2608.02645v1) | Postcondition checks, verification before retry, and idempotency keys under non-atomic failures. | Design-time analysis of whether the declared observations admit completing recovery, including interfaces with inconclusive readback. |
| [AgentX](https://github.com/studivox/agentx) | Transactional MCP recovery, durable ledger, verification, replay protection, compensation, and receipts. | No live execution or transaction middleware; analyze a supplied model before runtime integration. |
| [Temporal Nexus](https://docs.temporal.io/nexus/operations) | Operational contracts for durable operations, retries, and cancellation. | Interface analysis complements these mechanisms; it does not replace a durable runtime. |
| [ToolFuzz](https://github.com/eth-sri/ToolFuzz) | Fault-oriented testing of agent tools. | Exhaustive bounded reasoning over declared finite outcomes, rather than executing and fuzzing a tool implementation. |

[Failing Tools: Benchmarking LLM Agent Recovery Under Runtime Tool Failures](https://openreview.net/forum?id=j7YsSnA64D)
is also related work on recovery evaluation. Its release/acceptance status was
not established in the documented search. A primary PDF access challenge
prevented complete reinspection, so this project makes no detailed comparative
performance or coverage claim about it.

These descriptions summarize public documentation and papers checked on
2026-10-07. Preprint results are the authors' claims. Competitors were not
benchmarked, and no measured superiority is implied.

## What was searched

The project-selection search on 2026-10-07 covered public GitHub repositories
and neighboring terms, package registries, papers, and protocol documentation.
It included contingent planning, partial-observation safety, uncertain tool
effects, retry verification, interface operability, agent failure evaluation,
and recovery middleware. That search informed comparison of more than twenty
candidate directions and five finalists.

No exact active packaged substitute for the documented
contract-to-independently-checked recovery artifact workflow was established
in that search. This is a bounded search observation, not a proof that no
substitute exists. Close technical substitutes do exist, and discovery of a
similar tool would not invalidate the finite semantics or tests.

## Claims to keep separate

A checked artifact proves a property of its finite model at its horizon.
It does not prove real API conformance, exactly-once execution, rollback on
cancellation, unbounded liveness, fairness, or policy optimality. A hash does
not establish model truth. A synthetic corpus does not measure production
accuracy or adoption. Agent review is not a human user study.

Contingram's [authoring examples](authoring.md) make the key product tradeoff
visible: writing a faithful model has a cost. The benefit must come from
exposing missing observations, invalidating a recovery table after an
interface change, or checking complete branch coverage. Simpler testing or
runtime mechanisms may be sufficient for a particular connector.
