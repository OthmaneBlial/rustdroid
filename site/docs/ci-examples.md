# CI Examples

RustDroid already ships GitHub Actions workflows for fast checks, host integration, release packaging, and a manual crates.io readiness check. Publication itself is intentionally not automated by default.

These examples show the intended usage pattern.

## Fast Checks Only

Use the built-in CI workflow for formatting, clippy, tests, build, and shell validation.

Relevant files:

- `.github/workflows/ci.yml`
- `scripts/ci-shell-check.sh`

## Host Integration Shape

The host integration workflow expects:

- Android emulator support on the runner
- writable `/dev/kvm` access for the job user on Linux runners
- a real AVD
- host integration scripts from `scripts/ci-host-check.sh`

That workflow uploads failure artifacts and performance artifacts.
It is kept separate from the default PR CI so the emulator lane can fail without blocking the rest of the pipeline.

## Adopt the Receipt Action

For a repository-owned APK smoke check, provision KVM and an AVD in the job, then use the immutable [RustDroid action revision](github-action.md). It creates JSON, HTML, JUnit, Markdown, and log evidence in one directory and appends the short Markdown result to the job summary.

The repository's [`action-contract.yml`](../.github/workflows/action-contract.yml) invokes the exact pinned snippet against `tests/fixtures/apks/launch-success.apk` each week. It is the reference shape to copy before replacing the fixture path with a Gradle output.

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
