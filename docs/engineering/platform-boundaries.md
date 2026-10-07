# HTTP authentication and verifier process boundary

The service accepts credentials only through the Authorization bearer header.
Ambient browser cookies must not authorize a mutation. Keep Spring Security's
CSRF filter enabled: resource-server support exempts resolved bearer requests,
while an unsafe request without a bearer credential remains CSRF-protected.
Missing credentials on safe requests return 401; unsafe requests without a CSRF
token return 403. Invalid signed/header credentials still fail authentication.

The executable path is trusted deployment configuration. Contract/report contents
are private temporary files, never command fragments. Observations are untrusted
HTTP input and must be checked individually before entering the argument list:
1–64 ASCII letters/digits/underscores/hyphens, with no leading hyphen. The service
accepts at most 32 observations. No shell is involved. The core CLI consumes
observations positionally, but the service also rejects option-like input so a
future CLI option parser cannot expand the service's authority. The core's public
CLI contract is unchanged.

Final PR CodeQL analysis identified the original global CSRF disable and a tainted
collection flow into ProcessBuilder. The fix preserves the scanner and its query
set, keeps CSRF defaults, and moves allowlist checks onto each value used by the
process. Tests cover valid bearer mutations, forged credentials, cookie-only
mutations, safe unauthenticated requests, and rejected process arguments. A
missing executable distinguishes accepted observation syntax from rejection
before process creation. Hosted analysis remains required evidence.
