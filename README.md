# RustDroid

RustDroid is a Linux-first CLI for the boring but important APK loop:

1. Boot an emulator.
2. Install an APK.
3. Launch the app.
4. Watch logs.
5. Stop cleanly.

That loop should be quick. A lot of Android tooling makes it heavier than it needs to be. RustDroid keeps the local path tight and gives you two ways to run it:

- host backend for the fastest local emulator workflow
- Docker backend when you want containerized setup and browser or VNC access

If you need a device farm, use a device farm. RustDroid is for the stage before that: local smoke tests, launch checks, crash triage, and repeatable APK validation on your own machine.

## Why RustDroid Exists

`budtmo/docker-android` is useful, but it solves a broader problem. RustDroid is narrower on purpose.

It is built for people who mostly want to:

- open an emulator quickly
- install an APK or APK set
- launch the package
- inspect logs
- rerun the flow without fighting the environment

That is the whole pitch. Less setup. Less ceremony. Faster feedback.

## What You Get

- one CLI for `open`, `install`, `run`, `launch`, `logs`, and `stop`
- host and Docker emulator backends
- support for `.apk`, split APK installs, `.apks`, and `.xapk`
- `watch` mode for rebuild-install-launch loops
- `clear-data` and `uninstall` flows for quick reset passes
- `doctor`, `self-test`, `devices`, `avds`, `bench`, and `version`
- `scrcpy`, browser, VNC, and headless UI paths
- config profiles, config inheritance, and environment overrides
- JSON output for setup and discovery commands
- artifact capture for runs and smoke checks
- install and uninstall scripts plus tag-driven release packaging

## Pick The Right Backend

### Host backend

Use this when you care about raw local speed.

- no Docker in the hot path
- uses your local Android SDK emulator and AVDs
- works well with `scrcpy`
- usually the best option for day-to-day development

### Docker backend

Use this when you want the environment wrapped up in a container.

- built around `budtmo/docker-android`
- supports browser UI and VNC
- supports Docker GPU passthrough
- useful for reproducible local or CI-style runs

## Install

Latest release:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/OthmaneBlial/rustdroid/main/install.sh)
```

Tagged release:

```bash
RUSTDROID_VERSION=v0.2.0 bash <(curl -fsSL https://raw.githubusercontent.com/OthmaneBlial/rustdroid/main/install.sh)
```

Source-only install:

```bash
./install.sh --source
```

Remove a local install:

```bash
./uninstall.sh
./uninstall.sh --dry-run
```

## Requirements

- Linux host with KVM available
- Rust toolchain
- `adb`
- `aapt` or `apkanalyzer`
- `scrcpy` if you want the desktop UI path
- Docker if you want the Docker backend
- Android SDK emulator and at least one AVD if you want the host backend

## Quick Start

Build locally:

```bash
cargo build
```

Run the health checks:

```bash
rustdroid doctor
rustdroid self-test
```

Fast local host path:

```bash
rustdroid --profile host-fast --host-avd-name test_avd run app-debug.apk
```

Containerized fast path:

```bash
rustdroid fast-local app-debug.apk
rustdroid-run fast-local app-debug.apk
```

Basic daily loop:

```bash
rustdroid --boot-mode warm open
rustdroid install base.apk config.en.apk
rustdroid launch --package com.example.app
rustdroid logs --package com.example.app --since-start
rustdroid stop --all
```

Repeat the build-install-launch loop:

```bash
rustdroid watch build/outputs/apk/debug --duration-secs 2 --keep-alive true
```

## Common Workflows

Host backend with `scrcpy`:

```bash
./run.sh host-local app-debug.apk
rustdroid --runtime-backend host --host-avd-name test_avd run app-debug.apk
```

Docker backend with `scrcpy`:

```bash
./run.sh fast-local app-debug.apk
rustdroid fast app-debug.apk
rustdroid fast-local app-debug.apk
```

Browser UI:

```bash
./run.sh web app-debug.apk
```

Headless:

```bash
./run.sh headless app-debug.apk
```

Discovery commands:

```bash
rustdroid version
rustdroid devices
rustdroid avds
```

Release smoke matrix:

```bash
./scripts/run-smoke-matrix.sh
```

## Performance Notes

If the emulator feels slow, start here:

- prefer an APK that includes `x86_64`
- use the host backend when you can
- use `scrcpy` before you reach for browser UI or VNC
- on Docker, try `--emulator-gpu-mode host` or `--emulator-gpu-mode auto`

In plain English: if you want the fastest loop, start with the host backend or `fast-local`, not the browser.

## Positioning

RustDroid is a good fit when:

- `docker-android` feels too heavy for your normal APK workflow
- you want a scriptable local loop
- you need quick smoke coverage before paying for hosted infrastructure
- you care more about iteration speed than broad platform features

RustDroid is a bad fit when:

- you need a remote device cloud
- you need a large shared device farm
- browser or VNC access is your main workflow instead of an occasional fallback

## Guides

- [First install](docs/first-install.md)
- [Host backend](docs/host-backend.md)
- [Troubleshooting](docs/troubleshooting.md)
- [CI examples](docs/ci-examples.md)
- [Fixture testing](docs/fixture-testing.md)
- [Release process](docs/release-process.md)
- [1.0 checklist](docs/1.0-checklist.md)
- [Versioning policy](docs/versioning-policy.md)
- [Support scope](docs/support-scope.md)
- [Changelog policy](docs/changelog-policy.md)
- [Changelog](CHANGELOG.md)
- [Contributing](CONTRIBUTING.md)

## License

MIT. See [LICENSE](./LICENSE).
