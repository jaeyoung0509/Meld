# [Issue] Implement Single-Port REST (HTTP/1.1) + gRPC (HTTP/2) Multiplexing

## Background
- Core product requirement is serving REST and gRPC from one listener.

## Goal
- Integrate with `tonic::transport::server::Routes::into_axum_router()` and merged routing.

## Tasks
- Merge REST router and gRPC-converted router
- Serve both via one `TcpListener` using `axum::serve`
- Add validation script for `curl` + `grpcurl`
- Document fallback manual routing strategy (header/content-type based)

## Acceptance Criteria
- REST request succeeds on shared port
- gRPC request succeeds on shared port
- Build passes without response body type mismatch errors

## Suggested Labels
- `type:feature`
- `area:multiplexing`
- `priority:critical`
