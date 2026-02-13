# production-api

Production-oriented reference example for Meld.

This example demonstrates:
- explicit env configuration validation
- PostgreSQL-backed REST endpoints
- auth-protected REST route (`/protected/*`)
- liveness/health/readiness probes
- single-port REST + gRPC serving

## Prerequisites

- Docker + Docker Compose
- `grpcurl`
- `python3` (for local dev JWT helper)

## 1) Start PostgreSQL

From repository root:

```bash
docker compose -f examples/production-api/docker-compose.yml up -d
```

## 2) Run server

```bash
export PROD_API_DATABASE_URL='postgres://postgres:postgres@127.0.0.1:55432/meld'
export PROD_API_ADDR='127.0.0.1:4100'
export PROD_API_SERVICE_NAME='production-api'
export PROD_API_RUN_MIGRATIONS='true'

# auth enabled for protected REST + gRPC
export MELD_AUTH_ENABLED='true'
export MELD_AUTH_JWT_SECRET='dev-secret'
export MELD_AUTH_ISSUER='https://issuer.local'
export MELD_AUTH_AUDIENCE='meld-api'

cargo run -p production-api
```

## 3) Probe endpoints

```bash
curl -s http://127.0.0.1:4100/livez
curl -s http://127.0.0.1:4100/health
curl -s -i http://127.0.0.1:4100/readyz
```

## 4) DB-backed REST smoke test

```bash
curl -s -X POST http://127.0.0.1:4100/v1/notes \
  -H 'content-type: application/json' \
  -d '{"title":"Production note","body":"hello"}'

curl -s 'http://127.0.0.1:4100/v1/notes?limit=10'
```

## 5) Auth-protected REST path

Without token (expected `401`):

```bash
curl -i http://127.0.0.1:4100/protected/notes/1
```

Create dev token:

```bash
TOKEN=$(python3 scripts/generate_dev_jwt.py \
  --secret dev-secret \
  --issuer https://issuer.local \
  --audience meld-api)
```

Call with token (expected `200` when note exists):

```bash
curl -s http://127.0.0.1:4100/protected/notes/1 \
  -H "authorization: Bearer ${TOKEN}"
```

## 6) gRPC call path on same port

Without token (expected `UNAUTHENTICATED`):

```bash
grpcurl -plaintext \
  -import-path crates/meld-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:4100 \
  meld.v1.Greeter/SayHello
```

With token (expected success):

```bash
grpcurl -plaintext \
  -H "authorization: Bearer ${TOKEN}" \
  -import-path crates/meld-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:4100 \
  meld.v1.Greeter/SayHello
```

## Failure Recovery Basics

1. Stop PostgreSQL:

```bash
docker compose -f examples/production-api/docker-compose.yml stop postgres
```

2. Readiness should fail (`503`):

```bash
curl -i http://127.0.0.1:4100/readyz
```

3. Start PostgreSQL again:

```bash
docker compose -f examples/production-api/docker-compose.yml start postgres
```

4. Readiness should recover (`200`):

```bash
curl -i http://127.0.0.1:4100/readyz
```

## Troubleshooting

- `missing required environment variable: PROD_API_DATABASE_URL`
  - Set `PROD_API_DATABASE_URL` before running.
- `401 unauthorized` on `/protected/*`
  - Verify `MELD_AUTH_ENABLED`, `MELD_AUTH_JWT_SECRET`, `MELD_AUTH_ISSUER`, `MELD_AUTH_AUDIENCE`.
  - Recreate token with `scripts/generate_dev_jwt.py`.
- `UNAUTHENTICATED` in gRPC call
  - Confirm `authorization: Bearer <token>` metadata and issuer/audience match.
- `503` from `/readyz`
  - Check PostgreSQL container health and DB URL host/port/database credentials.
- `relation \"notes\" does not exist`
  - Ensure migrations are enabled (`PROD_API_RUN_MIGRATIONS=true`) and wait for migration retry logs to succeed.

## Shutdown / Cleanup

```bash
docker compose -f examples/production-api/docker-compose.yml down -v
```
