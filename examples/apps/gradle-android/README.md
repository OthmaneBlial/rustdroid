# Gradle Android fixture

This is a plain Android `Activity`, intentionally free of AndroidX and private
application code. It proves the conventional Gradle APK path used by the
receipt template.

Requirements: JDK 17, Android SDK platform/build tools 35, `curl`, and
`unzip`. The build script downloads Gradle 8.10.2 only after checking its
published SHA-256 digest.

```sh
./scripts/build-debug-apk.sh
```

The resulting APK is
`app/build/outputs/apk/debug/app-debug.apk`.
