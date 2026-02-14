# Observability Guide

Openportio includes basic observability primitives through middleware and tracing.

## Built-in Signals

- Structured logs via `tracing` / `tracing-subscriber`
- Request ID propagation (`x-request-id`)
- Health endpoint: `GET /health`
- API contract endpoints:
  - `GET /openapi.json`
  - `GET /grpc/contracts`

## Logging Baseline

Set log level using standard Rust tracing environment:

```bash
RUST_LOG=info cargo run -p openportio-server
```

For deeper diagnostics:

```bash
RUST_LOG=debug,openportio_server=debug cargo run -p openportio-server
```

## Runtime Verification

Use preflight endpoint checks:

```bash
OPENPORTIO_PREFLIGHT_BOOT_SERVER=true ./scripts/prod_preflight.sh
```

This validates:
- `/health`
- `/openapi.json`
- `/grpc/contracts`

## Recommended External Integrations

- Centralized log sink (ELK, Loki, Cloud Logging)
- Metrics pipeline via reverse-proxy/service mesh metrics
- Alerting on:
  - health endpoint failures
  - auth failure spikes (401 / UNAUTHENTICATED trends)
  - latency and saturation indicators
