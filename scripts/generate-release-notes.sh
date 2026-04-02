#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-}"
TARGET="${2:-x86_64-unknown-linux-musl}"
OUTPUT_PATH="${3:-dist/release-notes.md}"
SNIPPET_PATH="${4:-dist/install-snippet.txt}"
REPO="${RUSTDROID_REPO:-OthmaneBlial/rustdroid}"
ARCHIVE_NAME="rustdroid-$TARGET.tar.gz"
CHECKSUM_NAME="$ARCHIVE_NAME.sha256"
SNIPPET_NAME="$(basename "$SNIPPET_PATH")"
CURATED_NOTES_PATH="$ROOT_DIR/docs/releases/${VERSION}.md"

if [[ -z "$VERSION" ]]; then
  echo "usage: scripts/generate-release-notes.sh <version> [target] [output-path] [snippet-path]" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUTPUT_PATH")" "$(dirname "$SNIPPET_PATH")"

cat >"$SNIPPET_PATH" <<EOF
bash <(curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh)

# Or install a specific tag:
RUSTDROID_VERSION=${VERSION} bash <(curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh)
EOF

if [[ -f "$CURATED_NOTES_PATH" ]]; then
  cp "$CURATED_NOTES_PATH" "$OUTPUT_PATH"
  exit 0
fi

previous_tag="$(git tag --sort=-v:refname | grep -vx "$VERSION" | head -n 1 || true)"
if [[ -n "$previous_tag" ]]; then
  change_range="${previous_tag}..HEAD"
else
  change_range="HEAD"
fi

changes="$(git log --pretty='- %s (%h)' "$change_range" | head -n 20 || true)"
if [[ -z "$changes" ]]; then
  changes="- Initial public release."
fi

cat >"$OUTPUT_PATH" <<EOF
# RustDroid ${VERSION}

Fast Android emulator orchestration for local APK testing.

## Install

\`\`\`bash
$(cat "$SNIPPET_PATH")
\`\`\`

## Assets

- \`${ARCHIVE_NAME}\`
- \`${CHECKSUM_NAME}\`
- \`${SNIPPET_NAME}\`

## What Changed

${changes}

## Version Bump Checklist

Use [docs/version-bump-checklist.md](https://github.com/${REPO}/blob/main/docs/version-bump-checklist.md) before tagging the next release.
EOF
