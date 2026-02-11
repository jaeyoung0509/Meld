use std::{net::SocketAddr, sync::Arc};

use alloy_core::AppState;
use alloy_server::build_router;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = Arc::new(AppState::local("alloy-server"));

    let app = build_router(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(%addr, "alloy-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
