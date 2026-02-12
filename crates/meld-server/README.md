# meld-server

Server runtime crate for Meld.

Includes:
- single-port REST + gRPC serving
- middleware stack (timeouts, request-id, CORS, limits, tracing)
- OpenAPI and gRPC contract discovery endpoints
- FastAPI-like builder and extractor ergonomics
