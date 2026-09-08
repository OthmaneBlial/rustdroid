#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p "$ROOT_DIR/dist"
INSTALL_ROOT="${1:-$(mktemp -d "$ROOT_DIR/dist/cargo-install-XXXXXX")}"

if [[ -e "$INSTALL_ROOT/bin/rustdroid" ]]; then
  echo "Refusing to overwrite an existing installation: $INSTALL_ROOT" >&2
  exit 2
fi
mkdir -p "$INSTALL_ROOT"

cargo package --locked --allow-dirty
cargo publish --dry-run --locked --allow-dirty
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
PACKAGE_DIR="${CARGO_TARGET_DIR:-target}/package/rustdroid-$VERSION"
cargo install --locked --path "$PACKAGE_DIR" --root "$INSTALL_ROOT"
"$INSTALL_ROOT/bin/rustdroid" version >/dev/null
