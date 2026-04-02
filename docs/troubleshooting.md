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
