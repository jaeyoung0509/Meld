use alloy_core::AppState;
use alloy_rpc::{build_hello_response, Greeter, GreeterServer, HelloRequest, HelloResponse};
use tonic::{Request, Response, Status};

#[derive(Default)]
struct TestGreeter;

#[tonic::async_trait]
impl Greeter for TestGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        let req = request.into_inner();
        Ok(Response::new(HelloResponse {
            message: format!("hi {}", req.name),
        }))
    }
}

#[test]
fn generated_types_and_service_are_usable() {
    let _svc = GreeterServer::new(TestGreeter);
    let _req = HelloRequest {
        name: "Rust".to_string(),
    };
}

#[test]
fn build_hello_response_matches_domain_behavior() {
    let state = AppState::local("rpc-contract-test");
    let result = build_hello_response(
        &state,
        HelloRequest {
            name: "Rust".to_string(),
        },
    )
    .expect("response should be built");

    assert_eq!(result.message, "Hello, Rust!");
}
