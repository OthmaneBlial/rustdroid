# APK loop recipes

RustDroid prepares and verifies a local Android APK loop, then hands off to an existing UI-test framework when a workflow needs deeper coverage.

## Split APKs and `.apks`

```bash
rustdroid --profile host-fast --host-avd-name test_avd \
  run app.apks --duration-secs 2 --keep-alive false \
  --artifacts-dir artifacts/rustdroid

rustdroid --profile host-fast --host-avd-name test_avd \
  run base.apk config.en.apk config.x86_64.apk \
  --duration-secs 2 --keep-alive false --artifacts-dir artifacts/rustdroid
```

RustDroid orders a base APK before configuration splits and emits the same receipt contract.

## `.xapk` with OBB data

```bash
rustdroid --profile host-fast --host-avd-name test_avd \
  run game.xapk --duration-secs 2 --keep-alive false \
  --artifacts-dir artifacts/rustdroid
```

RustDroid extracts APK payloads and attempts the OBB copy. Some emulator images restrict shell writes below `Android/obb`; treat that warning as image storage-policy evidence, not an archive-parser failure.

## Cold versus warm startup

```bash
rustdroid --dry-run --profile host-fast --boot-mode cold run app-debug.apk
rustdroid --profile host-fast --host-avd-name test_avd --boot-mode cold bench app-debug.apk --artifacts-dir artifacts/bench-cold
rustdroid --profile host-fast --host-avd-name test_avd --boot-mode warm bench app-debug.apk --artifacts-dir artifacts/bench-warm
```

Keep the same AVD, API image, fixture, ABI, and command line when comparing timings.

## Crash and ANR triage

```bash
rustdroid --profile host-fast --host-avd-name test_avd \
  run app-debug.apk --duration-secs 10 --keep-alive false \
  --artifacts-dir artifacts/rustdroid-failure
```

Read `run-summary.json` first, then `run-report.html`. When available, inspect `logs/logcat.txt` and `forensics/`. The host CI lane also classifies KVM, AVD, boot, ADB, install, launch, logs, and cleanup failures.

## Prepare for Maestro or Appium

```bash
rustdroid --profile host-fast --host-avd-name test_avd \
  run app-debug.apk --duration-secs 2 --keep-alive true \
  --artifacts-dir artifacts/rustdroid

ADB_SERIAL=emulator-5554 maestro test .maestro/smoke.yaml
# or: APPIUM_DEVICE_NAME=emulator-5554 npm run e2e:android

rustdroid --profile host-fast --host-avd-name test_avd stop
```

The RustDroid receipt proves installation and foreground launch, not product-flow correctness.
