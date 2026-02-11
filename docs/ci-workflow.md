# CI Workflow

This project uses focused CI jobs so failures are clearly scoped:

1. `Core Build And Test`
- `cargo check --workspace`
- `cargo test --workspace`

2. `REST gRPC E2E`
- `cargo test -p meld-server --test multiplexing -- --nocapture`

3. `Docs Contract Drift Check`
- `./scripts/check_contracts_bundle.sh`
- `cargo test -p meld-server openapi_json_is_available -- --nocapture`

4. `Security Audit`
- `cargo audit`

## Local Equivalent

Run:

```bash
./scripts/ci_local.sh
```

This runs the same command set as CI in a single local flow.
