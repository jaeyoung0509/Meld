# gRPC Contract Documentation

Generated from `crates/alloy-rpc/proto/service.proto`.

## Package

- `alloy.v1`

## Services And Methods

### `Greeter`

- `SayHello`: `HelloRequest` -> `HelloResponse`

## Messages

### `HelloRequest`

- `name` (`string`, field #1)

### `HelloResponse`

- `message` (`string`, field #1)

## gRPC Error Model

Common status codes exposed by the runtime:

- `INVALID_ARGUMENT` (3): validation failures
- `INTERNAL` (13): unexpected server failures

## Artifacts

- Markdown: `docs/generated/grpc-contracts.md`
- OpenAPI bridge: `docs/generated/grpc-openapi-bridge.json`

