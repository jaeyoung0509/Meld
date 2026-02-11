use std::{convert::Infallible, env, net::SocketAddr, sync::Arc};

use alloy_core::AppState;
use axum::Router;
use http::{Request, Response};
use tokio::net::TcpListener;
use tonic::{body::BoxBody, server::NamedService, service::Routes};
use tower::Service;

use crate::{build_router, grpc, middleware};

type RouterCustomizer = Box<dyn Fn(Router) -> Router + Send + Sync + 'static>;
type StartupHook = Box<dyn Fn(SocketAddr) + Send + Sync + 'static>;
type ShutdownHook = Box<dyn Fn() + Send + Sync + 'static>;

pub struct AlloyServer {
    state: Arc<AppState>,
    addr: SocketAddr,
    rest_router: Option<Router>,
    grpc_routes: Option<Routes>,
    middleware_config: middleware::MiddlewareConfig,
    middleware_customizers: Vec<RouterCustomizer>,
    startup_hooks: Vec<StartupHook>,
    shutdown_hooks: Vec<ShutdownHook>,
}

impl AlloyServer {
    pub fn new() -> Self {
        let state = Arc::new(AppState::local("alloy-server"));
        Self {
            grpc_routes: Some(Routes::new(grpc::build_grpc_service(state.clone())).prepare()),
            state,
            addr: load_addr_from_env().unwrap_or(SocketAddr::from(([127, 0, 0, 1], 3000))),
            rest_router: None,
            middleware_config: middleware::MiddlewareConfig::from_env(),
            middleware_customizers: Vec::new(),
            startup_hooks: Vec::new(),
            shutdown_hooks: Vec::new(),
        }
    }

    pub fn with_addr(mut self, addr: SocketAddr) -> Self {
        self.addr = addr;
        self
    }

    pub fn with_state(mut self, state: Arc<AppState>) -> Self {
        self.state = state;
        self
    }

    pub fn with_rest_router(mut self, router: Router) -> Self {
        self.rest_router = Some(router);
        self
    }

    pub fn without_grpc(mut self) -> Self {
        self.grpc_routes = None;
        self
    }

    pub fn with_grpc_service<S>(mut self, service: S) -> Self
    where
        S: Service<Request<BoxBody>, Response = Response<BoxBody>, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
    {
        let routes = match self.grpc_routes.take() {
            Some(existing) => existing.add_service(service).prepare(),
            None => Routes::new(service).prepare(),
        };
        self.grpc_routes = Some(routes);
        self
    }

    pub fn with_grpc_routes(mut self, routes: Routes) -> Self {
        self.grpc_routes = Some(routes.prepare());
        self
    }

    pub fn with_middleware_config(mut self, config: middleware::MiddlewareConfig) -> Self {
        self.middleware_config = config;
        self
    }

    pub fn with_middleware<F>(mut self, f: F) -> Self
    where
        F: Fn(Router) -> Router + Send + Sync + 'static,
    {
        self.middleware_customizers.push(Box::new(f));
        self
    }

    pub fn on_startup<F>(mut self, hook: F) -> Self
    where
        F: Fn(SocketAddr) + Send + Sync + 'static,
    {
        self.startup_hooks.push(Box::new(hook));
        self
    }

    pub fn on_shutdown<F>(mut self, hook: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.shutdown_hooks.push(Box::new(hook));
        self
    }

    pub fn build_app(&self) -> Router {
        let rest = self
            .rest_router
            .clone()
            .unwrap_or_else(|| build_router(self.state.clone()));
        let merged = match &self.grpc_routes {
            Some(routes) => rest.merge(routes.clone().into_axum_router()),
            None => rest,
        };

        let app = middleware::apply_shared_middleware(merged, &self.middleware_config);
        self.middleware_customizers
            .iter()
            .fold(app, |acc, customizer| customizer(acc))
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.build_app();
        let listener = TcpListener::bind(self.addr).await?;

        for hook in &self.startup_hooks {
            hook(self.addr);
        }
        tracing::info!(addr = %self.addr, "alloy-server listening");

        let shutdown_hooks = self.shutdown_hooks;
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                for hook in &shutdown_hooks {
                    hook();
                }
            })
            .await?;
        Ok(())
    }
}

impl Default for AlloyServer {
    fn default() -> Self {
        Self::new()
    }
}

fn load_addr_from_env() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    match env::var("ALLOY_SERVER_ADDR") {
        Ok(raw) => Ok(raw.parse()?),
        Err(env::VarError::NotPresent) => Ok(SocketAddr::from(([127, 0, 0, 1], 3000))),
        Err(err) => Err(Box::new(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn builder_creates_working_app() {
        let app = AlloyServer::new().build_app();
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .expect("health request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn builder_supports_custom_rest_and_middleware_chain() {
        let custom_router = Router::new().route("/custom", get(|| async { "custom-ok" }));
        let app = AlloyServer::new()
            .without_grpc()
            .with_rest_router(custom_router)
            .with_middleware(|router| {
                router.route("/ping", get(|| async { "pong" }))
            })
            .build_app();

        let ping_response = app
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .expect("ping request should succeed");
        assert_eq!(ping_response.status(), StatusCode::OK);
    }
}
