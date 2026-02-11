use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub service_name: String,
}

#[derive(Debug, Error)]
pub enum AlloyError {
    #[error("internal error: {0}")]
    Internal(String),
}
