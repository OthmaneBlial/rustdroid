# Configuration ownership

RustDroid reads `rustdroid.toml` by default. Use `--config .rustdroid.toml` when a project deliberately keeps a dotfile, then commit that file only when the AVD name and backend are shared project assumptions.

The reviewed starting points live in [`examples/configs/`](../examples/configs/):

- [`host-fast.rustdroid.toml`](../examples/configs/host-fast.rustdroid.toml) for a visible host loop;
- [`headless-ci.rustdroid.toml`](../examples/configs/headless-ci.rustdroid.toml) for a host receipt in CI;
- [`low-ram.rustdroid.toml`](../examples/configs/low-ram.rustdroid.toml) for constrained Docker hosts.

## Precedence

Lowest to highest precedence:

1. RustDroid defaults.
2. The selected profile inside the config file (`profile` or `extends`).
3. Explicit values in that config file.
4. `RUSTDROID_PROFILE`, then the supported `RUSTDROID_*` environment overrides.
5. `--profile`.
6. Explicit CLI flags such as `--runtime-backend`, `--host-avd-name`, and `--headless`.

This lets a checked-in file express the common path while CI supplies its own AVD or artifact directory without rewriting project configuration.

## Supported environment overrides

Use only the documented variables so a command remains portable:

```text
RUSTDROID_PROFILE
RUSTDROID_RUNTIME_BACKEND
RUSTDROID_BOOT_MODE
RUSTDROID_IMAGE
RUSTDROID_CONTAINER_NAME
RUSTDROID_HOST_AVD_NAME
RUSTDROID_HOST_EMULATOR_PORT
RUSTDROID_EMULATOR_GPU_MODE
RUSTDROID_UI_BACKEND
RUSTDROID_LOGCAT_FILTERS
RUSTDROID_ARTIFACTS_DIR
```

Example CI override:

```bash
RUSTDROID_HOST_AVD_NAME=test_avd \
RUSTDROID_ARTIFACTS_DIR="$RUNNER_TEMP/rustdroid" \
rustdroid --config .rustdroid.toml run app/build/outputs/apk/debug/app-debug.apk
```

Run `rustdroid config init --profile host-fast` to generate a full file, or copy a narrow example and add only settings your project owns.
