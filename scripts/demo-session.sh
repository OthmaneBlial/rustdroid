#!/usr/bin/env bash
set -euo pipefail
mark() { printf '%s,%s\n' "$1" "$(date +%s.%N)" >> demo-artifacts/timeline.csv; }
trap 'touch demo-artifacts/session-ended' EXIT
clear
printf 'RustDroid 0.3.2 candidate | real Android API 30\nWarm emulator; setup excluded. Full logs retained.\n\n'
rustdroid version
mark intro
sleep 4
for scenario in success failure; do
  apk=tests/fixtures/apks/launch-success.apk
  [[ "$scenario" == failure ]] && apk=tests/fixtures/apks/missing-launcher.apk
  mark "$scenario-start"
  printf '\n$ rustdroid --profile host-fast --host-avd-name test_avd \\\n  run %s \\\n  --duration-secs 3 --keep-alive true \\\n  --artifacts-dir demo-artifacts/%s\n' "$apk" "$scenario"
  result=0
  rustdroid --profile host-fast --host-avd-name test_avd \
    run "$apk" --duration-secs 3 --keep-alive true \
    --artifacts-dir "demo-artifacts/$scenario" > "demo-artifacts/$scenario-console.txt" 2>&1 || result=$?
  printf 'Exit code: %s\n' "$result"
  jq '{status, failure_stage, failure_classification}' "demo-artifacts/$scenario/run-summary.json"
  if [[ "$scenario" == success ]]; then
    [[ "$result" == 0 ]]
    jq -e '.status == "passed"' demo-artifacts/success/run-summary.json >/dev/null
  else
    [[ "$result" == 1 ]]
    jq -e '.failure_stage == "app_launch"' demo-artifacts/failure/run-summary.json >/dev/null
  fi
  mark "$scenario-result"
  sleep 6
done
google-chrome --no-sandbox --disable-gpu --no-first-run \
  --user-data-dir="$RUNNER_TEMP/demo-browser" --window-position=0,0 --window-size=1280,720 \
  "file://$PWD/demo-artifacts/failure/run-report.html" > demo-artifacts/browser.log 2>&1 &
browser_pid=$!
report_window=$(timeout 60 xdotool search --sync --onlyvisible --name 'RustDroid Run Receipt' | head -n 1)
[[ -n "$report_window" ]]
kill -0 "$browser_pid"
xdotool windowactivate --sync "$report_window"
mark report
sleep 6
xdotool key --clearmodifiers ctrl+End
mark report-artifacts
sleep 12
touch demo-artifacts/session-passed
