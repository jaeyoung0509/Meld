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

## Notes

- Default `AlloyServer::new()` enables both REST and gRPC on a single listener.
- Default address uses `ALLOY_SERVER_ADDR` if set, otherwise `127.0.0.1:3000`.
