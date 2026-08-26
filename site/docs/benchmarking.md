# Reproducible benchmarks

Use a benchmark receipt, not a copied timing, when describing RustDroid as fast.

```bash
rustdroid --profile host-fast --host-avd-name test_avd --boot-mode cold \
  bench tests/fixtures/apks/launch-success.apk \
  --artifacts-dir artifacts/bench-cold
```

The resulting JSON and Markdown record the host platform, CPU count, runner image when available, API/AVD, boot mode, emulator configuration, input SHA-256, and boot/install/launch/total timings. Repeat with `--boot-mode warm` using the same environment before comparing results.

The checked-in [v0.1 reference](performance-notes/v0.1.0.md) is a historical baseline, not a promise for your machine. Publish median and spread from repeated comparable runs when adding a new release benchmark.

The [v0.3.1 performance notes](performance-notes/v0.3.1.md) show this release
format with three independently completed public-fixture matrices, source links,
environment details, and an explicit sample spread.
