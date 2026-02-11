# [Issue] Bridge gRPC Contracts to Human-Readable API Docs (Swagger-Compatible Strategy)

## Background
- gRPC has explicit contracts in `.proto`, but product/users still need discoverable docs similar to Swagger.

## Goal
- Define and implement a documentation strategy that maps gRPC contracts into browsable API docs.

## Tasks
- Evaluate approaches:
  - Proto-first docs portal (protobuf descriptors/reflection)
  - gRPC-gateway/OpenAPI generation for HTTP facade
  - Buf/protoc plugins for doc generation
- Select one canonical strategy for Alloy
- Add build step/tooling to generate docs from `.proto`
- Publish docs endpoint or artifact path and usage guide

## Acceptance Criteria
- Team can discover gRPC methods/messages/errors from generated docs
- Generation is reproducible in local and CI
- Chosen approach is documented with trade-offs and limitations

## Suggested Labels
- `type:research`
- `area:grpc-docs`
- `priority:high`
