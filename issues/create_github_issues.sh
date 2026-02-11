#!/usr/bin/env bash
set -euo pipefail

REPO="jaeyoung0509/alloy"

create_issue() {
  local title="$1"
  local file="$2"
  gh issue create \
    --repo "$REPO" \
    --title "$title" \
    --body-file "$file"
}

create_issue "Scaffold Rust Workspace (alloy-core / alloy-rpc / alloy-server)" "issues/01-workspace-scaffold.md"
create_issue "Define Shared Domain/Error/AppState in alloy-core" "issues/02-core-domain-state.md"
create_issue "Build gRPC Proto + Codegen Pipeline (alloy-rpc)" "issues/03-rpc-proto-and-codegen.md"
create_issue "Bootstrap alloy-server and Implement REST Health Endpoint" "issues/04-server-bootstrap-and-health.md"
create_issue "Implement Single-Port REST (HTTP/1.1) + gRPC (HTTP/2) Multiplexing" "issues/05-single-port-rest-grpc-multiplexing.md"
create_issue "Apply Shared Middleware and Observability for REST + gRPC" "issues/06-shared-middleware-observability.md"
create_issue "Generate OpenAPI/Swagger for REST Endpoints" "issues/07-openapi-swagger-for-rest.md"
create_issue "Bridge gRPC Contracts to Human-Readable API Docs (Swagger-Compatible Strategy)" "issues/08-grpc-contract-doc-bridge.md"
create_issue "Design FastAPI-Like Alloy Builder API" "issues/09-fastapi-like-builder-api.md"
create_issue "Add End-to-End Tests and CI for REST + gRPC + Docs" "issues/10-integration-tests-and-ci.md"
