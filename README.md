# RustDroid

Fast Android emulator orchestration for local APK testing.

If you have used [`budtmo/docker-android`](https://github.com/budtmo/docker-android), you already know the good part: it gives you a full Android-in-Docker stack with VNC, web UI, Appium hooks, and a lot of flexibility.

You probably also know the bad part. When all you want is to boot an emulator, install an APK, launch it, and watch logs, that full stack can feel heavier than the job.

RustDroid exists for that narrower, faster loop.

It keeps the useful parts, cuts the ceremony, and adds a second path that skips Docker entirely when you want the Android SDK emulator running directly on the host. The result is simple:

- faster local APK smoke testing
- less time fighting emulator overhead
- a free local alternative before you reach for a paid device cloud

## Why RustDroid

RustDroid is built for one workflow:

1. Start an emulator quickly.
2. Install an APK.
3. Launch the app.
4. Stream logs.
5. Stop everything cleanly.

That sounds basic. It is. That is the point.

`docker-android` is a broader platform. RustDroid is a sharper tool. If you need a full Android automation environment with VNC, noVNC, Appium, log sharing, and image variants for many cases, `docker-android` is still useful. If you need a tight local loop for APK validation, RustDroid is usually the better fit.

## What Makes It Faster

- `scrcpy`-first workflow instead of browser-first workflow
- host-native emulator backend when you want to bypass Docker completely
- Docker GPU passthrough support through `/dev/dri` in addition to `/dev/kvm`
- smaller fast-local defaults for local iteration
- runtime tuning after boot
- APK ABI inspection, with warnings when you are forcing ARM translation on an x86_64 emulator

For local work, the best path is usually:

1. an APK that includes `x86_64`
2. the host runtime or the `fast-local` Docker mode
3. `scrcpy`
4. only using web/VNC when you truly need them

## Backends

RustDroid now supports two emulator backends.

### 1. Docker backend

This uses Docker and works well when you want containerized emulator management.

Highlights:

- built around the `budtmo/docker-android` emulator images
- optional browser UI and VNC
- Docker GPU passthrough support
- good fit for reproducible local and CI-style environments

### 2. Host backend

This starts the Android SDK emulator directly on the host machine.

Highlights:

- no Docker in the hot path
- usually the fastest option for local development
- works well with `scrcpy`
- uses your local Android SDK, local AVDs, and local GPU stack directly

Current limitation:

- host mode supports `scrcpy` or headless usage
- web UI and VNC still require the Docker backend

## Why It Is Also a Good Free Alternative to Paid Emulator Services

Paid device and emulator clouds have their place. They are useful when you need team sharing, remote devices, managed farms, or cross-browser and cross-device coverage at scale.

But for a lot of day-to-day APK work, they are overkill.

If you are doing local smoke tests, launch checks, crash inspection, log review, or quick regression passes, RustDroid gives you a free path that runs on your own machine:

- no per-minute billing
- no waiting for remote sessions
- no vendor lock-in for basic emulator work
- no reason to pay just to confirm that your APK boots and behaves

It is not trying to replace a full device lab. It is trying to stop you from paying for one too early.

## Features

- start, stop, install, run, and log stream from a single CLI
- `doctor`, `self-test`, `devices`, `avds`, and `version` commands
- `bench`, `profile`, `config init`, and `clean --dry-run`
- explicit `open` and `launch` flows for reusing a prepared emulator
- warm vs cold boot selection through `--boot-mode`
- multi-APK install support for split APK sets
- run summaries with optional artifact output folders
- Docker runtime and host-native runtime
- `scrcpy`, web, VNC, and headless modes
- Docker `/dev/kvm` support
- Docker GPU passthrough via `/dev/dri`
- APK metadata inspection with ABI detection
- post-boot performance tuning
- configurable CPU, RAM, heap, density, resolution, and GPU mode
- bash and zsh completion generation
- helper script for common local workflows
- GitHub Actions CI split into fast checks, host integration validation, and package validation
- tag-driven releases with a bundled install snippet and generated release notes
- a curated `v0.1.0` first-release note plus announcement and rollback checklists
- crates.io-ready manifest metadata plus manual publish workflow when a token is available

## Install

Fast path:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/OthmaneBlial/rustdroid/main/install.sh)
```

That installer tries the latest GitHub release first, then falls back to a local source build when release assets are not available.

Archive install with checksum verification:

```bash
./install.sh \
  --archive dist/rustdroid-x86_64-unknown-linux-musl.tar.gz \
  --checksum dist/rustdroid-x86_64-unknown-linux-musl.tar.gz.sha256 \
  --health-check
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

