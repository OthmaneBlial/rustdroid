#!/usr/bin/env bash
set -euo pipefail

# Edit the real GitHub Actions desktop capture into the public, silent demo.
# The input is never modified. The timeline is written by
# scripts/record-product-demo.sh, so hosted-runner setup waits can be
# compressed without inventing a result or an Android frame.
input=${1:?usage: edit-product-demo.sh RAW_MP4 OUTPUT_MP4 [POSTER_PNG]}
output=${2:?usage: edit-product-demo.sh RAW_MP4 OUTPUT_MP4 [POSTER_PNG]}
poster=${3:-"${output%.*}-poster.png"}

[[ -s "$input" ]]
command -v ffmpeg >/dev/null
command -v ffprobe >/dev/null

duration=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$input")
awk -v d="$duration" 'BEGIN { if ((d + 0) < 100) exit 1 }'

mkdir -p "$(dirname "$output")"
caption_font=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
[[ -f "$caption_font" ]] || caption_font=$(fc-match -f '%{file}' sans 2>/dev/null || true)
[[ -n "$caption_font" && -f "$caption_font" ]]

timeline="$(dirname "$input")/timeline.csv"
[[ -s "$timeline" ]]
event() { awk -F, -v name="$1" '$1 == name { print $2; exit }' "$timeline"; }
recording_start=$(event recording-start)
success_start=$(event success-start)
success_result=$(event success-result)
failure_start=$(event failure-start)
failure_result=$(event failure-result)
report_start=$(event report)
success_report_start=$(event success-report-start || true)
success_report_end=$(event success-report-end || true)
for value in "$recording_start" "$success_start" "$success_result" "$failure_start" "$failure_result" "$report_start"; do
  [[ "$value" =~ ^[0-9]+\.[0-9]+$ ]]
done
relative() { awk -v value="$1" -v origin="$recording_start" 'BEGIN { printf "%.6f", (value - origin) }'; }
success_start=$(relative "$success_start")
success_result=$(relative "$success_result")
failure_start=$(relative "$failure_start")
failure_result=$(relative "$failure_result")
report_start=$(relative "$report_start")
if [[ "$success_report_start" =~ ^[0-9]+\.[0-9]+$ && "$success_report_end" =~ ^[0-9]+\.[0-9]+$ ]]; then
  success_report_start=$(relative "$success_report_start")
  success_report_end=$(relative "$success_report_end")
  has_success_report=true
else
  has_success_report=false
fi

success_wait_end=$(awk -v t="$success_result" 'BEGIN { printf "%.6f", t - 10 }')
success_result_end=$(awk -v t="$success_result" 'BEGIN { printf "%.6f", t + 1 }')
success_screen_start=$(awk -v t="$success_result" 'BEGIN { printf "%.6f", t + 1 }')
success_screen_limit=$(awk -v a="$success_result" -v b="$success_report_start" -v c="$failure_start" \
  'BEGIN { limit = a + 7; if (b + 0 > 0 && b < limit) limit = b - .5; if (c < limit) limit = c; printf "%.6f", limit }')
failure_end=$(awk -v t="$failure_result" 'BEGIN { printf "%.6f", t + 6 }')
report_end=$(awk -v t="$report_start" 'BEGIN { printf "%.6f", t + 12 }')
for value in "$success_wait_end" "$success_result_end" "$success_screen_limit" "$failure_end" "$report_end"; do
  [[ "$value" =~ ^[0-9]+\.[0-9]+$ ]]
done
awk -v a="$success_start" -v b="$success_wait_end" -v c="$success_result_end" \
  -v d="$success_screen_start" -v e="$success_screen_limit" -v f="$failure_start" \
  -v g="$failure_end" -v h="$report_start" -v i="$report_end" \
  'BEGIN { if (a <= 0 || b <= a || c <= b || d < c || e <= d || f < e || g <= f || h <= g || i <= h) exit 1 }'

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/rustdroid-demo-edit.XXXXXX")
trap 'rm -rf "$work_dir"' EXIT
concat_list="$work_dir/concat.txt"
: > "$concat_list"
declare -a labels starts ends durations speeds
segment_count=0

