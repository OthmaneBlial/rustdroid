#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET="${1:-x86_64-unknown-linux-musl}"
VERSION="${2:-ci}"
DIST_DIR="$ROOT_DIR/dist"
ARCHIVE_PATH="$DIST_DIR/rustdroid-$TARGET.tar.gz"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"
LISTING_PATH="$DIST_DIR/rustdroid-$TARGET.contents"

./scripts/check-cargo-distribution.sh "$DIST_DIR/cargo-install-root"
./scripts/package-release.sh "$TARGET" "$VERSION"
tar -tzf "$ARCHIVE_PATH" >"$LISTING_PATH"

grep -q '/rustdroid$' "$LISTING_PATH"
grep -q '/README.md$' "$LISTING_PATH"
grep -q '/LICENSE$' "$LISTING_PATH"
grep -q '/install.sh$' "$LISTING_PATH"
grep -q '/run.sh$' "$LISTING_PATH"
grep -q '/uninstall.sh$' "$LISTING_PATH"
sha256sum --check "$CHECKSUM_PATH"
