extern crate self as alloy_server;
use std::{convert::Infallible, env, sync::Arc, time::Duration};

use alloy_core::{AlloyError, AppState};
use alloy_rpc::{
    build_hello_response, grpc_contract_docs_markdown, grpc_contract_openapi_bridge_json,
    HelloRequest,
};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Extension, Path, State},
    http::header,
    http::StatusCode,
    middleware::from_fn_with_state,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use tokio_stream::{once, wrappers::IntervalStream, Stream, StreamExt};
use tonic::service::Routes;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod api;
pub mod auth;
pub mod builder;
pub mod di;
pub mod grpc;
pub mod middleware;
use crate::api::ApiErrorResponse;
pub use alloy_macros::route;
pub use builder::AlloyServer;

pub mod prelude {
    pub use crate::api::{
        ApiError, ApiErrorResponse, ValidatedJson, ValidatedParts, ValidatedPath, ValidatedQuery,
    };
    pub use crate::di::{with_dependency_override, Depends};
    pub use crate::route;
    pub use crate::AlloyServer;
}

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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProtectedWhoAmIResponse {
    pub subject: String,
    pub issuer: Option<String>,
}

#[derive(Debug, Serialize)]
struct ServerSentEventPayload {
    sequence: u64,
    kind: &'static str,
    service_name: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(root, health, hello, protected_whoami),
    components(schemas(
        RootResponse,
        HealthResponse,
        HelloRestResponse,
        ProtectedWhoAmIResponse,
        ApiErrorResponse
    )),
    tags(
        (name = "rest", description = "Alloy REST endpoints")
    )
)]
struct ApiDoc;

pub fn build_router(state: Arc<AppState>) -> Router {
    build_router_with_auth(state, auth::AuthRuntimeConfig::from_env())
}

pub fn build_router_with_auth(state: Arc<AppState>, auth_cfg: auth::AuthRuntimeConfig) -> Router {
    let protected = Router::new()
        .route("/protected/whoami", get(protected_whoami))
        .route_layer(from_fn_with_state(auth_cfg, auth::rest_auth_middleware));

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/hello/:name", get(hello))
        .route("/events", get(events))
        .route("/ws", get(ws_handler))
        .merge(protected)
        .route("/grpc/contracts", get(grpc_contracts_markdown))
        .route(
            "/grpc/contracts/openapi.json",
            get(grpc_contracts_openapi_bridge),
        )
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}

pub fn build_multiplexed_router(state: Arc<AppState>) -> Router {
    build_multiplexed_router_with_auth(state, auth::AuthRuntimeConfig::from_env())
}

pub fn build_multiplexed_router_with_auth(
    state: Arc<AppState>,
    auth_cfg: auth::AuthRuntimeConfig,
) -> Router {
    let rest = build_router_with_auth(state.clone(), auth_cfg.clone());
    let grpc = Routes::new(grpc::build_grpc_service_with_auth(state, auth_cfg))
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
        (status = 400, description = "Validation error", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
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

#[utoipa::path(
    get,
    path = "/protected/whoami",
    tag = "rest",
    responses(
        (status = 200, description = "Current principal (authenticated user or anonymous when auth is disabled)", body = ProtectedWhoAmIResponse),
        (status = 401, description = "Unauthorized (auth enabled and token missing/invalid)", body = ApiErrorResponse)
    )
)]
async fn protected_whoami(
    Extension(principal): Extension<alloy_core::auth::AuthPrincipal>,
) -> Json<ProtectedWhoAmIResponse> {
    Json(ProtectedWhoAmIResponse {
        subject: principal.subject,
        issuer: principal.issuer,
    })
}

async fn events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = sse_event_stream(state.config.service_name.clone());
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}

const WS_DEFAULT_MAX_TEXT_BYTES: usize = 4 * 1024;
const WS_DEFAULT_IDLE_TIMEOUT_SECS: u64 = 45;

#[derive(Clone, Copy)]
struct WsRuntimeConfig {
    max_text_bytes: usize,
    idle_timeout: Duration,
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
    let cfg = ws_runtime_config();
    ws.max_message_size(cfg.max_text_bytes)
        .on_upgrade(move |socket| handle_ws_session(socket, cfg))
}

async fn handle_ws_session(mut socket: WebSocket, cfg: WsRuntimeConfig) {
    tracing::info!("websocket connection opened");
    loop {
        let next_message = tokio::time::timeout(cfg.idle_timeout, socket.recv()).await;
        let Some(result) = (match next_message {
            Ok(result) => result,
            Err(_) => {
                tracing::info!("websocket idle timeout reached, closing connection");
                let _ = socket.close().await;
                return;
            }
        }) else {
            tracing::info!("websocket connection closed by client");
            return;
        };

        match result {
            Ok(Message::Text(text)) => {
                if text.len() > cfg.max_text_bytes {
                    let _ = socket.send(Message::Close(None)).await;
                    return;
                }
                if socket
                    .send(Message::Text(format!("echo: {text}")))
                    .await
                    .is_err()
                {
                    tracing::warn!("failed to send websocket text frame");
                    return;
                }
            }
            Ok(Message::Ping(payload)) => {
                if socket.send(Message::Pong(payload)).await.is_err() {
                    tracing::warn!("failed to send websocket pong");
                    return;
                }
            }
            Ok(Message::Close(_)) => {
                let _ = socket.close().await;
                return;
            }
            Ok(_) => {}
            Err(err) => {
                tracing::warn!(error = %err, "websocket receive error");
                return;
            }
        }
    }
}

