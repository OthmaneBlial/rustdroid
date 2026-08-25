# GitHub Action contract

The root composite action runs an APK receipt on a Linux runner that already has KVM access, Android command-line tools, and a booted AVD. It deliberately does not hide emulator provisioning or use a device cloud.

The action builds RustDroid from the exact action revision selected by `uses:`. It then writes canonical JSON/HTML/JUnit/Markdown evidence and appends the Markdown receipt to the GitHub job summary.

The caller is responsible for:

1. checking out the APK;
2. enabling KVM access;
3. provisioning a compatible x86_64 Android AVD, such as `test_avd` with `reactivecircus/android-emulator-runner`;
4. uploading the returned receipt directory with `actions/upload-artifact`.

A pinned reference workflow is added after the action revision is committed. Use its exact immutable `uses:` revision rather than an unpinned branch when adopting the action.

The action accepts APK, `.apks`, and `.xapk` inputs. The generated receipt has the [schema v1 contract](receipt-schema-v1.md); logs can contain app output, so keep artifact retention and visibility appropriate for the application.
