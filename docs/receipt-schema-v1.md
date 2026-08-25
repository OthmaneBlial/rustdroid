# Run receipt schema v1

`rustdroid run --artifacts-dir artifacts/rustdroid` writes a versioned launch receipt. The canonical JSON file is `run-summary.json`; `reports/run-summary.json` is an identical convenience copy.

## Compatibility

- `schema_version` is `1` for this contract.
- Consumers must reject an unknown major schema version or preserve unfamiliar fields when they only display a receipt.
- `status` is `passed` for a completed APK launch receipt. `failure_classification` is `none` for this successful path; the host workflow classification artifact covers a failed environment run.

## Stable fields

| Field | Purpose |
| --- | --- |
| `tool_version` | RustDroid version that created the receipt. |
| `runtime_backend`, `profile` | Selected execution path and named profile when known. |
| `emulator` | ADB serial, AVD name when configured, API level when readable, device label, headless state, and GPU mode. |
| `inputs[]` | Input file name, byte count, and SHA-256 digest. Full local paths and APK contents are intentionally omitted. |
| `package_name`, `launchable_activity`, `native_abis` | APK metadata needed to interpret the launch result. |
| `boot_duration_ms`, `install_duration_ms`, `launch_duration_ms`, `total_duration_ms` | Measured timing stages. |
| `artifacts` | Relative names of JSON, HTML, JUnit, Markdown, and available log files. No absolute runner path is emitted. |

## CI companion formats

When artifacts are requested, RustDroid also writes:

- `junit.xml` for test-report ingestion;
- `run-summary.md` for a concise Markdown job summary;
- `run-report.html` for browser review.

Use `--junit-path path/to/report.xml` or `--markdown-summary-path path/to/summary.md` to copy either derived report to a CI-specific location. These flags do not change the canonical JSON receipt.

The schema records only evidence needed to reproduce or compare an APK run. Treat generated logs as potentially sensitive and upload them only to the CI audience that is allowed to inspect application output.

## Benchmark receipt

`rustdroid bench app.apk --artifacts-dir artifacts/bench` writes `bench-summary.json` and `bench-summary.md`. The benchmark receipt records the tool version, host OS/architecture/CPU count, runner image when supplied by CI, AVD/API, boot mode, emulator CPU/RAM/GPU configuration, safe input digest, and stage timings. It does not send machine data anywhere.
