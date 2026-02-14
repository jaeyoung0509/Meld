use axum::{
    extract::{FromRequest, FromRequestParts, Path, Query},
    http::{request::Parts, StatusCode},
    response::IntoResponse,
    Json,
};
use openportio_core::OpenportioError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use tonic::Status;
use validator::{Validate, ValidationErrors};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, PartialEq, Eq)]
pub struct ApiValidationIssue {
    pub loc: Vec<String>,
    pub msg: String,
    #[serde(rename = "type")]
    pub issue_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Vec<ApiValidationIssue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ApiErrorResponse {
    pub fn validation(
        message: impl Into<String>,
        detail: Option<Vec<ApiValidationIssue>>,
        details: Option<Value>,
    ) -> Self {
        Self {
            code: "validation_error".to_string(),
            message: message.into(),
            detail,
            details,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "bad_request".to_string(),
            message: message.into(),
            detail: None,
            details: None,
        }
    }

    pub fn internal_server_error() -> Self {
        Self {
            code: "internal_error".to_string(),
            message: "internal server error".to_string(),
            detail: None,
            details: None,
        }
    }
}

pub type ApiError = (StatusCode, Json<ApiErrorResponse>);

pub fn validation_error(err: ValidationErrors) -> ApiError {
    validation_error_with_source(err, "request")
}

pub fn validation_error_with_source(err: ValidationErrors, source: &'static str) -> ApiError {
    let mut fields = serde_json::Map::new();
    let mut detail = Vec::new();
    for (field, errors) in err.field_errors() {
        let messages: Vec<String> = errors
            .iter()
            .map(|e| {
                e.message
                    .clone()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| e.code.to_string())
            })
            .collect();
        for error in errors {
            let msg = error
                .message
                .clone()
                .map(|m| m.to_string())
                .unwrap_or_else(|| error.code.to_string());
            detail.push(ApiValidationIssue {
                loc: vec![source.to_string(), field.to_string()],
                msg,
                issue_type: error.code.to_string(),
            });
        }
        fields.insert(field.to_string(), json!(messages));
    }

    (
        StatusCode::BAD_REQUEST,
        Json(ApiErrorResponse::validation(
            "request validation failed",
            Some(detail),
            Some(Value::Object(fields)),
        )),
    )
}

pub fn bad_request(message: impl Into<String>) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiErrorResponse::bad_request(message)),
    )
}

pub fn map_domain_error_to_rest(err: OpenportioError) -> ApiError {
    match err {
        OpenportioError::Validation(message) => {
            let detail = ApiValidationIssue {
                loc: vec!["domain".to_string()],
                msg: message.clone(),
                issue_type: "domain_validation".to_string(),
            };
            (
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse::validation(
                    message,
                    Some(vec![detail]),
                    None,
                )),
            )
        }
        OpenportioError::Internal(message) => {
            tracing::error!(error = %message, "internal domain error surfaced in REST handler");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse::internal_server_error()),
            )
        }
    }
}

pub fn map_domain_error_to_grpc(err: OpenportioError) -> Status {
    match err {
        OpenportioError::Validation(message) => Status::invalid_argument(message),
        OpenportioError::Internal(message) => {
            tracing::error!(error = %message, "internal domain error surfaced in gRPC handler");
            Status::internal("internal server error")
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|err| bad_request(format!("invalid json body: {err}")))?;

        value
            .validate()
            .map_err(|err| validation_error_with_source(err, "body"))?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedQuery<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|err| bad_request(format!("invalid query: {err}")))?;
        value
            .validate()
            .map_err(|err| validation_error_with_source(err, "query"))?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedPath<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequestParts<S> for ValidatedPath<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|err| bad_request(format!("invalid path: {err}")))?;
        value
            .validate()
            .map_err(|err| validation_error_with_source(err, "path"))?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedParts<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequestParts<S> for ValidatedParts<T>
where
    T: FromRequestParts<S> + Validate,
    T::Rejection: std::fmt::Display,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let value = T::from_request_parts(parts, state)
            .await
            .map_err(|err| bad_request(format!("invalid request parts: {err}")))?;
        value
            .validate()
            .map_err(|err| validation_error_with_source(err, "parts"))?;
        Ok(Self(value))
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[derive(Debug, serde::Deserialize, Validate)]
    struct BodyDto {
        #[validate(length(min = 3))]
        name: String,
    }

    #[test]
    fn validation_error_includes_fastapi_like_detail_shape() {
        let dto = BodyDto {
            name: "ab".to_string(),
        };
        let err = dto.validate().expect_err("must fail");
        let (status, Json(body)) = validation_error_with_source(err, "body");

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_error");
        let detail = body.detail.expect("detail should exist");
        assert!(detail
            .iter()
            .any(|issue| issue.loc == vec!["body".to_string(), "name".to_string()]));
    }

    #[test]
    fn internal_domain_errors_are_sanitized_for_rest_clients() {
        let (status, Json(body)) =
            map_domain_error_to_rest(OpenportioError::Internal("db exploded".to_string()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "internal_error");
        assert_eq!(body.message, "internal server error");
        assert!(body.detail.is_none());
    }

    #[test]
    fn internal_domain_errors_are_sanitized_for_grpc_clients() {
        let status = map_domain_error_to_grpc(OpenportioError::Internal("db exploded".to_string()));
        assert_eq!(status.code(), tonic::Code::Internal);
        assert_eq!(status.message(), "internal server error");
    }
}
