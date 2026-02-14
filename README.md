# Openportio

![Openportio Logo](docs/assets/openportio-logo.svg)

Openportio is a Rust server framework focused on **FastAPI-like developer ergonomics** with **single-port REST + gRPC** delivery.

> Migration note: the project was renamed from `Meld` to `Openportio`. Runtime keeps `MELD_*` env aliases for backward compatibility, but new setups should use `OPENPORTIO_*`.

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
  - `OpenportioServer::new().with_...().run()`
- Single-attribute DTO macro:
  - `#[openportio_server::dto]` for `Deserialize + Validate + ToSchema`
  - keep `utoipa` in your crate dependencies for schema derive expansion
- Depends-style DI extractor with request cache:
  - `openportio_server::di::Depends<T>`
- Shared middleware stack:
  - tracing, request-id propagation, CORS, timeout, concurrency limit

## Quick Start (7 Minutes)

### 1) Start the default server

```bash
cargo run -p openportio-server
```

Default address: `127.0.0.1:3000`  
Override with:

```bash
OPENPORTIO_SERVER_ADDR=127.0.0.1:4000 cargo run -p openportio-server
```

### 2) Verify REST

```bash
curl -s http://127.0.0.1:3000/health
curl -s http://127.0.0.1:3000/hello/Rust
curl -N http://127.0.0.1:3000/events
# ws check (requires websocat): websocat ws://127.0.0.1:3000/ws
```

WebSocket defaults:
- max text frame: `4096` bytes (`OPENPORTIO_WS_MAX_TEXT_BYTES`)
- idle timeout: `45` seconds (`OPENPORTIO_WS_IDLE_TIMEOUT_SECS`)

Middleware defaults:
- request timeout: `15` seconds (`OPENPORTIO_TIMEOUT_SECONDS`)
- max in-flight requests: `1024` (`OPENPORTIO_MAX_IN_FLIGHT_REQUESTS`)
- request body limit: `1048576` bytes (`OPENPORTIO_REQUEST_BODY_LIMIT_BYTES`)
- CORS: disabled by default; set `OPENPORTIO_CORS_ALLOW_ORIGINS` to a comma-separated allowlist (use `*` only when you intentionally want wildcard CORS)

Auth defaults:
- disabled by default (`OPENPORTIO_AUTH_ENABLED=false`)
- when enabled, JWT HMAC secret required (`OPENPORTIO_AUTH_JWT_SECRET`)
- optional issuer/audience checks:
  - `OPENPORTIO_AUTH_ISSUER`
  - `OPENPORTIO_AUTH_AUDIENCE`
- `/protected/whoami` behavior:
  - auth disabled: returns `200` with anonymous principal
  - auth enabled: requires bearer JWT and returns `401` when missing/invalid
- compatibility aliases (deprecated): `MELD_AUTH_*`, `MELD_TIMEOUT_SECONDS`, `MELD_SERVER_ADDR`, and other `MELD_*` runtime keys are still accepted.

### 3) Verify gRPC (auth disabled)

`grpcurl` is required for gRPC smoke tests.

```bash
grpcurl -plaintext 127.0.0.1:3000 list
grpcurl -plaintext \
  -import-path crates/openportio-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:3000 \
  openportio.v1.Greeter/SayHello
```

### 4) Verify gRPC (auth enabled)

Restart server with auth enabled:

```bash
OPENPORTIO_AUTH_ENABLED=true \
OPENPORTIO_AUTH_JWT_SECRET=dev-secret \
OPENPORTIO_AUTH_ISSUER=https://issuer.local \
OPENPORTIO_AUTH_AUDIENCE=openportio-api \
cargo run -p openportio-server
```

Call without token (expected: `UNAUTHENTICATED`):

```bash
grpcurl -plaintext \
  -import-path crates/openportio-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:3000 \
  openportio.v1.Greeter/SayHello
```

Generate a dev token (development only):

```bash
TOKEN=$(python3 scripts/generate_dev_jwt.py \
  --secret dev-secret \
  --issuer https://issuer.local \
  --audience openportio-api)
```

