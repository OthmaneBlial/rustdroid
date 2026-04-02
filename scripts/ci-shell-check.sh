#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

scripts=(
  install.sh
  uninstall.sh
  run.sh
  scripts/package-release.sh
  scripts/generate-fixture-apks.sh
  scripts/run-smoke-matrix.sh
)

for script in "${scripts[@]}"; do
  bash -n "$script"
done
