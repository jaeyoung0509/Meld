#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RPC_PATCH_ARGS=(
  --config "patch.crates-io.meld-core.path=\"crates/meld-core\""
)

SERVER_PATCH_ARGS=(
  --config "patch.crates-io.meld-core.path=\"crates/meld-core\""
  --config "patch.crates-io.meld-macros.path=\"crates/meld-macros\""
  --config "patch.crates-io.meld-rpc.path=\"crates/meld-rpc\""
)

echo "[1/4] cargo publish --dry-run -p meld-core --allow-dirty"
cargo publish --dry-run -p meld-core --allow-dirty

echo "[2/4] cargo publish --dry-run -p meld-macros --allow-dirty"
cargo publish --dry-run -p meld-macros --allow-dirty

echo "[3/4] cargo publish --dry-run -p meld-rpc --allow-dirty"
cargo publish --dry-run -p meld-rpc --allow-dirty "${RPC_PATCH_ARGS[@]}"

echo "[4/4] cargo publish --dry-run -p meld-server --allow-dirty"
cargo publish --dry-run -p meld-server --allow-dirty "${SERVER_PATCH_ARGS[@]}"

echo "Release readiness checks succeeded."
echo "Note: local crates.io patch overrides are used to validate publish order before first registry index propagation."
