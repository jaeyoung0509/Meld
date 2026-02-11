use std::{net::SocketAddr, sync::Arc};

use alloy_core::AppState;
use alloy_server::{
    api::{bad_request, ApiError},
    AlloyServer,
};
use axum::{
    extract::{FromRequestParts, Path, State},
    http::request::Parts,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize)]
struct NotePath {
    id: String,
}

#[derive(Debug, Deserialize, Validate)]
struct NoteQuery {
    #[validate(length(max = 80))]
    q: Option<String>,
    #[validate(range(min = 1, max = 100))]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize, Validate)]
struct CreateNoteBody {
    #[validate(length(min = 2, max = 120))]
    title: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NoteResponse {
    id: String,
    title: String,
    request_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NotesListResponse {
    query: Option<String>,
    limit: u32,
}

#[derive(Debug, Clone)]
struct RequestContext {
    request_id: Option<String>,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let request_id = parts
            .headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        Ok(Self { request_id })
    }
}

async fn get_note(
    ctx: RequestContext,
    State(state): State<Arc<AppState>>,
    Path(path): Path<NotePath>,
) -> Result<Json<NoteResponse>, ApiError> {
    let title = state
        .greet(&path.id)
        .map_err(|err| bad_request(err.to_string()))?;
    Ok(Json(NoteResponse {
        id: path.id,
        title,
        request_id: ctx.request_id,
    }))
}

#[alloy_server::route(get, "/notes", auto_validate)]
async fn list_notes(
    axum::extract::Query(query): axum::extract::Query<NoteQuery>,
) -> Json<NotesListResponse> {
    Json(NotesListResponse {
        query: query.q,
        limit: query.limit.unwrap_or(20),
    })
}

#[alloy_server::route(post, "/notes", auto_validate)]
async fn create_note(ctx: RequestContext, Json(body): Json<CreateNoteBody>) -> Json<NoteResponse> {
    Json(NoteResponse {
        id: "note-1".to_string(),
        title: body.title,
        request_id: ctx.request_id,
    })
}

#[alloy_server::route(post, "/notes/raw")]
async fn create_note_raw(Json(body): Json<CreateNoteBody>) -> Json<NoteResponse> {
    Json(NoteResponse {
        id: "note-raw".to_string(),
        title: body.title,
        request_id: None,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState::local("simple-server"));
    let custom_router = Router::new()
        .route("/notes", get(list_notes).post(create_note))
        .route("/notes/raw", axum::routing::post(create_note_raw))
        .route("/notes/:id", get(get_note))
        .with_state(state.clone());

    AlloyServer::new()
        .with_state(state)
        .with_rest_router(custom_router)
        .with_addr(SocketAddr::from(([127, 0, 0, 1], 4000)))
        .on_startup(|addr| {
            println!("simple-server started on {addr}");
        })
        .run()
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, http::Request};
    use tower::util::ServiceExt;

    fn app() -> Router {
        let state = Arc::new(AppState::local("simple-server-test"));
        Router::new()
            .route("/notes", get(list_notes).post(create_note))
            .route("/notes/raw", axum::routing::post(create_note_raw))
            .route("/notes/:id", get(get_note))
            .with_state(state)
    }

    #[tokio::test]
    async fn invalid_body_returns_structured_400() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/notes")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"title":"x"}"#))
                    .unwrap(),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let parsed: alloy_server::api::ApiErrorResponse =
            serde_json::from_slice(&body).expect("api error json");
        assert_eq!(parsed.code, "validation_error");
    }

    #[tokio::test]
    async fn valid_body_returns_note() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/notes")
                    .header("content-type", "application/json")
                    .header("x-request-id", "req-1")
                    .body(axum::body::Body::from(r#"{"title":"My Note"}"#))
                    .unwrap(),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let parsed: NoteResponse = serde_json::from_slice(&body).expect("note json");
        assert_eq!(parsed.title, "My Note");
        assert_eq!(parsed.request_id.as_deref(), Some("req-1"));
    }

    #[tokio::test]
    async fn invalid_query_returns_structured_400() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/notes?limit=0")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let parsed: alloy_server::api::ApiErrorResponse =
            serde_json::from_slice(&body).expect("api error json");
        assert_eq!(parsed.code, "validation_error");
    }

    #[tokio::test]
    async fn without_auto_validate_keeps_original_behavior() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/notes/raw")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"title":"x"}"#))
                    .unwrap(),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
