use std::sync::Arc;

use alloy_core::{AlloyError, AppState};
use alloy_rpc::{build_hello_response, HelloRequest};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Router,
};
use tonic::service::Routes;

pub mod grpc;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/hello/:name", get(hello))
        .with_state(state)
}

pub fn build_multiplexed_router(state: Arc<AppState>) -> Router {
    let rest = build_router(state.clone());
    let grpc = Routes::new(grpc::build_grpc_service(state))
        .prepare()
        .into_axum_router();

    rest.merge(grpc)
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn health_returns_ok() {
        let app = build_router(Arc::new(AppState::local("test-server")));
        let response = app
            .oneshot(Request::builder().uri("/health").body(axum::body::Body::empty()).unwrap())
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
