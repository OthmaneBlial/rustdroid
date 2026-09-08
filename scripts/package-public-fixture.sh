#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="${1:-$(sed -n 's/^version = "\(.*\)"/v\1/p' "$root/Cargo.toml" | head -n 1)}"
output="${2:-$root/dist}"
if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "expected a version such as v0.3.2" >&2
  exit 2
fi
fixture="$root/tests/fixtures/apks/launch-success.apk"
test -s "$fixture"
mkdir -p "$output"
filename="rustdroid-fixture-$version.apk"
install -m 0644 "$fixture" "$output/$filename"
(
  cd "$output"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$filename" > "$filename.sha256"
  else
    shasum -a 256 "$filename" > "$filename.sha256"
  fi
)
echo "Prepared $output/$filename and its portable checksum (not uploaded)"
