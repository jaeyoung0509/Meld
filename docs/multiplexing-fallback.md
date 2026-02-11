# Manual Multiplexing Fallback (Header-Based Routing)

If `Routes::into_axum_router()` cannot be used (for example: temporary version lock or custom dispatch rules), use a manual fallback strategy:

1. Inspect request metadata.
- Route to gRPC branch when `content-type` starts with `application/grpc`.
- Route all other requests to REST branch.

2. Normalize response body type.
- gRPC response body and REST response body may differ at concrete type level.
- Convert gRPC body to `axum::body::Body` using `axum::body::Body::new(...)`.

3. Keep one listener.
- Continue serving from one `TcpListener` with `axum::serve(...)`.
- This preserves the single-port deployment model.

This fallback is more verbose and easier to break than native `Routes::into_axum_router()`, so prefer native routing whenever possible.
