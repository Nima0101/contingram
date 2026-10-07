# Telemetry and privacy

Contingram supports a telemetry/tracking interface for aggregate adoption and
release-quality feedback. **Telemetry is off by default. v0.1 collects and
sends no telemetry, even if you request opt-in.** No trustworthy first-party
collection endpoint is configured, and the release contains no network
transport, telemetry queue, or disk spool.

Consequently, this release cannot measure adoption or usage through telemetry.
The subsystem provides the consent and event boundary for future collection;
it is not evidence that any user or company has adopted Contingram.

## Inspect, disable, or request opt-in

```sh
contingram telemetry status
contingram telemetry explain
```

`status` exposes `opt_in_requested`, `do_not_track`, `collection_enabled`,
`endpoint`, `operator`, `retention`, and a reason. In v0.1, collection is always
false and endpoint/operator are null.

| Environment setting | Behavior |
| --- | --- |
| Unset `CONTINGRAM_TELEMETRY` | Off. |
| `CONTINGRAM_TELEMETRY=0` | Explicit kill switch. |
| `CONTINGRAM_TELEMETRY=1` | Request opt-in; v0.1 still reports `no_first_party_endpoint` and sends nothing. |
| Any other value | Off. |
| `DO_NOT_TRACK` or `DNT` nonempty and not `0` | Veto collection, overriding an opt-in request. |

There are no consent prompts or persisted preference files. Configure these
variables using your shell or process launcher. POSIX example:

```sh
CONTINGRAM_TELEMETRY=0 contingram demo
CONTINGRAM_TELEMETRY=1 contingram telemetry status
DO_NOT_TRACK=1 contingram telemetry status
```

## Closed event schema

The subsystem can construct the following coarse fields for a future approved
first-party transport. They are not transmitted in v0.1.

| Field | Allowed content |
| --- | --- |
| `schema_version` | Event format integer. |
| `version` | Compiled Contingram version. |
| `os_family` | Linux, macOS, Windows, or other. |
| `architecture` | x86_64, aarch64, or other. |
| `feature` | Analyze, verify, lower, explain, follow, or demo. |
| `outcome` | Policy found, bounded no-policy, unknown, success, or a coarse input/I/O/check/internal error category. |
| `duration` | Under 10 ms, under 100 ms, under 1 s, under 10 s, or at least 10 s. |

There are no arbitrary event properties. The schema excludes prompts, source
code, file contents or paths, tool requests or responses, contracts, artifacts,
model hashes or labels, diagnostic messages, secrets, tokens, usernames,
email addresses, IP addresses, hostnames, repository or project names,
advertising IDs, and persistent installation identifiers. No cross-project
fingerprint is created. No telemetry data is sold or shared.

No installation identifier is needed: aggregate command counts and version,
platform, feature, and error distributions can inform release work without
identifying a user or company. Without identifiers, event counts would not
measure unique users. This document does not describe network metadata as
“anonymous.”

## Operator, endpoint, and retention

For v0.1 there is **no telemetry operator or endpoint**, because no collection
service is configured. Application telemetry retention is **none**: no event
is transmitted or persisted. The project maintainer is
[Nima Khaki](https://github.com/Nima0101); this does not imply an operational
collection service.

Before collection can be enabled in a later release, that release must
document the first-party operator, exact endpoint, event retention and
deletion policy, and handling of incidental network metadata. Application
logic must not intentionally store IP addresses. Any transport must be
asynchronous, bounded, failure-isolated, opt-in, and subordinate to the kill
switch and Do Not Track. Delivery failures must never change command success,
policy semantics, canonical hashes, or certificate bytes.

The current `try_emit` interface immediately reports no submission. Unit and
CLI tests cover the off state, explicit opt-in without an endpoint, Do Not
Track precedence, and the closed payload schema. There is no background
network work to delay a command. See [security](../SECURITY.md) and the
[threat model](threat-model.md).
