# Frozen held-out contracts

An independent fixture author produced these nine original synthetic contracts and their expected horizon results from manual reasoning before inspecting or running the solver. `expectations.json` preserves those verdicts and explanations; `metrics.json` preserves counts obtained by separate direct predicate evaluation and finite outcome enumeration. The JSON files were copied unchanged into this public corpus. They are algorithm test fixtures, not measured service behavior or training examples.

`author_fixtures.py` reproducibly writes the nine contracts and expectations with Python's standard library; it contains no policy search. It does not regenerate the independently recorded metrics. `tests/heldout.rs` checks every frozen expectation and the world/initial/edge counts, then verifies each synthesized certificate independently.

The paired changes are:

- Stale receipts: refresh that must become current versus refresh that may stay stale forever.
- Cancellation: acknowledgement versus a fence that settles or excludes pending effects.
- Partial completion: object existence versus per-stage observations.
- Identity: retained deduplication, expiry during retry, and an authoritative ledger that survives expiry.

No policy is guaranteed against real tools by these synthetic results. Actual tool contracts require external validation, especially for races, observation freshness, operation identity and expiration.
