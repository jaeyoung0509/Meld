#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

./scripts/generate_contracts_bundle.sh

git diff --exit-code -- \
  docs/generated/rest-openapi.json \
  docs/generated/grpc-contracts.md \
  docs/generated/grpc-openapi-bridge.json \
  docs/generated/contracts-bundle.json

echo "Contracts bundle artifacts are up to date."
