# Stack-specific reference workflows

The three templates in [`examples/workflows/`](../examples/workflows/) are ready to copy into an Android repository. Each builds a conventional debug APK, provisions the same Linux/KVM AVD shape, starts a persistent Android 35 AVD, invokes the immutable RustDroid action revision, uploads the receipt, and stops the AVD in an `always()` step.

| Stack | Template | APK path |
| --- | --- | --- |
| Gradle Android | [`gradle-android-receipt.yml`](../examples/workflows/gradle-android-receipt.yml) | `app/build/outputs/apk/debug/app-debug.apk` |
| Flutter | [`flutter-receipt.yml`](../examples/workflows/flutter-receipt.yml) | `build/app/outputs/flutter-apk/app-debug.apk` |
| React Native / Expo prebuild | [`react-native-expo-receipt.yml`](../examples/workflows/react-native-expo-receipt.yml) | `android/app/build/outputs/apk/debug/app-debug.apk` |

They all emit the [receipt schema](receipt-schema-v1.md). The provisioner action creates the AVD definition but owns and stops its short-lived emulator process, so the explicit boot step is required before the receipt action can connect to `emulator-5554`. These are workflow templates, not sample apps: RustDroid receives the APK each stack already builds.
