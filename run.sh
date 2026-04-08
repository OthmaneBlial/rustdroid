#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DEFAULT_APK="app-debug.apk"
DEFAULT_DURATION_SECS="10"
DEFAULT_FAST_LOCAL_ADB_PORT="5556"
DEFAULT_LOCAL_ADB_PORT="5557"
DEFAULT_WEB_VNC_PORT="6090"
DEFAULT_VNC_PORT="5900"

BINARY_PATH=""
REPO_ROOT=""
BINARY_MODE=""
CONTAINER_NAME=""
RUNTIME_ARGS=()
INTERRUPTED=0

print_usage() {
  cat <<'EOF'
RustDroid helper

Usage:
  ./run.sh [mode] [apk]
  rustdroid-run [mode] [apk]

Modes:
  fast-local  Aggressive local run tuned for speed
  local       Balanced local run with better app compatibility
  host-local  Host Android SDK emulator with scrcpy
  host-headless  Host Android SDK emulator without UI
  host-logs   Stream logs from the active host helper emulator
  web         Visible browser run
  vnc         Visible VNC run
  headless    Headless run with log streaming
  logs        Stream logs from the active helper container
  stop        Stop all run.sh-managed emulator containers
  help        Show this help

Examples:
  ./run.sh
  ./run.sh fast-local
  rustdroid-run host-local app-debug.apk
  ./run.sh web app-debug.apk
  ./run.sh stop
EOF
}

choose_mode() {
  cat >&2 <<'EOF'
Choose a RustDroid mode:
  1) fast-local  Aggressive speed mode
  2) local       Balanced local mode
  3) host-local  Host SDK emulator + scrcpy
  4) host-headless Host SDK emulator, no UI
  5) host-logs   Stream logs from host runtime
  6) web         Browser UI
  7) vnc         Native VNC UI
  8) headless    Headless run
  9) logs        Stream logs
  10) stop       Stop all helper runtimes
EOF
  printf 'Selection [1-10]: ' >&2
  read -r selection

  case "$selection" in
    1) printf 'fast-local' ;;
    2) printf 'local' ;;
    3) printf 'host-local' ;;
    4) printf 'host-headless' ;;
    5) printf 'host-logs' ;;
    6) printf 'web' ;;
    7) printf 'vnc' ;;
    8) printf 'headless' ;;
    9) printf 'logs' ;;
    10) printf 'stop' ;;
    *) echo "invalid selection: $selection" >&2; exit 1 ;;
  esac
}

resolve_binary() {
  if [[ -n "${RUSTDROID_BIN_PATH:-}" ]]; then
    BINARY_PATH="$RUSTDROID_BIN_PATH"
    BINARY_MODE="explicit"
    return
  fi

  if [[ -x "$SCRIPT_DIR/rustdroid" ]]; then
    BINARY_PATH="$SCRIPT_DIR/rustdroid"
    BINARY_MODE="installed"
    return
  fi

  if [[ -f "$SCRIPT_DIR/Cargo.toml" ]]; then
    REPO_ROOT="$SCRIPT_DIR"
    BINARY_PATH="$REPO_ROOT/target/debug/rustdroid"
    BINARY_MODE="repo"
    return
  fi

  if command -v rustdroid >/dev/null 2>&1; then
    BINARY_PATH="$(command -v rustdroid)"
    BINARY_MODE="path"
    return
  fi

  echo "error: unable to locate the rustdroid binary" >&2
  echo "hint: install rustdroid first or set RUSTDROID_BIN_PATH" >&2
  exit 1
}

ensure_binary() {
  resolve_binary

  if [[ "$BINARY_MODE" != "repo" ]]; then
    [[ -x "$BINARY_PATH" ]] || {
      echo "error: rustdroid binary is not executable: $BINARY_PATH" >&2
      exit 1
    }
    return
  fi

  if [[ ! -x "$BINARY_PATH" ]]; then
    echo "building rustdroid"
    cargo build --manifest-path "$REPO_ROOT/Cargo.toml"
    return
  fi

  if [[ "$REPO_ROOT/Cargo.toml" -nt "$BINARY_PATH" ]]; then
    echo "rebuilding rustdroid"
    cargo build --manifest-path "$REPO_ROOT/Cargo.toml"
    return
  fi

  while IFS= read -r file; do
    if [[ "$file" -nt "$BINARY_PATH" ]]; then
      echo "rebuilding rustdroid"
      cargo build --manifest-path "$REPO_ROOT/Cargo.toml"
      return
    fi
  done < <(find "$REPO_ROOT/src" -type f)
}

stop_container() {
  local container="$1"
  [[ -z "$container" ]] && return 0

  echo
  echo "stopping $container"
  "$BINARY_PATH" "${RUNTIME_ARGS[@]}" --container-name "$container" stop >/dev/null 2>&1 || true
}

