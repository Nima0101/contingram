# Agent execution contract

This repository is part of the prepared open-source employment expansion workspace.

## Source of truth
Code, executable tests, build artifacts and provider/runtime evidence outrank prose. Documentation must never claim more than the strongest available evidence.

## Hard rules
- Do not delete or weaken existing functionality to make the new work easier.
- Do not skip, mute, xfail, ignore or loosen tests/lints/security checks to get green.
- Do not use fabricated results, fake integrations or documentation-only implementations.
- Do not spawn AI sub-agents or nested model sessions.
- Do not commit known-red code. Run `scripts/verify-local.sh` before commit.
- Do not push to main, force-push, merge, deploy or release.
- New reliability/security behavior needs positive tests and at least one negative control.
- Keep unsupported behavior explicit and fail closed.
- Record exact commands and limitations for verification claims.
- Synthetic benchmarks/load are labelled synthetic and never presented as production scale.
- Preserve public contracts unless the change is intentional, documented and tested.

## Status vocabulary
- Implemented: code path exists.
- Locally verified: repository gate passed on a named commit/worktree.
- CI verified: required hosted checks passed for that commit.
- Release verified: published artifact and release checks passed.
- Externally adopted: independent usage evidence exists.

Never substitute a stronger status for a weaker one.
