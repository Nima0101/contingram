#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
cargo fmt --all -- --check
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --release --all-targets
cargo test --locked --doc
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
cargo run --locked -- demo
python3 scripts/public_gate.py
python3 scripts/public_gate.py --self-test
python3 scripts/package_consumer.py
if [ -x platform/scripts/verify-local.sh ]; then platform/scripts/verify-local.sh; fi
