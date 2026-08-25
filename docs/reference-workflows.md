# Stack-specific reference workflows

The three templates in [`examples/workflows/`](../examples/workflows/) are ready to copy into an Android repository. Each builds a conventional debug APK, provisions the same Linux/KVM AVD shape, starts a persistent Android 35 AVD, invokes the immutable RustDroid action revision, uploads the receipt, and stops the AVD in an `always()` step.

| Stack | Template | APK path |
| --- | --- | --- |
| Gradle Android | [`gradle-android-receipt.yml`](../examples/workflows/gradle-android-receipt.yml) | `app/build/outputs/apk/debug/app-debug.apk` |
| Flutter | [`flutter-receipt.yml`](../examples/workflows/flutter-receipt.yml) | `build/app/outputs/flutter-apk/app-debug.apk` |
| React Native / Expo prebuild | [`react-native-expo-receipt.yml`](../examples/workflows/react-native-expo-receipt.yml) | `android/app/build/outputs/apk/debug/app-debug.apk` |

They all emit the [receipt schema](receipt-schema-v1.md). The provisioner action creates the AVD definition but owns and stops its short-lived emulator process, so the explicit boot step is required before the receipt action can connect to `emulator-5554`.

## Executable public source fixtures

The templates now have deliberately small, public source counterparts in
[`examples/apps/`](../examples/apps/). They exist to prove the exact APK paths
above without using an unpublished application, and they keep generated SDK,
Gradle, Flutter, and JavaScript directories out of Git.

| Stack | Source fixture | Reproducible build entrypoint | APK path |
| --- | --- | --- | --- |
| Gradle Android | [`gradle-android`](../examples/apps/gradle-android/) | `./scripts/build-debug-apk.sh` | `app/build/outputs/apk/debug/app-debug.apk` |
| Flutter | [`flutter`](../examples/apps/flutter/) | `./scripts/build-debug-apk.sh` | `build/app/outputs/flutter-apk/app-debug.apk` |
| React Native / Expo prebuild | [`expo-prebuild`](../examples/apps/expo-prebuild/) | `./scripts/build-debug-apk.sh` | `android/app/build/outputs/apk/debug/app-debug.apk` |

[`reference-stack-fixtures`](../.github/workflows/reference-stack-fixtures.yml)
builds these three sources, checks that the documented APK exists, boots the
same Android 35 AVD, invokes the immutable receipt action, and uploads each
receipt. It runs on the first day of every month and supports manual dispatch;
use its run history as proof for a specific stack rather than assuming a copied
template will work in every application.
