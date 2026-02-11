use std::sync::Arc;

use alloy_core::{AlloyError, AppState};
use alloy_rpc::{
    build_hello_response, grpc_contract_docs_markdown, grpc_contract_openapi_bridge_json,
    HelloRequest,
};
use axum::{
    extract::{Path, State},
    http::header,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use tonic::service::Routes;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod grpc;
pub mod middleware;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RootResponse {
    pub service_name: String,
    pub environment: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HelloRestResponse {
    pub message: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(root, health, hello),
    components(schemas(RootResponse, HealthResponse, HelloRestResponse)),
    tags(
        (name = "rest", description = "Alloy REST endpoints")
    )
)]
struct ApiDoc;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/hello/:name", get(hello))
        .route("/grpc/contracts", get(grpc_contracts_markdown))
        .route("/grpc/contracts/openapi.json", get(grpc_contracts_openapi_bridge))
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}

pub fn build_multiplexed_router(state: Arc<AppState>) -> Router {
    let rest = build_router(state.clone());
    let grpc = Routes::new(grpc::build_grpc_service(state))
        .prepare()
        .into_axum_router();

    rest.merge(grpc)
}

#[utoipa::path(
    get,
    path = "/",
    tag = "rest",
    responses(
        (status = 200, description = "Root endpoint", body = RootResponse)
    )
)]
async fn root(State(state): State<Arc<AppState>>) -> Json<RootResponse> {
    Json(RootResponse {
        service_name: state.config.service_name.clone(),
        environment: state.config.environment.clone(),
    })
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "rest",
    responses(
        (status = 200, description = "Health status", body = HealthResponse)
    )
)]
async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    state.metrics.incr_counter("http.health.requests");
    Json(HealthResponse {
        status: "OK".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/hello/{name}",
    tag = "rest",
    params(
        ("name" = String, Path, description = "Name to greet")
    ),
    responses(
        (status = 200, description = "Hello response", body = HelloRestResponse),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal error")
    )
)]
async fn hello(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<HelloRestResponse>, (StatusCode, String)> {
    let response = build_hello_response(&state, HelloRequest { name }).map_err(map_error)?;
    Ok(Json(HelloRestResponse {
        message: response.message,
    }))
}

async fn grpc_contracts_markdown() -> ([(header::HeaderName, &'static str); 1], &'static str) {
    (
        [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
        grpc_contract_docs_markdown(),
    )
}

async fn grpc_contracts_openapi_bridge() -> Result<Json<Value>, (StatusCode, String)> {
    serde_json::from_str(grpc_contract_openapi_bridge_json())
        .map(Json)
        .map_err(|err| {
            tracing::error!(error = %err, "failed to parse generated grpc openapi bridge json");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
            )
        })
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
    use axum::body::to_bytes;
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

    #[tokio::test]
    async fn openapi_json_is_available() {
        let app = build_router(Arc::new(AppState::local("test-server")));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/openapi.json")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body_text = String::from_utf8(bytes.to_vec()).expect("valid utf8");
        assert!(body_text.contains("/health"));
        assert!(body_text.contains("/hello/{name}"));
    }

    #[tokio::test]
    async fn grpc_contract_docs_are_available() {
        let app = build_router(Arc::new(AppState::local("test-server")));
        let markdown_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/grpc/contracts")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("markdown request should succeed");

        assert_eq!(markdown_response.status(), StatusCode::OK);
        let markdown_bytes = to_bytes(markdown_response.into_body(), usize::MAX)
            .await
            .expect("markdown body bytes");
        let markdown_text = String::from_utf8(markdown_bytes.to_vec()).expect("valid markdown");
        assert!(markdown_text.contains("gRPC Contract Documentation"));

        let json_response = app
            .oneshot(
                Request::builder()
                    .uri("/grpc/contracts/openapi.json")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("json request should succeed");
        assert_eq!(json_response.status(), StatusCode::OK);

        let json_bytes = to_bytes(json_response.into_body(), usize::MAX)
            .await
            .expect("json body bytes");
        let json_text = String::from_utf8(json_bytes.to_vec()).expect("valid json text");
        assert!(json_text.contains("/alloy.v1.Greeter/SayHello"));
    }
}
