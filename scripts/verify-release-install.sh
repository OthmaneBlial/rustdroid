#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET="${1:-x86_64-unknown-linux-musl}"
VERSION="${2:-v0.1.0}"
ARCHIVE_PATH="$ROOT_DIR/dist/rustdroid-$TARGET.tar.gz"
STAGE_DIR="$(mktemp -d)"
INSTALL_DIR="$STAGE_DIR/bin"

cleanup() {
  rm -rf "$STAGE_DIR"
}

trap cleanup EXIT

./scripts/package-release.sh "$TARGET" "$VERSION"
mkdir -p "$INSTALL_DIR"
tar -xzf "$ARCHIVE_PATH" -C "$STAGE_DIR"

release_root="$(find "$STAGE_DIR" -maxdepth 1 -type d -name "rustdroid-$TARGET" | head -n 1)"
[[ -n "$release_root" ]] || {
  echo "error: failed to locate extracted release directory" >&2
  exit 1
}

install -m 0755 "$release_root/rustdroid" "$INSTALL_DIR/rustdroid"
"$INSTALL_DIR/rustdroid" version >/dev/null
"$INSTALL_DIR/rustdroid" --help >/dev/null
