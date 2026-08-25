#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SERIAL="${RUSTDROID_HOST_TEST_SERIAL:-emulator-5554}"
AVD="${RUSTDROID_HOST_TEST_AVD:-test_avd}"
ARTIFACT_ROOT="${RUSTDROID_CI_HOST_ARTIFACTS_DIR:-$ROOT_DIR/ci-artifacts/host}"
RESULTS_FILE="$ARTIFACT_ROOT/results.env"
DIAGNOSTICS_FILE="$ARTIFACT_ROOT/host-diagnostics.txt"
CLASSIFICATION_FILE="$ARTIFACT_ROOT/failure-classification.md"

mkdir -p "$ARTIFACT_ROOT"
rm -f "$RESULTS_FILE"

runtime_status=0
backend_status=0
smoke_status=0

record_host_diagnostics() {
  {
    printf '# RustDroid host integration diagnostics\n\n'
    printf 'generated_at_utc: '
    date -u +'%Y-%m-%dT%H:%M:%SZ'
    printf 'workspace: %s\n' "$ROOT_DIR"
    printf 'runner: '
    uname -a
    printf 'user: '
    id
    printf '\n## Toolchain\n'
    rustc --version || true
    cargo --version || true
    adb version || true
    emulator -version || true
    printf '\n## Virtualization\n'
    if [[ -e /dev/kvm ]]; then
      ls -l /dev/kvm
    else
      printf '/dev/kvm is unavailable\n'
    fi
    printf '\n## Android devices\n'
    adb devices -l || true
    printf '\n## Available AVDs\n'
    emulator -list-avds || true
    printf '\n## RustDroid host state\n'
    find "${TMPDIR:-/tmp}/rustdroid/host" -maxdepth 3 -type f -print 2>/dev/null || true
  } >"$DIAGNOSTICS_FILE" 2>&1
}

log_contains() {
  local pattern="$1"
  grep -Eqi "$pattern" \
    "$ARTIFACT_ROOT/integration-host-runtime.log" \
    "$ARTIFACT_ROOT/integration-host-backend.log" \
    "$ARTIFACT_ROOT/smoke-matrix.log" 2>/dev/null
}

record_failure_classification() {
  {
    printf '# Host integration failure classification\n\n'
    printf '| Stage | Exit status |\n| --- | ---: |\n'
    printf '| integration_host_runtime | %s |\n' "$runtime_status"
    printf '| integration_host_backend | %s |\n' "$backend_status"
    printf '| smoke_matrix | %s |\n\n' "$smoke_status"

    if [[ "$runtime_status" -eq 0 && "$backend_status" -eq 0 && "$smoke_status" -eq 0 ]]; then
      printf 'Classification: success.\n'
      return
    fi

    printf 'Classification candidates:\n\n'
    if log_contains '(/dev/kvm|kvm).*([Pp]ermission denied|unavailable|not accessible)|hardware acceleration'; then
      printf -- '- `kvm-access`: the runner cannot use hardware virtualization.\n'
    fi
    if log_contains 'no host avd|no available avds|avd.*not found'; then
      printf -- '- `avd-setup`: the requested AVD is missing or unreadable.\n'
    fi
    if log_contains 'timed out waiting|boot.*timeout|failed to boot'; then
      printf -- '- `emulator-boot`: the emulator did not reach a booted state in time.\n'
    fi
    if log_contains 'adb.*(offline|failed|cannot connect|device.*not found)|failed to connect host adb'; then
      printf -- '- `adb-bridge`: ADB could not reach the emulator reliably.\n'
    fi
    if log_contains 'install.*(failed|error)|failed to install'; then
      printf -- '- `apk-install`: the fixture could not be installed.\n'
    fi
    if log_contains 'launch.*(failed|error)|failed to launch|foreground'; then
      printf -- '- `app-launch`: the fixture did not reach its expected launch state.\n'
    fi
    if log_contains 'logcat.*(failed|error)|failed to collect'; then
      printf -- '- `log-capture`: logs or run evidence could not be captured.\n'
    fi
    if log_contains 'failed to stop|cleanup|dropping stale host state'; then
      printf -- '- `cleanup`: RustDroid could not clean up the managed emulator state.\n'
    fi
    printf '\nRead the three stage logs and `host-diagnostics.txt` before retrying.\n'
  } >"$CLASSIFICATION_FILE"
}

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

record_host_diagnostics
record_failure_classification

cat >"$RESULTS_FILE" <<EOF
runtime_status=$runtime_status
backend_status=$backend_status
smoke_status=$smoke_status
EOF

if [[ "$runtime_status" -ne 0 || "$backend_status" -ne 0 || "$smoke_status" -ne 0 ]]; then
  exit 1
fi
