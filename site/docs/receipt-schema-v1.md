# Run receipt schema v1

`rustdroid run --artifacts-dir artifacts/rustdroid` writes a versioned launch receipt. The canonical JSON file is `run-summary.json`; `reports/run-summary.json` is an identical convenience copy.

## Compatibility

- `schema_version` is `1` for this contract.
- Consumers must reject an unknown major schema version or preserve unfamiliar fields when they only display a receipt.
- `status` is `passed` for a completed APK launch receipt and `failed` when the run stops at a recorded stage.
- `failure_stage`, `last_completed_stage`, and `error_summary` are additive schema-v1 fields. Consumers written before these fields existed must continue to ignore unknown fields.
- A passed receipt uses `null` for `failure_stage` and `error_summary`, with `failure_classification` set to `none`.

## Stable fields

| Field | Purpose |
| --- | --- |
| `tool_version` | RustDroid version that created the receipt. |
| `status` | `passed` or `failed`. |
| `failure_stage` | The stage that stopped: `input_preflight`, `emulator_boot`, `apk_install`, `app_launch`, `app_runtime`, `log_capture`, `artifact_capture`, or `cleanup`; `null` on success. |
| `last_completed_stage` | The last fully completed stage, or `null` when input preflight never completed. |
| `failure_classification` | Stable routing category such as `input`, `emulator`, `install`, `launch`, `crash`, `anr`, `capture`, or `cleanup`; `none` on success. |
| `error_summary` | A bounded, path-free explanation suitable for a shared receipt. Full command errors remain on the console. |
| `runtime_backend`, `profile` | Selected execution path and named profile when known. |
| `emulator` | ADB serial, AVD name when configured, API level when readable, device label, headless state, and GPU mode. |
| `inputs[]` | Input file name, byte count, and SHA-256 digest. Full local paths and APK contents are intentionally omitted. |
| `package_name`, `launchable_activity`, `native_abis` | APK metadata needed to interpret the launch result. |
| `boot_duration_ms`, `install_duration_ms`, `launch_duration_ms`, `total_duration_ms` | Measured timing stages. |
| `artifacts` | Relative names of JSON, HTML, JUnit, Markdown, and available log files. No absolute runner path is emitted. |

## Failure receipts

After CLI parsing, configuration loading, and runtime-backend selection succeed, `rustdroid run --artifacts-dir ...` writes the receipt formats even when input preparation, emulator boot, installation, launch, runtime observation, log capture, or cleanup fails.

The canonical JSON deliberately does not copy the raw error string because it can include an absolute input path or environment detail. It records a stable stage, classification, and safe summary instead. The original error still appears on stderr for the local operator. Runtime logs and forensics remain best-effort: a boot or ADB failure can make some files unavailable.

Argument-parsing errors, an unreadable configuration, and a runtime-backend connection failure can happen before the orchestrator owns an artifact directory; those errors do not currently produce a run receipt.

## CI companion formats

When artifacts are requested, RustDroid also writes:

- `junit.xml` for test-report ingestion, with one failed testcase when `status` is `failed`;
- `run-summary.md` for a concise Markdown job summary;
- `run-report.html` for browser review.

Use `--junit-path path/to/report.xml` or `--markdown-summary-path path/to/summary.md` to copy either derived report to a CI-specific location. These flags do not change the canonical JSON receipt.

The schema records only evidence needed to reproduce or compare an APK run. Treat generated logs as potentially sensitive and upload them only to the CI audience that is allowed to inspect application output.

## Benchmark receipt

`rustdroid bench app.apk --artifacts-dir artifacts/bench` writes `bench-summary.json` and `bench-summary.md`. The benchmark receipt records the tool version, host OS/architecture/CPU count, runner image when supplied by CI, AVD/API, boot mode, emulator CPU/RAM/GPU configuration, safe input digest, and stage timings. It does not send machine data anywhere.
