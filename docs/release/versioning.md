# Versioning Policy

Meld follows an initial `0.y.z` policy until first stable `1.0.0`.

## Baseline

- Workspace crates share the same release version during the `0.x` phase.
- Releases are coordinated for:
  - `meld-core`
  - `meld-macros`
  - `meld-rpc`
  - `meld-server`

## Bump Rules

- `z` (patch): backward-compatible fixes/docs/internal changes
- `y` (minor): backward-incompatible API changes or major feature milestones during `0.x`

## Release Notes Expectations

Each release should include:
- notable API changes
- migration notes (if any)
- security-impacting changes
- CI/release pipeline updates
