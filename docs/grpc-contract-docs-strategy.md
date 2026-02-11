# gRPC Contract Docs Strategy

## Goal

Provide human-readable and Swagger-compatible discovery docs for gRPC contracts defined in `.proto`.

## Approaches Evaluated

1. Proto-first docs portal (descriptor/reflection)
- Pros: closest to gRPC source-of-truth
- Cons: less familiar for teams expecting Swagger/OpenAPI UX

2. gRPC-gateway/OpenAPI facade
- Pros: native OpenAPI output for HTTP facade
- Cons: adds gateway runtime and can drift from pure gRPC behavior

3. Buf/protoc plugin docs generation
- Pros: rich ecosystem and mature tooling
- Cons: extra toolchain management and onboarding cost

## Chosen Strategy (Canonical For Alloy)

Use a lightweight proto-first generator that emits:
- Markdown contract docs (`docs/generated/grpc-contracts.md`)
- OpenAPI-compatible bridge JSON (`docs/generated/grpc-openapi-bridge.json`)

Why this choice:
- Keeps `.proto` as source-of-truth
- Produces browsable docs without introducing an HTTP gateway runtime
- Works with current Alloy stack and can run in CI as a deterministic check

## Tooling

- Generator: `scripts/generate_grpc_contract_docs.py`
- Drift check: `scripts/check_grpc_contract_docs.sh`

Reproducible flow:
1. Run `python3 scripts/generate_grpc_contract_docs.py`
2. Commit updated artifacts under `docs/generated/`
3. In CI, run `scripts/check_grpc_contract_docs.sh` and fail on drift

## Published Paths

- Artifact paths:
  - `docs/generated/grpc-contracts.md`
  - `docs/generated/grpc-openapi-bridge.json`
- Runtime endpoints:
  - `GET /grpc/contracts`
  - `GET /grpc/contracts/openapi.json`

## Limitations

- OpenAPI bridge is contract-discovery oriented, not a drop-in REST execution spec.
- Advanced protobuf constructs (oneof/map/nested complexity) are intentionally simplified in the bridge generator.
