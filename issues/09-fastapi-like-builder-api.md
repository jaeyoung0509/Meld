# [Issue] Design FastAPI-Like Alloy Builder API

## Background
- We want a framework-level developer experience beyond raw Axum/Tonic wiring.

## Goal
- Provide ergonomic builder API like `AlloyServer::new().with_rest(...).with_grpc(...).run()`.

## Tasks
- Design builder types and method chaining
- Support app state injection and lifecycle hooks
- Expose middleware registration points
- Add examples that mirror FastAPI-style quick start

## Acceptance Criteria
- Minimal app can boot with a small, fluent setup API
- REST and gRPC service registration is clear and type-safe
- API surface is documented with examples

## Suggested Labels
- `type:feature`
- `area:framework`
- `priority:medium`
