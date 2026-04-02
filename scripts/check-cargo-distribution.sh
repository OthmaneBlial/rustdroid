#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

INSTALL_ROOT="${1:-$ROOT_DIR/dist/cargo-install-root}"

rm -rf "$INSTALL_ROOT"
mkdir -p "$INSTALL_ROOT"

cargo package --allow-dirty
cargo publish --dry-run --allow-dirty
cargo install --path . --root "$INSTALL_ROOT"
"$INSTALL_ROOT/bin/rustdroid" version >/dev/null
