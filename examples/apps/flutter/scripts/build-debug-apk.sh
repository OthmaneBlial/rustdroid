#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ ! -d android ]]; then
  flutter create --platforms=android --org dev.rustdroid.examples \
    --project-name rustdroid_flutter_fixture .
fi

flutter pub get
flutter test
flutter build apk --debug
