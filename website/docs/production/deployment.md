# Production Deployment Checklist

## Runtime Baseline

- Health endpoint wired (`/health`)
- OpenAPI endpoint wired (`/openapi.json`)
- gRPC contract docs reachable (`/grpc/contracts`)
- Auth/env variables configured per environment

## Quality Gates

Run before deployment:

```bash
./scripts/ci_local.sh
./scripts/prod_preflight.sh
```

## Docs Site Build Gate

Docs portal must build cleanly:

```bash
cd website
npm ci
npm run docs:build
```

## Suggested Rollout

1. Promote reviewed changes to `develop`.
2. Sync `develop -> main` with release PR.
3. Run release workflow and verify crates publish.
4. Publish docs site artifact/deployment.

## Deep References

- [`docs/production/deployment.md`](https://github.com/jaeyoung0509/Openportio/blob/develop/docs/production/deployment.md)
- [`docs/release/publish-runbook.md`](https://github.com/jaeyoung0509/Openportio/blob/develop/docs/release/publish-runbook.md)
