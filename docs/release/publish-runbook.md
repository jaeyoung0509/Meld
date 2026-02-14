# crates.io Publish Runbook

This runbook documents the reproducible release flow for Openportio crates.

Rename context:
- Project name changed from `Meld` to `Openportio`.
- Runtime `MELD_*` env keys remain as compatibility aliases, but all docs/scripts/defaults use `OPENPORTIO_*`.

## Publish Targets

- `openportio-core`
- `openportio-macros`
- `openportio-rpc`
- `openportio-server`

Examples are intentionally non-publishable (`publish = false`).

## Pre-release Checklist

Run from repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/check_contracts_bundle.sh
./scripts/prod_preflight.sh
./scripts/release_dry_run.sh
```

`scripts/release_dry_run.sh` does:
- `cargo publish --dry-run` for all publishable crates:
  - `openportio-core`
  - `openportio-macros`
  - `openportio-rpc`
  - `openportio-server`
- applies local `patch.crates-io` overrides so dry-run can validate dependent crates
  before first crates.io index propagation.

## Publish Order

Use this order to respect dependency graph:

1. `openportio-core`
2. `openportio-macros`
3. `openportio-rpc`
4. `openportio-server`

Publish commands:

```bash
cargo publish -p openportio-core
cargo publish -p openportio-macros
cargo publish -p openportio-rpc
cargo publish -p openportio-server
```

## Automated GitHub Release Path (Recommended)

This repository includes tag-driven automation in `.github/workflows/release.yml`.

Prerequisites:
- GitHub Actions secret: `CRATES_IO_TOKEN`
- `release` environment configured with required reviewer(s)
- protected `main` branch

Execution:

1. Merge `develop` into `main` through a PR.
2. Tag from `main` and push:

```bash
git checkout main
git pull --ff-only origin main
git tag v0.1.0
git push origin v0.1.0
```

3. GitHub Actions will:
- re-run release quality gates
- publish crates in dependency order (`openportio-core` -> `openportio-macros` -> `openportio-rpc` -> `openportio-server`)
- create/update GitHub release notes

The workflow rejects tags that are not reachable from `main`.

## First Release Candidate Tag Procedure

1. Ensure `develop` is green on CI and release checklist is complete.
2. Create release candidate tag:

```bash
git tag v0.1.0-rc.1
git push origin v0.1.0-rc.1
```

3. Validate crates.io dry-run one final time.
4. Publish crates in order above.
5. Create final stable tag once publish is confirmed:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Rollback / Mitigation

crates.io publishes are immutable, so rollback means forward-fix:

- If publish fails mid-sequence:
  - stop immediately
  - document exact failure in release notes
  - patch broken crate(s), bump version, rerun dry-run
- If already published crate has critical issue:
  - publish fixed patch version (`0.1.1`, etc.)
  - yank affected version if needed:

```bash
cargo yank --vers <version> <crate-name>
```
