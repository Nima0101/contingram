# Security policy

Contingram is a local, offline analyzer of finite recovery contracts. Its
security-critical surfaces are contract parsing and lowering, bounded search,
artifact verification, observation following, CLI file handling, and release
integrity. The detailed [threat model](docs/threat-model.md) describes the
assumptions and controls.

## Reporting

Report a vulnerability through the repository's
[private vulnerability reporting page](https://github.com/Nima0101/contingram/security/advisories/new).
Include the affected revision, platform, a minimal synthetic reproducer,
expected and actual behavior, and the impact. Do not include credentials,
private contracts, or live tool payloads. If private reporting is unavailable,
open an issue requesting a private contact channel without exploit details.

Security fixes target the latest release and development branch. Earlier
pre-1.0 versions do not have a separate maintenance commitment. No response
time or third-party audit is promised.

## Properties that must hold

- A checked policy covers every possible observation and reaches a safe goal
  within its declared horizon. A checked refutation excludes every action.
- Resource exhaustion never becomes a policy or impossibility claim.
- The checker derives its own beliefs and does not trust solver evidence
  about hidden state, branch coverage, counters, or validity.
- Hostile input remains bounded before expansion and during search, checking,
  and output. Core code executes no supplied scripts, plugins, or tools.
- A model mismatch returns an error rather than an unchecked next action.
- Telemetry is off by default and obeys explicit disable and Do Not Track
  preferences. v0.1 has no endpoint, transport, spool, or collection, including
  when opt-in is requested. See [telemetry](docs/telemetry.md).

Incorrect acceptance of a forged artifact, a quantifier or lowering error
that changes the result, a reachable resource-limit bypass, input disclosure,
unintended code execution, or unexpected telemetry collection is reportable.
Impact depends on the reachable path and affected guarantee; there is no
blanket severity exemption for a local CLI.

## Trust and limitations

A verified result is relative to the supplied finite model. Deliberately
omitting a real API failure can make an inaccurate model provable; this is
an abstraction limitation, not a proof about the real service. A defect in
implementing the declared semantics remains reportable.

The parser and compiler are shared by search and verification. Rust, the OS,
and dependencies remain trusted. SHA-256 binds model semantics and does not
authenticate an author. Reports intentionally contain modeled labels, so use
synthetic labels and avoid secrets in contracts. `follow` executes no actions
and confers no authorization to execute them elsewhere.

Development dependency tools and GitHub release infrastructure can access
the network; that is separate from the shipped CLI's offline behavior.
