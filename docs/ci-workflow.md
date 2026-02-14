# CI Workflow

This project uses focused CI jobs so failures are clearly scoped:

1. `Core Build And Test`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo test -p production-api -- --nocapture`

2. `REST gRPC E2E`
- `cargo test -p meld-server --test multiplexing -- --nocapture`

3. `Docs Contract Drift Check`
- `./scripts/check_contracts_bundle.sh`
- `cargo test -p meld-server openapi_json_is_available -- --nocapture`

4. `Security Audit`
- `cargo audit`

5. `Production Preflight`
- `./scripts/prod_preflight.sh` with secure-mode CI environment

6. `Release Dry Run`
- `./scripts/release_dry_run.sh`

## Local Equivalent

Run:

```bash
./scripts/ci_local.sh
```

This runs the same command set as CI in a single local flow.
