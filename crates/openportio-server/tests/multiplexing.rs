use std::sync::Arc;

use axum::http::header;
use futures_util::{SinkExt, StreamExt};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use openportio_core::AppState;
use openportio_rpc::{GreeterClient, HelloRequest};
use openportio_server::{
    auth::AuthRuntimeConfig, build_multiplexed_router, build_multiplexed_router_with_auth,
    middleware, OpenportioServer,
};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message;
use tonic::metadata::MetadataValue;

#[tokio::test]
async fn serves_rest_and_grpc_on_single_port() {
    let state = Arc::new(AppState::local("multiplexing-test"));
    let app = middleware::apply_shared_middleware(
        build_multiplexed_router(state),
        &middleware::MiddlewareConfig::default(),
    );
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
    let ws_url = format!("ws://{addr}/ws");

    // Basic retry to avoid flaky startup race in CI.
    let rest_client = reqwest::Client::builder()
        .build()
        .expect("build reqwest client");

    let health_response = loop {
        match rest_client
            .get(format!("{base_url}/health"))
            .header("x-request-id", "test-request-id")
            .send()
            .await
        {
            Ok(resp) => break resp.status(),
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
        }
    };
    assert_eq!(health_response.as_u16(), 200);

    let events_response = rest_client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("sse endpoint should be reachable");
    assert_eq!(events_response.status().as_u16(), 200);
    let events_content_type = events_response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type header on sse")
        .to_str()
        .expect("content-type should be valid");
    assert!(events_content_type.starts_with("text/event-stream"));
    drop(events_response);

    let hello_response = rest_client
        .get(format!("{base_url}/hello/Rust"))
        .header("x-request-id", "rest-hello-id")
        .send()
        .await
        .expect("rest hello should succeed");
    assert_eq!(
        hello_response
            .headers()
            .get("x-request-id")
            .expect("x-request-id should be propagated")
            .to_str()
            .expect("request id must be valid header"),
        "rest-hello-id"
    );

    let docs_response = rest_client
        .get(format!("{base_url}/docs"))
        .send()
        .await
        .expect("swagger ui should be reachable");
    assert_eq!(docs_response.status().as_u16(), 200);

    let whoami_response = rest_client
        .get(format!("{base_url}/protected/whoami"))
        .send()
        .await
        .expect("protected whoami should be reachable");
    assert_eq!(whoami_response.status().as_u16(), 200);
    let whoami_body = whoami_response
        .text()
        .await
        .expect("protected whoami body should be readable");
    assert!(whoami_body.contains("anonymous"));

    let openapi_response = rest_client
        .get(format!("{base_url}/openapi.json"))
        .send()
        .await
        .expect("openapi json should be reachable");
    assert_eq!(openapi_response.status().as_u16(), 200);
    assert_eq!(
        openapi_response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content-type header")
            .to_str()
            .expect("content-type value"),
        "application/json"
    );

    let grpc_contracts_html_response = rest_client
        .get(format!("{base_url}/grpc/contracts"))
        .send()
        .await
        .expect("grpc contracts html should be reachable");
    assert_eq!(grpc_contracts_html_response.status().as_u16(), 200);
    let grpc_contracts_html_content_type = grpc_contracts_html_response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("grpc contracts html content-type header")
        .to_str()
        .expect("grpc contracts html content-type value");
    assert!(grpc_contracts_html_content_type.starts_with("text/html"));
    let grpc_contracts_html_body = grpc_contracts_html_response
        .text()
        .await
        .expect("grpc contracts html body");
    assert!(grpc_contracts_html_body.contains("/grpc/contracts/openapi.json"));

    let grpc_contracts_markdown_response = rest_client
        .get(format!("{base_url}/grpc/contracts.md"))
        .send()
        .await
        .expect("grpc contracts markdown should be reachable");
    assert_eq!(grpc_contracts_markdown_response.status().as_u16(), 200);
    let grpc_contracts_markdown_content_type = grpc_contracts_markdown_response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("grpc contracts markdown content-type header")
        .to_str()
        .expect("grpc contracts markdown content-type value");
    assert!(grpc_contracts_markdown_content_type.starts_with("text/markdown"));

    let grpc_bridge_response = rest_client
        .get(format!("{base_url}/grpc/contracts/openapi.json"))
        .send()
        .await
        .expect("grpc bridge json should be reachable");
    assert_eq!(grpc_bridge_response.status().as_u16(), 200);

    let (mut ws_stream, ws_resp) = tokio_tungstenite::connect_async(ws_url)
        .await
        .expect("websocket handshake should succeed");
    assert_eq!(ws_resp.status().as_u16(), 101);
    ws_stream
        .send(Message::Text("hello-openportio".to_string()))
        .await
        .expect("ws send should succeed");
    let ws_message = ws_stream
        .next()
        .await
        .expect("ws message item should exist")
        .expect("ws message should be valid");
    match ws_message {
        Message::Text(text) => assert_eq!(text, "echo: hello-openportio"),
        other => panic!("expected text frame, got {other:?}"),
    }
    ws_stream
        .close(None)
        .await
        .expect("ws close should succeed");

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

#[derive(serde::Serialize)]
struct TestClaims {
    sub: String,
    exp: usize,
    iss: String,
    aud: String,
}

fn issue_test_token(secret: &str) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        &TestClaims {
            sub: "user-1".to_string(),
            exp: 4_102_444_800,
            iss: "https://issuer.local".to_string(),
            aud: "openportio-api".to_string(),
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("token should encode")
}

#[tokio::test]
async fn grpc_auth_interceptor_rejects_missing_token_and_accepts_valid_token() {
    let state = Arc::new(AppState::local("multiplexing-auth-test"));
    let auth_cfg = AuthRuntimeConfig {
        enabled: true,
        jwt_secret: Some("dev-secret".to_string()),
        expected_issuer: Some("https://issuer.local".to_string()),
        expected_audience: Some("openportio-api".to_string()),
    };
    let app = middleware::apply_shared_middleware(
        build_multiplexed_router_with_auth(state, auth_cfg),
        &middleware::MiddlewareConfig::default(),
    );
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
    let mut grpc_client = GreeterClient::connect(base_url.clone())
        .await
        .expect("grpc client connect");

    let missing = grpc_client
        .say_hello(tonic::Request::new(HelloRequest {
            name: "Rust".to_string(),
        }))
        .await
        .expect_err("missing token should fail");
    assert_eq!(missing.code(), tonic::Code::Unauthenticated);
    assert_eq!(missing.message(), "missing bearer token");

    let token = issue_test_token("dev-secret");
    let mut request = tonic::Request::new(HelloRequest {
        name: "Rust".to_string(),
    });
    request.metadata_mut().insert(
        "authorization",
        MetadataValue::try_from(format!("Bearer {token}")).expect("metadata value"),
    );
    let response = grpc_client
        .say_hello(request)
        .await
        .expect("valid token should succeed")
        .into_inner();
    assert_eq!(response.message, "Hello, Rust!");

    let _ = shutdown_tx.send(());
    let _ = server.await;
}

async fn reserve_local_addr() -> std::net::SocketAddr {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral listener");
    let addr = listener.local_addr().expect("ephemeral local addr");
    drop(listener);
    addr
}

#[tokio::test]
async fn serves_rest_and_grpc_on_explicit_dual_ports() {
    let rest_addr = reserve_local_addr().await;
    let grpc_addr = reserve_local_addr().await;
    let local = tokio::task::LocalSet::new();

    local
        .run_until(async move {
            let server = tokio::task::spawn_local(async move {
                OpenportioServer::new()
                    .with_state(Arc::new(AppState::local("dual-port-test")))
                    .with_rest_addr(rest_addr)
                    .with_grpc_addr(grpc_addr)
                    .run()
                    .await
                    .expect("dual-port server should run");
            });

            let rest_base_url = format!("http://{rest_addr}");
            let grpc_base_url = format!("http://{grpc_addr}");
            let rest_client = reqwest::Client::builder()
                .build()
                .expect("build reqwest client");

            let health_status = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    match rest_client
                        .get(format!("{rest_base_url}/health"))
                        .send()
                        .await
                    {
                        Ok(resp) => break resp.status(),
                        Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
                    }
                }
            })
            .await
            .expect("rest listener should become healthy in time");
            assert_eq!(health_status.as_u16(), 200);

            let mut grpc_client = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    match GreeterClient::connect(grpc_base_url.clone()).await {
                        Ok(client) => break client,
                        Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
                    }
                }
            })
            .await
            .expect("grpc listener should become reachable in time");

            let grpc_response = grpc_client
                .say_hello(tonic::Request::new(HelloRequest {
                    name: "DualPort".to_string(),
                }))
                .await
                .expect("grpc call should succeed on grpc listener")
                .into_inner();
            assert_eq!(grpc_response.message, "Hello, DualPort!");

            server.abort();
            let _ = server.await;
        })
        .await;
}
