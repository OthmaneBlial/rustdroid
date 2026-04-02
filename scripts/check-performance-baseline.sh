#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARTIFACT_DIR="${RUSTDROID_PERF_ARTIFACTS_DIR:-$ROOT_DIR/ci-artifacts/performance}"
BASELINE_PATH="${RUSTDROID_PERF_BASELINE_PATH:-$ROOT_DIR/base/performance-baselines.json}"
AVD="${RUSTDROID_PERF_AVD:-test_avd}"
PORT="${RUSTDROID_PERF_PORT:-5670}"
SERIAL="${RUSTDROID_PERF_SERIAL:-emulator-${PORT}}"
GPU_MODE="${RUSTDROID_PERF_GPU_MODE:-swiftshader_indirect}"
EMULATOR_ARGS="${RUSTDROID_PERF_EMULATOR_ARGS:--no-audio -no-boot-anim -no-snapshot -no-snapshot-save -no-metrics -camera-back none -camera-front none -skip-adb-auth -read-only}"
FIXTURE_APK="$ROOT_DIR/tests/fixtures/apks/launch-success.apk"
HOST_LOG_ROOT="${TMPDIR:-/tmp}/rustdroid/host"

mkdir -p "$ARTIFACT_DIR"
rm -f "$ARTIFACT_DIR"/bench.json "$ARTIFACT_DIR"/result.txt "$ARTIFACT_DIR"/emulator.log

cleanup() {
  adb -s "$SERIAL" emu kill >/dev/null 2>&1 || true
}

trap cleanup EXIT

adb -s "$SERIAL" emu kill >/dev/null 2>&1 || true

bench_json="$(
  ./target/debug/rustdroid \
    --runtime-backend host \
    --host-avd-name "$AVD" \
    --host-emulator-port "$PORT" \
    --adb-serial "$SERIAL" \
    --boot-mode cold \
    --headless true \
    --emulator-gpu-mode "$GPU_MODE" \
    "--emulator-additional-args=$EMULATOR_ARGS" \
    --json bench "$FIXTURE_APK"
)"

printf '%s\n' "$bench_json" >"$ARTIFACT_DIR/bench.json"
host_log_path="$(
  find "$HOST_LOG_ROOT" -mindepth 2 -maxdepth 2 -name emulator.log -print 2>/dev/null \
    | xargs -r ls -1t 2>/dev/null \
    | head -n 1
)"
if [[ -n "$host_log_path" && -f "$host_log_path" ]]; then
  cp "$host_log_path" "$ARTIFACT_DIR/emulator.log"
fi

python3 - "$BASELINE_PATH" "$ARTIFACT_DIR/bench.json" <<'PY'
import json
import sys

baseline_path, bench_path = sys.argv[1], sys.argv[2]
baseline = json.load(open(baseline_path))["host_fixture_launch_success"]
bench = json.load(open(bench_path))

checks = {
    "boot_duration_ms": baseline["boot_duration_ms_max"],
    "install_duration_ms": baseline["install_duration_ms_max"],
    "launch_duration_ms": baseline["launch_duration_ms_max"],
    "total_duration_ms": baseline["total_duration_ms_max"],
}

violations = []
for key, limit in checks.items():
    value = bench.get(key)
    if value is None:
        violations.append(f"missing {key}")
        continue
    if value > limit:
        violations.append(f"{key}={value} exceeds {limit}")

with open(bench_path.replace("bench.json", "result.txt"), "w", encoding="utf-8") as handle:
    if violations:
      handle.write("failed\n" + "\n".join(violations) + "\n")
    else:
      handle.write("passed\n")

if violations:
    raise SystemExit("\n".join(violations))
PY
