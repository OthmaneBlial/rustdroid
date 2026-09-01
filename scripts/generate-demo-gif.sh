#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_SVG="$ROOT_DIR/assets/rustdroid-proof.svg"
OUTPUT_GIF="$ROOT_DIR/assets/rustdroid-demo.gif"
WORK_DIR="$(mktemp -d)"

trap 'rm -rf "$WORK_DIR"' EXIT

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

render_frame() {
  local number="$1"
  local title="$2"
  local command_line="$3"
  local detail_one="$4"
  local detail_two="$5"
  local detail_three="$6"
  local svg_path="$WORK_DIR/frame-${number}.svg"
  local png_path="$WORK_DIR/frame-${number}.png"

  awk \
    -v title="$title" \
    -v command_line="$command_line" \
    -v detail_one="$detail_one" \
    -v detail_two="$detail_two" \
    -v detail_three="$detail_three" \
    '
      /<\/svg>/ {
        print "  <g font-family=\"Menlo, Monaco, Consolas, monospace\">"
        print "    <rect x=\"62\" y=\"120\" width=\"1156\" height=\"490\" rx=\"8\" fill=\"#07100f\" fill-opacity=\".96\" stroke=\"#44544d\" stroke-width=\"1\"/>"
        print "    <rect x=\"62\" y=\"120\" width=\"1156\" height=\"42\" rx=\"8\" fill=\"#1d2724\"/>"
        print "    <circle cx=\"88\" cy=\"141\" r=\"6\" fill=\"#df664a\"/>"
        print "    <circle cx=\"108\" cy=\"141\" r=\"6\" fill=\"#f2b655\"/>"
        print "    <circle cx=\"128\" cy=\"141\" r=\"6\" fill=\"#7aa88b\"/>"
        print "    <text x=\"158\" y=\"146\" fill=\"#a8b8af\" font-size=\"13\">public fixture walkthrough — real steps, timing varies by host</text>"
        print "    <text x=\"98\" y=\"218\" fill=\"#f5a623\" font-size=\"14\" letter-spacing=\"2\">" title "</text>"
        print "    <text x=\"98\" y=\"278\" fill=\"#f5a623\" font-size=\"16\" font-weight=\"700\">$</text>"
        print "    <text x=\"124\" y=\"278\" fill=\"#f1f4ef\" font-size=\"16\">" command_line "</text>"
        print "    <line x1=\"98\" y1=\"318\" x2=\"1182\" y2=\"318\" stroke=\"#3d4d46\"/>"
        print "    <text x=\"98\" y=\"370\" fill=\"#79c692\" font-size=\"16\">✓ " detail_one "</text>"
        print "    <text x=\"98\" y=\"416\" fill=\"#79c692\" font-size=\"16\">✓ " detail_two "</text>"
        print "    <text x=\"98\" y=\"462\" fill=\"#79c692\" font-size=\"16\">✓ " detail_three "</text>"
        print "    <rect x=\"98\" y=\"512\" width=\"1084\" height=\"58\" rx=\"5\" fill=\"#0b1311\" stroke=\"#5d845f\"/>"
        print "    <text x=\"124\" y=\"548\" fill=\"#8fe0a6\" font-size=\"16\" font-weight=\"700\">APK path in. Launch receipt out.</text>"
        print "  </g>"
      }
      { print }
    ' "$SOURCE_SVG" > "$svg_path"

  if command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert --output "$png_path" "$svg_path"
  elif command -v sips >/dev/null 2>&1; then
    sips -s format png "$svg_path" --out "$png_path" >/dev/null
  else
    echo "install rsvg-convert or run this generator on macOS with sips" >&2
    exit 1
  fi
}

require_command awk
require_command ffmpeg
test -f "$SOURCE_SVG"

render_frame "01" "01 / 04  DIAGNOSE THE HOST" \
  "rustdroid --json doctor" \
  "select the host backend and inspect stable check IDs" \
  "confirm Android SDK tools and the test_avd before changing state" \
  "use the reviewable setup plan only when a prerequisite is missing"

render_frame "02" "02 / 04  RUN ONE PUBLIC FIXTURE" \
  "rustdroid --profile host-fast run launch-success.apk" \
  "read APK metadata and confirm the emulator ABI" \
  "boot or reuse the local Android Virtual Device" \
  "install the checked-in fixture without an Android Studio project"

render_frame "03" "03 / 04  OBSERVE THE LAUNCH" \
  "boot → install → launch → log capture" \
  "resolve the package and launchable activity" \
  "observe the fixture in the foreground" \
  "classify failures as emulator, ADB, install, launch, or log capture"

render_frame "04" "04 / 04  KEEP THE RECEIPT" \
  "artifacts/rustdroid-demo/run-summary.md" \
  "write JSON, HTML, JUnit, Markdown, and log evidence" \
  "upload the same directory from CI or inspect it locally" \
  "reproduce the result instead of trusting an opaque timeout"

ffmpeg -hide_banner -loglevel error -y \
  -framerate 1/2 -start_number 1 -i "$WORK_DIR/frame-%02d.png" \
  -filter_complex "fps=2,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=reserve_transparent=0[p];[b][p]paletteuse=dither=bayer" \
  -loop 0 "$OUTPUT_GIF"

echo "created $OUTPUT_GIF"
