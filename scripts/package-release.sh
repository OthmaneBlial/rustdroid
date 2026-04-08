#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET="${1:-}"
VERSION="${2:-${GITHUB_REF_NAME:-$(git describe --tags --always 2>/dev/null || echo dev)}}"

if [[ -z "$TARGET" ]]; then
  echo "usage: scripts/package-release.sh <target-triple> [version]" >&2
  exit 1
fi

if command -v rustup >/dev/null 2>&1; then
  if ! rustup target list --installed | grep -qx "$TARGET"; then
    rustup target add "$TARGET"
  fi
fi

DIST_DIR="$ROOT_DIR/dist"
STAGE_DIR="$DIST_DIR/rustdroid-$TARGET"
ARCHIVE_PATH="$DIST_DIR/rustdroid-$TARGET.tar.gz"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"

cargo build --release --locked --target "$TARGET"

install -m 0755 "target/$TARGET/release/rustdroid" "$STAGE_DIR/rustdroid"
install -m 0644 README.md "$STAGE_DIR/README.md"
install -m 0644 LICENSE "$STAGE_DIR/LICENSE"
install -m 0755 install.sh "$STAGE_DIR/install.sh"
install -m 0755 run.sh "$STAGE_DIR/run.sh"
install -m 0755 uninstall.sh "$STAGE_DIR/uninstall.sh"
printf '%s\n' "$VERSION" > "$STAGE_DIR/VERSION"

tar -czf "$ARCHIVE_PATH" -C "$DIST_DIR" "$(basename "$STAGE_DIR")"
sha256sum "$ARCHIVE_PATH" > "$CHECKSUM_PATH"

echo "created $ARCHIVE_PATH"
echo "created $CHECKSUM_PATH"
