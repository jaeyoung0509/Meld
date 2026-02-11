# [Issue] Generate OpenAPI/Swagger for REST Endpoints

## Background
- FastAPI-like DX requires first-class API docs for REST.

## Goal
- Integrate OpenAPI generation (`utoipa`) and Swagger UI for Axum routes.

## Tasks
- Annotate REST handlers and schemas
- Generate OpenAPI document at build/runtime
- Expose Swagger UI endpoint (e.g., `/docs`)
- Keep schema aligned with actual request/response models

## Acceptance Criteria
- OpenAPI JSON is accessible (e.g., `/openapi.json`)
- Swagger UI renders and executes REST endpoints
- CI or checks catch schema drift in documented endpoints

## Suggested Labels
- `type:feature`
- `area:docs`
- `priority:high`
