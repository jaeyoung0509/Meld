use std::sync::Arc;

use thiserror::Error;

pub type AlloyResult<T> = Result<T, AlloyError>;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub service_name: String,
    pub environment: String,
}

impl AppConfig {
    pub fn local(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            environment: "local".to_string(),
        }
    }
}

#[derive(Debug, Error)]
pub enum AlloyError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub trait GreetingEngine: Send + Sync {
    fn greet(&self, name: &str) -> AlloyResult<String>;
}

pub trait MetricsSink: Send + Sync {
    fn incr_counter(&self, name: &str);
}

#[derive(Debug, Default)]
pub struct NoopMetrics;

impl MetricsSink for NoopMetrics {
    fn incr_counter(&self, _name: &str) {}
}

#[derive(Debug, Clone)]
pub struct StaticGreetingEngine {
    prefix: String,
}

impl StaticGreetingEngine {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl GreetingEngine for StaticGreetingEngine {
    fn greet(&self, name: &str) -> AlloyResult<String> {
        if name.trim().is_empty() {
            return Err(AlloyError::Validation("name must not be empty".to_string()));
        }
        Ok(format!("{}, {}!", self.prefix, name))
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub greeter: Arc<dyn GreetingEngine>,
    pub metrics: Arc<dyn MetricsSink>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        greeter: Arc<dyn GreetingEngine>,
        metrics: Arc<dyn MetricsSink>,
    ) -> Self {
        Self {
            config,
            greeter,
            metrics,
        }
    }

    pub fn local(service_name: impl Into<String>) -> Self {
        Self {
            config: AppConfig::local(service_name),
            greeter: Arc::new(StaticGreetingEngine::new("Hello")),
            metrics: Arc::new(NoopMetrics),
        }
    }

    pub fn greet(&self, name: &str) -> AlloyResult<String> {
        self.metrics.incr_counter("greet.requests");
        self.greeter.greet(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_rejects_empty_name() {
        let state = AppState::local("alloy-test");
        let result = state.greet("  ");
        assert!(matches!(result, Err(AlloyError::Validation(_))));
    }

    #[test]
    fn greet_returns_message() {
        let state = AppState::local("alloy-test");
        let result = state.greet("Rust");
        assert_eq!(result.expect("must greet"), "Hello, Rust!");
    }
}
