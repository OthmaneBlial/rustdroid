#!/usr/bin/env bash

set -euo pipefail

INSTALL_DIR="${RUSTDROID_INSTALL_DIR:-$HOME/.local/bin}"
BASH_COMPLETION_DIR="${RUSTDROID_BASH_COMPLETION_DIR:-$HOME/.local/share/bash-completion/completions}"
ZSH_COMPLETION_DIR="${RUSTDROID_ZSH_COMPLETION_DIR:-$HOME/.local/share/zsh/site-functions}"

rm -f "$INSTALL_DIR/rustdroid"
rm -f "$BASH_COMPLETION_DIR/rustdroid"
rm -f "$ZSH_COMPLETION_DIR/_rustdroid"

printf 'removed RustDroid from %s\n' "$INSTALL_DIR"
