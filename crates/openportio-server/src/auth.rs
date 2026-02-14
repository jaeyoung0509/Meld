use std::{env, str::FromStr};

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use openportio_core::auth::{validate_bearer_jwt, AuthPrincipal, JwtValidationConfig};
use tonic::Status;

use crate::api::ApiErrorResponse;

#[derive(Debug, Clone, Default)]
pub struct AuthRuntimeConfig {
    pub enabled: bool,
    pub jwt_secret: Option<String>,
    pub expected_issuer: Option<String>,
    pub expected_audience: Option<String>,
}

impl AuthRuntimeConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: read_env_bool_with_aliases(&[
                "OPENPORTIO_AUTH_ENABLED",
                "MELD_AUTH_ENABLED",
                "ALLOY_AUTH_ENABLED",
            ])
            .unwrap_or(false),
            jwt_secret: read_env_string_with_aliases(&[
                "OPENPORTIO_AUTH_JWT_SECRET",
                "MELD_AUTH_JWT_SECRET",
                "ALLOY_AUTH_JWT_SECRET",
            ]),
            expected_issuer: read_env_string_with_aliases(&[
                "OPENPORTIO_AUTH_ISSUER",
                "MELD_AUTH_ISSUER",
                "ALLOY_AUTH_ISSUER",
            ]),
            expected_audience: read_env_string_with_aliases(&[
                "OPENPORTIO_AUTH_AUDIENCE",
                "MELD_AUTH_AUDIENCE",
                "ALLOY_AUTH_AUDIENCE",
            ]),
        }
    }

    fn jwt_validation_config(&self) -> Result<JwtValidationConfig, AuthRejection> {
        let secret = self.jwt_secret.clone().ok_or_else(|| {
            AuthRejection::Misconfigured("OPENPORTIO_AUTH_JWT_SECRET is missing".to_string())
        })?;

        Ok(JwtValidationConfig {
            secret,
            expected_issuer: self.expected_issuer.clone(),
            expected_audience: self.expected_audience.clone(),
        })
    }

    pub fn authenticate_authorization_value_str(
        &self,
        auth_value: &str,
    ) -> Result<AuthPrincipal, AuthRejection> {
        if !self.enabled {
            return Ok(AuthPrincipal {
                subject: "anonymous".to_string(),
                issuer: None,
                audience: vec![],
                scopes: vec![],
            });
        }
        let token = parse_bearer_token(auth_value)?;
        let validation_cfg = self.jwt_validation_config()?;
        validate_bearer_jwt(token, &validation_cfg)
            .map_err(|err| AuthRejection::InvalidToken(err.to_string()))
    }

    pub fn authenticate_header_value(
        &self,
        auth_value: Option<&HeaderValue>,
    ) -> Result<AuthPrincipal, AuthRejection> {
        if !self.enabled {
            return Ok(AuthPrincipal {
                subject: "anonymous".to_string(),
                issuer: None,
                audience: vec![],
                scopes: vec![],
            });
        }

        let value = auth_value
            .ok_or(AuthRejection::MissingAuthorization)?
            .to_str()
            .map_err(|_| {
                AuthRejection::InvalidToken("authorization header is invalid".to_string())
            })?;

        self.authenticate_authorization_value_str(value)
    }

    pub fn authenticate_headers(
        &self,
        headers: &HeaderMap,
    ) -> Result<AuthPrincipal, AuthRejection> {
        self.authenticate_header_value(headers.get(header::AUTHORIZATION))
    }
}

#[derive(Debug, Clone)]
pub enum AuthRejection {
    MissingAuthorization,
    InvalidToken(String),
    Misconfigured(String),
}

impl AuthRejection {
    pub fn into_rest_response(self) -> Response {
        match self {
            Self::MissingAuthorization => (
                StatusCode::UNAUTHORIZED,
                Json(ApiErrorResponse {
                    code: "unauthorized".to_string(),
                    message: "missing bearer token".to_string(),
                    detail: None,
                    details: None,
                }),
            )
                .into_response(),
            Self::InvalidToken(message) => (
                StatusCode::UNAUTHORIZED,
                Json(ApiErrorResponse {
                    code: "unauthorized".to_string(),
                    message,
                    detail: None,
                    details: None,
                }),
            )
                .into_response(),
            Self::Misconfigured(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    code: "internal_error".to_string(),
                    message,
                    detail: None,
                    details: None,
                }),
            )
                .into_response(),
        }
    }

    pub fn into_grpc_status(self) -> Status {
        match self {
            Self::MissingAuthorization => Status::unauthenticated("missing bearer token"),
            Self::InvalidToken(message) => Status::unauthenticated(message),
            Self::Misconfigured(message) => Status::internal(message),
        }
    }
}

pub async fn rest_auth_middleware(
    State(cfg): State<AuthRuntimeConfig>,
    mut req: Request,
    next: Next,
) -> Response {
    match cfg.authenticate_headers(req.headers()) {
        Ok(principal) => {
            req.extensions_mut().insert(principal);
            next.run(req).await
        }
        Err(rejection) => rejection.into_rest_response(),
    }
}

pub fn parse_bearer_token(value: &str) -> Result<&str, AuthRejection> {
    let mut parts = value.splitn(2, ' ');
    let scheme = parts.next().unwrap_or_default();
    let token = parts.next().unwrap_or_default();

    if !scheme.eq_ignore_ascii_case("bearer") || token.trim().is_empty() {
        return Err(AuthRejection::InvalidToken(
            "authorization header must be Bearer <token>".to_string(),
        ));
    }

    Ok(token.trim())
}

fn read_env_bool(name: &str) -> Option<bool> {
    env::var(name)
        .ok()
        .and_then(|raw| bool::from_str(raw.trim()).ok())
}

fn read_env_bool_with_aliases(names: &[&str]) -> Option<bool> {
    names.iter().find_map(|name| read_env_bool(name))
}

fn read_env_string_with_aliases(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| env::var(name).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn from_env_supports_meld_compatibility_aliases() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        clear_auth_env();

        env::set_var("MELD_AUTH_ENABLED", "true");
        env::set_var("MELD_AUTH_JWT_SECRET", "legacy-secret");
        env::set_var("MELD_AUTH_ISSUER", "https://issuer.legacy");
        env::set_var("MELD_AUTH_AUDIENCE", "legacy-audience");

        let cfg = AuthRuntimeConfig::from_env();
        assert!(cfg.enabled);
        assert_eq!(cfg.jwt_secret.as_deref(), Some("legacy-secret"));
        assert_eq!(
            cfg.expected_issuer.as_deref(),
            Some("https://issuer.legacy")
        );
        assert_eq!(cfg.expected_audience.as_deref(), Some("legacy-audience"));

        clear_auth_env();
    }

    fn clear_auth_env() {
        for key in [
            "OPENPORTIO_AUTH_ENABLED",
            "OPENPORTIO_AUTH_JWT_SECRET",
            "OPENPORTIO_AUTH_ISSUER",
            "OPENPORTIO_AUTH_AUDIENCE",
            "MELD_AUTH_ENABLED",
            "MELD_AUTH_JWT_SECRET",
            "MELD_AUTH_ISSUER",
            "MELD_AUTH_AUDIENCE",
            "ALLOY_AUTH_ENABLED",
            "ALLOY_AUTH_JWT_SECRET",
            "ALLOY_AUTH_ISSUER",
            "ALLOY_AUTH_AUDIENCE",
        ] {
            env::remove_var(key);
        }
    }
}
