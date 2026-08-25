#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET="${1:-x86_64-unknown-linux-musl}"
VERSION="${2:-ci}"
ARCHIVE_PATH="$ROOT_DIR/dist/rustdroid-$TARGET.tar.gz"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"
CONTAINER_IMAGE="${RUSTDROID_INSTALL_TEST_IMAGE:-debian:bookworm-slim}"
REQUIRE_CONTAINER="${RUSTDROID_REQUIRE_CONTAINER:-0}"

if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then
  if [[ "$REQUIRE_CONTAINER" == "1" ]]; then
    echo "error: Docker is required for the clean-container release install check" >&2
    exit 1
  fi
  echo "skipping clean-container release install check: Docker is unavailable" >&2
  exit 0
fi

[[ -f "$ARCHIVE_PATH" ]] || {
  echo "error: release archive not found: $ARCHIVE_PATH" >&2
  exit 1
}
[[ -f "$CHECKSUM_PATH" ]] || {
  echo "error: release checksum not found: $CHECKSUM_PATH" >&2
  exit 1
}

docker run --rm \
  --network none \
  --volume "$ROOT_DIR/dist:/release:ro" \
  --env "RUSTDROID_TARGET=$TARGET" \
  --env "RUSTDROID_VERSION=$VERSION" \
  "$CONTAINER_IMAGE" \
  bash -euo pipefail -c '
    stage_dir="$(mktemp -d)"
    trap "rm -rf \"$stage_dir\"" EXIT
    archive_path="/release/rustdroid-${RUSTDROID_TARGET}.tar.gz"
    checksum_path="${archive_path}.sha256"
    tar -xzf "$archive_path" -C "$stage_dir"
    release_root="$stage_dir/rustdroid-${RUSTDROID_TARGET}"
    install_dir="$stage_dir/bin"
    completions_dir="$stage_dir/completions"

    RUSTDROID_BASH_COMPLETION_DIR="$completions_dir/bash" \
    RUSTDROID_ZSH_COMPLETION_DIR="$completions_dir/zsh" \
      "$release_root/install.sh" \
      --archive "$archive_path" \
      --checksum "$checksum_path" \
      --install-dir "$install_dir"

    "$install_dir/rustdroid" version
    "$install_dir/rustdroid" completions bash >/dev/null
    "$install_dir/rustdroid-run" help >/dev/null
    set +e
    "$install_dir/rustdroid" --json doctor >"$stage_dir/doctor.json"
    doctor_status=$?
    set -e
    [[ "$doctor_status" == "0" || "$doctor_status" == "10" ]]
    grep -q "\"checks\"" "$stage_dir/doctor.json"
  '

printf 'clean-container install verification passed for %s (%s)\n' "$TARGET" "$VERSION"
