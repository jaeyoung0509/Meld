use std::{env, str::FromStr, time::Duration};

use axum::{
    error_handling::HandleErrorLayer,
    http::{HeaderName, StatusCode},
    BoxError, Router,
};
use tower::{limit::ConcurrencyLimitLayer, timeout::TimeoutLayer, ServiceBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    pub timeout_seconds: u64,
    pub max_in_flight_requests: usize,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 15,
            max_in_flight_requests: 1024,
        }
    }
}

impl MiddlewareConfig {
    pub fn from_env() -> Self {
        Self {
            timeout_seconds: read_env("ALLOY_TIMEOUT_SECONDS").unwrap_or(15),
            max_in_flight_requests: read_env("ALLOY_MAX_IN_FLIGHT_REQUESTS").unwrap_or(1024),
        }
    }
}

pub fn apply_shared_middleware(app: Router, config: &MiddlewareConfig) -> Router {
    app.layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_middleware_error))
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::new().allow_origin(Any))
            .layer(PropagateRequestIdLayer::new(header_name()))
            .layer(SetRequestIdLayer::new(header_name(), MakeRequestUuid))
            .layer(TimeoutLayer::new(Duration::from_secs(config.timeout_seconds)))
            .layer(ConcurrencyLimitLayer::new(config.max_in_flight_requests)),
    )
}

async fn handle_middleware_error(error: BoxError) -> (StatusCode, String) {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (StatusCode::REQUEST_TIMEOUT, "request timed out".to_string());
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("middleware error: {error}"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_reasonable() {
        let config = MiddlewareConfig::default();
        assert_eq!(config.timeout_seconds, 15);
        assert_eq!(config.max_in_flight_requests, 1024);
    }
}
