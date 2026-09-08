#!/usr/bin/env python3
"""Capture real CLI outputs and render an explicitly labelled FFmpeg replay.

This does not boot an emulator or pretend to record an Android launch.
Requires Pillow, ffmpeg, ffprobe and jq; builds use cargo build --locked first.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shlex
import subprocess
import textwrap
import time
from datetime import datetime, timezone

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parent.parent


def run(command, expected=0):
    started = time.monotonic()
    result = subprocess.run(
        command, cwd=ROOT, text=True, capture_output=True, timeout=30,
        env={**os.environ, "NO_COLOR": "1", "TERM": "dumb"},
    )
    if result.returncode != expected:
        raise RuntimeError(f"Unexpected exit {result.returncode}: {shlex.join(command)}\n{result.stderr}")
    return {
        "argv": command,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "exit_code": result.returncode,
        "elapsed_seconds": round(time.monotonic() - started, 4),
    }


def font(size):
    for candidate in (
        "/System/Library/Fonts/Menlo.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ):
        if Path(candidate).is_file():
            return ImageFont.truetype(candidate, size)
    raise RuntimeError("Install Menlo or DejaVu Sans Mono for legible output")


def draw_frame(path, number, title, subtitle, command, output, footer):
    im = Image.new("RGB", (1280, 720), "#101916")
    draw = ImageDraw.Draw(im)
    draw.text((44, 26), "RUSTDROID", font=font(20), fill="#b4e1c5")
    draw.text((1090, 26), f"0{number} / 05", font=font(18), fill="#aabbb1")
    draw.text((44, 70), title, font=font(30), fill="#f5f3e9")
    draw.text((44, 113), subtitle, font=font(16), fill="#aabbb1")
    draw.rounded_rectangle((36, 159, 1244, 646), radius=12, fill="#07100d", outline="#365548", width=2)
    draw.text((59, 177), "RECORDED COMMAND OUTPUT / EDITED READING TIME", font=font(14), fill="#87a596")
    y = 213
    for line in textwrap.wrap("$ " + command, width=91, subsequent_indent="  "):
        draw.text((59, y), line, font=font(20), fill="#edbd72")
        y += 28
    y += 13
    for line in output.splitlines():
        for wrapped in textwrap.wrap(line, width=94, replace_whitespace=False) or [""]:
            if y > 614:
                raise RuntimeError("Output does not fit; split the chapter instead of hiding output")
            draw.text((59, y), wrapped, font=font(19), fill="#d8e6dd")
            y += 25
    draw.text((44, 676), footer, font=font(15), fill="#aabbb1")
    im.save(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default="dist/cli-demo")
    args = parser.parse_args()
    out = (ROOT / args.output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    rel = out.relative_to(ROOT).as_posix()
    binary = "target/debug/rustdroid"
    config = f"{rel}/isolated.toml"
    common = [binary, "--config", config]
    failure = f"{rel}/failure"
    missing = f"{rel}/intentionally-missing.apk"
    if (ROOT / missing).exists():
        raise RuntimeError("The missing-input demonstration requires an absent file")
    entries = []

    def capture(title, subtitle, command, expected, duration):
        record = run(command, expected)
        record.update(title=title, subtitle=subtitle, display_seconds=duration)
        entries.append(record)

    capture("A real CLI. Inspectable evidence.",
            "CLI execution on macOS; Android runtime support remains Linux/KVM.",
            common + ["version"], 0, 5)
    capture("See the plan before changing anything.",
            "This is an explicit dry run. No emulator is started.",
            common + ["--profile", "host-fast", "--host-avd-name", "test_avd", "--dry-run",
                      "run", "tests/fixtures/apks/launch-success.apk", "--keep-alive", "false"], 0, 11)
    capture("A missing APK produces a failed receipt.",
            "Real failure, real nonzero exit. No Android host is needed for this check.",
            common + ["--profile", "host-fast", "run", missing, "--duration-secs", "1",
                      "--keep-alive", "false", "--artifacts-dir", failure], 1, 9)
    capture("The failure is data your CI can use.",
            "Reading the JSON file written by the previous command.",
            ["jq", "{status,failure_stage,failure_classification,error_summary}",
             f"{failure}/run-summary.json"], 0, 9)
    capture("A historical Linux launch is inspectable too.",
            "Archived Android 35 proof from 2026-09-01; this is not a new emulator run.",
            ["jq", "{tool_version,status,package_name,total_duration_ms}",
             "docs/receipts/reference-gradle.json"], 0, 9)

    manifest = {
        "recorded_at": datetime.now(timezone.utc).isoformat(),
        "host": platform.system(),
        "source_commit": run(["git", "rev-parse", "HEAD"])["stdout"].strip(),
        "binary_sha256": hashlib.sha256((ROOT / binary).read_bytes()).hexdigest(),
        "scope": "Actual CLI output replay with reading holds; no new emulator launch or screen capture.",
        "historical_receipt": "https://github.com/OthmaneBlial/rustdroid/actions/runs/33519017529",
        "commands": entries,
    }
    (out / "capture.json").write_text(json.dumps(manifest, indent=2) + "\n")
    concat = []
    for index, entry in enumerate(entries, 1):
        # Display the exact argv, including the isolated config and artifact paths.
        display = shlex.join(entry["argv"])
        output = entry["stdout"].strip()
        if entry["stderr"].strip():
            output += "\n" + entry["stderr"].strip()
        output += f"\n[exit {entry['exit_code']}]"
        frame = out / f"chapter-{index}.png"
        draw_frame(frame, index, entry["title"], entry["subtitle"], display, output,
                   "github.com/OthmaneBlial/rustdroid  |  Raw commands and outputs: capture.json")
        concat.extend([f"file '{frame.name}'", f"duration {entry['display_seconds']}"])
    concat.append("file 'chapter-5.png'")
    (out / "frames.txt").write_text("\n".join(concat) + "\n")
    subprocess.run([
        "ffmpeg", "-hide_banner", "-loglevel", "error", "-n", "-f", "concat", "-safe", "0",
        "-i", str(out / "frames.txt"), "-r", "30", "-c:v", "libx264", "-crf", "23",
        "-preset", "slow", "-profile:v", "high", "-level", "4.0", "-pix_fmt", "yuv420p",
        "-an", "-t", str(sum(entry["display_seconds"] for entry in entries)),
        "-movflags", "+faststart", str(out / "rustdroid-cli-proof.mp4"),
    ], check=True)
    probe = subprocess.run([
        "ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json",
        str(out / "rustdroid-cli-proof.mp4"),
    ], check=True, text=True, capture_output=True)
    (out / "probe.json").write_text(probe.stdout)
    print(out / "rustdroid-cli-proof.mp4")


if __name__ == "__main__":
    main()
