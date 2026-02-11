# [Issue] Add End-to-End Tests and CI for REST + gRPC + Docs

## Background
- Protocol integration and doc generation need regression protection.

## Goal
- Establish E2E tests and CI checks for core Alloy capabilities.

## Tasks
- Add integration tests for shared-port REST and gRPC calls
- Add contract tests for proto/codegen compatibility
- Add checks for OpenAPI generation and docs endpoint availability
- Configure CI pipeline (`check`, `test`, lint, docs generation)

## Acceptance Criteria
- CI validates build/test/docs on every PR
- Failures clearly identify whether REST, gRPC, or docs path regressed
- Local developer workflow matches CI commands

## Suggested Labels
- `type:feature`
- `area:ci`
- `priority:high`
