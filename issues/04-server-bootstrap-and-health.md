# [Issue] Bootstrap alloy-server and Implement REST Health Endpoint

## Background
- We need a stable server startup path before multiplexing protocols.

## Goal
- Run Axum server and expose `/health`.

## Tasks
- Initialize `tokio::main` and tracing subscriber
- Build base Axum `Router`
- Implement `GET /health` and root route
- Prepare config/port loading path

## Acceptance Criteria
- `curl http://localhost:<port>/health` returns `200 OK`
- Startup address and request logs are emitted

## Suggested Labels
- `type:feature`
- `area:server`
- `priority:high`
