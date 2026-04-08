#!/usr/bin/env bash

set -euo pipefail

REPO="${RUSTDROID_REPO:-OthmaneBlial/rustdroid}"
INSTALL_DIR="${RUSTDROID_INSTALL_DIR:-$HOME/.local/bin}"
BASH_COMPLETION_DIR="${RUSTDROID_BASH_COMPLETION_DIR:-$HOME/.local/share/bash-completion/completions}"
ZSH_COMPLETION_DIR="${RUSTDROID_ZSH_COMPLETION_DIR:-$HOME/.local/share/zsh/site-functions}"
HELPER_NAME="rustdroid-run"
MODE="auto"
VERSION="${RUSTDROID_VERSION:-latest}"
ARCHIVE_PATH=""
CHECKSUM_PATH=""
RUN_HEALTH_CHECK=0
TMP_DIR="$(mktemp -d)"
HELPER_INSTALLED=0

cleanup() {
  rm -rf "$TMP_DIR"
}

trap cleanup EXIT

usage() {
  cat <<'EOF'
RustDroid installer

Usage:
  ./install.sh [--release | --source] [--version TAG] [--install-dir PATH] [--archive PATH]

Options:
  --release           Only install from a GitHub release tarball
  --source            Only build from source
  --version TAG       Release tag to install. Defaults to the latest release.
  --archive PATH      Install from a local release archive
  --checksum PATH     Local or remote checksum file for --archive
  --install-dir PATH  Destination directory for the rustdroid tools
  --health-check      Run `rustdroid doctor` after install
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

host_architecture() {
  case "$(uname -m)" in
    x86_64|amd64)
      printf 'x86_64'
      ;;
    aarch64|arm64)
      printf 'aarch64'
      ;;
    *)
      return 1
      ;;
  esac
}

release_asset_name() {
  case "$(host_architecture)" in
    x86_64)
      printf 'rustdroid-x86_64-unknown-linux-musl.tar.gz'
      ;;
    aarch64)
      printf 'rustdroid-aarch64-unknown-linux-musl.tar.gz'
      ;;
    *)
      return 1
      ;;
  esac
}

install_runtime_artifacts() {
  local source_binary="$1"
  local source_helper="${2:-}"

  mkdir -p "$INSTALL_DIR"
  install -m 0755 "$source_binary" "$INSTALL_DIR/rustdroid"
  HELPER_INSTALLED=0
  if [[ -n "$source_helper" && -f "$source_helper" ]]; then
    install -m 0755 "$source_helper" "$INSTALL_DIR/$HELPER_NAME"
    HELPER_INSTALLED=1
  fi
  install_completions || log "warning: shell completions were not installed cleanly"
  log "installed rustdroid to $INSTALL_DIR/rustdroid"
  if [[ "$HELPER_INSTALLED" -eq 1 ]]; then
    log "installed helper to $INSTALL_DIR/$HELPER_NAME"
  else
    log "warning: helper script was not available in this package"
  fi
}

install_completions() {
  mkdir -p "$BASH_COMPLETION_DIR" "$ZSH_COMPLETION_DIR"
  "$INSTALL_DIR/rustdroid" completions bash > "$BASH_COMPLETION_DIR/rustdroid"
  "$INSTALL_DIR/rustdroid" completions zsh > "$ZSH_COMPLETION_DIR/_rustdroid"
  [[ -s "$BASH_COMPLETION_DIR/rustdroid" ]]
  [[ -s "$ZSH_COMPLETION_DIR/_rustdroid" ]]
}

ensure_source_prereqs() {
  have_command cargo || fail "cargo is required for source installation"
}

ensure_checksum_prereqs() {
  have_command sha256sum || fail "sha256sum is required to verify RustDroid release checksums"
}

verify_checksum() {
  local archive="$1"
  local checksum_file="$2"
  local checksum_line
  local expected
  local actual

  ensure_checksum_prereqs
  checksum_line="$(grep "$(basename "$archive")" "$checksum_file" | head -n 1 || true)"
  [[ -n "$checksum_line" ]] || fail "checksum file did not include $(basename "$archive")"

  expected="$(awk '{print $1}' <<<"$checksum_line")"
  actual="$(sha256sum "$archive" | awk '{print $1}')"
  [[ "$expected" == "$actual" ]] || fail "checksum verification failed for $(basename "$archive")"
}

