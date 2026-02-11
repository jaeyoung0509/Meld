use std::{env, str::FromStr};

use alloy_core::auth::{validate_bearer_jwt, AuthPrincipal, JwtValidationConfig};
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
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
            enabled: read_env_bool("ALLOY_AUTH_ENABLED").unwrap_or(false),
            jwt_secret: env::var("ALLOY_AUTH_JWT_SECRET").ok(),
            expected_issuer: env::var("ALLOY_AUTH_ISSUER").ok(),
            expected_audience: env::var("ALLOY_AUTH_AUDIENCE").ok(),
        }
    }

    fn jwt_validation_config(&self) -> Result<JwtValidationConfig, AuthRejection> {
        let secret = self.jwt_secret.clone().ok_or_else(|| {
            AuthRejection::Misconfigured("ALLOY_AUTH_JWT_SECRET is missing".to_string())
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
                    details: None,
                }),
            )
                .into_response(),
            Self::InvalidToken(message) => (
                StatusCode::UNAUTHORIZED,
                Json(ApiErrorResponse {
                    code: "unauthorized".to_string(),
                    message,
                    details: None,
                }),
            )
                .into_response(),
            Self::Misconfigured(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    code: "internal_error".to_string(),
                    message,
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
