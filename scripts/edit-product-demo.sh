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
awk -v d="$duration" 'BEGIN { if (d < 100) exit 1 }'

mkdir -p "$(dirname "$output")"
caption_font=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
[[ -f "$caption_font" ]] || caption_font=$(fc-match -f '%{file}' sans 2>/dev/null || true)
[[ -n "$caption_font" && -f "$caption_font" ]]

# Source timeline (seconds): intro 0–4.4, success 4.4–56, failure 60–84,
# report 90.6–104.0. Waiting/setup sections are time-compressed, while the
# visible launch, result and report remain readable. No frames are invented.
filter="\
[0:v]trim=start=0:end=4.4,setpts=PTS-STARTPTS[a];\
[0:v]trim=start=4.4:end=37,setpts=PTS-STARTPTS,setpts=PTS/3[b];\
[0:v]trim=start=37:end=56,setpts=PTS-STARTPTS[c];\
[0:v]trim=start=60:end=84,setpts=PTS-STARTPTS,setpts=PTS/1.5[d];\
[0:v]trim=start=90.6:end=104,setpts=PTS-STARTPTS,setpts=PTS/1.5[e];\
[a][b][c][d][e]concat=n=5:v=1:a=0,\
drawtext=fontfile='$caption_font':text='RustDroid 0.3.2 | REAL Linux/KVM runner | setup excluded':\
enable='between(t,0,4.4)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12,\
drawtext=fontfile='$caption_font':text='1  REAL LAUNCH | public signed APK | setup wait 3x':\
enable='between(t,4.4,15.27)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12,\
drawtext=fontfile='$caption_font':text='LAUNCH VERIFIED | Android screen + observed receipt':\
enable='between(t,15.27,34.27)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12,\
drawtext=fontfile='$caption_font':text='2  EXPECTED FAILURE | missing launcher | exit 1':\
enable='between(t,34.27,50.27)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12,\
drawtext=fontfile='$caption_font':text='HTML RECEIPT | stage app_launch | digest + artifacts':\
enable='between(t,50.27,60)':x=28:y=674:fontsize=24:fontcolor=white:box=1:boxcolor=0x101820dd:boxborderw=12"

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
awk '/^duration=/{d=$0; sub("duration=","",d)} /^size=/{s=$0; sub("size=","",s)} END { if (d < 45 || d > 60 || s > 15728640) exit 1 }' "${output%.*}.probe.txt"
printf '%s\n' \
  '0.0–4.4  intro and candidate provenance (real capture)' \
  '4.4–15.27  success command; setup wait visibly accelerated 3x' \
  '15.27–34.27  successful Android launch and receipt (real-time)' \
  '34.27–50.27  missing-launcher command and classified app_launch failure (1.5x)' \
  '50.27–60.0  generated HTML receipt (1.5x; final report is real)' \
  > "${output%.*}.edit-notes.txt"
