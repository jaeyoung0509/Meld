# Meld

![Meld Logo](docs/assets/meld-logo.svg)

Meld is a Rust server framework focused on **FastAPI-like developer ergonomics** with **single-port REST + gRPC** delivery.

## What You Get

- Single listener for REST (HTTP/1.1) and gRPC (HTTP/2)
- REST OpenAPI + Swagger UI:
  - `/openapi.json`
  - `/docs`
- gRPC contract bridge docs:
  - `/grpc/contracts` (rendered HTML)
  - `/grpc/contracts.md` (raw markdown)
  - `/grpc/contracts/openapi.json`
- Unified contract bundle artifact:
  - `docs/generated/contracts-bundle.json`
- REST SSE stream:
  - `/events`
- REST WebSocket echo:
  - `/ws`
- Optional REST auth-protected route:
  - `/protected/whoami`
- Fluent server builder API:
  - `MeldServer::new().with_...().run()`
- Single-attribute DTO macro:
  - `#[meld_server::dto]` for `Deserialize + Validate + ToSchema`
  - keep `utoipa` in your crate dependencies for schema derive expansion
- Depends-style DI extractor with request cache:
  - `meld_server::di::Depends<T>`
- Shared middleware stack:
  - tracing, request-id propagation, CORS, timeout, concurrency limit

## Quick Start (7 Minutes)

### 1) Start the default server

```bash
cargo run -p meld-server
```

Default address: `127.0.0.1:3000`  
Override with:

```bash
MELD_SERVER_ADDR=127.0.0.1:4000 cargo run -p meld-server
```

### 2) Verify REST

```bash
curl -s http://127.0.0.1:3000/health
curl -s http://127.0.0.1:3000/hello/Rust
curl -N http://127.0.0.1:3000/events
# ws check (requires websocat): websocat ws://127.0.0.1:3000/ws
```

WebSocket defaults:
- max text frame: `4096` bytes (`MELD_WS_MAX_TEXT_BYTES`)
- idle timeout: `45` seconds (`MELD_WS_IDLE_TIMEOUT_SECS`)

Middleware defaults:
- request timeout: `15` seconds (`MELD_TIMEOUT_SECONDS`)
- max in-flight requests: `1024` (`MELD_MAX_IN_FLIGHT_REQUESTS`)
- request body limit: `1048576` bytes (`MELD_REQUEST_BODY_LIMIT_BYTES`)
- CORS: disabled by default; set `MELD_CORS_ALLOW_ORIGINS` to a comma-separated allowlist (use `*` only when you intentionally want wildcard CORS)

Auth defaults:
- disabled by default (`MELD_AUTH_ENABLED=false`)
- when enabled, JWT HMAC secret required (`MELD_AUTH_JWT_SECRET`)
- optional issuer/audience checks:
  - `MELD_AUTH_ISSUER`
  - `MELD_AUTH_AUDIENCE`
- `/protected/whoami` behavior:
  - auth disabled: returns `200` with anonymous principal
  - auth enabled: requires bearer JWT and returns `401` when missing/invalid

### 3) Verify gRPC (auth disabled)

`grpcurl` is required for gRPC smoke tests.

```bash
grpcurl -plaintext 127.0.0.1:3000 list
grpcurl -plaintext \
  -import-path crates/meld-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:3000 \
  meld.v1.Greeter/SayHello
```

### 4) Verify gRPC (auth enabled)

Restart server with auth enabled:

```bash
MELD_AUTH_ENABLED=true \
MELD_AUTH_JWT_SECRET=dev-secret \
MELD_AUTH_ISSUER=https://issuer.local \
MELD_AUTH_AUDIENCE=meld-api \
cargo run -p meld-server
```

Call without token (expected: `UNAUTHENTICATED`):

```bash
grpcurl -plaintext \
  -import-path crates/meld-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:3000 \
  meld.v1.Greeter/SayHello
```

Generate a dev token (development only):

```bash
TOKEN=$(python3 scripts/generate_dev_jwt.py \
  --secret dev-secret \
  --issuer https://issuer.local \
  --audience meld-api)
```

Call with token (expected: success):

