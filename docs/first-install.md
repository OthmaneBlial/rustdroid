# First Install

This guide is for a clean Linux machine.

## Fast Path

Install from the latest release:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/OthmaneBlial/rustdroid/main/install.sh)
```

If the release asset is unavailable, the installer falls back to a local source build.
It installs both `rustdroid` and `rustdroid-run`.

## Host Requirements

RustDroid is built around the Android emulator fast loop, so install these first:

- Rust toolchain
- `adb`
- `aapt` or `apkanalyzer`
- Android SDK emulator
- at least one AVD
- `scrcpy` if you want the desktop UI path

For the host backend, KVM must be available and usable by your user.

## Verify The Install

Run the short health checks:

```bash
rustdroid version
rustdroid doctor
rustdroid self-test --backend host
```

If `doctor` fails, fix the reported issue before trying the main run loop.

## First Run

The fastest local path is usually host backend plus `scrcpy`:

```bash
rustdroid-run host-local app-debug.apk
rustdroid --runtime-backend host --host-avd-name test_avd run app-debug.apk
rustdroid --profile host-fast --host-avd-name test_avd run app-debug.apk
```

For the Docker fast loop:

```bash
rustdroid fast app-debug.apk
rustdroid fast-local app-debug.apk
```

For repeated local work:

```bash
rustdroid watch build/outputs/apk/debug --duration-secs 2 --keep-alive true
```
