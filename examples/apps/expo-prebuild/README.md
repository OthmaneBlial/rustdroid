# Expo prebuild fixture

This source-only Expo app demonstrates the Android prebuild handoff used by a
React Native repository. The generated `android/` project is intentionally
ignored; `npx expo prebuild` reproduces it from the locked JavaScript project.

Requirements: Node.js 22.13 or newer, Android SDK platform 36, and JDK 17.

```sh
./scripts/build-debug-apk.sh
```

The resulting APK is `android/app/build/outputs/apk/debug/app-debug.apk`.
