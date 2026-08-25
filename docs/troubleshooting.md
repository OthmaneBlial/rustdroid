# Troubleshooting

## `doctor` fails

Run:

```bash
rustdroid doctor
```

Fix the first hard failure before trying more commands. Common causes:

- KVM permissions
- missing `adb`
- missing Android SDK emulator
- no AVDs configured

### Stable `doctor --json` check IDs

Use `rustdroid --json doctor` when a script needs the stable IDs. The host checks become required only when the selected backend is `host`; Docker checks become required only when the selected backend is `docker`. `rustdroid setup --distro ubuntu` prints a reviewable plan and does not run `sudo` or change the machine.

| Check ID | Meaning | Safe next step |
| --- | --- | --- |
| `host.kvm.device` | Linux hardware virtualization device is unavailable. | Review `rustdroid setup --distro ubuntu`; use a Linux host with KVM enabled. |
| `host.kvm.permissions` | The current user cannot open `/dev/kvm`. | Review the KVM group step in `rustdroid setup --distro ubuntu`, then start a new login session. |
| `host.android_sdk.root` | `ANDROID_HOME` or `ANDROID_SDK_ROOT` was not detected. | Review the SDK export step in `rustdroid setup --distro ubuntu`. |
| `host.tool.emulator` | Android Emulator is not on `PATH`. | Review the Android SDK and `PATH` steps in `rustdroid setup --distro ubuntu`. |
| `host.tool.adb` | Android platform-tools are not on `PATH`. | Review the Android SDK and `PATH` steps in `rustdroid setup --distro ubuntu`. |
| `host.tool.aapt` | Android build-tools are not on `PATH`. | Install the documented build-tools set with the reviewed setup plan. |
| `host.tool.apkanalyzer` | Android command-line build tools are not on `PATH`. | Install the documented build-tools set with the reviewed setup plan. |
| `host.tool.scrcpy` | Optional native device display helper is unavailable. | Install `scrcpy` only if you want the desktop UI; headless runs remain supported. |
| `host.avds` | No usable Android Virtual Device is listed. | Review the `test_avd` creation step, then run `emulator -list-avds`. |
| `docker.daemon` | Docker is unavailable or its daemon is not reachable. | Start Docker only when you selected the Docker backend; otherwise use `--runtime-backend host`. |
| `docker.gpu_passthrough` | Optional Docker GPU passthrough is limited. | Treat this as optional; use host graphics or a headless Docker run. |

## Emulator is slow

Use the fastest path first:

- host backend
- `scrcpy` or headless mode
- `x86_64` APKs for emulator testing
- `swiftshader_indirect` for deterministic headless runs

Avoid using web or VNC for your normal iteration loop.

## App feels slow on the emulator

Check the APK ABI shape:

- `x86_64` support is best for x86_64 emulators
- ARM-only native libraries force translation and slow the app down

## `.xapk` install warns about OBB staging

RustDroid installs the APK payload and attempts the OBB copy.

Some Android images restrict writes under `Android/obb` for the shell user. Treat that warning as a storage-policy limitation of the image, not a RustDroid archive parsing failure.

## Host backend cannot find an emulator

Check:

```bash
rustdroid avds
emulator -list-avds
```

Pass `--host-avd-name` explicitly if the wrong AVD is being selected.

## Watch mode does nothing

`watch` accepts either:

- a file path ending in `.apk`, `.apks`, or `.xapk`
- a directory containing those files

The directory mode picks the newest supported file in that folder.
