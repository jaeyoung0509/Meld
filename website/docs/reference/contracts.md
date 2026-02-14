# Contract Artifacts

Openportio generates contract artifacts to keep REST and gRPC documentation in sync.

## Generated Outputs

- `docs/generated/rest-openapi.json`
- `docs/generated/grpc-contracts.md`
- `docs/generated/grpc-openapi-bridge.json`
- `docs/generated/contracts-bundle.json`

## Commands

Generate artifacts:

```bash
./scripts/generate_contracts_bundle.sh
```

Check drift:

```bash
./scripts/check_contracts_bundle.sh
```

## Why It Matters

- prevents REST/gRPC contract drift
- improves PR review confidence
- gives downstream clients stable contract sources

## Deep References

- [`contracts/links.toml`](https://github.com/jaeyoung0509/Openportio/blob/develop/contracts/links.toml)
- [`docs/generated/`](https://github.com/jaeyoung0509/Openportio/tree/develop/docs/generated)
- [`scripts/generate_contracts_bundle.py`](https://github.com/jaeyoung0509/Openportio/blob/develop/scripts/generate_contracts_bundle.py)
