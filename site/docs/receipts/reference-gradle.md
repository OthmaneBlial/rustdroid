# Real Gradle launch receipt

This is a checked-in copy of the canonical `run-summary.json` produced by RustDroid for the public Gradle Android fixture on September 1, 2026.

[Inspect the raw JSON receipt](receipts/reference-gradle.json) or [open the complete public workflow run](https://github.com/OthmaneBlial/rustdroid/actions/runs/33519017529).

## What ran

| Field | Recorded value |
| --- | --- |
| Source | `examples/apps/gradle-android` |
| Runner | GitHub-hosted `ubuntu-22.04` with KVM |
| Emulator | Android API 35, `test_avd`, headless |
| RustDroid | `0.3.1`, `host-fast` profile |
| Package | `dev.rustdroid.examples.gradle` |
| Result | `passed`, no crash or ANR summary |
| Boot / install / launch | 6,795 ms / 3,431 ms / 3,517 ms |
| Total receipt path | 15,746 ms |

The workflow built the APK from the checked-in public source, started the documented emulator, invoked the immutable RustDroid action, and uploaded the JSON, HTML, JUnit, Markdown, and log artifacts. The timings describe this run only; they are evidence, not a universal performance promise.

## Privacy boundary

The canonical receipt records the APK file name, size, and SHA-256 digest, not its source path or contents. Logs are intentionally not checked in because application output can contain sensitive data.
