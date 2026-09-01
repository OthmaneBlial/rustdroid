# RustDroid

<p align="center">
  <strong>Know whether an Android artifact installs and launches — before spending a device-cloud run.</strong>
</p>

<p align="center">
  <a href="https://github.com/OthmaneBlial/rustdroid/releases/latest"><img src="https://img.shields.io/github/v/release/OthmaneBlial/rustdroid?display_name=tag" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/platform-x86__64%20Linux-111a16" alt="x86_64 Linux release">
  <img src="https://img.shields.io/badge/Rust-stable-DEA584?logo=rust" alt="Rust stable">
  <a href="LICENSE"><img src="https://img.shields.io/github/license/OthmaneBlial/rustdroid" alt="MIT License"></a>
</p>

<p align="center">
  <a href="docs/demo.md"><img src="assets/rustdroid-demo.gif" alt="RustDroid diagnosing a host, running a public fixture, observing its launch, and keeping a receipt" width="100%"></a>
</p>

<p align="center">
  <strong>APK path in. Launch receipt out.</strong><br>
  RustDroid is a Linux-first CLI that turns an APK, split set, <code>.apks</code>, or <code>.xapk</code> into an inspectable local launch result.
</p>

<p align="center">
  <a href="https://othmaneblial.github.io/rustdroid/">Project site</a> ·
  <a href="docs/receipts/reference-gradle.md">Inspect a real receipt</a> ·
  <a href="docs/quickstart-linux.md">Linux quickstart</a> ·
  <a href="docs/github-action.md">GitHub Action</a>
</p>

## The missing gate after a build

An Android build can succeed while its output still has the wrong ABI, a broken split set, no launchable activity, an install failure, or an immediate crash. The usual response is a pile of ADB commands and a console timeout that the next person cannot reproduce.

RustDroid makes that hand-off explicit:

```text
APK path -> preflight -> boot or reuse -> install -> launch -> logs + receipt
```

One command answers three useful questions:

- Did the artifact install and reach its launch activity?
- If not, did the failure happen in the emulator, ADB, APK install, launch, crash/ANR, or log capture stage?
- What portable evidence can a teammate or CI job inspect next?

No test script is required. No APK upload or hosted RustDroid account is required.

## A real receipt, not a mock result

<p align="center">
  <a href="docs/receipts/reference-gradle.json"><img src="assets/rustdroid-proof.svg" alt="A RustDroid launch receipt showing boot, install, launch, and artifact evidence" width="100%"></a>
</p>

The checked-in [Gradle fixture receipt](docs/receipts/reference-gradle.md) came from a public GitHub-hosted Ubuntu/KVM run on September 1, 2026. RustDroid `0.3.1` booted the Android 35 AVD, installed the public fixture, observed its activity in the foreground, and wrote JSON, HTML, JUnit, Markdown, and log artifacts.

That run completed the receipt path in 15.746 seconds. It is one reproducible sample, not a universal speed promise; the [benchmark notes](docs/benchmarking.md) publish the environment and variance.

## Get to your first receipt

RustDroid currently targets Linux hosts with KVM, an Android SDK emulator, ADB, and an existing AVD. The [Linux quickstart](docs/quickstart-linux.md) gives exact Ubuntu/Debian and Fedora setup commands.

### 1. Install the verified release

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/OthmaneBlial/rustdroid/main/install.sh)
```

The prebuilt archive targets **x86_64 Linux**. ARM/aarch64 Linux uses the documented source-build path.

### 2. Check the host before changing anything

```bash
rustdroid version
rustdroid doctor
rustdroid self-test --backend host
```

`rustdroid setup --distro ubuntu` prints a reviewable setup plan. It does not run `sudo`, accept licenses, download an SDK, or create an AVD.

### 3. Run your APK

```bash
rustdroid \
  --profile host-fast \
  --host-avd-name test_avd \
  run app/build/outputs/apk/debug/app-debug.apk \
  --duration-secs 2 \
  --keep-alive false \
  --artifacts-dir artifacts/rustdroid
```

The resulting directory contains:

```text
artifacts/rustdroid/
├── run-summary.json   # stable schema for tools and CI
├── run-report.html    # human-readable review
├── junit.xml          # test-report ingestion
├── run-summary.md     # job summary
└── logcat.txt         # runtime evidence
```

Want a public input first? Clone the repository and replace your APK path with `tests/fixtures/apks/launch-success.apk`.

## Where RustDroid fits

| Tool | Best at | RustDroid's role |
| --- | --- | --- |
| Hand-written ADB scripts | One team's custom device commands | Replace repeated glue with a versioned artifact-to-receipt contract |
| Gradle Managed Devices | Project-owned instrumented tests | Validate an already-built APK or archive before/beside the Gradle test suite |
| Maestro, Appium, Espresso | UI flows and behavioral assertions | Prove the artifact can install and launch before deeper tests begin |
| Firebase Test Lab, AWS Device Farm | Remote matrices and real-device coverage | Catch first-mile packaging and launch failures locally before escalation |

RustDroid is deliberately not a UI-test language, device farm, iOS tool, or hosted dashboard. It complements those systems instead of asking you to replace them.

## Use the path you already have

### Local build loop

```bash
./gradlew assembleDebug
rustdroid --profile host-fast --host-avd-name test_avd \
  run app/build/outputs/apk/debug/app-debug.apk \
  --artifacts-dir artifacts/rustdroid
