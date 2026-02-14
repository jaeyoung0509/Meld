# openportio-server

Server runtime crate for Openportio.

Includes:
- single-port REST + gRPC serving
- middleware stack (timeouts, request-id, CORS, limits, tracing)
- OpenAPI and gRPC contract discovery endpoints
- FastAPI-like builder and extractor ergonomics
- DTO options: `#[dto]`, composable derive aliases, trait-first `RequestValidation`
