use std::sync::Arc;

use alloy_core::AppState;
use axum::{routing::get, Router};

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(|| async { "Alloy server" }))
        .route("/health", get(|| async { "OK" }))
        .with_state(state)
}
