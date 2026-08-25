# Executable Android stack fixtures

These are deliberately tiny, public source apps. They turn the three copyable
receipt templates into buildable adoption proofs without committing an APK,
Android SDK, Gradle cache, or `node_modules` directory.

| Stack | Build command | Debug APK produced |
| --- | --- | --- |
| Gradle Android | `cd gradle-android && ./scripts/build-debug-apk.sh` | `app/build/outputs/apk/debug/app-debug.apk` |
| Flutter | `cd flutter && ./scripts/build-debug-apk.sh` | `build/app/outputs/flutter-apk/app-debug.apk` |
| React Native / Expo prebuild | `cd expo-prebuild && ./scripts/build-debug-apk.sh` | `android/app/build/outputs/apk/debug/app-debug.apk` |

The Gradle fixture verifies its pinned distribution checksum before use. The
Flutter and Expo fixtures create their generated Android directories locally;
those directories are ignored because their source of truth is the checked-in
Flutter or Expo project. Each fixture contains its own setup notes.

The repository's [`reference-stack-fixtures` workflow](../../.github/workflows/reference-stack-fixtures.yml)
builds every fixture, starts an Android 35 AVD, runs the immutable RustDroid
receipt action, and uploads the receipt. It runs monthly and can be dispatched
manually when a stack dependency changes.
