# AI-assisted engineering

AI agents assisted with hypothesis generation, implementation, and adversarial
review. Product constraints, finite semantics, trust boundaries, and acceptance
criteria were recorded before implementation. Public sources were used to
check external claims and identify close technical alternatives.

Suggestions were treated as hypotheses. Correctness-sensitive changes were
turned into test cases or explicit specification changes before being relied
on. Separate review roles challenged the problem choice, systems semantics,
security boundary, interface usefulness, and release claims. Agent review is
not an independent human audit or a user study.

Executable checks include an independently traversed artifact verifier,
separately expressed reference tests, adversarial inputs, generated cases, and
resource-limit checks. CI and release verification provide additional evidence
for the exact revision and platforms they run. A model's confidence is not a
release criterion.

Contingram's core requires no model provider, network connection, or API key.
Its synthetic examples do not establish production accuracy. Verification
claims remain bounded by the supplied model, horizon, and recorded checks.
Raw prompts and private engineering conversations are not part of the public
project.
