#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo run -p alloy-rpc --bin grpc-docgen -- \
  --proto crates/alloy-rpc/proto/service.proto \
  --include crates/alloy-rpc/proto \
  --out-md docs/generated/grpc-contracts.md \
  --out-openapi docs/generated/grpc-openapi-bridge.json