add_segment() {
  local label="$1" start="$2" end="$3" speed="$4" mode="$5"
  local segment="$work_dir/segment-${segment_count}.mp4"
  local filter="setpts=PTS/${speed}"
  if [[ "$mode" == phone ]]; then
    # scrcpy is intentionally kept small beside the terminal in the raw
    # capture. This is the real Android surface, enlarged without redrawing
    # its pixels or replacing it with a mockup.
    filter="crop=165:292:930:80,scale=400:707:flags=lanczos,pad=1280:720:440:6:color=0x101820,setpts=PTS/${speed}"
  fi
  ffmpeg -hide_banner -loglevel warning -y -ss "$start" -to "$end" -i "$input" \
    -vf "$filter" -an -r 25 -c:v libx264 -crf 23 -preset medium -profile:v high -level 4.0 \
    -pix_fmt yuv420p "$segment"
  printf "file '%s'\n" "$segment" >> "$concat_list"
  labels[$segment_count]="$label"
  starts[$segment_count]="$start"
  ends[$segment_count]="$end"
  speeds[$segment_count]="$speed"
  durations[$segment_count]=$(awk -v a="$start" -v b="$end" -v s="$speed" 'BEGIN { printf "%.6f", (b - a) / s }')
  segment_count=$((segment_count + 1))
}

add_segment "intro and provenance" 0 "$success_start" 1 full
add_segment "success command" "$success_start" "$success_wait_end" 4 full
add_segment "success result" "$success_wait_end" "$success_result_end" 1 full
add_segment "Android launch screen" "$success_screen_start" "$success_screen_limit" 1 phone
if [[ "$has_success_report" == true ]]; then
  add_segment "success HTML receipt" "$success_report_start" "$success_report_end" 1 full
fi
add_segment "failure command" "$failure_start" "$failure_result" 3 full
add_segment "failure result" "$failure_result" "$failure_end" 1 full
add_segment "failure HTML receipt" "$report_start" "$report_end" 1 full

joined="$work_dir/joined.mp4"
ffmpeg -hide_banner -loglevel warning -y -f concat -safe 0 -i "$concat_list" \
  -an -c copy -movflags +faststart "$joined"

total=0
has_drawtext=false
if ffmpeg -hide_banner -filters 2>/dev/null | awk '$2 == "drawtext" { found = 1 } END { exit !found }'; then
  has_drawtext=true
fi
caption_filter="[0:v]"
caption_time=0
for ((index = 0; index < segment_count; index++)); do
  next_time=$(awk -v a="$caption_time" -v b="${durations[$index]}" 'BEGIN { printf "%.6f", a + b }')
  case "${labels[$index]}" in
    "intro and provenance") caption="RustDroid 0.3.2 | REAL Linux/KVM runner | setup excluded" ;;
    "success command") caption="1  REAL LAUNCH | signed APK | setup wait accelerated 4x" ;;
    "success result") caption="SUCCESS | exit code 0 | status passed | app_launch observed" ;;
    "Android launch screen") caption="REAL ANDROID SCREEN | RustDroid Launch Fixture" ;;
    "success HTML receipt") caption="SUCCESS RECEIPT | passed | launch completed | artifacts kept" ;;
    "failure command") caption="2  EXPECTED FAILURE | missing launcher | setup accelerated 3x" ;;
    "failure result") caption="EXPECTED FAILURE | exit code 1 | stage app_launch" ;;
    "failure HTML receipt") caption="FAILURE RECEIPT | launch classification | digest + artifacts" ;;
    *) caption="RustDroid real run evidence" ;;
  esac
  if [[ "$has_drawtext" == true ]]; then
    caption_filter+="drawtext=fontfile='$caption_font':text='$caption':enable='between(t,$caption_time,$next_time)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12,"
  fi
  caption_time="$next_time"
  total="$next_time"
