use std::sync::Arc;

use alloy_core::AppState;
use alloy_server::build_router;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState::local("simple-server"));
    let app = build_router(state);
    let listener = TcpListener::bind(("127.0.0.1", 4000)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
