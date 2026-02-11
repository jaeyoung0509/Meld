use alloy_core::{AlloyResult, AppState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub message: String,
}

pub fn build_hello_response(state: &AppState, request: HelloRequest) -> AlloyResult<HelloResponse> {
    let message = state.greet(&request.name)?;
    Ok(HelloResponse { message })
}
