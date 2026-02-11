#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo run -p meld-rpc --bin grpc-docgen -- \
  --proto crates/meld-rpc/proto/service.proto \
  --include crates/meld-rpc/proto \
  --out-md docs/generated/grpc-contracts.md \
  --out-openapi docs/generated/grpc-openapi-bridge.json