```bash
grpcurl -plaintext \
  -H "authorization: Bearer ${TOKEN}" \
  -import-path crates/meld-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:3000 \
  meld.v1.Greeter/SayHello
```

### 5) Open docs

- Swagger UI: [http://127.0.0.1:3000/docs](http://127.0.0.1:3000/docs)
- OpenAPI JSON: [http://127.0.0.1:3000/openapi.json](http://127.0.0.1:3000/openapi.json)
- gRPC contracts (rendered): [http://127.0.0.1:3000/grpc/contracts](http://127.0.0.1:3000/grpc/contracts)
- gRPC contracts (markdown): [http://127.0.0.1:3000/grpc/contracts.md](http://127.0.0.1:3000/grpc/contracts.md)
- gRPC OpenAPI bridge: [http://127.0.0.1:3000/grpc/contracts/openapi.json](http://127.0.0.1:3000/grpc/contracts/openapi.json)

## Repository Layout

```text
crates/meld-core     # domain, state, error model
crates/meld-rpc      # proto, tonic codegen, grpc-docgen tool
crates/meld-server   # REST + gRPC routing, middleware, builder API
contracts/           # explicit REST <-> gRPC mapping definitions
examples/production-api
examples/simple-server
examples/meld-app
docs/
scripts/
```

## FastAPI-Like Builder Example

```rust
use std::net::SocketAddr;

use meld_server::MeldServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    MeldServer::new()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .run()
        .await?;
    Ok(())
}
```

See:
- `docs/fastapi-like-builder.md`
- `docs/dx-scorecard.md`
- `examples/production-api/README.md`
- `examples/production-api/src/main.rs`
- `examples/simple-server/README.md`
- `examples/simple-server/src/main.rs`
- `examples/meld-app/src/main.rs` (dependency-rename-safe macro usage)

## Contract Artifact Generation

Generate artifacts:

```bash
./scripts/generate_contracts_bundle.sh
```

Check drift (used in CI):

```bash
./scripts/check_contracts_bundle.sh
```

This generates:
- `contracts/links.toml` (explicit REST <-> gRPC mapping source)
- `docs/generated/rest-openapi.json`
- `docs/generated/grpc-contracts.md`
- `docs/generated/grpc-openapi-bridge.json`
- `docs/generated/contracts-bundle.json`

## CI And Local Verification

Local equivalent of CI:

```bash
./scripts/ci_local.sh
```

This runs:
- `cargo check --workspace`
- `cargo test --workspace`
- REST+gRPC multiplexing integration test
- contract artifact drift check
- OpenAPI route check
- production preflight gate

Extended testing quality gate (nextest + coverage):

```bash
./scripts/test_quality.sh
```

See `docs/testing-toolchain.md` for setup and coverage artifacts.

## Production Readiness Docs

- `docs/production/deployment.md`
- `docs/production/security.md`
- `docs/production/observability.md`
- `docs/production/runbook.md`
- `docs/production/production-api-runbook.md`

Run production preflight locally:

```bash
MELD_PREFLIGHT_SECURE=true \
MELD_PREFLIGHT_BOOT_SERVER=true \
MELD_AUTH_ENABLED=true \
MELD_AUTH_JWT_SECRET=replace-me \
MELD_AUTH_ISSUER=https://issuer.local \
MELD_AUTH_AUDIENCE=meld-api \
MELD_CORS_ALLOW_ORIGINS=https://app.example.com \
./scripts/prod_preflight.sh
```

## Release Readiness

- `CHANGELOG.md`
- `docs/release/versioning.md`
- `docs/release/publish-runbook.md`
- `.github/release-template.md`

Validate publishability of release crates:

```bash
./scripts/release_dry_run.sh
```

The script performs `cargo publish --dry-run` for all release crates with temporary
local `patch.crates-io` overrides to validate publish order before first index propagation.

Automated release publishing is available via `.github/workflows/release.yml` and runs on `v*` tag pushes from `main`.

## Current Status

Core roadmap items are implemented:
- `#1` to `#10` complete
- `#19` descriptor-based gRPC docs generator complete

Follow-up improvements continue via GitHub issues on:
[jaeyoung0509/meld](https://github.com/jaeyoung0509/meld)
