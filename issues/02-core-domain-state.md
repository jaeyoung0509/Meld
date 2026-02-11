# [Issue] Define Shared Domain/Error/AppState in alloy-core

## Background
- State-sharing strategy between Axum handlers and Tonic services should be fixed early.

## Goal
- Define shared `Arc<AppState>` and common error model.

## Tasks
- Design `AppState` (config/repository/metrics handles)
- Define a common error enum and conversion paths
- Define reusable traits/service interfaces for REST and gRPC layers

## Acceptance Criteria
- `alloy-core` compiles and/or passes standalone tests
- `AppState` can be consumed from both `alloy-server` and `alloy-rpc`

## Suggested Labels
- `type:feature`
- `area:core`
- `priority:high`
