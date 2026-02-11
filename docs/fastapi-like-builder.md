# FastAPI-Like Builder API

Alloy exposes a fluent server builder for a compact startup flow.

## Quick Start

```rust
use std::net::SocketAddr;

use alloy_server::AlloyServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    AlloyServer::new()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .run()
        .await?;
    Ok(())
}
```

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

use alloy_core::AppState;
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

Alloy now includes reusable REST validation helpers:

- `alloy_server::api::ValidatedJson<T>`
- `alloy_server::api::ValidatedQuery<T>`
- `alloy_server::api::ApiErrorResponse`

Typical usage:

```rust
use alloy_server::api::{ApiError, ValidatedJson};
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

## Auto-Validate Route Macro (FastAPI-Like DX)

For a more FastAPI-like handler style, use `#[alloy_server::route(..., auto_validate)]`.

With `auto_validate`, handler arguments are rewritten at compile time:
- `Json<T>` -> `ValidatedJson<T>`
- `Query<T>` -> `ValidatedQuery<T>`

Example:

```rust
use alloy_server::api::ApiError;
use axum::Json;

#[alloy_server::route(post, "/notes", auto_validate)]
async fn create_note(Json(body): Json<CreateNoteBody>) -> Result<Json<String>, ApiError> {
    Ok(Json(body.title))
}
```

If you omit `auto_validate`, behavior stays unchanged.

## SSE Endpoint Pattern

Alloy supports Server-Sent Events (SSE) for lightweight one-way real-time updates.

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

## Depends-Like Extractor Pattern

For FastAPI `Depends(...)` style injection, define a custom extractor via `FromRequestParts`.
See `RequestContext` in `/examples/simple-server/src/main.rs` for a practical pattern.

## Notes

- Default `AlloyServer::new()` enables both REST and gRPC on a single listener.
- Default address uses `ALLOY_SERVER_ADDR` if set, otherwise `127.0.0.1:3000`.
