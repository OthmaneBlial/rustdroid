#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export NODE_ENV="${NODE_ENV:-development}"

npm ci

if [[ ! -d android ]]; then
  npx expo prebuild --platform android --no-install
fi

(
  cd android
  ./gradlew --no-daemon :app:assembleDebug
)
