# production-api Runbook

This runbook covers local boot, migration behavior, smoke tests, and dependency failure recovery for `examples/production-api`.

## Environment Variables

Required:

- `PROD_API_DATABASE_URL` (for example `postgres://postgres:postgres@127.0.0.1:55432/meld`)

Optional (defaults shown):

- `PROD_API_ADDR=127.0.0.1:4100`
- `PROD_API_SERVICE_NAME=production-api`
- `PROD_API_DB_MAX_CONNECTIONS=10`
- `PROD_API_RUN_MIGRATIONS=true`
- `PROD_API_MIGRATION_RETRY_SECONDS=5`

Auth-related (recommended for realistic production flow):

- `MELD_AUTH_ENABLED=true`
- `MELD_AUTH_JWT_SECRET=<secret>`
- `MELD_AUTH_ISSUER=<issuer>`
- `MELD_AUTH_AUDIENCE=<audience>`

## Boot Procedure

1. Start PostgreSQL:

```bash
docker compose -f examples/production-api/docker-compose.yml up -d
```

2. Export env vars and start API:

```bash
export PROD_API_DATABASE_URL='postgres://postgres:postgres@127.0.0.1:55432/meld'
export MELD_AUTH_ENABLED='true'
export MELD_AUTH_JWT_SECRET='dev-secret'
export MELD_AUTH_ISSUER='https://issuer.local'
export MELD_AUTH_AUDIENCE='meld-api'

cargo run -p production-api
```

3. Verify probes:

```bash
curl -s http://127.0.0.1:4100/livez
curl -s http://127.0.0.1:4100/health
curl -i http://127.0.0.1:4100/readyz
```

## Migration Behavior

- Migrations are loaded from `examples/production-api/migrations`.
- On startup, migration execution runs in a retry loop when `PROD_API_RUN_MIGRATIONS=true`.
- If DB is temporarily unavailable, the service remains up and retries migrations every `PROD_API_MIGRATION_RETRY_SECONDS` seconds.

## Smoke Tests

REST:

```bash
curl -s -X POST http://127.0.0.1:4100/v1/notes \
  -H 'content-type: application/json' \
  -d '{"title":"hello","body":"world"}'

curl -s 'http://127.0.0.1:4100/v1/notes?limit=5'
```

gRPC:

```bash
TOKEN=$(python3 scripts/generate_dev_jwt.py \
  --secret dev-secret \
  --issuer https://issuer.local \
  --audience meld-api)

grpcurl -plaintext \
  -H "authorization: Bearer ${TOKEN}" \
  -import-path crates/meld-rpc/proto \
  -proto service.proto \
  -d '{"name":"Rust"}' \
  127.0.0.1:4100 \
  meld.v1.Greeter/SayHello
```

## Failure Recovery

Scenario: PostgreSQL outage.

1. Simulate outage:

```bash
docker compose -f examples/production-api/docker-compose.yml stop postgres
```

2. Observe readiness failure (`503`):

```bash
curl -i http://127.0.0.1:4100/readyz
```

3. Recover DB:

```bash
docker compose -f examples/production-api/docker-compose.yml start postgres
```

4. Confirm readiness returns `200`.

If readiness does not recover:
- verify DB container health (`docker ps` / `docker logs`)
- verify `PROD_API_DATABASE_URL`
- verify migrations path and logs for migration retry errors

## Shutdown

```bash
docker compose -f examples/production-api/docker-compose.yml down -v
```
