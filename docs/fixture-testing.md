# Fixture Testing Guide

RustDroid keeps small deterministic APK fixtures under `tests/fixtures/`.

They exist to test the fast loop without depending on a real application repo.

## Current Fixture Shapes

- launchable single APK
- missing launcher activity
- x86_64 native-lib APK
- ARM-only native-lib APK
- split APK pair

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

## Archive Workflow Testing

`.apks` and `.xapk` handling is tested by synthesizing archives from the checked-in APK fixtures inside the Rust test suite.

That keeps the repo smaller while still covering real archive parsing logic.
