#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RPC_PATCH_ARGS=(
  --config "patch.crates-io.openportio-core.path=\"crates/openportio-core\""
)

SERVER_PATCH_ARGS=(
  --config "patch.crates-io.openportio-core.path=\"crates/openportio-core\""
  --config "patch.crates-io.openportio-macros.path=\"crates/openportio-macros\""
  --config "patch.crates-io.openportio-rpc.path=\"crates/openportio-rpc\""
)

echo "[1/4] cargo publish --dry-run -p openportio-core --allow-dirty"
cargo publish --dry-run -p openportio-core --allow-dirty

echo "[2/4] cargo publish --dry-run -p openportio-macros --allow-dirty"
cargo publish --dry-run -p openportio-macros --allow-dirty

echo "[3/4] cargo publish --dry-run -p openportio-rpc --allow-dirty"
cargo publish --dry-run -p openportio-rpc --allow-dirty "${RPC_PATCH_ARGS[@]}"

echo "[4/4] cargo publish --dry-run -p openportio-server --allow-dirty"
cargo publish --dry-run -p openportio-server --allow-dirty "${SERVER_PATCH_ARGS[@]}"

echo "Release readiness checks succeeded."
echo "Note: local crates.io patch overrides are used to validate publish order before first registry index propagation."
