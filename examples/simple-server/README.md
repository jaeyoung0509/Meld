# simple-server

Runnable sample for FastAPI-like DTO validation, DI, SSE, WebSocket, and single-port REST+gRPC.

## Prerequisites

- `grpcurl` installed
- `python3` installed (for development token helper)

## Run

From repository root:

```bash
cargo run -p simple-server
```

`simple-server` binds to `127.0.0.1:4000`.

## REST Smoke Check

```bash
curl -s http://127.0.0.1:4000/health
curl -s http://127.0.0.1:4000/notes?limit=3
```

## gRPC Quickstart (No Auth)

```bash
grpcurl -plaintext 127.0.0.1:4000 list
grpcurl -plaintext \
  -import-path crates/meld-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:4000 \
  meld.v1.Greeter/SayHello
```

## gRPC Auth Flow

Restart server with auth enabled:

```bash
MELD_AUTH_ENABLED=true \
MELD_AUTH_JWT_SECRET=dev-secret \
MELD_AUTH_ISSUER=https://issuer.local \
MELD_AUTH_AUDIENCE=meld-api \
cargo run -p simple-server
```

Call without token (expected `UNAUTHENTICATED`):

```bash
grpcurl -plaintext \
  -import-path crates/meld-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:4000 \
  meld.v1.Greeter/SayHello
```

Generate a development token (dev-only helper):

```bash
TOKEN=$(python3 scripts/generate_dev_jwt.py \
  --secret dev-secret \
  --issuer https://issuer.local \
  --audience meld-api)
```

Call with token (expected success):

```bash
grpcurl -plaintext \
  -H "authorization: Bearer ${TOKEN}" \
  -import-path crates/meld-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:4000 \
  meld.v1.Greeter/SayHello
```

## Troubleshooting

- `grpcurl: command not found`
  - Install grpcurl, then re-run commands.
- `python3: command not found`
  - Install Python 3 or generate token with another JWT tool.
- `Code: Unauthenticated`
  - Verify `MELD_AUTH_JWT_SECRET`, `MELD_AUTH_ISSUER`, `MELD_AUTH_AUDIENCE` match token inputs.
- `Failed to process proto source files` / import errors
  - Run commands from repository root so `crates/meld-rpc/proto` and `service.proto` resolve correctly.
