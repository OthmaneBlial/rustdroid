#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SERIAL="${RUSTDROID_HOST_TEST_SERIAL:-emulator-5554}"
AVD="${RUSTDROID_HOST_TEST_AVD:-test_avd}"
ARTIFACT_ROOT="${RUSTDROID_CI_HOST_ARTIFACTS_DIR:-$ROOT_DIR/ci-artifacts/host}"
RESULTS_FILE="$ARTIFACT_ROOT/results.env"

mkdir -p "$ARTIFACT_ROOT"
rm -f "$RESULTS_FILE"

runtime_status=0
backend_status=0
smoke_status=0

RUSTDROID_RUN_HOST_RUNTIME_TESTS=1 \
RUSTDROID_HOST_TEST_SERIAL="$SERIAL" \
cargo test --test integration_host_runtime -- --nocapture \
  >"$ARTIFACT_ROOT/integration-host-runtime.log" 2>&1 || runtime_status=$?

RUSTDROID_RUN_HOST_BACKEND_TESTS=1 \
RUSTDROID_HOST_TEST_SERIAL="$SERIAL" \
RUSTDROID_HOST_TEST_AVD="$AVD" \
cargo test --test integration_host_backend -- --nocapture \
  >"$ARTIFACT_ROOT/integration-host-backend.log" 2>&1 || backend_status=$?

RUSTDROID_SMOKE_AVD="$AVD" \
RUSTDROID_SMOKE_TMPDIR="$ARTIFACT_ROOT/smoke" \
./scripts/run-smoke-matrix.sh --skip-build \
  >"$ARTIFACT_ROOT/smoke-matrix.log" 2>&1 || smoke_status=$?

cat >"$RESULTS_FILE" <<EOF
runtime_status=$runtime_status
backend_status=$backend_status
smoke_status=$smoke_status
EOF

if [[ "$runtime_status" -ne 0 || "$backend_status" -ne 0 || "$smoke_status" -ne 0 ]]; then
  exit 1
fi
