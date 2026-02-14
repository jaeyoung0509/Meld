use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    Extension, Json, Router,
};

use crate::api::{ApiError, ApiErrorResponse};

#[derive(Debug, Clone)]
pub struct Depends<T>(pub T);

#[derive(Debug, Clone)]
pub struct DependencyOverride<T>(pub T);

#[derive(Clone, Default)]
pub struct DependencyOverrides {
    values: Arc<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl DependencyOverrides {
    pub fn with<T>(mut self, value: T) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        let map = Arc::make_mut(&mut self.values);
        map.insert(TypeId::of::<T>(), Arc::new(value));
        self
    }

    pub fn get<T>(&self) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.values
            .get(&TypeId::of::<T>())
            .and_then(|value| value.as_ref().downcast_ref::<T>())
            .cloned()
    }
}

#[derive(Clone, Default)]
struct DependencyCache {
    values: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

#[derive(Debug)]
enum DependencyCacheError {
    Poisoned,
}

impl DependencyCache {
    fn get<T>(&self) -> Result<Option<T>, DependencyCacheError>
    where
        T: Clone + Send + Sync + 'static,
    {
        let guard = self
            .values
            .lock()
            .map_err(|_| DependencyCacheError::Poisoned)?;
        Ok(guard
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref::<T>())
            .cloned())
    }

    fn insert<T>(&mut self, value: T) -> Result<(), DependencyCacheError>
    where
        T: Clone + Send + Sync + 'static,
    {
        let mut guard = self
            .values
            .lock()
            .map_err(|_| DependencyCacheError::Poisoned)?;
        guard.insert(TypeId::of::<T>(), Box::new(value));
        Ok(())
    }
}

#[axum::async_trait]
impl<T, S> FromRequestParts<S> for Depends<T>
where
    T: FromRef<S> + Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if let Some(cache) = parts.extensions.get::<DependencyCache>() {
            if let Some(value) = cache.get::<T>().map_err(|err| {
                tracing::error!(?err, "failed to read request dependency cache");
                internal_di_error("request dependency cache is unavailable")
            })? {
                return Ok(Self(value));
            }
        }

        let value = if let Some(overrides) = parts.extensions.get::<DependencyOverrides>() {
            if let Some(value) = overrides.get::<T>() {
                value
            } else if let Some(override_value) = parts.extensions.get::<DependencyOverride<T>>() {
                override_value.0.clone()
            } else {
                T::from_ref(state)
            }
        } else if let Some(override_value) = parts.extensions.get::<DependencyOverride<T>>() {
            override_value.0.clone()
        } else {
            T::from_ref(state)
        };

        if let Some(cache) = parts.extensions.get_mut::<DependencyCache>() {
            cache.insert(value.clone()).map_err(|err| {
                tracing::error!(?err, "failed to write request dependency cache");
                internal_di_error("request dependency cache is unavailable")
            })?;
        } else {
            let mut cache = DependencyCache::default();
            cache.insert(value.clone()).map_err(|err| {
                tracing::error!(?err, "failed to initialize request dependency cache");
                internal_di_error("request dependency cache is unavailable")
            })?;
            parts.extensions.insert(cache);
        }

        Ok(Self(value))
    }
}

pub fn with_dependency<S, T>(router: Router<S>, value: T) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    with_dependency_overrides(router, DependencyOverrides::default().with(value))
}

pub fn with_dependency_override<S, T>(router: Router<S>, value: T) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    router.layer(Extension(DependencyOverride(value)))
}

pub fn with_dependency_overrides<S>(router: Router<S>, overrides: DependencyOverrides) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(Extension(overrides))
}

pub fn internal_di_error(message: impl Into<String>) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiErrorResponse {
            code: "internal_error".to_string(),
            message: message.into(),
            detail: None,
            details: None,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::to_bytes, extract::State, http::Request, response::IntoResponse, routing::get,
    };
    use serde::Serialize;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tower::util::ServiceExt;

    #[derive(Clone)]
    struct TestState {
        label: String,
        build_counter: Arc<AtomicUsize>,
    }

    #[derive(Clone)]
    struct LabelDep {
        label: String,
    }

    #[derive(Clone)]
    struct VersionDep {
        version: String,
    }

    impl FromRef<TestState> for VersionDep {
        fn from_ref(_state: &TestState) -> Self {
            Self {
                version: "from-state".to_string(),
            }
        }
    }

    impl FromRef<TestState> for LabelDep {
        fn from_ref(state: &TestState) -> Self {
            state.build_counter.fetch_add(1, Ordering::SeqCst);
            Self {
                label: state.label.clone(),
            }
        }
    }

    #[derive(Serialize)]
    struct DepResponse {
        a: String,
        b: String,
    }

    async fn dep_handler(
        Depends(a): Depends<LabelDep>,
        Depends(b): Depends<LabelDep>,
    ) -> impl IntoResponse {
        Json(DepResponse {
            a: a.label,
            b: b.label,
        })
    }

    #[tokio::test]
    async fn depends_uses_request_scoped_cache() {
        let counter = Arc::new(AtomicUsize::new(0));
        let state = TestState {
            label: "state-value".to_string(),
            build_counter: counter.clone(),
        };
        let app = Router::new()
            .route("/dep", get(dep_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dep")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body = String::from_utf8(bytes.to_vec()).expect("utf8 body");
        assert!(body.contains("state-value"));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn depends_override_wins_over_state_resolution() {
        let counter = Arc::new(AtomicUsize::new(0));
        let state = TestState {
            label: "state-value".to_string(),
            build_counter: counter.clone(),
        };
        let app = with_dependency_override(
            Router::new()
                .route("/dep", get(dep_handler))
                .with_state(state),
            LabelDep {
                label: "override-value".to_string(),
            },
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dep")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body = String::from_utf8(bytes.to_vec()).expect("utf8 body");
        assert!(body.contains("override-value"));
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    async fn state_observe_handler(
        Depends(dep): Depends<LabelDep>,
        State(state): State<TestState>,
    ) -> impl IntoResponse {
        Json(DepResponse {
            a: dep.label,
            b: state.label,
        })
    }

    #[tokio::test]
    async fn depends_is_backward_compatible_with_state_extractor() {
        let state = TestState {
            label: "state-value".to_string(),
            build_counter: Arc::new(AtomicUsize::new(0)),
        };
        let app = Router::new()
            .route("/dep", get(state_observe_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dep")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[derive(Serialize, serde::Deserialize)]
    struct MultiDepResponse {
        label: String,
        version: String,
    }

    async fn multi_dep_handler(
        Depends(label): Depends<LabelDep>,
        Depends(version): Depends<VersionDep>,
    ) -> impl IntoResponse {
        Json(MultiDepResponse {
            label: label.label,
            version: version.version,
        })
    }

    #[tokio::test]
    async fn depends_supports_multiple_overrides() {
        let counter = Arc::new(AtomicUsize::new(0));
        let state = TestState {
            label: "state-value".to_string(),
            build_counter: counter.clone(),
        };
        let overrides = DependencyOverrides::default()
            .with(LabelDep {
                label: "override-label".to_string(),
            })
            .with(VersionDep {
                version: "v2".to_string(),
            });

        let app = with_dependency_overrides(
            Router::new()
                .route("/dep", get(multi_dep_handler))
                .with_state(state),
            overrides,
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dep")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: MultiDepResponse = serde_json::from_slice(&bytes).expect("json body");
        assert_eq!(body.label, "override-label");
        assert_eq!(body.version, "v2");
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
}
