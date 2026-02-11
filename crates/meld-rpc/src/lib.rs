use meld_core::{AppState, MeldResult};

pub mod proto {
    tonic::include_proto!("meld.v1");
}

pub use proto::greeter_client::GreeterClient;
pub use proto::greeter_server::{Greeter, GreeterServer};
pub use proto::{HelloRequest, HelloResponse};

pub fn build_hello_response(state: &AppState, request: HelloRequest) -> MeldResult<HelloResponse> {
    let message = state.greet(&request.name)?;
    Ok(HelloResponse { message })
}

pub fn grpc_contract_docs_markdown() -> &'static str {
    include_str!("../../../docs/generated/grpc-contracts.md")
}

pub fn grpc_contract_openapi_bridge_json() -> &'static str {
    include_str!("../../../docs/generated/grpc-openapi-bridge.json")
}