Call with token (expected: success):

```bash
grpcurl -plaintext \
  -H "authorization: Bearer ${TOKEN}" \
  -import-path crates/openportio-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:3000 \
  openportio.v1.Greeter/SayHello
```

### 5) Open docs

- Swagger UI: [http://127.0.0.1:3000/docs](http://127.0.0.1:3000/docs)
- OpenAPI JSON: [http://127.0.0.1:3000/openapi.json](http://127.0.0.1:3000/openapi.json)
- gRPC contracts (rendered): [http://127.0.0.1:3000/grpc/contracts](http://127.0.0.1:3000/grpc/contracts)
- gRPC contracts (markdown): [http://127.0.0.1:3000/grpc/contracts.md](http://127.0.0.1:3000/grpc/contracts.md)
- gRPC OpenAPI bridge: [http://127.0.0.1:3000/grpc/contracts/openapi.json](http://127.0.0.1:3000/grpc/contracts/openapi.json)

## Repository Layout

```text
crates/openportio-core     # domain, state, error model
crates/openportio-rpc      # proto, tonic codegen, grpc-docgen tool
crates/openportio-server   # REST + gRPC routing, middleware, builder API
contracts/           # explicit REST <-> gRPC mapping definitions
examples/production-api
examples/simple-server
examples/openportio-app
docs/
scripts/
```

## FastAPI-Like Builder Example

```rust
use std::net::SocketAddr;

use openportio_server::OpenportioServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    OpenportioServer::new()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .run()
        .await?;
    Ok(())
}
```

### Raw Axum/Tonic Escape Hatches

```rust
use axum::{routing::get, Router};
use tonic::service::Routes;
use openportio_server::OpenportioServer;

OpenportioServer::new()
    .merge_raw_router(
        Router::new().route("/metrics", get(|| async { "metrics-ok" })),
    )
    .configure_tonic(|routes| {
        let grpc_router = routes
            .into_axum_router()
            .route("/grpc-hook", get(|| async { "grpc-hook-ok" }));
        Routes::from(grpc_router)
    })
    .run()
    .await?;
```

Ordering guarantees:
- base REST router is built first (`with_rest_router(...)` or default routes)
- `merge_raw_router(...)` routers are merged in call order
- gRPC routes are merged after REST/raw merges
- shared middleware + dependency overrides are applied after merge composition

Notes:
- `configure_tonic(...)` is route-level customization over `tonic::service::Routes`; it is not a full `tonic::transport::Server` builder replacement.
- If `without_grpc()` is set, `configure_tonic(...)` is a no-op.

See:
- `docs/fastapi-like-builder.md`
- `docs/dx-scorecard.md`
- `examples/production-api/README.md`
- `examples/production-api/src/main.rs`
- `examples/simple-server/README.md`
- `examples/simple-server/src/main.rs`
- `examples/openportio-app/src/main.rs` (dependency-rename-safe macro usage)

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

Performance regression smoke gate (REST + gRPC):

```bash
./scripts/perf_gate.sh
```

Manual CI workflow is available at `.github/workflows/perf.yml`.
See `docs/performance-gates.md` for thresholds, artifacts, and tuning.

## Production Readiness Docs

- `docs/production/deployment.md`
- `docs/production/security.md`
- `docs/production/observability.md`
- `docs/production/runbook.md`
- `docs/production/production-api-runbook.md`

Run production preflight locally:

```bash
OPENPORTIO_PREFLIGHT_SECURE=true \
OPENPORTIO_PREFLIGHT_BOOT_SERVER=true \
OPENPORTIO_AUTH_ENABLED=true \
OPENPORTIO_AUTH_JWT_SECRET=replace-me \
OPENPORTIO_AUTH_ISSUER=https://issuer.local \
OPENPORTIO_AUTH_AUDIENCE=openportio-api \
OPENPORTIO_CORS_ALLOW_ORIGINS=https://app.example.com \
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
[jaeyoung0509/Openportio](https://github.com/jaeyoung0509/Openportio)
