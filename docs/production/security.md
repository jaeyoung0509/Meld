# Security Baseline

This page defines a practical secure baseline for deploying Openportio.

## Authentication

Use JWT auth in production:

- `OPENPORTIO_AUTH_ENABLED=true`
- `OPENPORTIO_AUTH_JWT_SECRET` must be set
- `OPENPORTIO_AUTH_ISSUER` recommended
- `OPENPORTIO_AUTH_AUDIENCE` recommended

If auth is disabled, protected routes are not enforcing identity and gRPC auth interceptor is bypassed.

## CORS

Do not use wildcard CORS in production:

- Avoid `OPENPORTIO_CORS_ALLOW_ORIGINS=*`
- Use explicit allowlist:
  - `OPENPORTIO_CORS_ALLOW_ORIGINS=https://app.example.com,https://admin.example.com`

## Request Hardening

- Set request timeout: `OPENPORTIO_TIMEOUT_SECONDS`
- Set request body size limit: `OPENPORTIO_REQUEST_BODY_LIMIT_BYTES`
- Set concurrency cap: `OPENPORTIO_MAX_IN_FLIGHT_REQUESTS`

## Secret Management

- Never commit JWT secrets to git
- Rotate `OPENPORTIO_AUTH_JWT_SECRET` regularly
- Inject secrets via secret manager / runtime environment

## Preflight Security Check

Run before release:

```bash
OPENPORTIO_PREFLIGHT_SECURE=true \
OPENPORTIO_PREFLIGHT_BOOT_SERVER=true \
OPENPORTIO_AUTH_ENABLED=true \
OPENPORTIO_AUTH_JWT_SECRET=replace-me \
OPENPORTIO_CORS_ALLOW_ORIGINS=https://app.example.com \
./scripts/prod_preflight.sh
```

Expected behavior:
- exits non-zero on critical failures
- emits warnings for unsafe but non-fatal settings