## Quick Start

### Requirements

- Linux host with KVM available
- Rust toolchain
- `adb`
- `aapt` or `apkanalyzer`
- `scrcpy` if you want the native desktop view
- Docker if you want the Docker backend
- Android SDK emulator and at least one AVD if you want the host backend

### Build

```bash
cargo build
```

### First Health Check

```bash
rustdroid doctor
rustdroid self-test
```

### Daily CLI Helpers

```bash
rustdroid profile list
rustdroid --config rustdroid.toml config init --profile host-fast
rustdroid --json bench
rustdroid --json clean --dry-run
./scripts/run-smoke-matrix.sh --list
```

### Daily APK Loop

```bash
rustdroid --boot-mode warm open
rustdroid install base.apk config.en.apk
rustdroid launch --package com.example.app
rustdroid run app.apk --duration-secs 10 --keep-alive false --artifacts-dir .rustdroid-artifacts
rustdroid logs --package com.example.app --since-start
rustdroid stop --all
```

### Fastest Local Path: Host Emulator + scrcpy

```bash
./run.sh host-local app-debug.apk
```

Or directly:

```bash
./target/debug/rustdroid \
  --runtime-backend host \
  --host-avd-name test_avd \
  run app-debug.apk
```

### Containerized Path: Docker + scrcpy

```bash
./run.sh fast-local app-debug.apk
```

### Browser UI

```bash
./run.sh web app-debug.apk
```

### Headless

```bash
./run.sh headless app-debug.apk
```

### Discovery Commands

```bash
rustdroid version
rustdroid devices
rustdroid avds
```

### Release-Safe Smoke Matrix

RustDroid now ships a single host-smoke entrypoint for the minimum flows that should stay green before a release:

```bash
./scripts/run-smoke-matrix.sh
```

Set `RUSTDROID_SMOKE_ENABLE_GUI=1` when you want the visible `scrcpy` fast lane included on a GUI-capable machine.
By default the smoke lane uses a read-only host emulator profile plus `swiftshader_indirect` so the checklist stays reproducible and fast on repeat runs.

That matrix covers:

- host fast path
- host headless
- cold boot
- warm reuse
- install-only
- launch-only
- artifact-enabled run
- split APK install

### Machine-Readable Output

RustDroid now supports `--json` for the discovery and setup commands, including `version`, `doctor`, `devices`, `avds`, `self-test`, `bench`, `profile`, `config init`, and `clean`.

### Reproducible Team Setups

RustDroid already supports per-project config files through `--config`, and now also supports environment overrides for shared shells and CI. Useful variables include:

- `RUSTDROID_PROFILE`
- `RUSTDROID_RUNTIME_BACKEND`
- `RUSTDROID_BOOT_MODE`
- `RUSTDROID_IMAGE`
- `RUSTDROID_CONTAINER_NAME`
- `RUSTDROID_HOST_AVD_NAME`
- `RUSTDROID_HOST_EMULATOR_PORT`
- `RUSTDROID_EMULATOR_GPU_MODE`
- `RUSTDROID_UI_BACKEND`

## Helper Modes

`run.sh` includes these shortcuts:

- `fast-local`
- `local`
- `host-local`
- `host-headless`
- `host-logs`
- `web`
- `vnc`
- `headless`
- `logs`
- `stop`

## Performance Notes

If the emulator still feels slow, check these first:

- Your APK may be ARM-only. On an x86_64 emulator, that means translation overhead.
- `web` and `vnc` are convenient, but they are slower than `scrcpy`.
- For Docker, test `--emulator-gpu-mode host` or `--emulator-gpu-mode auto`.
- For pure local speed, use the host runtime.

In plain English: if you want the fastest loop, do not start with browser UI. Start with `host-local` or `fast-local`.

## Positioning

RustDroid is a good fit when:

- `docker-android` feels too heavy for your local APK workflow
- you want something scriptable and local
- you want a free emulator workflow before paying for a hosted device platform
- you care more about fast iteration than feature breadth

RustDroid is a worse fit when:

- you need a full remote device cloud
- you need large-scale parallel device farms
- you need browser-based sharing as the primary workflow

## License

MIT. See [LICENSE](./LICENSE).
