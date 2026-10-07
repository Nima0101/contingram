# Verification and evidence contract

Every material claim introduced by this expansion must be reproducible.

For each phase, record the commit SHA, exact command, exit code and relevant environment/runtime. A passing unit test is evidence for the tested unit only; it is not evidence of production deployment, scalability or external adoption.

Failure-handling features require a negative control that demonstrably fails before or under the unsafe condition and passes only after the intended behavior is present. Idempotency claims require duplicate/redelivery tests. Recovery claims require interruption/fault tests. Authorization claims require denied-path tests.

Hosted CI is required for cross-platform claims. Local macOS success cannot prove Windows or Linux behavior. Docker Compose, kind/minikube and Terraform validation are local/infrastructure validation only, not live deployment evidence.

Never modify expected outputs, fixtures, thresholds or tests solely to match buggy behavior. If an existing test is wrong, document the independent reason and add a regression demonstrating the correction.

PR text and README wording must use the status vocabulary from AGENTS.md.
