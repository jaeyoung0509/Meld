# [Issue] Apply Shared Middleware and Observability for REST + gRPC

## Background
- Unified runtime requires consistent logging, tracing, and cross-cutting controls.

## Goal
- Apply common middleware stack for both REST and gRPC traffic.

## Tasks
- Add `TraceLayer`/structured logging for HTTP and gRPC
- Add request ID propagation strategy
- Add CORS policy for REST endpoints
- Define timeout/retry/concurrency limits where applicable

## Acceptance Criteria
- REST and gRPC requests produce traceable logs with correlation IDs
- Middleware configuration is centralized and documented
- No protocol regression introduced by middleware stack

## Suggested Labels
- `type:feature`
- `area:observability`
- `priority:medium`
