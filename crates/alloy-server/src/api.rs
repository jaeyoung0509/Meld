use axum::{
    extract::{FromRequest, FromRequestParts, Path, Query},
    http::{request::Parts, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use validator::{Validate, ValidationErrors};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ApiErrorResponse {
    pub fn validation(message: impl Into<String>, details: Option<Value>) -> Self {
        Self {
            code: "validation_error".to_string(),
            message: message.into(),
            details,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "bad_request".to_string(),
            message: message.into(),
            details: None,
        }
    }
}

pub type ApiError = (StatusCode, Json<ApiErrorResponse>);

pub fn validation_error(err: ValidationErrors) -> ApiError {
    let mut fields = serde_json::Map::new();
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
        fields.insert(field.to_string(), json!(messages));
    }

    (
        StatusCode::BAD_REQUEST,
        Json(ApiErrorResponse::validation(
            "request validation failed",
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

#[derive(Debug, Clone)]
pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|err| bad_request(format!("invalid json body: {err}")))?;

        value.validate().map_err(validation_error)?;
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
        value.validate().map_err(validation_error)?;
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
        value.validate().map_err(validation_error)?;
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
        value.validate().map_err(validation_error)?;
        Ok(Self(value))
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}
