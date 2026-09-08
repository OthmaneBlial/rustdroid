# The RustDroid Demo Receipt

The repository contains a small signed APK fixture so that a new user can see the real APK loop without an Android Studio project or a private app.

![A four-step RustDroid terminal walkthrough: doctor, a public fixture run, launch observation, and receipt artifacts](../assets/rustdroid-demo.gif)

The GIF is an illustration generated from four SVG scenes, not a recording of command execution. It intentionally shows no universal timing promise; use the receipt and benchmark documentation to compare a supported Linux host.

## Recorded CLI proof

[Watch the MP4](../assets/rustdroid-cli-proof.mp4) · [raw captured commands and outputs](../assets/rustdroid-cli-proof.json)

This 43-second silent H.264 video replays actual CLI command output captured on macOS on September 8, 2026, with reading holds. It shows the compiled binary version, an explicit dry-run plan, a real missing-input failure with exit code 1, and the JSON receipt that failure generated. The last chapter reads the archived September 1 Linux receipt and labels its historical provenance on screen.

It is an edited output replay, not a desktop screen capture or a new Android/emulator run. No install, successful launch or runtime timing is fabricated. The source commit, binary digest, argv, stdout, stderr and execution durations are retained in the adjacent JSON file. The MP4 reading holds are not command timings.

Reproduce from the repository root using a fresh output directory:

```bash
cargo build --locked
python3 -m venv dist/demo-tools
dist/demo-tools/bin/pip install Pillow
dist/demo-tools/bin/python scripts/record-cli-demo.py --output dist/new-cli-demo
```

Requires FFmpeg, ffprobe, jq and a Menlo or DejaVu Sans Mono font. The script uses an isolated config, refuses to overwrite its output directory and checks expected command exit codes. It renders captured output, then uses the FFmpeg web profile: H.264, CRF 23, 30 fps, 1280×720, yuv420p, faststart, no audio. Inspect the chapter PNGs and decode the entire MP4 before sharing it.

### Full emulator demo

ROADMAP.md M3 specifies a 45–60 second real Linux/KVM success/failure recording: command and Android screen, broken public fixture, generated HTML report, then the quickstart. GitHub Actions was re-enabled with owner approval. The accepted [capture run 34209082936](https://github.com/OthmaneBlial/rustdroid/actions/runs/34209082936) produced the raw desktop recording, real success/failure receipts, terminal logs and source metadata using the verified 0.3.2 candidate archive. The [green FFmpeg export run 34210495967](https://github.com/OthmaneBlial/rustdroid/actions/runs/34210495967) re-edited that raw capture from its measured timeline. The checked-in [final MP4](../assets/rustdroid-product-demo.mp4) is 57.36 seconds, silent, captioned and 1280×720; the [poster](../assets/rustdroid-product-demo-poster.png), [captions](../assets/rustdroid-product-demo.srt), [uncut source](../assets/rustdroid-product-demo-uncut.mp4) and [provenance receipt](receipts/product-demo.md) travel with it.

Recording tools and Android/KVM run on the GitHub Linux runner only. No emulator or KVM installation on macOS is needed. The reproducible capture entry point is `.github/workflows/product-demo.yml`, with `scripts/record-product-demo.sh`, `scripts/demo-session.sh` and `scripts/edit-product-demo.sh`. The edit notes label the warmed state, omitted setup, cuts and speed changes. The raw source is checked in for durable inspection and also retained in the linked Actions artifact.

The repository README and project site include native `<video controls>` players
with the poster and SRT track, plus a direct fallback download. GitHub's final
rendering and media delivery are checked after the assets land on `main`.
Release preparation is tracked in [the v0.3.2 draft](releases/v0.3.2.md); no new
release has been published by this recording work.

For raw evidence, inspect the checked-in [Gradle `run-summary.json`](receipts/reference-gradle.json) and its [provenance note](receipts/reference-gradle.md). That receipt came from the public September 1, 2026 workflow run that built the source fixture, launched it on Android 35, and uploaded the complete artifact bundle.

The visual in [`assets/rustdroid-proof.svg`](../assets/rustdroid-proof.svg) is a reference cold host-fixture run measured on April 2, 2026. Its numbers are context, not a performance promise; reproduce the workflow on your own Linux host and AVD.

## Prerequisites

Use a supported Linux host with KVM, the Android SDK emulator, ADB, an AVD named `test_avd` (or pass your own name), and the RustDroid binary. The [first-install guide](first-install.md) and [support matrix](support-matrix.md) describe the supported path.

## Run the checked-in fixture

```bash
rustdroid \
  --profile host-fast \
  --host-avd-name test_avd \
  run tests/fixtures/apks/launch-success.apk \
  --duration-secs 2 \
  --keep-alive false \
  --artifacts-dir artifacts/rustdroid-demo
```

The run succeeds only after RustDroid has booted or reused the emulator, inspected the APK, installed it, resolved its launch activity, and observed the app in the foreground.

The resulting receipt contains:

- `run-summary.json` for machines and CI;
- `run-report.html` for a quick human review;
- `logcat.txt` and, when available, process/ANR/tombstone evidence.

## Try the same loop with your build

```bash
./gradlew assembleDebug

rustdroid \
  --profile host-fast \
  --host-avd-name test_avd \
  run app/build/outputs/apk/debug/app-debug.apk \
  --duration-secs 2 \
  --keep-alive false \
  --artifacts-dir artifacts/rustdroid
```

For a retained local loop, remove `--keep-alive false` and use `rustdroid watch` against the directory that receives your APK.

## Keep the proof honest

The fixture validates RustDroid's APK path, not the correctness of your app. The timing depends on CPU, KVM availability, emulator image, cold/warm state, and APK ABI. Capture the generated receipt with your bug report or CI artifact rather than copying a benchmark number from this page.
