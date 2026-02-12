#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[1/9] cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "[2/9] cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "[3/9] cargo check --workspace"
cargo check --workspace

echo "[4/9] cargo test --workspace"
cargo test --workspace

echo "[5/9] cargo test -p meld-server --test multiplexing -- --nocapture"
cargo test -p meld-server --test multiplexing -- --nocapture

echo "[6/9] scripts/check_contracts_bundle.sh"
./scripts/check_contracts_bundle.sh

echo "[7/9] cargo test -p meld-server openapi_json_is_available -- --nocapture"
cargo test -p meld-server openapi_json_is_available -- --nocapture

echo "[8/9] scripts/prod_preflight.sh"
MELD_PREFLIGHT_SECURE=true \
MELD_PREFLIGHT_BOOT_SERVER=true \
MELD_PREFLIGHT_BASE_URL=http://127.0.0.1:3000 \
MELD_AUTH_ENABLED=true \
MELD_AUTH_JWT_SECRET=local-dev-secret \
MELD_AUTH_ISSUER=https://issuer.local \
MELD_AUTH_AUDIENCE=meld-api \
MELD_CORS_ALLOW_ORIGINS=https://app.example.com \
MELD_TIMEOUT_SECONDS=15 \
MELD_REQUEST_BODY_LIMIT_BYTES=1048576 \
MELD_MAX_IN_FLIGHT_REQUESTS=1024 \
./scripts/prod_preflight.sh

echo "[9/9] scripts/release_dry_run.sh"
./scripts/release_dry_run.sh

if cargo audit -V >/dev/null 2>&1; then
  echo "[optional] cargo audit"
  cargo audit
else
  echo "[optional] cargo audit skipped (cargo-audit not installed)"
fi

echo "Local CI flow completed."
