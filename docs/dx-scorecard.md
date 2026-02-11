# DX Upgrade Scorecard (Meld vs FastAPI-like Workflows)

This document tracks the developer-experience upgrade delivered for issue `#48`.

## 1) Validation DTO Boilerplate

Before:

```rust
#[derive(serde::Deserialize, validator::Validate, utoipa::ToSchema)]
struct CreateNoteBody {
    #[validate(length(min = 2, max = 120))]
    title: String,
}
```

After:

```rust
#[meld_server::dto]
struct CreateNoteBody {
    #[validate(length(min = 2, max = 120))]
    title: String,
}
```

Result:
- DTO annotation reduced to one attribute
- Serde decode + validator + OpenAPI schema derive are applied together

## 2) Validation Error Shape

Meld now returns validation failures with stable fields inspired by FastAPI-style payloads:

```json
{
  "code": "validation_error",
  "message": "request validation failed",
  "detail": [
    { "loc": ["body", "title"], "msg": "length", "type": "length" }
  ]
}
```

Result:
- Client-side field-level parsing is straightforward (`detail[*].loc/msg/type`)
- Existing `code/message` structure remains stable

## 3) OpenAPI Boilerplate

Meld OpenAPI generation now auto-injects shared error responses:
- `400` (validation / bad request)
- `500` (internal error)
- `401` for `/protected/*` routes

Result:
- Less per-handler response annotation boilerplate
- Uniform docs for error models across endpoints

## 4) DI Ergonomics

Added provider-style override utilities:
- `MeldServer::with_dependency(value)`
- `meld_server::di::with_dependency(...)`
- `meld_server::di::with_dependency_overrides(...)`

Result:
- Cleaner test-time wiring for multiple dependencies
- Request-scoped dependency caching still guaranteed

## 5) Error Mapping Policy

REST and gRPC now share one domain-error mapping policy:
- Validation errors preserve actionable messages
- Internal errors are sanitized for clients and fully logged on server side

Result:
- Safer external error surface
- Better observability without leaking internals
