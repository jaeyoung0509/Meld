# gRPC Contract Documentation

Generated from protobuf descriptor set (descriptor-based parser).

## Packages

- `alloy.v1`

## Services And Methods

### `alloy.v1.Greeter`

- `SayHello`: `alloy.v1.HelloRequest` -> `alloy.v1.HelloResponse` (client_streaming=false, server_streaming=false)

## Messages

### `alloy.v1.HelloRequest`

- `name` (`string`, field #1)

### `alloy.v1.HelloResponse`

- `message` (`string`, field #1)

## Enums


## gRPC Error Model

Common status codes exposed by the runtime:

- `INVALID_ARGUMENT` (3): validation failures
- `INTERNAL` (13): unexpected server failures

## Artifacts

- Markdown: `docs/generated/grpc-contracts.md`
- OpenAPI bridge: `docs/generated/grpc-openapi-bridge.json`