stop_all_helper_containers() {
  local containers=(
    rustdroid-run-fast-local
    rustdroid-run-local
    rustdroid-run-web
    rustdroid-run-vnc
    rustdroid-run-headless
  )
  local host_containers=(
    rustdroid-run-host-local
    rustdroid-run-host-headless
  )
  local container

  for container in "${containers[@]}"; do
    "$BINARY_PATH" --container-name "$container" stop >/dev/null 2>&1 || true
  done

  for container in "${host_containers[@]}"; do
    "$BINARY_PATH" --runtime-backend host --container-name "$container" stop >/dev/null 2>&1 || true
  done
}

port_in_use() {
  local port="$1"
  ss -Hltn "( sport = :$port )" | grep -q .
}

find_free_port() {
  local start_port="$1"
  local end_port="$2"
  local port="$start_port"

  while (( port <= end_port )); do
    if ! port_in_use "$port"; then
      printf '%s' "$port"
      return 0
    fi
    ((port++))
  done

  echo "no free TCP port found between ${start_port} and ${end_port}" >&2
  exit 1
}

existing_bound_port() {
  local container="$1"
  local container_port="$2"

  docker inspect \
    --format "{{with index .HostConfig.PortBindings \"${container_port}\"}}{{(index . 0).HostPort}}{{end}}" \
    "$container" 2>/dev/null || true
}

container_running() {
  local container="$1"

  docker inspect --format '{{.State.Running}}' "$container" 2>/dev/null | grep -qx 'true'
}

host_pid_file() {
  local container="$1"
  printf '/tmp/rustdroid/host/%s/emulator.pid' "$container"
}

host_instance_running() {
  local container="$1"
  local pid_file
  local pid

  pid_file="$(host_pid_file "$container")"
  [[ -f "$pid_file" ]] || return 1
  pid="$(tr -d '[:space:]' < "$pid_file" 2>/dev/null || true)"
  [[ -n "$pid" && -d "/proc/$pid" ]]
}

resolve_host_port() {
  local container="$1"
  local container_port="$2"
  local preferred_port="$3"
  local end_port="$4"
  local current_port

  current_port="$(existing_bound_port "$container" "$container_port")"
  if [[ -n "$current_port" ]]; then
    if container_running "$container" || ! port_in_use "$current_port"; then
      printf '%s' "$current_port"
      return 0
    fi
  fi

  find_free_port "$preferred_port" "$end_port"
}

active_logs_target() {
  local host_containers=(
    rustdroid-run-host-local
    rustdroid-run-host-headless
  )
  local containers=(
    rustdroid-run-fast-local
    rustdroid-run-local
    rustdroid-run-web
    rustdroid-run-vnc
    rustdroid-run-headless
  )
  local container

  for container in "${host_containers[@]}"; do
    if host_instance_running "$container"; then
      printf 'host:%s' "$container"
      return 0
    fi
  done

  for container in "${containers[@]}"; do
    if container_running "$container"; then
      printf 'docker:%s' "$container"
      return 0
    fi
  done

  printf 'docker:rustdroid-run-fast-local'
}

on_interrupt() {
  INTERRUPTED=1
}

run_with_cleanup() {
  local exit_code=0

  trap on_interrupt INT TERM
  set +e
  "$@"
  exit_code=$?
  set -e
  trap - INT TERM

  if [[ $INTERRUPTED -eq 1 || $exit_code -eq 130 || $exit_code -eq 143 ]]; then
    stop_container "$CONTAINER_NAME"
  fi

  return "$exit_code"
}

MODE="${1:-}"
APK_PATH="${2:-$DEFAULT_APK}"

if [[ -z "$MODE" ]]; then
  MODE="$(choose_mode)"
fi

case "$MODE" in
  help|-h|--help)
    print_usage
    exit 0
    ;;
esac

ensure_binary

