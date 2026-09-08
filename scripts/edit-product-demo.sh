#!/usr/bin/env bash
set -euo pipefail

# Edit the real GitHub Actions desktop capture into the public, silent demo.
# The input is never modified. Times below are offsets in the capture produced
# by record-product-demo.sh; update them only with a newly inspected timeline.
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

# Read the measured event times from the same recording. Emulator boot and
# command durations vary between hosted runners, so fixed offsets can cut away
# the report. Waiting/setup sections are time-compressed, while the visible
# launch, result and report remain readable. No frames are invented.
timeline="$(dirname "$input")/timeline.csv"
[[ -s "$timeline" ]]
event() { awk -F, -v name="$1" '$1 == name { print $2; exit }' "$timeline"; }
recording_start=$(event recording-start)
success_start=$(event success-start)
success_result=$(event success-result)
failure_start=$(event failure-start)
failure_result=$(event failure-result)
report_start=$(event report)
for value in "$recording_start" "$success_start" "$success_result" "$failure_start" "$failure_result" "$report_start"; do
  [[ "$value" =~ ^[0-9]+\.[0-9]+$ ]]
done
relative() { awk -v value="$1" -v origin="$recording_start" 'BEGIN { printf "%.6f", (value - origin) }'; }
success_start=$(relative "$success_start")
success_result=$(relative "$success_result")
failure_start=$(relative "$failure_start")
failure_result=$(relative "$failure_result")
report_start=$(relative "$report_start")
success_live_start=$(awk -v t="$success_result" 'BEGIN { printf "%.6f", t - 10 }')
success_live_end=$(awk -v t="$success_result" 'BEGIN { printf "%.6f", t + 6 }')
failure_end=$(awk -v t="$failure_result" 'BEGIN { printf "%.6f", t + 6 }')
report_end=$(awk -v t="$report_start" 'BEGIN { printf "%.6f", t + 12 }')
awk -v a="$success_start" -v b="$success_live_start" -v c="$success_live_end" \
  -v d="$failure_start" -v e="$failure_end" -v f="$report_start" -v g="$report_end" \
  'BEGIN { if (a <= 0 || b <= a || c <= b || d <= c || e <= d || f <= e || g <= f) exit 1 }'

intro_len="$success_start"
warm_len=$(awk -v start="$success_start" -v end="$success_live_start" 'BEGIN { printf "%.6f", (end - start) / 3 }')
launch_len=$(awk -v start="$success_live_start" -v end="$success_live_end" 'BEGIN { printf "%.6f", end - start }')
failure_len=$(awk -v start="$failure_start" -v end="$failure_end" 'BEGIN { printf "%.6f", (end - start) / 1.5 }')
report_len=$(awk -v start="$report_start" -v end="$report_end" 'BEGIN { printf "%.6f", (end - start) / 1.5 }')
caption_1=$(awk -v a="$intro_len" 'BEGIN { printf "%.3f", a }')
caption_2=$(awk -v a="$intro_len" -v b="$warm_len" 'BEGIN { printf "%.3f", a + b }')
caption_3=$(awk -v a="$intro_len" -v b="$warm_len" -v c="$launch_len" 'BEGIN { printf "%.3f", a + b + c }')
caption_4=$(awk -v a="$intro_len" -v b="$warm_len" -v c="$launch_len" -v d="$failure_len" 'BEGIN { printf "%.3f", a + b + c + d }')
caption_5=$(awk -v a="$intro_len" -v b="$warm_len" -v c="$launch_len" -v d="$failure_len" -v e="$report_len" 'BEGIN { printf "%.3f", a + b + c + d + e }')

filter="\
[0:v]trim=start=0:end=$success_start,setpts=PTS-STARTPTS[a];\
[0:v]trim=start=$success_start:end=$success_live_start,setpts=PTS-STARTPTS,setpts=PTS/3[b];\
[0:v]trim=start=$success_live_start:end=$success_live_end,setpts=PTS-STARTPTS[c];\
[0:v]trim=start=$failure_start:end=$failure_end,setpts=PTS-STARTPTS,setpts=PTS/1.5[d];\
[0:v]trim=start=$report_start:end=$report_end,setpts=PTS-STARTPTS,setpts=PTS/1.5[e];\
[a][b][c][d][e]concat=n=5:v=1:a=0,\
drawtext=fontfile='$caption_font':text='RustDroid 0.3.2 | REAL Linux/KVM runner | setup excluded':\
enable='between(t,0,$caption_1)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12,\
drawtext=fontfile='$caption_font':text='1  REAL LAUNCH | public signed APK | setup wait 3x':\
enable='between(t,$caption_1,$caption_2)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12,\
drawtext=fontfile='$caption_font':text='LAUNCH VERIFIED | Android screen + observed receipt':\
enable='between(t,$caption_2,$caption_3)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12,\
drawtext=fontfile='$caption_font':text='2  EXPECTED FAILURE | missing launcher | exit 1':\
enable='between(t,$caption_3,$caption_4)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12,\
drawtext=fontfile='$caption_font':text='HTML RECEIPT | stage app_launch | digest + artifacts':\
enable='between(t,$caption_4,$caption_5)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12"
filter="${filter}[v]"

ffmpeg -hide_banner -loglevel warning -y -i "$input" -filter_complex "$filter" \
  -map '[v]' -an -c:v libx264 -crf 23 -preset slow -profile:v high -level 4.0 \
  -pix_fmt yuv420p -movflags +faststart "$output"

ffmpeg -hide_banner -loglevel error -ss 52 -i "$output" -frames:v 1 -vf scale=1280:-2 \
  -y "$poster"

cat > "${output%.*}.srt" <<'EOF'
1
00:00:00,000 --> 00:00:04,400
RustDroid 0.3.2 | real Linux/KVM runner | setup excluded

2
00:00:04,400 --> 00:00:15,270
REAL LAUNCH | public signed APK | setup wait accelerated 3x

3
00:00:15,270 --> 00:00:34,270
LAUNCH VERIFIED | Android screen and observed receipt

4
00:00:34,270 --> 00:00:50,270
EXPECTED FAILURE | missing launcher | exit code 1

5
00:00:50,270 --> 00:01:00,000
HTML RECEIPT | stage app_launch | digest and artifacts
EOF

ffprobe -v error -show_entries stream=codec_name,width,height,pix_fmt:format=duration,size \
  -of default=nw=1 "$output" > "${output%.*}.probe.txt"
awk '/^duration=/{d=$0; sub("duration=","",d)} /^size=/{s=$0; sub("size=","",s)} END { if ((d + 0) < 45 || (d + 0) > 60 || (s + 0) > 15728640) exit 1 }' "${output%.*}.probe.txt"
printf '%s\n' \
  "0.0–${caption_1}  intro and candidate provenance (real capture)" \
  "${caption_1}–${caption_2}  success command; setup wait visibly accelerated 3x" \
  "${caption_2}–${caption_3}  successful Android launch and receipt (real-time)" \
  "${caption_3}–${caption_4}  missing-launcher command and classified app_launch failure (1.5x)" \
  "${caption_4}–${caption_5}  generated HTML receipt (1.5x; final report is real)" \
  > "${output%.*}.edit-notes.txt"
