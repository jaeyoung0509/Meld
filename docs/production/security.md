# Security Baseline

This page defines a practical secure baseline for deploying Meld.

## Authentication

Use JWT auth in production:

- `MELD_AUTH_ENABLED=true`
- `MELD_AUTH_JWT_SECRET` must be set
- `MELD_AUTH_ISSUER` recommended
- `MELD_AUTH_AUDIENCE` recommended

If auth is disabled, protected routes are not enforcing identity and gRPC auth interceptor is bypassed.

## CORS

Do not use wildcard CORS in production:

- Avoid `MELD_CORS_ALLOW_ORIGINS=*`
- Use explicit allowlist:
  - `MELD_CORS_ALLOW_ORIGINS=https://app.example.com,https://admin.example.com`

## Request Hardening

- Set request timeout: `MELD_TIMEOUT_SECONDS`
- Set request body size limit: `MELD_REQUEST_BODY_LIMIT_BYTES`
- Set concurrency cap: `MELD_MAX_IN_FLIGHT_REQUESTS`

## Secret Management

- Never commit JWT secrets to git
- Rotate `MELD_AUTH_JWT_SECRET` regularly
- Inject secrets via secret manager / runtime environment

## Preflight Security Check

Run before release:

```bash
MELD_PREFLIGHT_SECURE=true \
MELD_PREFLIGHT_BOOT_SERVER=true \
MELD_AUTH_ENABLED=true \
MELD_AUTH_JWT_SECRET=replace-me \
MELD_CORS_ALLOW_ORIGINS=https://app.example.com \
./scripts/prod_preflight.sh
```

Expected behavior:
- exits non-zero on critical failures
- emits warnings for unsafe but non-fatal settings
