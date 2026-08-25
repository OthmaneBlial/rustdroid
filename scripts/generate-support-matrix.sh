#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_PATH="$ROOT_DIR/docs/support-matrix.json"
EXPECTED_PATH="$(mktemp)"
trap 'rm -f "$EXPECTED_PATH"' EXIT

cat >"$EXPECTED_PATH" <<'JSON'
{
  "schema_version": 1,
  "generated_by": "scripts/generate-support-matrix.sh",
  "contract_combinations": [
    {
      "backend": "host",
      "runner": "GitHub-hosted Ubuntu 22.04 with KVM",
      "android_api": "30",
      "abi": "x86_64",
      "ui_mode": "headless",
      "evidence": "host-integration-runtime",
      "evidence_url": "https://github.com/OthmaneBlial/rustdroid/actions/runs/32907975602/attempts/2",
      "verification_state": "verified"
    },
    {
      "backend": "host",
      "runner": "fresh Ubuntu 22.04 contract",
      "android_api": "35",
      "abi": "x86_64",
      "ui_mode": "headless",
      "evidence": "fresh-machine-contract",
      "evidence_url": "https://github.com/OthmaneBlial/rustdroid/actions/runs/32905479252",
      "verification_state": "verified"
    },
    {
      "backend": "host",
      "runner": "pinned action contract",
      "android_api": "35",
      "abi": "x86_64",
      "ui_mode": "headless",
      "evidence": "action-contract",
      "evidence_url": "https://github.com/OthmaneBlial/rustdroid/actions/runs/32907241889",
      "verification_state": "verified"
    }
  ],
  "source_only": ["aarch64 Linux release archives"],
  "not_supported": ["iOS", "remote device farms"]
}
JSON

case "${1:-}" in
  --check) diff -u "$OUTPUT_PATH" "$EXPECTED_PATH" ;;
  "") cp "$EXPECTED_PATH" "$OUTPUT_PATH" ;;
  *) echo "usage: $0 [--check]" >&2; exit 2 ;;
esac
