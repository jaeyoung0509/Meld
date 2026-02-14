#!/usr/bin/env bash
set -euo pipefail

ADDR="${1:-127.0.0.1:3000}"
HOST="http://${ADDR}"

echo "[1/3] REST health check"
curl -sf "${HOST}/health" >/dev/null
echo "  OK: GET /health"

echo "[2/3] REST hello check"
HELLO="$(curl -sf "${HOST}/hello/Rust")"
echo "  OK: GET /hello/Rust -> ${HELLO}"

echo "[3/3] gRPC SayHello check"
if ! command -v grpcurl >/dev/null 2>&1; then
  echo "  SKIP: grpcurl is not installed. Install grpcurl to run gRPC CLI validation."
  exit 0
fi

grpcurl -plaintext \
  -import-path crates/openportio-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  "${ADDR}" openportio.v1.Greeter/SayHello

echo "Done: REST + gRPC are both reachable on ${ADDR}"
