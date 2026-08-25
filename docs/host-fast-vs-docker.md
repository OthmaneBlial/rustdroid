# Host-fast loop versus a Docker Android stack

RustDroid is deliberately opinionated about the first question in an APK loop: *can this artifact boot, install, launch, and leave evidence on this machine?* It is not a device cloud or a replacement for broad UI automation.

## The two useful paths

| Path | Choose it when | Trade-off |
| --- | --- | --- |
| Host-fast | You already own an Android SDK/AVD and want the shortest local rerun loop | Requires Linux KVM and an intentionally configured host SDK |
| Docker backend | You need a contained setup or a browser/VNC-oriented fallback | Container/runtime overhead is outside the fastest loop |

Both paths aim to produce the same inspectable receipt. They should not be compared from timings collected on different hosts, Android images, APKs, or boot modes.

## Host-fast: keep the hot path close to the SDK

After creating the documented `test_avd`, a local receipt run is:

```bash
rustdroid \
  --profile host-fast \
  --host-avd-name test_avd \
  run path/to/app-debug.apk \
  --duration-secs 2 \
  --keep-alive false \
  --artifacts-dir artifacts/rustdroid
```

RustDroid owns the repeatable APK portion: preflight, emulator reuse or boot, package installation, launch check, log capture, cleanup, and an HTML/JSON/JUnit/Markdown receipt. The host path is a good fit for an Android developer who would otherwise repeatedly wire the emulator, `adb`, the APK path, and log collection by hand.

Before a run, inspect prerequisites without changing the machine:

```bash
rustdroid --json doctor
rustdroid --json setup --distro ubuntu
```

Use the [Linux quickstart](quickstart-linux.md) for SDK, AVD, and KVM commands. A host failure should be reported with its stable doctor IDs and redacted receipt, not an unbounded log dump.

## Docker: keep runtime ownership contained

The Docker backend is appropriate when a team prefers container ownership or needs the project’s browser/VNC-oriented path. Select it explicitly so the receipt records the chosen backend:

```bash
rustdroid \
  --runtime-backend docker \
  run path/to/app-debug.apk \
  --duration-secs 2 \
  --keep-alive false \
  --artifacts-dir artifacts/rustdroid-docker
```

Docker does not remove Android-image, emulator, APK, or observability concerns; it moves runtime setup behind a container boundary. It is useful when that boundary is the requirement, not because it is automatically the quickest iteration path.

## What RustDroid intentionally does not replace

- A device cloud for remote, shared, or broad device coverage.
- A UI test framework that proves product workflows beyond launch.
- An Android build system; Gradle, Flutter, or Expo still produce the APK that RustDroid consumes.

Use RustDroid before those systems when an APK-level answer is the missing signal. The [reference workflows](reference-workflows.md) show Gradle, Flutter, and React Native/Expo shapes; the [receipt schema](receipt-schema-v1.md) describes the evidence each path writes.

## Compare with evidence, not adjectives

For a fair host/Docker comparison, keep the APK fixture, runner image, Android API image, AVD configuration, and cold/warm state equal. Capture a receipt for each run:

```bash
rustdroid bench tests/fixtures/apks/launch-success.apk \
  --artifacts-dir artifacts/rustdroid-bench
```

The benchmark receipt records its environment and timings locally. It does not upload machine data or claim a universal number. See [benchmarking](benchmarking.md) for the reproducibility contract.