find_release_root() {
  local search_root="$1"
  local binary_path

  binary_path="$(find "$search_root" -type f -name rustdroid | head -n 1)"
  [[ -n "$binary_path" ]] || fail "release archive did not contain a rustdroid binary"
  dirname "$binary_path"
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
  install_runtime_artifacts "$source_dir/target/release/rustdroid" "$source_dir/run.sh"
}

install_from_release() {
  local resolved_version="$VERSION"
  local asset
  local archive
  local checksum
  local release_root

  asset="$(release_asset_name)" || return 1
  if [[ "$resolved_version" == "latest" ]]; then
    resolved_version="$(resolve_latest_release)"
  fi
  [[ -n "$resolved_version" ]] || return 1

  archive="$TMP_DIR/$asset"
  checksum="$TMP_DIR/$asset.sha256"
  log "downloading ${REPO} ${resolved_version} release"
  download "https://github.com/${REPO}/releases/download/${resolved_version}/${asset}" "$archive"
  download "https://github.com/${REPO}/releases/download/${resolved_version}/${asset}.sha256" "$checksum"
  verify_checksum "$archive" "$checksum"
  tar -xzf "$archive" -C "$TMP_DIR"
  release_root="$(find_release_root "$TMP_DIR")"
  install_runtime_artifacts "$release_root/rustdroid" "$release_root/run.sh"
}

install_from_archive() {
  local archive="$ARCHIVE_PATH"
  local checksum_file="$CHECKSUM_PATH"
  local release_root

  [[ -n "$archive" ]] || fail "--archive requires a path"
  [[ -f "$archive" ]] || fail "archive not found: $archive"

  if [[ -n "$checksum_file" ]]; then
    [[ -f "$checksum_file" ]] || fail "checksum file not found: $checksum_file"
    verify_checksum "$archive" "$checksum_file"
  fi

  tar -xzf "$archive" -C "$TMP_DIR"
  release_root="$(find_release_root "$TMP_DIR")"
  install_runtime_artifacts "$release_root/rustdroid" "$release_root/run.sh"
}

run_health_check() {
  log
  log "running post-install health check"
  "$INSTALL_DIR/rustdroid" doctor
}

print_post_install() {
  local detected_arch
  detected_arch="$(host_architecture 2>/dev/null || true)"

  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    log
    log "add this to your shell profile if rustdroid is not on PATH yet:"
    log "  export PATH=\"$INSTALL_DIR:\$PATH\""
  fi

  if [[ -n "$detected_arch" ]]; then
    log
    log "detected architecture: $detected_arch"
  fi

  log
  log "completion files:"
  log "  bash: $BASH_COMPLETION_DIR/rustdroid"
  log "  zsh:  $ZSH_COMPLETION_DIR/_rustdroid"

  if [[ "$HELPER_INSTALLED" -eq 1 ]]; then
    log
    log "helper:"
    log "  $INSTALL_DIR/$HELPER_NAME"
  fi

  log
  log "next steps:"
  log "  rustdroid doctor"
  log "  rustdroid self-test"
  if [[ "$HELPER_INSTALLED" -eq 1 ]]; then
    log "  $HELPER_NAME fast-local app-debug.apk"
  fi
  log "  rustdroid --profile fast-local run app-debug.apk"
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
    --archive)
      shift
      [[ $# -gt 0 ]] || fail "--archive requires a value"
      ARCHIVE_PATH="$1"
      MODE="archive"
      ;;
    --checksum)
      shift
      [[ $# -gt 0 ]] || fail "--checksum requires a value"
      CHECKSUM_PATH="$1"
      ;;
    --install-dir)
      shift
      [[ $# -gt 0 ]] || fail "--install-dir requires a value"
      INSTALL_DIR="$1"
      ;;
    --health-check)
      RUN_HEALTH_CHECK=1
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
  archive)
    install_from_archive
    ;;
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

if [[ "$RUN_HEALTH_CHECK" -eq 1 ]]; then
  run_health_check
fi

print_post_install
