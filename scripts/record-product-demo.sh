#!/usr/bin/env bash
set -euo pipefail
[[ "$(uname -s)" == Linux && -e /dev/kvm ]]
mkdir -p demo-artifacts
export PATH="$PWD/demo-binary/rustdroid-x86_64-unknown-linux-musl:$PATH"
sha256sum demo-binary/rustdroid-x86_64-unknown-linux-musl.tar.gz > demo-artifacts/archive.sha256
printf '%s\n' '2754c3ee6fc3bb7795f0b2f0a8e100febb4be900' > demo-artifacts/product-source.txt
git rev-parse HEAD > demo-artifacts/recording-source.txt
adb -s emulator-5554 shell getprop > demo-artifacts/android-properties.txt
openbox > demo-artifacts/window-manager.log 2>&1 &
scrcpy --serial emulator-5554 --window-title Android --window-x 930 --window-y 25 \
  --window-width 330 --window-height 660 > demo-artifacts/scrcpy.log 2>&1 &
sleep 3
ffmpeg -hide_banner -loglevel warning -f x11grab -framerate 25 -video_size 1280x720 \
  -i "$DISPLAY" -c:v libx264 -preset ultrafast -crf 16 -pix_fmt yuv420p \
  demo-artifacts/desktop-uncut.mp4 > demo-artifacts/ffmpeg.log 2>&1 &
recorder_pid=$!
trap 'kill -INT "$recorder_pid" 2>/dev/null || true; wait "$recorder_pid" || true' EXIT
printf 'recording-start,%s\n' "$(date +%s.%N)" > demo-artifacts/timeline.csv
xterm -geometry 95x32+0+0 -fa 'DejaVu Sans Mono' -fs 13 -bg '#101820' -fg '#eef4ef' \
  -e bash scripts/demo-session.sh &
for attempt in $(seq 1 180); do
  [[ -f demo-artifacts/session-ended ]] && break
  sleep 1
done
[[ -f demo-artifacts/session-passed ]]
