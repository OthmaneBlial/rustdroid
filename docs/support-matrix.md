# Support Matrix

This table records what RustDroid verifies today. It is not a promise that every Android image or host setup will behave identically.

| Area | Supported and verified | Notes |
| --- | --- | --- |
| Host OS | Linux with KVM | The host runtime is Linux-first. macOS can build and test the CLI, but is not a supported emulator host. |
| Prebuilt binary | `x86_64-unknown-linux-musl` | Release archive, checksum, provenance attestation, and clean-container installation check. |
| ARM/aarch64 Linux | Source build | No ARM release archive is advertised or downloaded until it has an equivalent release test. |
| Host backend | Android SDK emulator, ADB, one AVD | The preferred local fast path; uses `scrcpy` or headless mode. |
| Docker backend | Linux Docker plus `/dev/kvm` where required by the image | Browser and VNC are optional, not the preferred fast path. |
| APK inputs | `.apk`, split APKs, `.apks`, `.xapk` | Deterministic fixtures cover normal launch, no launcher, ABI metadata, and a locale split. |
| CI | GitHub-hosted Ubuntu/KVM shape | Fast checks, packaging, host integration, release installation, and artifacts are separate gates. |
| Device clouds / iOS | Not supported | RustDroid is the local smoke-test step before a device cloud. |

## Release contract

Every published binary target must have all of the following:

1. A reproducible release archive and SHA-256 checksum.
2. A provenance attestation attached to the GitHub release workflow.
3. A clean-container installation test.
4. An explicit row in this document and matching installer behavior.

If any item is missing, the target remains source-only.
