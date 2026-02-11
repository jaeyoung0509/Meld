#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[1/7] cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "[2/7] cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "[3/7] cargo check --workspace"
cargo check --workspace

echo "[4/7] cargo test --workspace"
cargo test --workspace

echo "[5/7] cargo test -p meld-server --test multiplexing -- --nocapture"
cargo test -p meld-server --test multiplexing -- --nocapture

echo "[6/7] scripts/check_grpc_contract_docs.sh"
./scripts/check_grpc_contract_docs.sh

echo "[7/7] cargo test -p meld-server openapi_json_is_available -- --nocapture"
cargo test -p meld-server openapi_json_is_available -- --nocapture

if cargo audit -V >/dev/null 2>&1; then
  echo "[optional] cargo audit"
  cargo audit
else
  echo "[optional] cargo audit skipped (cargo-audit not installed)"
fi

echo "Local CI flow completed."
