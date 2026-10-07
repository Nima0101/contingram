# Employment-signal expansion

Contingram remains an offline bounded verifier at its core. The expansion adds a separate production-style control-plane vertical slice to demonstrate how verified recovery artifacts can participate in a real service architecture without letting the verifier execute external tools.

## Target evidence
The new platform surface should demonstrate Java/JVM backend engineering, PostgreSQL migrations, event-driven processing, idempotency, authorization, append-only audit, OpenTelemetry and failure recovery. Kafka/Redpanda-compatible event flows must include duplicate/out-of-order handling, DLQ/replay and explicit event schemas.

A Backstage/TypeScript surface is useful only when wired to real backend behavior. Kubernetes/Terraform assets are acceptable only with validation and with no claim that they are deployed.

## Boundary
The Rust verifier remains independently testable and offline. The platform may consume Contingram contracts/artifacts through a stable boundary, but must not silently turn the verifier into a side-effect executor.

## Done means
A reproducible local stack or integration harness exercises an authenticated invocation from intake through durable decision/audit/events and at least one Contingram-informed recovery path. Duplicate invocation must not duplicate durable side effects. Failure tests cover unauthorized access, event redelivery and replay. Existing Rust gates remain green.
