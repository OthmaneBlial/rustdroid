#!/usr/bin/env python3
"""CI-only ADB fault injection; all non-targeted commands use the real ADB."""
import os
import sys
import time
from pathlib import Path

arguments = sys.argv[1:]
mode = os.environ.get("RUSTDROID_TEST_ADB_FAULT", "")
streaming = "logcat" in arguments and "-d" not in arguments
if mode in ("cleanup-failure", "reader-exit-cleanup"):
    state = Path(os.environ["RUSTDROID_TEST_STATE_FILE"])
    armed = state.parent.parent.parent / "fault-armed"
    if streaming:
        armed.parent.mkdir(parents=True, exist_ok=True)
        armed.touch()
    if armed.exists() and "getprop" in arguments and "ro.build.version.sdk" in arguments:
        state.parent.mkdir(parents=True, exist_ok=True)
        state.write_text("intentional invalid test state\n")
if mode in ("reader-exit", "reader-exit-cleanup") and streaming:
    print("injected logcat reader exit", file=sys.stderr)
    sys.exit(42)
if mode == "marker-timeout" and "log" in arguments and "RustDroid" in arguments:
    time.sleep(20)  # exceeds the production five-second marker deadline
    sys.exit(43)
os.execv(os.environ["RUSTDROID_TEST_REAL_ADB"], ["adb", *arguments])
