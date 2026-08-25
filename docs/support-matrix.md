# Support Matrix

This table records what RustDroid verifies today. It is not a promise that every Android image or host setup will behave identically.

The generated [support matrix](support-matrix.json) names the backend/API/ABI/UI combinations covered by repository contracts. A row is evidence only after its `verification_state` is `verified` and its `evidence_url` points to a successful run; it is never a promise that every related setup behaves identically.

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

## Contract combinations

| Backend | Runner contract | API | ABI | UI | Evidence workflow |
| --- | --- | ---: | --- | --- | --- |
| Host | GitHub-hosted Ubuntu 22.04 with KVM | 30 | x86_64 | Headless | [`host-integration-runtime` run](https://github.com/OthmaneBlial/rustdroid/actions/runs/32907975602/attempts/2) |
| Host | Fresh Ubuntu contract | 35 | x86_64 | Headless | [`fresh-machine-contract` run](https://github.com/OthmaneBlial/rustdroid/actions/runs/32905479252) |
| Host | Pinned action contract | 35 | x86_64 | Headless | [`action-contract` run](https://github.com/OthmaneBlial/rustdroid/actions/runs/32907241889) |

The contracts below have linked successful runs. A future failing run is handled as an operational failure to investigate; it does not turn a historical proof into a blanket compatibility promise.

## Release contract

Every published binary target must have all of the following:

1. A reproducible release archive and SHA-256 checksum.
2. A provenance attestation attached to the GitHub release workflow.
3. A clean-container installation test.
4. An explicit row in this document and matching installer behavior.

If any item is missing, the target remains source-only.
