# FastAPI-Like Builder API

Meld exposes a fluent server builder for a compact startup flow.

## Quick Start

```rust
use std::net::SocketAddr;

use meld_server::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    MeldServer::new()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .run()
        .await?;
    Ok(())
}
```

`meld_server::prelude::*` includes:
- `MeldServer`
- `route` macro
- common validation extractors (`ValidatedJson`, `ValidatedQuery`, `ValidatedPath`, `ValidatedParts`)
- `Depends` DI extractor

## Common Customization Points

- `with_state(...)`: inject shared app state
- `with_rest_router(...)`: replace default REST router
- `with_grpc_service(...)`: add typed gRPC service
- `without_grpc()`: run REST-only mode
- `with_middleware_config(...)`: configure shared middleware
- `with_middleware(...)`: add custom router-level middleware
- `on_startup(...)` / `on_shutdown(...)`: attach lifecycle hooks

## DTO And Dependency Injection Pattern

You can model FastAPI-like DTOs with `serde` and inject shared dependencies using `State<Arc<AppState>>`.

```rust
use std::sync::Arc;

use meld_core::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct NotePath {
    id: String,
}

#[derive(Deserialize)]
struct NoteQuery {
    q: Option<String>,
    limit: Option<u32>,
}

#[derive(Deserialize)]
struct CreateNoteBody {
    title: String,
}

#[derive(Serialize)]
struct NoteResponse {
    id: String,
    title: String,
}

async fn get_note(
    State(state): State<Arc<AppState>>,
    Path(path): Path<NotePath>,
) -> Json<NoteResponse> {
    let title = state.greet(&path.id).unwrap_or_else(|_| "fallback".to_string());
    Json(NoteResponse { id: path.id, title })
}
```

See `/examples/simple-server/src/main.rs` for a runnable end-to-end example using these DTO styles.

## Validation And Error DTO Pattern

Meld now includes reusable REST validation helpers:

- `meld_server::api::ValidatedJson<T>`
- `meld_server::api::ValidatedQuery<T>`
- `meld_server::api::ApiErrorResponse`

Typical usage:

```rust
use meld_server::api::{ApiError, ValidatedJson};
use validator::Validate;

#[derive(serde::Deserialize, Validate)]
struct CreateNoteBody {
    #[validate(length(min = 2, max = 120))]
    title: String,
}

async fn create_note(
    ValidatedJson(body): ValidatedJson<CreateNoteBody>,
) -> Result<axum::Json<String>, ApiError> {
    Ok(axum::Json(body.title))
}
```

Validation failures return a structured `400` error JSON with:
- `code`
- `message`
- `details` (field-level messages)

OpenAPI wiring:
- shared error schema uses `ApiErrorResponse`
- REST path annotations can reference the same response body for `400/401/500`

## Auto-Validate Route Macro (FastAPI-Like DX)

For a more FastAPI-like handler style, use `#[meld_server::route(..., auto_validate)]`.

With `auto_validate`, handler arguments are rewritten at compile time:
- `Json<T>` -> `ValidatedJson<T>`
- `Query<T>` -> `ValidatedQuery<T>`
- `Path<T>` -> `ValidatedPath<T>`

Example:

```rust
use meld_server::api::ApiError;
use axum::Json;

#[meld_server::route(post, "/notes", auto_validate)]
async fn create_note(Json(body): Json<CreateNoteBody>) -> Result<Json<String>, ApiError> {
    Ok(Json(body.title))
}
```

If you omit `auto_validate`, behavior stays unchanged.

For header/cookie wrapper patterns, use `ValidatedParts<T>` with your custom parts extractor
that implements `Validate`.

Macro portability:
- `#[route(...)]` expansion is dependency-rename safe.
- Example compile coverage exists under `examples/renamed-meld-app`.

## SSE Endpoint Pattern

Meld supports Server-Sent Events (SSE) for lightweight one-way real-time updates.

```rust
use std::{convert::Infallible, time::Duration};

use axum::response::sse::{Event, KeepAlive, Sse};
use tokio_stream::{once, wrappers::IntervalStream, Stream, StreamExt};

async fn events() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let initial = once(Ok(Event::default().event("heartbeat").data("ready")));
    let mut sequence = 0u64;
    let ticks = IntervalStream::new(tokio::time::interval(Duration::from_secs(2)))
        .map(move |_| {
            sequence += 1;
            Ok(Event::default()
                .event("message")
                .data(format!("tick-{sequence}")))
        });

    Sse::new(initial.chain(ticks)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}
```

Client reconnect guidance:
- Use automatic reconnect with exponential backoff (for example 1s, 2s, 4s ... capped at 30s).
- Add small jitter to avoid synchronized reconnect spikes.
- Resume with `Last-Event-ID` when your client stack supports it.

## WebSocket Endpoint Pattern

Meld supports WebSocket upgrade handlers for bidirectional realtime flows.

```rust
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures_util::SinkExt;

async fn ws_echo(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
    ws.max_message_size(4096).on_upgrade(handle_ws)
}

async fn handle_ws(mut socket: WebSocket) {
    while let Some(Ok(Message::Text(text))) = socket.recv().await {
        let _ = socket.send(Message::Text(format!("echo: {text}"))).await;
    }
}
```

Server defaults in `meld-server`:
- max text frame bytes: `MELD_WS_MAX_TEXT_BYTES` (default `4096`)
- idle timeout seconds: `MELD_WS_IDLE_TIMEOUT_SECS` (default `45`)

## OAuth2/OIDC JWT Auth Pattern

Meld includes shared JWT claim validation used by both REST and gRPC layers.

Environment configuration:
- `MELD_AUTH_ENABLED=true`
- `MELD_AUTH_JWT_SECRET=<hmac-secret>`
- optional: `MELD_AUTH_ISSUER=<issuer>`
- optional: `MELD_AUTH_AUDIENCE=<audience>`

When enabled:
- REST protected route example: `GET /protected/whoami` (Bearer token required)
- gRPC interceptor validates `authorization: Bearer <token>` metadata

When disabled:
- existing baseline routes continue without auth enforcement.

## Depends-Like Extractor Pattern

For FastAPI `Depends(...)` style injection, use `meld_server::di::Depends<T>`.
`Depends<T>` resolves dependencies from state (`FromRef`) with request-scoped caching.

```rust
use std::sync::Arc;

use meld_core::AppState;
use meld_server::di::Depends;
use axum::extract::FromRef;

#[derive(Clone)]
struct ServiceInfo {
    service_name: String,
}

impl FromRef<Arc<AppState>> for ServiceInfo {
    fn from_ref(state: &Arc<AppState>) -> Self {
        Self {
            service_name: state.config.service_name.clone(),
        }
    }
}

async fn handler(Depends(info): Depends<ServiceInfo>) -> String {
    info.service_name
}
```

Test override helper:
- `meld_server::di::with_dependency_override(router, value)`

## Notes

- Default `MeldServer::new()` enables both REST and gRPC on a single listener.
- Default address uses `MELD_SERVER_ADDR` if set, otherwise `127.0.0.1:3000`.
