#!/usr/bin/env bash

set -euo pipefail

INSTALL_DIR="${RUSTDROID_INSTALL_DIR:-$HOME/.local/bin}"
BASH_COMPLETION_DIR="${RUSTDROID_BASH_COMPLETION_DIR:-$HOME/.local/share/bash-completion/completions}"
ZSH_COMPLETION_DIR="${RUSTDROID_ZSH_COMPLETION_DIR:-$HOME/.local/share/zsh/site-functions}"
DRY_RUN=0

usage() {
  cat <<'EOF'
RustDroid uninstaller

Usage:
  ./uninstall.sh [--dry-run]

Options:
  --dry-run  Show which files would be removed
  -h, --help Show this help
EOF
}

remove_path() {
  local path="$1"

  if [[ ! -e "$path" ]]; then
    return 0
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf 'would remove %s\n' "$path"
    return 0
  fi

  rm -f "$path"
  printf 'removed %s\n' "$path"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'error: unknown argument: %s\n' "$1" >&2
      exit 1
      ;;
  esac
  shift
done

remove_path "$INSTALL_DIR/rustdroid"
remove_path "$BASH_COMPLETION_DIR/rustdroid"
remove_path "$ZSH_COMPLETION_DIR/_rustdroid"

if [[ "$DRY_RUN" -eq 1 ]]; then
  printf 'dry-run only; no files were removed\n'
else
  printf 'removed RustDroid from %s\n' "$INSTALL_DIR"
fi
