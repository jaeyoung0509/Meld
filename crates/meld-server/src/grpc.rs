use std::sync::Arc;

use crate::auth::AuthRuntimeConfig;
use meld_core::AppState;
use meld_rpc::{build_hello_response, Greeter, GreeterServer, HelloRequest, HelloResponse};
use tonic::{service::interceptor::InterceptedService, Request, Response, Status};

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
        let response =
            build_hello_response(&self.state, request.into_inner()).map_err(map_error)?;
        Ok(Response::new(response))
    }
}

pub fn build_grpc_service(
    state: Arc<AppState>,
) -> InterceptedService<GreeterServer<GreeterService>, GrpcAuthInterceptor> {
    build_grpc_service_with_auth(state, AuthRuntimeConfig::from_env())
}

pub fn build_grpc_service_with_auth(
    state: Arc<AppState>,
    auth_cfg: AuthRuntimeConfig,
) -> InterceptedService<GreeterServer<GreeterService>, GrpcAuthInterceptor> {
    let service = GreeterServer::new(GreeterService::new(state));
    InterceptedService::new(service, GrpcAuthInterceptor { auth_cfg })
}

fn map_error(err: meld_core::MeldError) -> Status {
    crate::api::map_domain_error_to_grpc(err)
}

#[derive(Clone)]
pub struct GrpcAuthInterceptor {
    auth_cfg: AuthRuntimeConfig,
}

impl tonic::service::Interceptor for GrpcAuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        if !self.auth_cfg.enabled {
            return Ok(request);
        }

        let auth_value = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("missing bearer token"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("authorization metadata is invalid"))?;

        let principal = self
            .auth_cfg
            .authenticate_authorization_value_str(auth_value)
            .map_err(|err| err.into_grpc_status())?;
        request.extensions_mut().insert(principal);
        Ok(request)
    }
}
