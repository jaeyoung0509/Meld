use std::sync::Arc;

use alloy_core::{AlloyError, AppState};
use alloy_rpc::{build_hello_response, HelloRequest};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Router,
};

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/hello/:name", get(hello))
        .with_state(state)
}

async fn root(State(state): State<Arc<AppState>>) -> String {
    format!(
        "{} ({})",
        state.config.service_name, state.config.environment
    )
}

async fn health(State(state): State<Arc<AppState>>) -> &'static str {
    state.metrics.incr_counter("http.health.requests");
    "OK"
}

async fn hello(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<String, (StatusCode, String)> {
    let response = build_hello_response(&state, HelloRequest { name }).map_err(map_error)?;
    Ok(response.message)
}

fn map_error(err: AlloyError) -> (StatusCode, String) {
    match err {
        AlloyError::Validation(message) => (StatusCode::BAD_REQUEST, message),
        AlloyError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}
