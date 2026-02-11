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

## Notes

- Default `AlloyServer::new()` enables both REST and gRPC on a single listener.
- Default address uses `ALLOY_SERVER_ADDR` if set, otherwise `127.0.0.1:3000`.
