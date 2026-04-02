# RustDroid Test Layout

RustDroid keeps test scope explicit.

## Categories

- `src/**` unit tests:
  fast logic checks for config parsing, CLI mapping, report generation, and helpers.
- `tests/integration_*.rs`:
  command-level tests that execute the compiled `rustdroid` binary with isolated temp config.
- `tests/smoke_*.rs`:
  cheap user-path checks that prove a critical command shape still works without requiring a full emulator boot.
- `tests/release_*.rs`:
  packaging and release-contract checks for scripts, workflow files, and install-facing assets.
- `tests/fixtures/`:
  canonical APK fixtures plus a machine-readable manifest used by integration coverage.

## Naming Rules

- Prefix test files with their category: `integration_`, `smoke_`, `release_`.
- Name tests by behavior, not by file or function name.
- Keep one concern per test file.
- Put all reusable helpers under `tests/common/`.

## Helper Rules

- Use `tests/common/mod.rs` for temp dirs, temp configs, command execution, and output assertions.
- Test code should default to isolated config files and never write into the repo root.
- Runtime-heavy flows should be opt-in and live in dedicated integration suites, not in the cheap smoke layer.
- Refresh checked-in APK fixtures with `./scripts/generate-fixture-apks.sh`.

## Suggested Commands

```bash
cargo test
cargo test --test integration_cli
cargo test --test integration_fixtures
cargo test --test smoke_cli
cargo test --test release_contract
```