case "$MODE" in
  fast-local)
    RUNTIME_ARGS=(--profile fast-local)
    CONTAINER_NAME="rustdroid-run-fast-local"
    ADB_CONNECT_PORT="$(resolve_host_port "$CONTAINER_NAME" '5555/tcp' "$DEFAULT_FAST_LOCAL_ADB_PORT" 5599)"
    echo "mode: fast-local"
    echo "apk: $APK_PATH"
    echo "android image: budtmo/docker-android:emulator_12.0"
    echo "adb connect port: $ADB_CONNECT_PORT"
    run_with_cleanup \
      "$BINARY_PATH" \
      "${RUNTIME_ARGS[@]}" \
      --container-name "$CONTAINER_NAME" \
      --adb-connect-port "$ADB_CONNECT_PORT" \
      run "$APK_PATH" \
      --duration-secs "$DEFAULT_DURATION_SECS"
    ;;
  local)
    RUNTIME_ARGS=(--profile stable-local)
    CONTAINER_NAME="rustdroid-run-local"
    ADB_CONNECT_PORT="$(resolve_host_port "$CONTAINER_NAME" '5555/tcp' "$DEFAULT_LOCAL_ADB_PORT" 5599)"
    echo "mode: local"
    echo "apk: $APK_PATH"
    echo "android image: budtmo/docker-android:emulator_14.0"
    echo "adb connect port: $ADB_CONNECT_PORT"
    run_with_cleanup \
      "$BINARY_PATH" \
      "${RUNTIME_ARGS[@]}" \
      --container-name "$CONTAINER_NAME" \
      --adb-connect-port "$ADB_CONNECT_PORT" \
      run "$APK_PATH" \
      --duration-secs "$DEFAULT_DURATION_SECS"
    ;;
  host-local)
    RUNTIME_ARGS=(--profile host-fast)
    CONTAINER_NAME="rustdroid-run-host-local"
    echo "mode: host-local"
    echo "apk: $APK_PATH"
    echo "runtime backend: host"
    run_with_cleanup \
      "$BINARY_PATH" \
      "${RUNTIME_ARGS[@]}" \
      --container-name "$CONTAINER_NAME" \
      run "$APK_PATH" \
      --duration-secs "$DEFAULT_DURATION_SECS"
    ;;
  host-headless)
    RUNTIME_ARGS=(--profile host-fast)
    CONTAINER_NAME="rustdroid-run-host-headless"
    echo "mode: host-headless"
    echo "apk: $APK_PATH"
    echo "runtime backend: host"
    run_with_cleanup \
      "$BINARY_PATH" \
      "${RUNTIME_ARGS[@]}" \
      --container-name "$CONTAINER_NAME" \
      --headless true \
      run "$APK_PATH" \
      --duration-secs "$DEFAULT_DURATION_SECS"
    ;;
  host-logs)
    RUNTIME_ARGS=(--runtime-backend host)
    CONTAINER_NAME="rustdroid-run-host-local"
    if host_instance_running "rustdroid-run-host-headless"; then
      CONTAINER_NAME="rustdroid-run-host-headless"
    fi
    echo "mode: host-logs"
    echo "runtime backend: host"
    echo "container: $CONTAINER_NAME"
    run_with_cleanup \
      "$BINARY_PATH" \
      "${RUNTIME_ARGS[@]}" \
      --container-name "$CONTAINER_NAME" \
      logs
    ;;
  web)
    RUNTIME_ARGS=(--profile browser-demo)
    CONTAINER_NAME="rustdroid-run-web"
    WEB_VNC_PORT="$(resolve_host_port "$CONTAINER_NAME" '6080/tcp' "$DEFAULT_WEB_VNC_PORT" 6199)"
    echo "mode: web"
    echo "apk: $APK_PATH"
    echo "open: http://127.0.0.1:${WEB_VNC_PORT}?autoconnect=true"
    run_with_cleanup \
      "$BINARY_PATH" \
      "${RUNTIME_ARGS[@]}" \
      --container-name "$CONTAINER_NAME" \
      --web-vnc-port "$WEB_VNC_PORT" \
      run "$APK_PATH" \
      --duration-secs "$DEFAULT_DURATION_SECS"
    ;;
  vnc)
    RUNTIME_ARGS=()
    CONTAINER_NAME="rustdroid-run-vnc"
    VNC_PORT="$(resolve_host_port "$CONTAINER_NAME" '5900/tcp' "$DEFAULT_VNC_PORT" 5999)"
    echo "mode: vnc"
    echo "apk: $APK_PATH"
    echo "connect VNC client to: 127.0.0.1:${VNC_PORT}"
    run_with_cleanup \
      "$BINARY_PATH" \
      --container-name "$CONTAINER_NAME" \
      --headless false \
      --ui-backend vnc \
      --vnc-port "$VNC_PORT" \
      run "$APK_PATH" \
      --duration-secs "$DEFAULT_DURATION_SECS"
    ;;
  headless)
    RUNTIME_ARGS=(--profile fast-local)
    CONTAINER_NAME="rustdroid-run-headless"
    echo "mode: headless"
    echo "apk: $APK_PATH"
    run_with_cleanup \
      "$BINARY_PATH" \
      "${RUNTIME_ARGS[@]}" \
      --container-name "$CONTAINER_NAME" \
      --headless true \
      run "$APK_PATH" \
      --duration-secs "$DEFAULT_DURATION_SECS"
    ;;
  logs)
    IFS=':' read -r LOG_BACKEND CONTAINER_NAME <<< "$(active_logs_target)"
    RUNTIME_ARGS=()
    if [[ "$LOG_BACKEND" == "host" ]]; then
      RUNTIME_ARGS=(--runtime-backend host)
    fi
    echo "mode: logs"
    echo "runtime backend: ${LOG_BACKEND}"
    echo "container: $CONTAINER_NAME"
    run_with_cleanup \
      "$BINARY_PATH" \
      "${RUNTIME_ARGS[@]}" \
      --container-name "$CONTAINER_NAME" \
      logs
    ;;
  stop)
    echo "stopping helper containers"
    stop_all_helper_containers
    ;;
  *)
    echo "unknown mode: $MODE" >&2
    print_usage >&2
    exit 1
    ;;
esac
