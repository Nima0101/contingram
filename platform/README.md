# Contingram control-plane reference

A separate Java 21 / Spring Boot service records authenticated invocation intents
and offline recovery recommendations. It never invokes external tools. The Rust
library and CLI retain their offline contract. This is a bounded development
reference, not a deployed service or an exactly-once external executor.

## Verification and local use

Requires Java 21, Maven 3.9+, Rust and a running Docker engine:

```sh
platform/scripts/verify-local.sh
```

Run from the repository root. The gate builds the actual Rust CLI and runs Java
format, compilation and integration tests against disposable PostgreSQL 17.6 and
Kafka 3.9.1 containers. It also builds the OCI image and boots it against local
Compose dependencies, checks an unauthenticated HTTP denial, and runs the bundled
Linux verifier demo. Docker is required; missing Docker fails the gate.
The dependency versions are pinned by the Maven parent/BOM and direct versions;
image tags identify tested versions but are not content-addressed supply-chain locks.

For an interactive local service, first run the gate, then:

```sh
docker compose -f platform/compose.yaml up -d
export DATABASE_URL=jdbc:postgresql://localhost:54329/contingram
export DATABASE_USER=contingram DATABASE_PASSWORD=local-development-only
export KAFKA_BOOTSTRAP_SERVERS=localhost:19092
export CONTINGRAM_BINARY="$PWD/target/debug/contingram"
export OIDC_ISSUER=https://your-issuer.example
export OIDC_PUBLIC_KEY=file:/absolute/path/to/issuer-public.pem
java -jar platform/target/control-plane-0.1.0-SNAPSHOT.jar
```

Supply a real issuer's RSA public key and matching RS256 access tokens. No test
signing key is shipped. Tokens require issuer, expiry, subject, audience
`contingram-platform`, and `scope` containing `invoke` or `admin`. Key rotation and
OIDC discovery are not implemented; replace the configured trust key and restart.
The integration suite generates an ephemeral issuer key and signs actual JWTs.

The OCI image consumes the locally verified jar and builds a Linux verifier:
`docker build -f platform/Dockerfile -t contingram-platform:local .`.
Compose provisions development dependencies only. Remove them with
`docker compose -f platform/compose.yaml down`. Neither command deploys a service.

## HTTP contract

All routes are under `/v1`; JSON unknown properties fail. Admin and invoke scopes
are distinct. Credentials are accepted only through the Authorization bearer header.
CSRF protection remains enabled for requests without bearer credentials: unsafe
requests missing CSRF tokens receive 403, while safe unauthenticated requests
receive 401. Cookies do not authenticate requests. Errors are non-success HTTP responses; clients must check status.

| Method/path | Body/result | Required scope |
| --- | --- | --- |
| POST `/admin/tools` | `{id,contract,report,risk}`; contract/report are JSON strings, risk 0–100 | admin |
| POST `/intents` | `{key,tool}` → `{id,tool,decision}` | invoke |
| POST `/intents/{id}/recovery` | observation string array → checked follower output | invoke, owner |
| GET `/intents/{id}/audit` | ordered `{seq,kind,detail}` array | invoke, owner |
| POST `/admin/replay/{event-id}` | replay held sequence gap | admin |

Registration is immutable; reuse of an ID returns conflict. The actual CLI checks
positive evidence before registration. Risk at most 30 admits an intent; higher
risk records a denial. This deterministic policy is deliberately small. Recovery
is denied for denied intents. Authorship of the finite model is an administrator
trust decision, not something proof checking establishes.

## Reliability contract and trust boundaries

PostgreSQL atomically commits one intent per `(subject,key)`, its audit entry and
outbox event. Matching retries return the durable result; changing the tool under
the same key conflicts. Concurrent intake is serialized by the unique constraint.
Tool ID binds immutable contract, report and risk. Recovery locks the intent row
and appends a new recommendation each time; it is not a side-effect execution API.

An outbox worker publishes to `contingram.events.v1` using intent UUID as Kafka
key, then marks publication in the same database transaction. A lost acknowledgment
can redeliver, so publication is at least once. Broker timeout leaves rows pending.
Projection processing locks per intent, deduplicates event UUIDs and requires the
next sequence. Gaps enter durable `inbox.state=dead`; admin replay succeeds only
after predecessors arrive. A lost event is recovered by outbox retry; permanent
broker data loss after a confirmed send needs operator reconciliation.

Events are UTF-8 JSON with exactly `version` (1), `id` and `intent` (UUID), `seq`
(positive integer), `kind` (`decision` or `recovery`), and `detail` (string).
Consumers reject unknown JSON fields/versions and require an exact authoritative
outbox match. This projection is local to this control plane, not a generic
cross-service consumer. Invalid transport messages retry without committing the
Kafka offset; operators must investigate poison records. Ordered gaps can be
replayed; malformed/forged messages cannot be authorized by replay.

An audit trigger rejects UPDATE, DELETE and TRUNCATE. The database owner can alter
that trigger; this is application-level append-only history, not tamper-proof
storage. Use separate migration/runtime database roles and least-privilege Kafka
ACLs before any exposure outside a trusted development network. TLS termination,
request/concurrency/rate limits, data retention and multi-tenant administration
are outside this reference. Local Compose uses plaintext development credentials.

Artifacts are bounded to 64 KiB each; observation prefixes to 32. Each observation
must match `[a-zA-Z0-9_][a-zA-Z0-9_-]{0,63}`; option-like or shell-like input is
rejected before starting the verifier. A configured
trusted CLI executable runs without a shell, in a private temporary directory,
with a five-second deadline. OS/process availability, the model author and the
verifier remain trust boundaries. OpenTelemetry emits recovery spans and decision
counters to logging exporters, without subject, payload or token attributes.
This is independent of the Rust CLI's disabled opt-in telemetry interface.

Tests cover signed-token rejection, scope and owner boundaries, immutable
registration, durable concurrent retries, conflicting keys, denied risk,
append-only audit, forged policy, impossible observations, event gaps, duplicate
and lost-ack delivery, real broker outage, and emitted SDK spans/metrics.
Cross-platform Java/container support, production capacity and external adoption
are not inferred from these local tests.
