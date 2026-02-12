# crates.io Publish Runbook

This runbook documents the reproducible release flow for Meld crates.

## Publish Targets

- `meld-core`
- `meld-macros`
- `meld-rpc`
- `meld-server`

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
  - `meld-core`
  - `meld-macros`
  - `meld-rpc`
  - `meld-server`
- applies local `patch.crates-io` overrides so dry-run can validate dependent crates
  before first crates.io index propagation.

## Publish Order

Use this order to respect dependency graph:

1. `meld-core`
2. `meld-macros`
3. `meld-rpc`
4. `meld-server`

Publish commands:

```bash
cargo publish -p meld-core
cargo publish -p meld-macros
cargo publish -p meld-rpc
cargo publish -p meld-server
```

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
