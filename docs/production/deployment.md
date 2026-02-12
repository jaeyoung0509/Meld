# Production Deployment

This guide describes minimum production deployment patterns for Meld.

## Runtime Prerequisites

- Rust binary built from `meld-server`
- Port binding for your target address (default `127.0.0.1:3000`)
- Reverse proxy / ingress for TLS termination

## Required Environment Variables (Secure Baseline)

- `MELD_AUTH_ENABLED=true`
- `MELD_AUTH_JWT_SECRET=<strong-random-secret>`
- `MELD_AUTH_ISSUER=<issuer-url>` (recommended)
- `MELD_AUTH_AUDIENCE=<audience>` (recommended)
- `MELD_CORS_ALLOW_ORIGINS=<comma-separated allowlist>`

Recommended hardening:
- `MELD_TIMEOUT_SECONDS=15` (or lower based on SLO)
- `MELD_REQUEST_BODY_LIMIT_BYTES=1048576` (or stricter)
- `MELD_MAX_IN_FLIGHT_REQUESTS=1024` (size to capacity)

## Local Production-Like Run

```bash
MELD_SERVER_ADDR=127.0.0.1:3000 \
MELD_AUTH_ENABLED=true \
MELD_AUTH_JWT_SECRET=replace-me \
MELD_AUTH_ISSUER=https://issuer.local \
MELD_AUTH_AUDIENCE=meld-api \
MELD_CORS_ALLOW_ORIGINS=https://app.example.com \
cargo run -p meld-server
```

## Preflight Gate (Recommended Before Rollout)

```bash
MELD_PREFLIGHT_SECURE=true \
MELD_PREFLIGHT_BOOT_SERVER=true \
MELD_AUTH_ENABLED=true \
MELD_AUTH_JWT_SECRET=replace-me \
MELD_AUTH_ISSUER=https://issuer.local \
MELD_AUTH_AUDIENCE=meld-api \
MELD_CORS_ALLOW_ORIGINS=https://app.example.com \
./scripts/prod_preflight.sh
```

## Systemd Example

```ini
[Unit]
Description=Meld Server
After=network.target

[Service]
WorkingDirectory=/opt/meld
ExecStart=/opt/meld/meld-server
Restart=always
RestartSec=5
Environment=MELD_SERVER_ADDR=0.0.0.0:3000
Environment=MELD_AUTH_ENABLED=true
Environment=MELD_AUTH_JWT_SECRET=replace-me
Environment=MELD_CORS_ALLOW_ORIGINS=https://app.example.com

[Install]
WantedBy=multi-user.target
```

## Kubernetes Notes

- Use readiness probe on `/health`
- Put secrets in `Secret`, non-sensitive config in `ConfigMap`
- Terminate TLS at ingress and forward to Meld over private network
