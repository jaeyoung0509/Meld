#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[1/5] cargo check --workspace"
cargo check --workspace

echo "[2/5] cargo test --workspace"
cargo test --workspace

echo "[3/5] cargo test -p alloy-server --test multiplexing -- --nocapture"
cargo test -p alloy-server --test multiplexing -- --nocapture

echo "[4/5] scripts/check_grpc_contract_docs.sh"
./scripts/check_grpc_contract_docs.sh

echo "[5/5] cargo test -p alloy-server openapi_json_is_available -- --nocapture"
cargo test -p alloy-server openapi_json_is_available -- --nocapture

echo "Local CI flow completed."
