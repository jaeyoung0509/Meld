#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[1/10] cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "[2/10] cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "[3/10] cargo check --workspace"
cargo check --workspace

echo "[4/10] cargo test --workspace"
cargo test --workspace

echo "[5/10] cargo test -p production-api -- --nocapture"
cargo test -p production-api -- --nocapture

echo "[6/10] cargo test -p openportio-server --test multiplexing -- --nocapture"
cargo test -p openportio-server --test multiplexing -- --nocapture

echo "[7/10] scripts/check_contracts_bundle.sh"
./scripts/check_contracts_bundle.sh

echo "[8/10] cargo test -p openportio-server openapi_json_is_available -- --nocapture"
cargo test -p openportio-server openapi_json_is_available -- --nocapture

echo "[9/10] scripts/prod_preflight.sh"
OPENPORTIO_PREFLIGHT_SECURE=true \
OPENPORTIO_PREFLIGHT_BOOT_SERVER=true \
OPENPORTIO_PREFLIGHT_WAIT_SECONDS=120 \
OPENPORTIO_PREFLIGHT_BASE_URL=http://127.0.0.1:3000 \
OPENPORTIO_AUTH_ENABLED=true \
OPENPORTIO_AUTH_JWT_SECRET=local-dev-secret \
OPENPORTIO_AUTH_ISSUER=https://issuer.local \
OPENPORTIO_AUTH_AUDIENCE=openportio-api \
OPENPORTIO_CORS_ALLOW_ORIGINS=https://app.example.com \
OPENPORTIO_TIMEOUT_SECONDS=15 \
OPENPORTIO_REQUEST_BODY_LIMIT_BYTES=1048576 \
OPENPORTIO_MAX_IN_FLIGHT_REQUESTS=1024 \
./scripts/prod_preflight.sh

echo "[10/10] scripts/release_dry_run.sh"
./scripts/release_dry_run.sh

if cargo audit -V >/dev/null 2>&1; then
  echo "[optional] cargo audit"
  cargo audit
else
  echo "[optional] cargo audit skipped (cargo-audit not installed)"
fi

echo "Local CI flow completed."
