# Linux Quickstart: a Verified Host Fixture

This is the supported host-emulator path for Ubuntu, Debian, and Fedora Linux. It prepares one x86_64 Android 35 AVD named `test_avd`, then uses the checked-in APK fixture to verify the complete RustDroid loop.

The commands below change a machine only when you run them yourself. Start by printing the same plan from RustDroid:

```bash
rustdroid setup
rustdroid --json setup --distro ubuntu
```

`setup` is intentionally non-destructive: it does not run `sudo`, accept Android licenses, download an SDK, write a config file, or create an AVD.

## 1. Install OS prerequisites

Ubuntu or Debian:

```bash
sudo apt-get update
sudo apt-get install --yes openjdk-17-jdk unzip wget curl qemu-kvm
sudo usermod -aG kvm "$USER"
```

Fedora:

```bash
sudo dnf makecache
sudo dnf install --assumeyes java-17-openjdk-devel unzip wget curl qemu-kvm
sudo usermod -aG kvm "$USER"
```

Log out and back in after changing group membership. `newgrp kvm` can refresh the current shell, but a fresh login is the reliable check.

## 2. Install Android command-line tools

Download **Command line tools only** for Linux from [Android Studio downloads](https://developer.android.com/studio#command-tools). Create `$ANDROID_SDK_ROOT/cmdline-tools/latest`, then unpack the archive so that `sdkmanager` is at `$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/sdkmanager`.

For the current shell:

```bash
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}"
export PATH="$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:$ANDROID_SDK_ROOT/platform-tools:$ANDROID_SDK_ROOT/emulator:$PATH"
```

After verifying the paths, add those exports to your shell profile. RustDroid also discovers tools below `ANDROID_HOME` when that variable is already used by your project.

## 3. Install the fixed SDK baseline and create the AVD

API 35 is the documented fixture baseline. Keeping this explicit makes a setup failure comparable across contributors.

```bash
sdkmanager --licenses
sdkmanager "platform-tools" "emulator" "build-tools;35.0.0" "platforms;android-35" "system-images;android-35;google_apis;x86_64"
echo no | avdmanager create avd --force --name test_avd --package "system-images;android-35;google_apis;x86_64" --device pixel_5
emulator -list-avds
```

If you want RustDroid to open the native desktop UI, install `scrcpy`; it is optional for headless work:

```bash
# Ubuntu/Debian
sudo apt-get install --yes scrcpy

# Fedora
sudo dnf install --assumeyes scrcpy
```

## 4. Verify the selected backend before the first APK

The backend changes which failures are blocking. Host diagnostics mark KVM, Android SDK, emulator, ADB, APK inspectors, and an AVD as required; Docker and `scrcpy` remain optional. Docker diagnostics invert that rule for the Docker daemon.

```bash
rustdroid --runtime-backend host doctor
rustdroid --runtime-backend host --json doctor
```

`doctor --json` emits schema version `1`. Each check has a stable `id`, `required` boolean, state, hint, and reviewable `remediation` commands. Use the check ID in issue reports rather than copying private paths from the output.

## 5. Run the repository fixture

```bash
rustdroid \
  --profile host-fast \
  --host-avd-name test_avd \
  run tests/fixtures/apks/launch-success.apk \
  --duration-secs 2 \
  --keep-alive false \
  --artifacts-dir artifacts/rustdroid-demo
```

A successful run leaves `run-summary.json`, `run-report.html`, and `logcat.txt` under `artifacts/rustdroid-demo/`. The [demo receipt](demo.md) explains what that proof does and does not validate.

For persistent project settings, copy a reviewed sample from [`examples/configs/`](../examples/configs/) and read [configuration ownership](configuration.md).
