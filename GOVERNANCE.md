# Governance

[Nima Khaki](https://github.com/Nima0101) maintains Contingram and makes final
decisions about scope, compatibility, releases, and repository access.
Contributors are credited for accepted work. Maintainer status is earned
through sustained, constructive contributions and granted explicitly; a
contribution does not automatically confer release authority.

Technical decisions should be justified in issues or pull requests with a
concrete use case, stated semantic consequences, and executable evidence.
Changes to the finite semantics, certificate trust boundary, telemetry, or
license require explicit maintainer review and documentation. Disagreement
should focus on the model, tests, and tradeoffs; see the
[code of conduct](CODE_OF_CONDUCT.md).

Pre-1.0 interfaces may evolve, but format version changes and breaking behavior
must be recorded in the changelog. Releases are tied to reviewed source and
verification evidence. Tests, security checks, or required platform failures
are not bypassed to meet a release date.

The project is open source under [Apache-2.0](LICENSE). Forking rights remain
subject to that license, with project identity described in
[TRADEMARKS.md](TRADEMARKS.md). Telemetry is separate from attribution and
must follow the [privacy policy](docs/telemetry.md).
