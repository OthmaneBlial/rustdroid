#!/usr/bin/env bash

set -euo pipefail

REPO="${RUSTDROID_REPO:-OthmaneBlial/rustdroid}"
INSTALL_DIR="${RUSTDROID_INSTALL_DIR:-$HOME/.local/bin}"
BASH_COMPLETION_DIR="${RUSTDROID_BASH_COMPLETION_DIR:-$HOME/.local/share/bash-completion/completions}"
ZSH_COMPLETION_DIR="${RUSTDROID_ZSH_COMPLETION_DIR:-$HOME/.local/share/zsh/site-functions}"
MODE="auto"
VERSION="${RUSTDROID_VERSION:-latest}"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}

trap cleanup EXIT

usage() {
  cat <<'EOF'
RustDroid installer

Usage:
  ./install.sh [--release | --source] [--version TAG] [--install-dir PATH]

Options:
  --release           Only install from a GitHub release tarball
  --source            Only build from source
  --version TAG       Release tag to install. Defaults to the latest release.
  --install-dir PATH  Destination directory for the rustdroid binary
  -h, --help          Show this help
EOF
}

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

have_command() {
  command -v "$1" >/dev/null 2>&1
}

download() {
  local url="$1"
  local destination="$2"

  if have_command curl; then
    curl -fsSL "$url" -o "$destination"
    return
  fi

  if have_command wget; then
    wget -qO "$destination" "$url"
    return
  fi

  fail "curl or wget is required to download RustDroid"
}

resolve_latest_release() {
  local metadata="$TMP_DIR/latest-release.json"
  download "https://api.github.com/repos/${REPO}/releases/latest" "$metadata"
  grep -m1 '"tag_name"' "$metadata" | sed -E 's/.*"([^"]+)".*/\1/'
}

release_asset_name() {
  case "$(uname -m)" in
    x86_64|amd64)
      printf 'rustdroid-x86_64-unknown-linux-musl.tar.gz'
      ;;
    aarch64|arm64)
      printf 'rustdroid-aarch64-unknown-linux-musl.tar.gz'
      ;;
    *)
      return 1
      ;;
  esac
}

install_binary() {
  local source_binary="$1"

  mkdir -p "$INSTALL_DIR"
  install -m 0755 "$source_binary" "$INSTALL_DIR/rustdroid"
  install_completions
  log "installed rustdroid to $INSTALL_DIR/rustdroid"
}

install_completions() {
  mkdir -p "$BASH_COMPLETION_DIR" "$ZSH_COMPLETION_DIR"
  "$INSTALL_DIR/rustdroid" completions bash > "$BASH_COMPLETION_DIR/rustdroid"
  "$INSTALL_DIR/rustdroid" completions zsh > "$ZSH_COMPLETION_DIR/_rustdroid"
}

ensure_source_prereqs() {
  have_command cargo || fail "cargo is required for source installation"
}

build_from_source() {
  ensure_source_prereqs

  local source_dir="$PWD"
  if [[ ! -f "$source_dir/Cargo.toml" ]]; then
    have_command git || fail "git is required to clone RustDroid for source installation"
    source_dir="$TMP_DIR/rustdroid"
    log "cloning https://github.com/${REPO}.git"
    git clone --depth 1 "https://github.com/${REPO}.git" "$source_dir" >/dev/null 2>&1
  fi

  log "building rustdroid from source"
  (cd "$source_dir" && cargo build --release --locked)
  install_binary "$source_dir/target/release/rustdroid"
}

install_from_release() {
  local resolved_version="$VERSION"
  local asset
  local archive
  local binary_path

  asset="$(release_asset_name)" || return 1
  if [[ "$resolved_version" == "latest" ]]; then
    resolved_version="$(resolve_latest_release)"
  fi
  [[ -n "$resolved_version" ]] || return 1

  archive="$TMP_DIR/$asset"
  log "downloading ${REPO} ${resolved_version} release"
  download "https://github.com/${REPO}/releases/download/${resolved_version}/${asset}" "$archive"
  tar -xzf "$archive" -C "$TMP_DIR"
  binary_path="$(find "$TMP_DIR" -type f -name rustdroid | head -n 1)"
  [[ -n "$binary_path" ]] || fail "release archive did not contain a rustdroid binary"
  install_binary "$binary_path"
}

print_post_install() {
  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    log
    log "add this to your shell profile if rustdroid is not on PATH yet:"
    log "  export PATH=\"$INSTALL_DIR:\$PATH\""
  fi

  log
  log "next steps:"
  log "  rustdroid doctor"
  log "  rustdroid self-test"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release)
      MODE="release"
      ;;
    --source)
      MODE="source"
      ;;
    --version)
      shift
      [[ $# -gt 0 ]] || fail "--version requires a value"
      VERSION="$1"
      ;;
    --install-dir)
      shift
      [[ $# -gt 0 ]] || fail "--install-dir requires a value"
      INSTALL_DIR="$1"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
  shift
done

case "$MODE" in
  release)
    install_from_release || fail "release installation failed"
    ;;
  source)
    build_from_source
    ;;
  auto)
    if ! install_from_release; then
      log "release install unavailable, falling back to source build"
      build_from_source
    fi
    ;;
  *)
    fail "unsupported install mode: $MODE"
    ;;
esac

print_post_install
