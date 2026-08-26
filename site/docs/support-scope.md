# Support Scope

RustDroid is intentionally narrow.

It is a fast local APK testing CLI, not a full Android automation platform.

## In Scope

- local APK install, launch, watch, and log loops
- host backend usage on Linux
- deterministic fixture-backed testing
- release packaging, install flow, and CI checks
- `.apk`, `.apks`, and `.xapk` inputs for normal app testing

## Out Of Scope

- remote device clouds
- iOS support
- full Android farm management
- browser and VNC as the preferred local loop
- feature work that makes the normal host fast loop slower without a clear benefit

## Current Primary Support Target

- Linux
- Android SDK emulator
- host backend
- `scrcpy` or headless usage

Docker remains available, but it is not the primary performance path.