fn ws_runtime_config() -> WsRuntimeConfig {
    let max_text_bytes = env::var("ALLOY_WS_MAX_TEXT_BYTES")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(WS_DEFAULT_MAX_TEXT_BYTES);
    let idle_timeout_secs = env::var("ALLOY_WS_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(WS_DEFAULT_IDLE_TIMEOUT_SECS);

    WsRuntimeConfig {
        max_text_bytes,
        idle_timeout: Duration::from_secs(idle_timeout_secs),
    }
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

fn sse_event_stream(service_name: String) -> impl Stream<Item = Result<Event, Infallible>> {
    let init_name = service_name.clone();
    let initial = once(Ok(build_sse_event(0, "heartbeat", &init_name)));
    let mut sequence = 0u64;
    let ticks = IntervalStream::new(tokio::time::interval(Duration::from_secs(2))).map(move |_| {
        sequence += 1;
        let kind = if sequence % 5 == 0 {
            "heartbeat"
        } else {
            "message"
        };
        Ok(build_sse_event(sequence, kind, &service_name))
    });

    initial.chain(ticks)
}

fn build_sse_event(sequence: u64, kind: &'static str, service_name: &str) -> Event {
    let payload = ServerSentEventPayload {
        sequence,
        kind,
        service_name: service_name.to_string(),
    };

    match Event::default()
        .id(sequence.to_string())
        .event(kind)
        .json_data(payload)
    {
        Ok(event) => event,
        Err(err) => {
            tracing::error!(error = %err, "failed to serialize sse payload");
            Event::default()
                .event("internal_error")
                .data("failed to serialize event payload")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{header::AUTHORIZATION, HeaderValue, Request, StatusCode};
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde::Serialize;
    use tokio::time::{timeout, Duration};
    use tokio_stream::StreamExt;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn health_returns_ok() {
        let app = build_router(Arc::new(AppState::local("test-server")));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
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
        assert!(body_text.contains("/protected/whoami"));
        assert!(body_text.contains("ApiErrorResponse"));
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

    #[tokio::test]
    async fn events_stream_returns_sse_headers_and_heartbeat_payload() {
        let app = build_router(Arc::new(AppState::local("test-server")));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/events")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content type should exist")
            .to_str()
            .expect("content type value");
        assert!(content_type.starts_with("text/event-stream"));

        let mut stream = response.into_body().into_data_stream();
        let first_chunk = timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("first chunk should arrive")
            .expect("body stream item")
            .expect("body bytes");
        let first_text = String::from_utf8(first_chunk.to_vec()).expect("utf8 chunk");
        assert!(first_text.contains("event: heartbeat"));
        assert!(first_text.contains("\"kind\":\"heartbeat\""));
    }

    #[derive(Serialize)]
    struct TestClaims {
        sub: String,
        exp: usize,
        iss: String,
        aud: String,
    }

    fn issue_test_token(secret: &str) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            &TestClaims {
                sub: "user-123".to_string(),
                exp: 4_102_444_800,
                iss: "https://issuer.local".to_string(),
                aud: "alloy-api".to_string(),
            },
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("token should encode")
    }

    #[tokio::test]
    async fn protected_route_returns_anonymous_when_auth_disabled() {
        let app = build_router_with_auth(
            Arc::new(AppState::local("test-server")),
            auth::AuthRuntimeConfig {
                enabled: false,
                jwt_secret: None,
                expected_issuer: None,
                expected_audience: None,
            },
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected/whoami")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body_text = String::from_utf8(bytes.to_vec()).expect("utf8");
        assert!(body_text.contains("anonymous"));
    }

    #[tokio::test]
    async fn protected_route_rejects_missing_token_when_auth_enabled() {
        let app = build_router_with_auth(
            Arc::new(AppState::local("test-server")),
            auth::AuthRuntimeConfig {
                enabled: true,
                jwt_secret: Some("dev-secret".to_string()),
                expected_issuer: Some("https://issuer.local".to_string()),
                expected_audience: Some("alloy-api".to_string()),
            },
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected/whoami")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should succeed");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_accepts_valid_token_when_auth_enabled() {
        let secret = "dev-secret";
        let token = issue_test_token(secret);
        let app = build_router_with_auth(
            Arc::new(AppState::local("test-server")),
            auth::AuthRuntimeConfig {
                enabled: true,
                jwt_secret: Some(secret.to_string()),
                expected_issuer: Some("https://issuer.local".to_string()),
                expected_audience: Some("alloy-api".to_string()),
            },
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected/whoami")
                    .header(
                        AUTHORIZATION,
                        HeaderValue::from_str(&format!("Bearer {token}")).expect("header value"),
                    )
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body_text = String::from_utf8(bytes.to_vec()).expect("utf8");
        assert!(body_text.contains("user-123"));
    }
}
