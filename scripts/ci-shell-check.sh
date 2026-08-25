#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

scripts=(
  install.sh
  uninstall.sh
  run.sh
  scripts/package-release.sh
  scripts/verify-release-install.sh
  scripts/verify-release-install-container.sh
  scripts/ci-host-check.sh
  scripts/ci-package-check.sh
  scripts/generate-fixture-apks.sh
  scripts/generate-support-matrix.sh
  scripts/run-smoke-matrix.sh
  examples/apps/gradle-android/scripts/build-debug-apk.sh
  examples/apps/flutter/scripts/build-debug-apk.sh
  examples/apps/expo-prebuild/scripts/build-debug-apk.sh
)

for script in "${scripts[@]}"; do
  bash -n "$script"
done

./scripts/generate-support-matrix.sh --check
