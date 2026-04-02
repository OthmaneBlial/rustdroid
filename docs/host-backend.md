# Host Backend

The host backend is the default performance path for RustDroid.

It starts the Android SDK emulator directly on the host instead of routing the hot path through Docker.

## Why Use It

- lower emulator overhead
- cleaner GPU integration
- simpler `scrcpy` usage
- fewer moving pieces in local development

## Required Tools

- `emulator`
- `adb`
- at least one AVD

Check what RustDroid can see:

```bash
rustdroid avds
rustdroid devices
```

## Common Host Commands

```bash
rustdroid --runtime-backend host --host-avd-name test_avd open
rustdroid --runtime-backend host --host-avd-name test_avd install app.apk
rustdroid --runtime-backend host --host-avd-name test_avd launch app.apks
rustdroid --runtime-backend host --host-avd-name test_avd clear-data app.xapk
rustdroid --runtime-backend host --host-avd-name test_avd stop --all
```

## Stable Headless Defaults

For reproducible local or CI-style host runs, prefer:

```bash
rustdroid \
  --runtime-backend host \
  --host-avd-name test_avd \
  --headless true \
  --emulator-gpu-mode swiftshader_indirect \
  --emulator-additional-args "-no-audio -no-boot-anim -no-snapshot -no-snapshot-save -read-only" \
  run app.apk
```

## Notes

- Host mode supports `scrcpy` and headless flows.
- Browser and VNC UI still belong to the Docker backend.
- `.xapk` installs stage APK contents directly. OBB copy is attempted, but some emulator images restrict writes under `Android/obb`.
