# Alloy Agent Playbook

This document is the working contract for contributors and AI agents in this repository.
It consolidates what we already know, what we are building next, and how to execute safely.

## 1) Product Direction

Build an Alloy framework in Rust that supports:
- REST API on HTTP/1.1 (`axum` + `serde`)
- gRPC API on HTTP/2 (`tonic` + `prost`)
- Single-port serving for REST + gRPC
- FastAPI-like developer experience (builder-style setup)
- API documentation strategy for both REST and gRPC contracts

## 2) Current Knowledge (Validated from `research.md`)

- Axum + Tonic integration can fail at compile-time due to response body type mismatches.
- Preferred integration path is Tonic `Routes::into_axum_router()` and then `merge` into Axum router.
- If native merge is not available for any reason, body unification can be done with `axum::body::Body::new(...)`.
- Single listener architecture should run via `axum::serve` with a shared `TcpListener`.
- Shared state should be injected as `Arc<AppState>` and consumed consistently across REST/gRPC.

## 3) Architecture Baseline

Recommended workspace layout:
- `crates/alloy-core`: domain, state, errors, shared traits
- `crates/alloy-rpc`: `.proto`, `build.rs`, generated gRPC bindings
- `crates/alloy-server`: startup, routing, middleware, serving
- `examples/simple-server`: minimal runnable example

Dependency baseline (subject to exact lockfile decisions):
- `axum` 0.7
- `tonic` 0.12
- `tokio` 1.x
- `tower` / `tower-http`
- `serde`
- `utoipa` (REST OpenAPI/Swagger)

## 4) Issue Roadmap (GitHub)

Tracking repository: `https://github.com/jaeyoung0509/alloy`

Issue source-of-truth policy:
- GitHub Issues are the canonical source of backlog truth.
- Do not maintain issue detail documents under repository `issues/`.
- Pull issue context with `gh issue` commands (or issue-fetch skill) when planning/implementing work.

Open issues:
- #1 Scaffold Rust Workspace (alloy-core / alloy-rpc / alloy-server)
- #2 Define Shared Domain/Error/AppState in alloy-core
- #3 Build gRPC Proto + Codegen Pipeline (alloy-rpc)
- #4 Bootstrap alloy-server and Implement REST Health Endpoint
- #5 Implement Single-Port REST + gRPC Multiplexing
- #6 Apply Shared Middleware and Observability
- #7 Generate OpenAPI/Swagger for REST Endpoints
- #8 Bridge gRPC Contracts to Human-Readable API Docs
- #9 Design FastAPI-Like Alloy Builder API
- #10 Add End-to-End Tests and CI

Execution order should default to:
1. #1
2. #2
3. #3
4. #4
5. #5
6. #7
7. #8
8. #6
9. #9
10. #10

## 5) Definition of Done (Per Feature)

- Compiles with `cargo check --workspace`
- Has focused tests (unit/integration) where applicable
- Has docs updates (README or crate-level doc comments)
- Does not break shared-port REST + gRPC behavior

## 6) Git & Branching Policy

We use Git Flow.

Branch model:
- `main`: production-ready history
- `develop`: integration branch
- `feature/*`: implementation branches for issues
- `release/*`: release preparation
- `hotfix/*`: urgent fixes from production

Mapping issues to branches:
- Example: issue #1 -> `feature/1-workspace-scaffold`
- Keep one concern per feature branch.

Commit style (recommended):
- `feat: ...`, `fix: ...`, `chore: ...`, `docs: ...`, `test: ...`

## 7) Working Rules for Agents

- Before coding: confirm target issue number and acceptance criteria.
- Prefer minimal vertical slices that are runnable end-to-end.
- Do not introduce broad abstractions before issue #5 baseline is stable.
- Keep API/documentation claims consistent with implemented behavior.
- Prefer explicit tradeoff notes for gRPC docs bridging decisions (#8).

## 8) Commands Checklist

Workspace setup and validation:
- `cargo check --workspace`
- `cargo test --workspace`

GitHub issue operations:
- List: `gh issue list --repo jaeyoung0509/alloy --limit 50`
- View: `gh issue view <number> --repo jaeyoung0509/alloy`
- JSON detail: `gh issue view <number> --repo jaeyoung0509/alloy --json number,title,body,labels,assignees`

Local protocol checks (when server is running):
- REST health: `curl http://localhost:3000/health`
- gRPC check: `grpcurl -plaintext localhost:3000 list`

## 9) Immediate Next Action

Start with issue #1:
- Create workspace root and crate scaffolding.
- Ensure the full workspace compiles before moving to #2.
