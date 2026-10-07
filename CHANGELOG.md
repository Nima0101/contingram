# Changelog

## Unreleased

- Add an optional Java 21 control-plane reference with JWT authorization, PostgreSQL migrations and transactional intent/audit/outbox state, Kafka projection/replay, and OpenTelemetry recovery instrumentation. The offline Rust verifier remains unchanged and never executes tools.
- Exercise authorization, concurrent duplicate intake, event gaps/redelivery, broker outage and checked recovery with real containers; build and smoke-test a local OCI image.
- Document the separate service trust boundary and unreleased, non-production scope.

## 0.1.0

Initial release scope:

- Data-only finite tool contracts with explicit effects and visible outcomes.
- Deterministic bounded policy synthesis, bounded no-policy evidence, and
  resource UNKNOWN.
- Independently traversed verification of positive and negative artifacts;
  canonical model binding and offline policy following.
- CLI and Rust library, synthetic recovery examples, manual-policy checks,
  reference and adversarial tests.
- Apache-2.0 license, attribution and trademark guidance, and a closed opt-in
  telemetry interface with collection disabled because no endpoint is configured.

This entry describes functionality. Platform and release-integrity claims
depend on the checks and assets attached to the published release.
