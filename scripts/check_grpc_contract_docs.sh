#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

./scripts/generate_grpc_contract_docs.sh

git diff --exit-code -- \
  docs/generated/grpc-contracts.md \
  docs/generated/grpc-openapi-bridge.json \
  crates/openportio-rpc/generated/grpc-contracts.md \
  crates/openportio-rpc/generated/grpc-openapi-bridge.json

echo "gRPC contract docs are up to date."
