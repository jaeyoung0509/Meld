use alloy_core::{AlloyResult, AppState};

pub mod proto {
    tonic::include_proto!("alloy.v1");
}

pub use proto::greeter_client::GreeterClient;
pub use proto::greeter_server::{Greeter, GreeterServer};
pub use proto::{HelloRequest, HelloResponse};

pub fn build_hello_response(state: &AppState, request: HelloRequest) -> AlloyResult<HelloResponse> {
    let message = state.greet(&request.name)?;
    Ok(HelloResponse { message })
}
