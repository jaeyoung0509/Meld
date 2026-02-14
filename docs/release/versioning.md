# Versioning Policy

Openportio follows an initial `0.y.z` policy until first stable `1.0.0`.

## Baseline

- Workspace crates share the same release version during the `0.x` phase.
- Releases are coordinated for:
  - `openportio-core`
  - `openportio-macros`
  - `openportio-rpc`
  - `openportio-server`

## Bump Rules

- `z` (patch): backward-compatible fixes/docs/internal changes
- `y` (minor): backward-incompatible API changes or major feature milestones during `0.x`

## Release Notes Expectations

Each release should include:
- notable API changes
- migration notes (if any)
- security-impacting changes
- CI/release pipeline updates

## Rename Migration (Meld -> Openportio)

- Crate names moved to `openportio-*`.
- Runtime env keys now use `OPENPORTIO_*`.
- Backward-compatible env aliases (`MELD_*`) are still accepted for now, but considered deprecated.
- Release notes must call out when deprecated aliases are removed.