```

Keep the emulator warm while builds change:

```bash
rustdroid --profile host-fast --host-avd-name test_avd open
rustdroid watch build/outputs/apk/debug --duration-secs 2 --keep-alive true
```

### CI receipt

RustDroid ships a reusable composite action. Pair it with the emulator provisioner you already trust, then upload the returned receipt directory:

```yaml
- id: rustdroid
  uses: OthmaneBlial/rustdroid@964ed16d32d4fa12b52dea21b95484a7b96e9854
  with:
    apk-path: app/build/outputs/apk/debug/app-debug.apk
    profile: host-fast
    runtime-backend: host
    host-avd-name: test_avd
    artifacts-dir: artifacts/rustdroid

- uses: actions/upload-artifact@v7
  with:
    name: rustdroid-receipt
    path: ${{ steps.rustdroid.outputs.receipt-dir }}
```

See the complete [CI example](docs/ci-examples.md), [GitHub Action contract](docs/github-action.md), and public [Gradle, Flutter, and Expo workflows](docs/reference-workflows.md).

### APK archives and split installs

The same `run` command accepts:

- a single APK;
- multiple split APK paths;
- a `.apks` archive;
- an `.xapk` archive, including OBB staging when present.

The [APK loop recipes](docs/recipes.md) show the exact command for each input shape.

## Host fast path or Docker?

| Backend | Choose it when | Trade-off |
| --- | --- | --- |
| **Host** | You want the shortest daily loop on an existing Linux/KVM Android SDK | Owns the local emulator process; pairs with `scrcpy` or headless mode |
| **Docker** | You value containment or browser/VNC access more than startup speed | Still needs compatible Linux virtualization and carries more moving parts |
| **Device cloud** | You need shared remote capacity, OEM hardware, or broad coverage | Outside RustDroid's scope; use after the local receipt gate |

Read [host-fast versus Docker](docs/host-fast-vs-docker.md), the [host backend guide](docs/host-backend.md), and the [support matrix](docs/support-matrix.md) before standardizing a team workflow.

## Command surface

```text
doctor       check host and runtime prerequisites
setup        print a non-destructive Linux setup plan
self-test    exercise the selected backend
open         boot or reuse an emulator UI
run          install, launch, observe, and write a receipt
watch        rerun when an APK output changes
bench        measure boot, install, and launch stages
logs         stream emulator or app logs
devices      list ADB-visible devices
avds         list host Android Virtual Devices
profile      inspect or write named profiles
clean        remove RustDroid-managed state
```

Use `rustdroid --help` for every command and flag. Checked-in settings are explained in [configuration ownership](docs/configuration.md); reviewable `--dry-run` plans are documented in [operation plans](docs/operation-plans.md).

## Evidence and limits

A passed RustDroid receipt proves that the recorded Android artifact was inspected, installed, launched, and observed on the recorded emulator path. It does **not** prove that every screen, business flow, device model, permission path, accessibility behavior, or production environment works.

Generated logs can contain application data. RustDroid keeps runs local by default and records file names, sizes, and SHA-256 input digests rather than absolute APK paths, but you should still restrict who can read uploaded CI artifacts.

See the [receipt schema](docs/receipt-schema-v1.md), [support scope](docs/support-scope.md), and [fixture testing contract](docs/fixture-testing.md) for the precise boundaries.

## Documentation

Start here:

- [Demo receipt](docs/demo.md)
- [First install](docs/first-install.md)
- [Linux quickstart](docs/quickstart-linux.md)
- [Troubleshooting](docs/troubleshooting.md)

Build a workflow:

- [Configuration ownership](docs/configuration.md)
- [CI examples](docs/ci-examples.md)
- [Reusable GitHub Action](docs/github-action.md)
- [Stack-specific reference workflows](docs/reference-workflows.md)
- [Executable Gradle, Flutter, and Expo fixtures](examples/apps/README.md)
- [Host-fast versus Docker](docs/host-fast-vs-docker.md)
- [Operation plans and dry runs](docs/operation-plans.md)
- [APK loop recipes](docs/recipes.md)

Verify the contract:

- [Run receipt schema](docs/receipt-schema-v1.md)
- [Reproducible benchmarks](docs/benchmarking.md)
- [Support matrix](docs/support-matrix.md)
- [Support scope](docs/support-scope.md)
- [1.0 checklist](docs/1.0-checklist.md)

## Contributing

Bug reports should include the failing stage, `rustdroid doctor --json` output, and the smallest safe receipt you can share. Read [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), and [SUPPORT.md](SUPPORT.md) before opening an issue. Release history is in [CHANGELOG.md](CHANGELOG.md).

RustDroid is available under the [MIT License](LICENSE).
