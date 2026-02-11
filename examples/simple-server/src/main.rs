use std::{net::SocketAddr, sync::Arc};

use alloy_core::AppState;
use alloy_server::AlloyServer;
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct NotePath {
    id: String,
}

#[derive(Debug, Deserialize)]
struct NoteQuery {
    q: Option<String>,
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CreateNoteBody {
    title: String,
}

#[derive(Debug, Serialize)]
struct NoteResponse {
    id: String,
    title: String,
}

#[derive(Debug, Serialize)]
struct NotesListResponse {
    query: Option<String>,
    limit: u32,
}

async fn get_note(
    State(state): State<Arc<AppState>>,
    Path(path): Path<NotePath>,
) -> Result<Json<NoteResponse>, (axum::http::StatusCode, String)> {
    let title = state.greet(&path.id).map_err(|err| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("validation error: {err}"),
        )
    })?;
    Ok(Json(NoteResponse {
        id: path.id,
        title,
    }))
}

async fn list_notes(Query(query): Query<NoteQuery>) -> Json<NotesListResponse> {
    Json(NotesListResponse {
        query: query.q,
        limit: query.limit.unwrap_or(20),
    })
}

async fn create_note(Json(body): Json<CreateNoteBody>) -> Json<NoteResponse> {
    Json(NoteResponse {
        id: "note-1".to_string(),
        title: body.title,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState::local("simple-server"));
    let custom_router = Router::new()
        .route("/notes", get(list_notes).post(create_note))
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
