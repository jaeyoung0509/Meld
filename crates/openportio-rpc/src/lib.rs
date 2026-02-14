use openportio_core::{AppState, OpenportioResult};

pub mod proto {
    tonic::include_proto!("openportio.v1");
}

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("openportio_descriptor");

pub use proto::greeter_client::GreeterClient;
pub use proto::greeter_server::{Greeter, GreeterServer};
pub use proto::{HelloRequest, HelloResponse};

pub fn build_hello_response(
    state: &AppState,
    request: HelloRequest,
) -> OpenportioResult<HelloResponse> {
    let message = state.greet(&request.name)?;
    Ok(HelloResponse { message })
}

pub fn grpc_contract_docs_markdown() -> &'static str {
    include_str!("../generated/grpc-contracts.md")
}

pub fn grpc_contract_openapi_bridge_json() -> &'static str {
    include_str!("../generated/grpc-openapi-bridge.json")
}
