use std::sync::Arc;

use alloy_core::{AlloyError, AppState};
use alloy_rpc::{build_hello_response, Greeter, GreeterServer, HelloRequest, HelloResponse};
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct GreeterService {
    state: Arc<AppState>,
}

impl GreeterService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl Greeter for GreeterService {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        let response = build_hello_response(&self.state, request.into_inner()).map_err(map_error)?;
        Ok(Response::new(response))
    }
}

pub fn build_grpc_service(state: Arc<AppState>) -> GreeterServer<GreeterService> {
    GreeterServer::new(GreeterService::new(state))
}

fn map_error(err: AlloyError) -> Status {
    match err {
        AlloyError::Validation(message) => Status::invalid_argument(message),
        AlloyError::Internal(message) => Status::internal(message),
    }
}
