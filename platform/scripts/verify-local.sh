#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
command -v java
command -v mvn
docker info --format '{{.ServerVersion}}'
cargo build --locked --manifest-path ../Cargo.toml
export CONTINGRAM_BINARY="$(cd .. && pwd)/target/debug/contingram"
mvn --batch-mode verify
docker compose -f compose.yaml config --quiet
(cd .. && docker build -f platform/Dockerfile -t contingram-platform:local .)
python3 scripts/container_smoke.py
