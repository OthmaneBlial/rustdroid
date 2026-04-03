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
- `tests/integration_host_runtime.rs`:
  opt-in real host-emulator command coverage for the daily APK loop.
- `tests/integration_host_backend.rs`:
  opt-in backend-focused checks for AVD discovery, managed start/stop, artifacts, and optional scrcpy coverage.
- `scripts/run-smoke-matrix.sh`:
  single host-smoke entrypoint for the minimum release-safe runtime flows.

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
- Enable the real host runtime suite with `RUSTDROID_RUN_HOST_RUNTIME_TESTS=1`.
- Point it at a running emulator with `RUSTDROID_HOST_TEST_SERIAL=emulator-5554`.
- Enable the backend-focused host suite with `RUSTDROID_RUN_HOST_BACKEND_TESTS=1`.
- Point backend artifact and scrcpy checks at a running emulator with `RUSTDROID_HOST_TEST_SERIAL=emulator-5554`.
- Enable the scrcpy-specific backend check with `RUSTDROID_RUN_HOST_SCRCPY_TESTS=1` on a runner with a GUI session.
- Run the live smoke checklist with `./scripts/run-smoke-matrix.sh`.
- Override `RUSTDROID_SMOKE_AVD`, `RUSTDROID_SMOKE_PORT`, or `RUSTDROID_BIN` when the default host test lane does not match your machine.
- Override `RUSTDROID_SMOKE_GPU_MODE` when your host emulator needs a different renderer than the default smoke-safe `swiftshader_indirect`.
- Override `RUSTDROID_SMOKE_EMULATOR_ARGS` when your host AVD needs a different launch shape than the default read-only smoke profile.
- Set `RUSTDROID_SMOKE_ENABLE_GUI=1` only on machines where you want the visible `scrcpy` fast lane included.

## Suggested Commands

```bash
cargo test
cargo test --test integration_cli
cargo test --test integration_fixtures
cargo test --test integration_host_runtime
cargo test --test integration_host_backend
cargo test --test smoke_cli
cargo test --test release_contract
./scripts/run-smoke-matrix.sh --list
RUSTDROID_SMOKE_AVD=test_avd ./scripts/run-smoke-matrix.sh
RUSTDROID_RUN_HOST_RUNTIME_TESTS=1 RUSTDROID_HOST_TEST_SERIAL=emulator-5554 cargo test --test integration_host_runtime -- --nocapture
RUSTDROID_RUN_HOST_BACKEND_TESTS=1 cargo test --test integration_host_backend host_backend_detects_avds_and_managed_start_stop -- --nocapture
RUSTDROID_RUN_HOST_BACKEND_TESTS=1 RUSTDROID_HOST_TEST_SERIAL=emulator-5554 cargo test --test integration_host_backend host_backend_run_writes_artifacts_for_running_device -- --nocapture
```
