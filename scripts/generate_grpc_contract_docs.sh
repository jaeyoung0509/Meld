#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo run -p openportio-rpc --bin grpc-docgen -- \
  --proto crates/openportio-rpc/proto/service.proto \
  --include crates/openportio-rpc/proto \
  --out-md docs/generated/grpc-contracts.md \
  --out-openapi docs/generated/grpc-openapi-bridge.json

mkdir -p crates/openportio-rpc/generated
cp docs/generated/grpc-contracts.md crates/openportio-rpc/generated/grpc-contracts.md
cp docs/generated/grpc-openapi-bridge.json crates/openportio-rpc/generated/grpc-openapi-bridge.json
