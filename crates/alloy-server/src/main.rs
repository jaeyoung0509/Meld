use std::{env, net::SocketAddr, sync::Arc};

use alloy_core::AppState;
use alloy_server::build_multiplexed_router;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = Arc::new(AppState::local("alloy-server"));

    let app = build_multiplexed_router(state).layer(TraceLayer::new_for_http());
    let addr = load_addr_from_env()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(%addr, "alloy-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn load_addr_from_env() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    match env::var("ALLOY_SERVER_ADDR") {
        Ok(raw) => Ok(raw.parse()?),
        Err(env::VarError::NotPresent) => Ok(SocketAddr::from(([127, 0, 0, 1], 3000))),
        Err(err) => Err(Box::new(err)),
    }
}
