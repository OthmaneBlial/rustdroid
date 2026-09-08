#!/usr/bin/env python3
"""CI-only ADB fault injection; all non-targeted commands use the real ADB."""
import os
import sys
import time

arguments = sys.argv[1:]
mode = os.environ.get("RUSTDROID_TEST_ADB_FAULT", "")
if mode == "reader-exit" and "logcat" in arguments and "-d" not in arguments:
    print("injected logcat reader exit", file=sys.stderr)
    sys.exit(42)
if mode == "marker-timeout" and "log" in arguments and "RustDroid" in arguments:
    time.sleep(20)  # exceeds the production five-second marker deadline
    sys.exit(43)
os.execv(os.environ["RUSTDROID_TEST_REAL_ADB"], ["adb", *arguments])
