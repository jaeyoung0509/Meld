# [Issue] Build gRPC Proto + Codegen Pipeline (alloy-rpc)

## Background
- Contract-first gRPC development requires `.proto` as source of truth.

## Goal
- Set up `proto/` + `build.rs` + `tonic-build` generation flow.

## Tasks
- Create initial `proto/service.proto` (at least one service such as Greeter)
- Configure `tonic-build` in `build.rs`
- Expose generated code in `src/lib.rs`
- Verify version compatibility (`prost`, `tonic`)

## Acceptance Criteria
- `cargo build -p alloy-rpc` succeeds
- Generated server/service traits are usable from server crate

## Suggested Labels
- `type:feature`
- `area:grpc`
- `priority:high`
