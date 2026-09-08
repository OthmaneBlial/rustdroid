# Candidate 0.3.2: real Android failure evidence

Source: `d9022f9`, GitHub-hosted Ubuntu, Android API 30, Nexus 5 AVD, serial `emulator-5554`. The [nine-case run](https://github.com/OthmaneBlial/rustdroid/actions/runs/34205061851) completed successfully because its assertions matched every expected outcome, not because every app passed.

Download the `runtime-failure-matrix` workflow artifact for the original console output, JSON, HTML, JUnit and Markdown reports. No device interaction was performed on the maintainer's Mac.

| Scenario | CLI exit | Receipt status | Stage / classification |
| --- | --- | --- | --- |
| Normal launch | 0 | passed | no failed stage / none |
| Process exits after launch | 1 | failed | app_runtime / crash |
| Java exception after launch | 1 | failed | app_runtime / crash |
| Missing launch activity | 1 | failed | app_launch / launch |
| Foreground broadcast blocks its receiver | 1 | failed | app_runtime / anr |
| Logcat reader exits unexpectedly | 1 | failed | log_capture / capture |
| Launch log marker exceeds its deadline | 1 | failed | log_capture / capture |
| Cleanup state cannot be parsed | 1 | failed | cleanup / cleanup |
| Reader failure followed by cleanup failure | 1 | failed | log_capture / capture |

## What was real, and what was injected

The APKs ran on a real Android emulator. Exit and crash fixtures terminate or throw deliberately. The ANR fixture blocks a registered receiver; Android itself records the broadcast timeout and ANR. These scenarios do not use fabricated log messages.

For infrastructure failure tests, a per-process ADB wrapper forwards ordinary operations to the real ADB executable. It exits the streaming reader, delays the marker, or corrupts only an isolated temporary host-state file before cleanup. The combined case verifies the warning about the secondary cleanup error while retaining the original capture failure. These are explicit fault injections, not ordinary application behavior and not footage for a product demonstration.

## Scope

Each case checks its exit code and canonical JSON outcome, then verifies the failure stage in the generated HTML, JUnit and Markdown. Local tests additionally cover report serialization, startup readiness, historical/package attribution and cancellation. This is not business-flow testing, a physical-device matrix, independent user adoption or proof of a published release.

The original [candidate Linux archive](https://github.com/OthmaneBlial/rustdroid/actions/runs/34205275762) was built separately at `2754c3e` and has a verified checksum and provenance. The published `v0.3.2` archive is built from `ce727e8`; keep these revisions distinct because each archive has its own runtime validation evidence.
