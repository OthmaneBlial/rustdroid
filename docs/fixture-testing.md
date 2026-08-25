# Fixture Testing Guide

RustDroid keeps small deterministic APK fixtures under `tests/fixtures/`.

They exist to test the fast loop without depending on a real application repo.

## Fixture inventory

The manifest in `tests/fixtures/manifest.json` is the source of truth for this table. Every file below is a deliberately tiny public test APK; do not replace it with an application APK or post a user APK path in an issue.

| Fixture | Path | Purpose | Native ABI | Expected launcher |
| --- | --- | --- | --- | --- |
| `launch-success` | `tests/fixtures/apks/launch-success.apk` | Happy-path install and foreground receipt | None | `com.rustdroid.fixture.launch.MainActivity` |
| `missing-launcher` | `tests/fixtures/apks/missing-launcher.apk` | Clear failure when package metadata has no launchable activity | None | None |
| `x86_64-native` | `tests/fixtures/apks/x86_64-native.apk` | x86_64 ABI detection and compatible emulator path | `x86_64` | `com.rustdroid.fixture.x86native.MainActivity` |
| `arm64-native` | `tests/fixtures/apks/arm64-native.apk` | ARM-only ABI detection and compatibility warning path | `arm64-v8a` | `com.rustdroid.fixture.armnative.MainActivity` |
| `split-base` | `tests/fixtures/apks/split-base.apk` | Base member of the `split-locale-en` install group | None | `com.rustdroid.fixture.split.MainActivity` |
| `split-config.en` | `tests/fixtures/apks/split-config.en.apk` | English config split paired with `split-base` | None | None; split `config.en` |

## Regenerate Fixtures

```bash
./scripts/generate-fixture-apks.sh
```

That script rebuilds and signs the checked-in fixture APKs.

## Related Tests

- `tests/integration_fixtures.rs`
- `tests/integration_host_runtime.rs`
- `tests/integration_host_backend.rs`
- `tests/smoke_cli.rs`

Run the fixture metadata contract with:

```bash
cargo test --locked --test integration_fixtures
```

## Archive Workflow Testing

`.apks` and `.xapk` handling is tested by synthesizing archives from the checked-in APK fixtures inside the Rust test suite.

That keeps the repo smaller while still covering real archive parsing logic.
