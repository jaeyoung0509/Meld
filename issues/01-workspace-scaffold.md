# [Issue] Scaffold Rust Workspace (alloy-core / alloy-rpc / alloy-server)

## Background
- We have architecture research, but no runnable workspace yet.
- We need crate separation before implementing REST + gRPC integration.

## Goal
- Create a Cargo workspace with three core crates.

## Tasks
- Define workspace members and shared dependencies in root `Cargo.toml`
- Create `crates/alloy-core`
- Create `crates/alloy-rpc`
- Create `crates/alloy-server`
- Create `examples/simple-server` (optional)

## Acceptance Criteria
- `cargo check --workspace` succeeds
- Each crate builds with minimal `lib.rs` / `main.rs`

## Suggested Labels
- `type:chore`
- `area:workspace`
- `priority:high`
