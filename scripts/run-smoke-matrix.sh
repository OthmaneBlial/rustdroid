#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="${RUSTDROID_BIN:-$ROOT_DIR/target/debug/rustdroid}"
FIXTURE_DIR="$ROOT_DIR/tests/fixtures/apks"
TMP_DIR="${RUSTDROID_SMOKE_TMPDIR:-$(mktemp -d)}"
OWN_TMP_DIR=0

if [[ -z "${RUSTDROID_SMOKE_TMPDIR:-}" ]]; then
  OWN_TMP_DIR=1
fi

mkdir -p "$TMP_DIR"

PORT="${RUSTDROID_SMOKE_PORT:-5660}"
SERIAL="${RUSTDROID_SMOKE_SERIAL:-emulator-${PORT}}"
RUNTIME_PORT="${RUSTDROID_SMOKE_RUNTIME_PORT:-$((PORT + 2))}"
RUNTIME_SERIAL="${RUSTDROID_SMOKE_RUNTIME_SERIAL:-emulator-${RUNTIME_PORT}}"
AVD="${RUSTDROID_SMOKE_AVD:-${RUSTDROID_HOST_AVD_NAME:-test_avd}}"
BOOT_TIMEOUT="${RUSTDROID_SMOKE_BOOT_TIMEOUT_SECS:-240}"
ENABLE_GUI_CASE="${RUSTDROID_SMOKE_ENABLE_GUI:-0}"
GPU_MODE="${RUSTDROID_SMOKE_GPU_MODE:-swiftshader_indirect}"
EMULATOR_ARGS="${RUSTDROID_SMOKE_EMULATOR_ARGS:--no-audio -no-boot-anim -no-snapshot -no-snapshot-save -no-metrics -camera-back none -camera-front none -skip-adb-auth -read-only}"
RUNTIME_HELPER_PID=""

PASSED_CASES=()
SKIPPED_CASES=()
CASE_SKIP_REASON=""

cleanup() {
  stop_runtime_helper
  if [[ -x "$BINARY" ]]; then
    "$BINARY" stop --all >/dev/null 2>&1 || true
  fi

  if [[ "$OWN_TMP_DIR" -eq 1 ]]; then
    rm -rf "$TMP_DIR"
  fi
}

trap cleanup EXIT

usage() {
  cat <<'EOF'
RustDroid smoke matrix

Usage:
  ./scripts/run-smoke-matrix.sh [--list] [--skip-build]

Environment:
  RUSTDROID_BIN                     Path to the rustdroid binary to test
  RUSTDROID_SMOKE_AVD              Host AVD name (default: test_avd)
  RUSTDROID_SMOKE_PORT             Host emulator console port (default: 5660)
  RUSTDROID_SMOKE_SERIAL           adb serial (default: emulator-${PORT})
  RUSTDROID_SMOKE_RUNTIME_PORT     Direct runtime helper emulator port (default: ${PORT}+2)
  RUSTDROID_SMOKE_RUNTIME_SERIAL   Direct runtime helper adb serial (default: emulator-${RUNTIME_PORT})
  RUSTDROID_SMOKE_BOOT_TIMEOUT_SECS Boot timeout for smoke cases
  RUSTDROID_SMOKE_ENABLE_GUI       Set to 1 to include the scrcpy-visible fast path
  RUSTDROID_SMOKE_GPU_MODE         Host emulator GPU mode for smoke runs
  RUSTDROID_SMOKE_EMULATOR_ARGS    Extra host emulator args for smoke runs
  RUSTDROID_SMOKE_TMPDIR           Reuse a fixed temp directory

Cases:
  - host-fast
  - host-headless
  - cold-boot
  - warm-reuse
  - install-only
  - launch-only
  - artifact-run
  - split-install
EOF
}

have_command() {
  command -v "$1" >/dev/null 2>&1
}

log() {
  printf '%s\n' "$*"
}

