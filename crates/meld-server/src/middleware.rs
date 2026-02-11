use std::{env, str::FromStr, time::Duration};

use axum::{
    error_handling::HandleErrorLayer,
    http::{HeaderName, HeaderValue, StatusCode},
    BoxError, Router,
};
use tower::{limit::ConcurrencyLimitLayer, timeout::TimeoutLayer, ServiceBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

const REQUEST_ID_HEADER: &str = "x-request-id";
const DEFAULT_TIMEOUT_SECONDS: u64 = 15;
const DEFAULT_MAX_IN_FLIGHT_REQUESTS: usize = 1024;
const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, Default)]
pub enum CorsAllowOrigins {
    #[default]
    None,
    Any,
    List(Vec<HeaderValue>),
}

#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    pub timeout_seconds: u64,
    pub max_in_flight_requests: usize,
    pub max_request_body_bytes: usize,
    pub cors_allow_origins: CorsAllowOrigins,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
            max_in_flight_requests: DEFAULT_MAX_IN_FLIGHT_REQUESTS,
            max_request_body_bytes: DEFAULT_REQUEST_BODY_LIMIT_BYTES,
            cors_allow_origins: CorsAllowOrigins::None,
        }
    }
}

impl MiddlewareConfig {
    pub fn from_env() -> Self {
        Self {
            timeout_seconds: read_env_with_fallback(
                "MELD_TIMEOUT_SECONDS",
                "ALLOY_TIMEOUT_SECONDS",
            )
            .unwrap_or(DEFAULT_TIMEOUT_SECONDS),
            max_in_flight_requests: read_env_with_fallback(
                "MELD_MAX_IN_FLIGHT_REQUESTS",
                "ALLOY_MAX_IN_FLIGHT_REQUESTS",
            )
            .unwrap_or(DEFAULT_MAX_IN_FLIGHT_REQUESTS),
            max_request_body_bytes: read_env_with_fallback(
                "MELD_REQUEST_BODY_LIMIT_BYTES",
                "ALLOY_REQUEST_BODY_LIMIT_BYTES",
            )
            .unwrap_or(DEFAULT_REQUEST_BODY_LIMIT_BYTES),
            cors_allow_origins: parse_cors_allow_origins(
                env::var("MELD_CORS_ALLOW_ORIGINS")
                    .ok()
                    .or_else(|| env::var("ALLOY_CORS_ALLOW_ORIGINS").ok()),
            ),
        }
    }
}

pub fn apply_shared_middleware(app: Router, config: &MiddlewareConfig) -> Router {
    let app = match &config.cors_allow_origins {
        CorsAllowOrigins::None => app,
        CorsAllowOrigins::Any => app.layer(CorsLayer::new().allow_origin(Any)),
        CorsAllowOrigins::List(origins) => {
            app.layer(CorsLayer::new().allow_origin(origins.clone()))
        }
    };

    app.layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_middleware_error))
            .layer(TraceLayer::new_for_http())
            .layer(PropagateRequestIdLayer::new(header_name()))
            .layer(SetRequestIdLayer::new(header_name(), MakeRequestUuid))
            .layer(RequestBodyLimitLayer::new(config.max_request_body_bytes))
            .layer(TimeoutLayer::new(Duration::from_secs(
                config.timeout_seconds,
            )))
            .layer(ConcurrencyLimitLayer::new(config.max_in_flight_requests)),
    )
}

async fn handle_middleware_error(error: BoxError) -> (StatusCode, String) {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (StatusCode::REQUEST_TIMEOUT, "request timed out".to_string());
    }

    tracing::error!(error = %error, "unhandled middleware error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal server error".to_string(),
    )
}

fn header_name() -> HeaderName {
    HeaderName::from_static(REQUEST_ID_HEADER)
}

fn read_env<T>(name: &str) -> Option<T>
where
    T: FromStr,
{
    env::var(name).ok().and_then(|raw| raw.parse::<T>().ok())
}

fn read_env_with_fallback<T>(primary: &str, legacy: &str) -> Option<T>
where
    T: FromStr,
{
    read_env(primary).or_else(|| read_env(legacy))
}

fn parse_cors_allow_origins(raw: Option<String>) -> CorsAllowOrigins {
    let Some(raw) = raw else {
        return CorsAllowOrigins::None;
    };

    let raw = raw.trim();
    if raw.is_empty() {
        return CorsAllowOrigins::None;
    }
    if raw == "*" {
        return CorsAllowOrigins::Any;
    }

    let origins = raw
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .filter_map(|origin| match HeaderValue::from_str(origin) {
            Ok(value) => Some(value),
            Err(err) => {
                tracing::warn!(origin = %origin, error = %err, "ignoring invalid cors origin");
                None
            }
        })
        .collect::<Vec<_>>();

    if origins.is_empty() {
        CorsAllowOrigins::None
    } else {
        CorsAllowOrigins::List(origins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{header::ORIGIN, Request},
        routing::{get, post},
    };
    use tower::util::ServiceExt;

    #[test]
    fn default_config_is_reasonable() {
        let config = MiddlewareConfig::default();
        assert_eq!(config.timeout_seconds, DEFAULT_TIMEOUT_SECONDS);
        assert_eq!(
            config.max_in_flight_requests,
            DEFAULT_MAX_IN_FLIGHT_REQUESTS
        );
        assert_eq!(
            config.max_request_body_bytes,
            DEFAULT_REQUEST_BODY_LIMIT_BYTES
        );
        assert!(matches!(config.cors_allow_origins, CorsAllowOrigins::None));
    }

    #[test]
    fn parse_cors_allow_origins_supports_wildcard_and_allowlist() {
        assert!(matches!(
            parse_cors_allow_origins(Some("*".to_string())),
            CorsAllowOrigins::Any
        ));

        let parsed =
            parse_cors_allow_origins(Some("https://one.example, https://two.example".to_string()));
        match parsed {
            CorsAllowOrigins::List(origins) => assert_eq!(origins.len(), 2),
            _ => panic!("expected allowlist"),
        }
    }

    #[tokio::test]
    async fn default_cors_policy_does_not_emit_allow_origin_header() {
        let app = apply_shared_middleware(
            Router::new().route("/health", get(|| async { "ok" })),
            &MiddlewareConfig::default(),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header(ORIGIN, "https://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should complete");

        assert!(response
            .headers()
            .get("access-control-allow-origin")
            .is_none());
    }

    #[tokio::test]
    async fn request_body_limit_rejects_oversized_payload() {
        let config = MiddlewareConfig {
            max_request_body_bytes: 8,
            ..MiddlewareConfig::default()
        };

        let app = apply_shared_middleware(
            Router::new().route("/echo", post(|body: String| async move { body })),
            &config,
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/echo")
                    .body(Body::from("0123456789012345"))
                    .unwrap(),
            )
            .await
            .expect("request should complete");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn middleware_internal_error_response_is_generic() {
        let (status, body) =
            handle_middleware_error(std::io::Error::other("sensitive").into()).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "internal server error");
    }
}
