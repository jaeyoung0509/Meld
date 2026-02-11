#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo run -p meld-server --bin openapi_docgen -- \
  --out docs/generated/rest-openapi.json

./scripts/generate_grpc_contract_docs.sh

python3 scripts/generate_contracts_bundle.py \
  --rest-openapi docs/generated/rest-openapi.json \
  --grpc-bridge docs/generated/grpc-openapi-bridge.json \
  --links contracts/links.toml \
  --out docs/generated/contracts-bundle.json
