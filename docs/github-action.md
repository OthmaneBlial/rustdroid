# GitHub Action contract

The root composite action runs an APK receipt on a Linux runner that already has KVM access, Android command-line tools, and a booted AVD. It deliberately does not hide emulator provisioning or use a device cloud.

The action builds RustDroid from the exact action revision selected by `uses:`. It then writes canonical JSON/HTML/JUnit/Markdown evidence and appends the Markdown receipt to the GitHub job summary.

The caller is responsible for:

1. checking out the APK;
2. enabling KVM access;
3. provisioning a compatible x86_64 Android AVD, such as `test_avd` with `reactivecircus/android-emulator-runner`;
4. uploading the returned receipt directory with `actions/upload-artifact`.

## Pinned reference workflow

The repository tests the exact action revision below against its public `launch-success.apk` fixture in [`action-contract.yml`](../.github/workflows/action-contract.yml). Copy the shape, then replace the APK path and AVD name for your project.

```yaml
- id: receipt
  uses: OthmaneBlial/rustdroid@964ed16d32d4fa12b52dea21b95484a7b96e9854
  with:
    apk-path: app/build/outputs/apk/debug/app-debug.apk
    profile: host-fast
    runtime-backend: host
    host-avd-name: test_avd
    artifacts-dir: artifacts/rustdroid
    duration-secs: "2"
    keep-alive: "false"

- uses: actions/upload-artifact@v7
  if: always()
  with:
    name: rustdroid-receipt
    path: ${{ steps.receipt.outputs.receipt-dir }}
```

The surrounding job must enable KVM and provision the AVD first, as shown in the reference workflow. The action appends its Markdown receipt to `$GITHUB_STEP_SUMMARY` and exposes `receipt-dir` for upload.

The action accepts APK, `.apks`, and `.xapk` inputs. The generated receipt has the [schema v1 contract](receipt-schema-v1.md); logs can contain app output, so keep artifact retention and visibility appropriate for the application.
