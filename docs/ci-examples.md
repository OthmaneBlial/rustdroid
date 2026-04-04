# CI Examples

RustDroid already ships GitHub Actions workflows for fast checks, host integration, release packaging, and crates.io publication.

These examples show the intended usage pattern.

## Fast Checks Only

Use the built-in CI workflow for formatting, clippy, tests, build, and shell validation.

Relevant files:

- `.github/workflows/ci.yml`
- `scripts/ci-shell-check.sh`

## Host Integration Shape

The host integration job expects:

- Android emulator support on the runner
- writable `/dev/kvm` access for the job user on Linux runners
- a real AVD
- host integration scripts from `scripts/ci-host-check.sh`

That job also uploads failure artifacts and performance artifacts.

## Package Validation Shape

The package validation job uses:

- `scripts/check-cargo-distribution.sh`
- `scripts/ci-package-check.sh`
- `scripts/package-release.sh`

This verifies:

- `cargo package`
- `cargo publish --dry-run`
- release archive structure
- local install verification

## Performance Guardrail Shape

Performance tracking uses:

- `docs/performance-baselines.json`
- `scripts/check-performance-baseline.sh`
- `docs/performance-notes/v0.1.0.md`

This is meant to catch major regressions, not benchmark micromanagement.
