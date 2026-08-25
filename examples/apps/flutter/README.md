# Flutter fixture

The checked-in Dart source is the fixture's source of truth. The first build
uses `flutter create` to regenerate only the standard Android host project,
then builds a debug APK with the installed Flutter SDK.

Requirements: Flutter 3.44.1 or a compatible stable Flutter release, Android
SDK platform 35 or newer, and JDK 17.

```sh
./scripts/build-debug-apk.sh
```

The resulting APK is `build/app/outputs/flutter-apk/app-debug.apk`.