done
if [[ "$has_drawtext" == true ]]; then
  caption_filter="${caption_filter%,}[v]"
else
  caption_filter="[0:v]null[v]"
fi

ffmpeg -hide_banner -loglevel warning -y -i "$joined" -filter_complex "$caption_filter" \
  -map '[v]' -an -c:v libx264 -crf 23 -preset slow -profile:v high -level 4.0 \
  -pix_fmt yuv420p -movflags +faststart "$output"

poster_time=$(awk -v t="$total" 'BEGIN { printf "%.3f", t * .72 }')
ffmpeg -hide_banner -loglevel error -ss "$poster_time" -i "$output" -frames:v 1 -vf scale=1280:-2 \
  -y "$poster"

srt_time() {
  awk -v value="$1" 'BEGIN {
    if (value < 0) value = 0
    h = int(value / 3600); value -= h * 3600
    m = int(value / 60); value -= m * 60
    s = int(value); ms = int((value - s) * 1000 + .5)
    if (ms >= 1000) { s++; ms -= 1000 }
    if (s >= 60) { m++; s -= 60 }
    if (m >= 60) { h++; m -= 60 }
    printf "%02d:%02d:%02d,%03d", h, m, s, ms
  }'
}
{
  for ((index = 0; index < segment_count; index++)); do
    start_time=0
    for ((prior = 0; prior < index; prior++)); do
      start_time=$(awk -v a="$start_time" -v b="${durations[$prior]}" 'BEGIN { printf "%.6f", a + b }')
    done
    end_time=$(awk -v a="$start_time" -v b="${durations[$index]}" 'BEGIN { printf "%.6f", a + b }')
    case "${labels[$index]}" in
      "intro and provenance") caption="RustDroid 0.3.2 | real Linux/KVM runner | setup excluded" ;;
      "success command") caption="REAL LAUNCH | signed APK | setup wait accelerated 4x" ;;
      "success result") caption="SUCCESS | exit code 0 | status passed | app_launch observed" ;;
      "Android launch screen") caption="REAL ANDROID SCREEN | RustDroid Launch Fixture" ;;
      "success HTML receipt") caption="SUCCESS RECEIPT | passed | launch completed | artifacts kept" ;;
      "failure command") caption="EXPECTED FAILURE | missing launcher | setup accelerated 3x" ;;
      "failure result") caption="EXPECTED FAILURE | exit code 1 | stage app_launch" ;;
      "failure HTML receipt") caption="FAILURE RECEIPT | launch classification | digest + artifacts" ;;
      *) caption="RustDroid real run evidence" ;;
    esac
    printf '%d\n%s --> %s\n%s\n\n' "$((index + 1))" "$(srt_time "$start_time")" "$(srt_time "$end_time")" "$caption"
  done
} > "${output%.*}.srt"

ffprobe -v error -show_entries stream=codec_name,width,height,pix_fmt,r_frame_rate:format=duration,size \
  -of default=nw=1 "$output" > "${output%.*}.probe.txt"
awk '/^duration=/{d=$0; sub("duration=","",d)} /^size=/{s=$0; sub("size=","",s)} \
  /^codec_name=/{c=$0} /^width=/{w=$0} /^height=/{h=$0} /^pix_fmt=/{p=$0} \
  END { if ((d + 0) < 45 || (d + 0) > 60 || (s + 0) > 15728640 || c != "codec_name=h264" || w != "width=1280" || h != "height=720" || p != "pix_fmt=yuv420p") exit 1 }' \
  "${output%.*}.probe.txt"
printf '%s\n' \
  "intro and candidate provenance (real capture)" \
  "success command and result (real launch; setup wait compressed 4x)" \
  "Android launch screen (real scrcpy pixels cropped and enlarged for legibility)" \
  "failure command and result (missing-launcher fixture; expected app_launch classification)" \
  "HTML receipts (real generated reports; no simulated UI)" \
  > "${output%.*}.edit-notes.txt"
