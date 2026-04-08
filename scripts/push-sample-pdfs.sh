#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PDF_DIR="$ROOT_DIR/sample-pdfs"
TARGET_DIR="${2:-/sdcard/Download}"
SERIAL="${1:-${ADB_SERIAL:-}}"
GENERATOR_SCRIPT="$ROOT_DIR/scripts/generate-sample-pdfs.sh"

ensure_sample_pdfs() {
  local pdf

  for pdf in "$PDF_DIR"/1.pdf "$PDF_DIR"/2.pdf "$PDF_DIR"/3.pdf "$PDF_DIR"/4.pdf; do
    [[ -f "$pdf" ]] || {
      "$GENERATOR_SCRIPT"
      return
    }
  done
}

detect_serial() {
  local devices

  mapfile -t devices < <(adb devices | awk 'NR > 1 && $2 == "device" { print $1 }')

  if (( ${#devices[@]} == 1 )); then
    printf '%s' "${devices[0]}"
    return 0
  fi

  if (( ${#devices[@]} == 0 )); then
    echo "error: no adb device is currently connected" >&2
    return 1
  fi

  echo "error: multiple adb devices detected; pass the serial explicitly" >&2
  printf 'devices: %s\n' "${devices[*]}" >&2
  return 1
}

ensure_sample_pdfs

if [[ -z "$SERIAL" ]]; then
  SERIAL="$(detect_serial)"
fi

adb -s "$SERIAL" shell mkdir -p "$TARGET_DIR" >/dev/null

for pdf in "$PDF_DIR"/1.pdf "$PDF_DIR"/2.pdf "$PDF_DIR"/3.pdf "$PDF_DIR"/4.pdf; do
  remote_path="$TARGET_DIR/$(basename "$pdf")"
  adb -s "$SERIAL" push "$pdf" "$remote_path" >/dev/null
  adb -s "$SERIAL" shell am broadcast \
    -a android.intent.action.MEDIA_SCANNER_SCAN_FILE \
    -d "file://$remote_path" >/dev/null 2>&1 || true
  printf 'pushed %s -> %s:%s\n' "$(basename "$pdf")" "$SERIAL" "$remote_path"
done

printf 'done: 4 PDFs pushed to %s\n' "$SERIAL"