run_case() {
  local case_name="$1"
  shift

  log "==> ${case_name}"
  if "$@"; then
    PASSED_CASES+=("$case_name")
    return 0
  fi

  local status=$?
  if [[ "$status" -eq 20 ]]; then
    SKIPPED_CASES+=("${case_name}: ${CASE_SKIP_REASON}")
    CASE_SKIP_REASON=""
    return 0
  fi

  return "$status"
}

skip_case() {
  CASE_SKIP_REASON="$1"
  return 20
}

require_file() {
  local path="$1"
  [[ -f "$path" ]] || {
    printf 'error: missing required file %s\n' "$path" >&2
    exit 1
  }
}

ensure_binary() {
  if [[ ! -x "$BINARY" ]]; then
    cargo build --locked >/dev/null
  fi
}

runtime_command() {
  local case_name="$1"
  shift

  "$BINARY" \
    --runtime-backend host \
    --host-avd-name "$AVD" \
    --host-emulator-port "$RUNTIME_PORT" \
    --adb-serial "$RUNTIME_SERIAL" \
    --boot-timeout-secs "$BOOT_TIMEOUT" \
    --poll-interval-secs 2 \
    --emulator-gpu-mode "$GPU_MODE" \
    "--emulator-additional-args=$EMULATOR_ARGS" \
    --container-name "rustdroid-smoke-runtime-${case_name}" \
    "$@"
}

base_command() {
  local case_name="$1"
  shift

  "$BINARY" \
    --runtime-backend host \
    --host-avd-name "$AVD" \
    --host-emulator-port "$PORT" \
    --adb-serial "$SERIAL" \
    --boot-timeout-secs "$BOOT_TIMEOUT" \
    --poll-interval-secs 2 \
    --emulator-gpu-mode "$GPU_MODE" \
    "--emulator-additional-args=$EMULATOR_ARGS" \
    --container-name "rustdroid-smoke-${case_name}" \
    "$@"
}

prepare_host_case() {
  "$BINARY" stop --all >/dev/null 2>&1 || true
}

wait_for_runtime_device() {
  local deadline=$((SECONDS + BOOT_TIMEOUT))

  while (( SECONDS < deadline )); do
    if adb -s "$RUNTIME_SERIAL" shell getprop sys.boot_completed 2>/dev/null | grep -q "1"; then
      return 0
    fi
    sleep 2
  done

  printf 'error: timed out waiting for %s to boot\n' "$RUNTIME_SERIAL" >&2
  return 1
}

ensure_runtime_helper() {
  if adb -s "$RUNTIME_SERIAL" get-state >/dev/null 2>&1; then
    wait_for_runtime_device
    return 0
  fi

  stop_runtime_helper
  mkdir -p "$TMP_DIR"
  emulator \
    -avd "$AVD" \
    -port "$RUNTIME_PORT" \
    -gpu "$GPU_MODE" \
    $EMULATOR_ARGS \
    -no-window \
    >"$TMP_DIR/runtime-helper.log" 2>&1 &
  RUNTIME_HELPER_PID="$!"
  wait_for_runtime_device
}

stop_runtime_helper() {
  if adb -s "$RUNTIME_SERIAL" get-state >/dev/null 2>&1; then
    adb -s "$RUNTIME_SERIAL" emu kill >/dev/null 2>&1 || true
  fi

  if [[ -n "$RUNTIME_HELPER_PID" ]] && kill -0 "$RUNTIME_HELPER_PID" >/dev/null 2>&1; then
    kill "$RUNTIME_HELPER_PID" >/dev/null 2>&1 || true
    wait "$RUNTIME_HELPER_PID" 2>/dev/null || true
  fi

  RUNTIME_HELPER_PID=""
}

run_host_fast() {
  if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
    CASE_SKIP_REASON="no GUI session"
    return 20
  fi
  if ! have_command scrcpy; then
    CASE_SKIP_REASON="scrcpy is unavailable"
    return 20
  fi

  prepare_host_case
  base_command "host-fast" --headless false --ui-backend scrcpy open --wait false
  "$BINARY" stop --all >/dev/null 2>&1 || true
}

