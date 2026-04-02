# Fixture APK Lab

These APKs are the canonical deterministic app inputs for RustDroid tests.

They exist to cover the APK shapes that tend to break fast APK loops:

- launchable single-APK app
- APK with no launcher activity
- APK that advertises `x86_64` native libs
- APK that advertises ARM-only native libs
- split APK set with a language split

## Inventory

The machine-readable inventory lives in `tests/fixtures/manifest.json`.

The checked-in outputs are:

- `tests/fixtures/apks/launch-success.apk`
- `tests/fixtures/apks/missing-launcher.apk`
- `tests/fixtures/apks/x86_64-native.apk`
- `tests/fixtures/apks/arm64-native.apk`
- `tests/fixtures/apks/split-base.apk`
- `tests/fixtures/apks/split-config.en.apk`

## Regeneration

Regenerate the fixture set from source templates with:

```bash
./scripts/generate-fixture-apks.sh
```

The script:

- compiles tiny Java activities into `classes.dex`
- packages minimal manifests with `aapt`
- adds native-lib directory entries where needed
- signs every fixture with `tests/fixtures/debug.keystore`

The committed APKs are the source of truth for tests. Regenerate them only when fixture behavior or metadata needs to change.

## Notes

- `tests/fixtures/debug.keystore` is only for local test assets. It must never be reused for real app signing.
- The native-lib fixtures use tiny placeholder `.so` files because RustDroid only needs the APK metadata shape for ABI detection tests.
