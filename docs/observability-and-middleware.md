# Shared Middleware And Observability

This project applies one centralized middleware stack for both REST and gRPC traffic.

Source:
- `crates/openportio-server/src/middleware.rs`

Included layers:
- `TraceLayer` for structured request tracing
- `SetRequestIdLayer` to generate `x-request-id` when missing
- `PropagateRequestIdLayer` to echo request ID in responses
- `CorsLayer` with permissive origin policy (for REST/browser integration)
- `TimeoutLayer` for request timeout boundaries
- `ConcurrencyLimitLayer` for in-flight request control

Environment variables:
- `OPENPORTIO_TIMEOUT_SECONDS` (default: `15`)
- `OPENPORTIO_MAX_IN_FLIGHT_REQUESTS` (default: `1024`)

Notes:
- Middleware is applied in `crates/openportio-server/src/main.rs`.
- Because the app is multiplexed (REST + gRPC on one listener), these layers are shared by both protocol paths.