run_host_headless() {
  prepare_host_case
  base_command "host-headless" --headless true start --wait false
  "$BINARY" stop --all >/dev/null 2>&1 || true
}

run_cold_boot() {
  prepare_host_case
  base_command "cold-boot" --headless true --boot-mode cold start --wait false
  "$BINARY" stop --all >/dev/null 2>&1 || true
}

run_warm_reuse() {
  local output_path="$TMP_DIR/warm-reuse.log"

  prepare_host_case
  base_command "warm-reuse" --headless true --boot-mode warm start --wait false >/dev/null
  base_command "warm-reuse" --headless true --boot-mode warm start --wait false >"$output_path" 2>&1

  grep -q "reusing managed host emulator" "$output_path" || {
    printf 'error: warm-reuse case did not report managed reuse\n%s\n' "$(cat "$output_path")" >&2
    exit 1
  }
  "$BINARY" stop --all >/dev/null 2>&1 || true
}

run_install_only() {
  local apk="$FIXTURE_DIR/launch-success.apk"

  ensure_runtime_helper
  runtime_command "install-only" --headless true install "$apk"
}

run_launch_only() {
  local apk="$FIXTURE_DIR/launch-success.apk"

  ensure_runtime_helper
  runtime_command "launch-only" --headless true install "$apk" >/dev/null
  runtime_command "launch-only" --headless true launch --package com.rustdroid.fixture.launch
}

run_artifact_enabled() {
  local apk="$FIXTURE_DIR/launch-success.apk"
  local artifact_dir="$TMP_DIR/artifacts"

  ensure_runtime_helper
  rm -rf "$artifact_dir"
  runtime_command "artifact-run" --headless true run "$apk" \
    --duration-secs 1 \
    --keep-alive false \
    --artifacts-dir "$artifact_dir"

  require_file "$artifact_dir/run-summary.json"
  require_file "$artifact_dir/logcat.txt"
  require_file "$artifact_dir/run-report.html"
}

run_split_install() {
  local base_apk="$FIXTURE_DIR/split-base.apk"
  local locale_apk="$FIXTURE_DIR/split-config.en.apk"

  ensure_runtime_helper
  runtime_command "split-install" --headless true install "$base_apk" "$locale_apk"
}

print_summary() {
  log
  log "Smoke matrix summary"
  for case_name in "${PASSED_CASES[@]}"; do
    log "  PASS  ${case_name}"
  done
  for skipped in "${SKIPPED_CASES[@]}"; do
    log "  SKIP  ${skipped}"
  done
}

SKIP_BUILD=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --list|-h|--help)
      usage
      exit 0
      ;;
    --skip-build)
      SKIP_BUILD=1
      ;;
    *)
      printf 'error: unknown argument: %s\n' "$1" >&2
      exit 1
      ;;
  esac
  shift
done

require_file "$FIXTURE_DIR/launch-success.apk"
require_file "$FIXTURE_DIR/split-base.apk"
require_file "$FIXTURE_DIR/split-config.en.apk"
have_command adb || { printf 'error: adb is required for the smoke matrix\n' >&2; exit 1; }
have_command emulator || { printf 'error: emulator is required for the smoke matrix\n' >&2; exit 1; }

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  ensure_binary
fi

if [[ "$ENABLE_GUI_CASE" != "1" ]]; then
  log "==> host-fast"
  SKIPPED_CASES+=("host-fast: set RUSTDROID_SMOKE_ENABLE_GUI=1 to include the visible scrcpy lane")
else
  run_case "host-fast" run_host_fast
fi
run_case "host-headless" run_host_headless
run_case "cold-boot" run_cold_boot
run_case "warm-reuse" run_warm_reuse
run_case "install-only" run_install_only
run_case "launch-only" run_launch_only
run_case "artifact-run" run_artifact_enabled
run_case "split-install" run_split_install
print_summary
