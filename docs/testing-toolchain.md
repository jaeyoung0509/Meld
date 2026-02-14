# Testing Toolchain

This project uses an extended test toolchain for faster execution and measurable reliability:

- `cargo nextest` for stable and parallel test execution
- `cargo llvm-cov` for workspace coverage reports
- property tests (`proptest`) for API invariants

## Install Tools

```bash
cargo install cargo-nextest --locked
cargo install cargo-llvm-cov --locked
rustup component add llvm-tools-preview
```

## Run Local Quality Gate

```bash
./scripts/test_quality.sh
```

Expected outputs:

- `target/coverage/summary.txt`
- `target/coverage/lcov.info`

## Run Individual Commands

```bash
cargo nextest run --workspace --all-targets
cargo llvm-cov --workspace --summary-only
cargo llvm-cov --workspace --lcov --output-path target/coverage/lcov.info
```

## Property Tests

Property-based API invariants are in:

- `crates/openportio-server/tests/property_api.rs`

They validate:

- validation error shape stability
- REST/gRPC domain error mapping invariants
- DTO boundary behavior
