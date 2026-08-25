#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GRADLE_VERSION="8.10.2"
GRADLE_SHA256="31c55713e40233a8303827ceb42ca48a47267a0ad4bab9177123121e71524c26"
CACHE_ROOT="${XDG_CACHE_HOME:-"$HOME/.cache"}/rustdroid-fixtures"
GRADLE_HOME="$CACHE_ROOT/gradle-$GRADLE_VERSION"

mkdir -p "$CACHE_ROOT"

if [[ ! -x "$GRADLE_HOME/bin/gradle" ]]; then
  download_dir="$(mktemp -d)"
  trap 'rm -rf "$download_dir"' EXIT
  archive="$download_dir/gradle-$GRADLE_VERSION-bin.zip"
  curl --fail --location --silent --show-error \
    "https://services.gradle.org/distributions/gradle-$GRADLE_VERSION-bin.zip" \
    --output "$archive"
  printf '%s  %s\n' "$GRADLE_SHA256" "$archive" | shasum -a 256 --check
  unzip -q "$archive" -d "$CACHE_ROOT"
fi

cd "$ROOT_DIR"
"$GRADLE_HOME/bin/gradle" --no-daemon :app:assembleDebug
