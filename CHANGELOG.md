# Changelog

All notable changes to this project will be documented in this file.

The format is inspired by Keep a Changelog, and versioning currently follows `0.y.z` policy.

## [Unreleased]

### Added
- Release dry-run pipeline for publishable crates
- Production preflight gate and production operations docs
- Unified REST+gRPC contracts bundle generation and validation

### Changed
- Rebranded workspace and crates from `meld-*` to `openportio-*`
- Switched primary runtime env prefix from `MELD_*` to `OPENPORTIO_*` with compatibility aliases retained

## [0.1.0-rc.1] - TBD

### Added
- Initial public release candidate for Openportio core/runtime crates
- Single-port REST + gRPC serving baseline
- FastAPI-like DTO/validation/DI ergonomics
- REST OpenAPI and gRPC contract discovery artifacts

### Security
- Shared auth runtime config for REST and gRPC
- Production preflight checks for secure baseline validation
