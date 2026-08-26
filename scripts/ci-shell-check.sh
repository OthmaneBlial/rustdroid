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

site_files=(
  site/index.html
  site/styles.css
  site/app.js
  site/docs.html
  site/docs.css
  site/docs.js
  site/docs/first-install.md
  site/docs/ci-examples.md
  site/docs/receipt-schema-v1.md
  site/assets/rustdroid-proof.svg
  site/assets/rustdroid-demo.gif
)

for file in "${site_files[@]}"; do
  test -f "$file"
done

node --check site/app.js
node --check site/docs.js

if grep -R -E 'href="/|src="/' --include='*.html' --include='*.js' site; then
  echo "The static site must keep links relative for GitHub Pages subpaths." >&2
  exit 1
fi
