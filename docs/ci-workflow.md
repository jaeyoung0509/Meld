# CI Workflow

This project uses focused CI jobs so failures are clearly scoped:

1. `Core Build And Test`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo test -p production-api -- --nocapture`

2. `Nextest`
- `cargo nextest run --workspace --all-targets`

3. `Coverage`
- `cargo llvm-cov --workspace --summary-only`
- `cargo llvm-cov --workspace --lcov --output-path target/coverage/lcov.info`
- uploads `coverage-report` artifact (summary + lcov)

4. `REST gRPC E2E`
- `cargo test -p meld-server --test multiplexing -- --nocapture`

5. `Docs Contract Drift Check`
- `./scripts/check_contracts_bundle.sh`
- `cargo test -p meld-server openapi_json_is_available -- --nocapture`

6. `Security Audit`
- `cargo audit`

7. `Production Preflight`
- `./scripts/prod_preflight.sh` with secure-mode CI environment

8. `Release Dry Run`
- `./scripts/release_dry_run.sh`

## Local Equivalent

Run:

```bash
./scripts/ci_local.sh
```

This runs the same command set as CI in a single local flow.

For dedicated nextest + coverage quality gates, run:

```bash
./scripts/test_quality.sh
```

See `docs/testing-toolchain.md` for installation and outputs.
