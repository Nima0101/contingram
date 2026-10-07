# Fuzzing

Install `cargo-fuzz 0.13.2` and Rust `nightly-2026-10-01`, then run from the repository root:

```sh
cargo +nightly-2026-10-01 fuzz run contract -- -max_total_time=60 -max_len=65536 -rss_limit_mb=2048
cargo +nightly-2026-10-01 fuzz run report -- -max_total_time=60 -max_len=65536 -rss_limit_mb=2048
```

The contract target parses, lowers, synthesizes with a 128-node/10,000-work cap,
round-trips the report, and independently checks completed evidence. The report
target subjects the independent verifier to arbitrary reports bound to the receipt
model. Both targets use AddressSanitizer through cargo-fuzz. CI seeds contracts
from the synthetic corpus and reports from positive, negative, and UNKNOWN cases.
The 64 KiB fuzz input cap is a smoke-test budget; integration tests separately
exercise the public 1 MiB and 8 MiB limits. These bounded runs do not prove absence
of bugs. Preserve any crashes as regression tests before continuing fuzzing.
