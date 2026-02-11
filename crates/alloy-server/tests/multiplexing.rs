use std::sync::Arc;

use alloy_core::AppState;
use alloy_rpc::{GreeterClient, HelloRequest};
use alloy_server::build_multiplexed_router;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[tokio::test]
async fn serves_rest_and_grpc_on_single_port() {
    let state = Arc::new(AppState::local("multiplexing-test"));
    let app = build_multiplexed_router(state);
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("server should run");
    });

    let base_url = format!("http://{addr}");

    // Basic retry to avoid flaky startup race in CI.
    let health_status = loop {
        match reqwest::get(format!("{base_url}/health")).await {
            Ok(resp) => break resp.status(),
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
        }
    };
    assert_eq!(health_status.as_u16(), 200);

    let mut grpc_client = GreeterClient::connect(base_url)
        .await
        .expect("grpc client connect");
    let grpc_response = grpc_client
        .say_hello(tonic::Request::new(HelloRequest {
            name: "Rust".to_string(),
        }))
        .await
        .expect("grpc call should succeed")
        .into_inner();

    assert_eq!(grpc_response.message, "Hello, Rust!");

    let _ = shutdown_tx.send(());
    let _ = server.await;
}
