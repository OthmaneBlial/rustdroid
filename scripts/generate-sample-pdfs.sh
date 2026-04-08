#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/sample-pdfs"

write_pdf() {
  local output_path="$1"
  local title="$2"
  local stream
  local stream_length
  local header
  local obj1
  local obj2
  local obj3
  local obj4
  local obj5
  local offset1
  local offset2
  local offset3
  local offset4
  local offset5
  local xref_offset

  stream=$'BT\n/F1 28 Tf\n72 720 Td\n('"$title"$') Tj\nET\n'
  stream_length=${#stream}

  header=$'%PDF-1.4\n'
  obj1=$'1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n'
  obj2=$'2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n'
  obj3=$'3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n'
  obj4=$'4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n'
  obj5=$'5 0 obj\n<< /Length '"$stream_length"$' >>\nstream\n'"$stream"$'endstream\nendobj\n'

  offset1=${#header}
  offset2=$((offset1 + ${#obj1}))
  offset3=$((offset2 + ${#obj2}))
  offset4=$((offset3 + ${#obj3}))
  offset5=$((offset4 + ${#obj4}))
  xref_offset=$((offset5 + ${#obj5}))

  {
    printf '%s' "$header"
    printf '%s' "$obj1"
    printf '%s' "$obj2"
    printf '%s' "$obj3"
    printf '%s' "$obj4"
    printf '%s' "$obj5"
    printf 'xref\n0 6\n'
    printf '0000000000 65535 f \n'
    printf '%010d 00000 n \n' "$offset1"
    printf '%010d 00000 n \n' "$offset2"
    printf '%010d 00000 n \n' "$offset3"
    printf '%010d 00000 n \n' "$offset4"
    printf '%010d 00000 n \n' "$offset5"
    printf 'trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n' "$xref_offset"
  } > "$output_path"
}

mkdir -p "$OUTPUT_DIR"

for number in 1 2 3 4; do
  write_pdf "$OUTPUT_DIR/${number}.pdf" "This document number ${number}"
done

printf 'generated sample PDFs in %s\n' "$OUTPUT_DIR"
