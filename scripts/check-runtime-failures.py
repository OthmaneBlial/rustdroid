#!/usr/bin/env python3
"""Exercise public APK success/exit/crash/launcher cases on explicit Linux/KVM AVD.

Uses actual RustDroid and Android processes. Never substitutes a mock runtime.
The caller provisions the dedicated emulator; this script leaves it running.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--avd", required=True)
    parser.add_argument("--binary", default="target/debug/rustdroid")
    parser.add_argument("--output", default="ci-artifacts/runtime-failures")
    parser.add_argument("--fixture-dir", default="tests/fixtures/apks")
    parser.add_argument("--include-anr", action="store_true")
    args = parser.parse_args()
    if sys.platform != "linux" or not os.access("/dev/kvm", os.R_OK | os.W_OK):
        parser.error("a Linux host with usable /dev/kvm is required; no runtime checks ran")
    if not args.serial.startswith("emulator-"):
        parser.error("use a dedicated emulator serial, not a personal physical device")
    root = Path(__file__).resolve().parent.parent
    out = (root / args.output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    subprocess.run(["adb", "-s", args.serial, "get-state"], check=True, timeout=10)
    results = []
    cases = [
        ("launch-success", None, "none", 2),
        ("launch-then-exit", "app_runtime", "crash", 15),
        ("launch-then-crash", "app_runtime", "crash", 15),
        ("missing-launcher", "app_launch", "launch", 2),
    ]
    if args.include_anr:
        cases.append(("launch-then-anr", "app_runtime", "anr", 45))
    for fixture, stage, classification, duration in cases:
        directory = out / fixture
        directory.mkdir()
        command = [
            str((root / args.binary).resolve()), "--config", str(out / "isolated.toml"),
            "--profile", "host-fast", "--adb-serial", args.serial,
            "--host-avd-name", args.avd, "--headless", "true", "--boot-mode", "warm",
            "run", str(root / args.fixture_dir / f"{fixture}.apk"),
            "--duration-secs", str(duration), "--keep-alive", "true",
            "--artifacts-dir", str(directory),
        ]
        record = {"fixture": fixture, "command": command, "verified": False}
        try:
            with (directory / "console.txt").open("w") as console:
                if fixture != "launch-then-anr":
                    process = subprocess.run(command, stdout=console, stderr=subprocess.STDOUT, timeout=120)
                else:
                    # A real foreground broadcast blocks inside the fixture receiver.
                    # No fabricated logcat lines or mocked ANR notification are used.
                    with subprocess.Popen(command, stdout=console, stderr=subprocess.STDOUT) as process:
                        trigger = None
                        try:
                            deadline = time.monotonic() + 45
                            while time.monotonic() < deadline:
                                pid = subprocess.run(
                                    ["adb", "-s", args.serial, "shell", "pidof", "com.rustdroid.fixture.anr"],
                                    capture_output=True, text=True, timeout=5,
                                )
                                if pid.stdout.strip():
                                    break
                                if process.poll() is not None:
                                    raise AssertionError("ANR fixture exited before trigger")
                                time.sleep(1)
                            else:
                                raise AssertionError("ANR fixture did not start")
                            time.sleep(3)  # allow onCreate to register the receiver
                            trigger = subprocess.Popen(
                                ["adb", "-s", args.serial, "shell", "am", "broadcast",
                                 "--receiver-foreground", "-a", "com.rustdroid.fixture.BLOCK",
                                 "-p", "com.rustdroid.fixture.anr"],
                                stdout=console, stderr=subprocess.STDOUT,
                            )
                            process.wait(timeout=90)
                        finally:
                            for child in [trigger, process]:
                                if child is not None and child.poll() is None:
                                    child.kill()
                                    child.wait(timeout=5)
            receipt = json.loads((directory / "run-summary.json").read_text())
            expected_status = "failed" if stage else "passed"
            assert process.returncode == (1 if stage else 0), f"exit={process.returncode}"
            assert receipt["status"] == expected_status, receipt["status"]
            assert receipt.get("failure_stage") == stage, receipt.get("failure_stage")
            assert receipt["failure_classification"] == classification, receipt["failure_classification"]
            for report in ["run-report.html", "junit.xml", "run-summary.md"]:
                content = (directory / report).read_text()
                if stage:
                    assert stage in content, f"{report} omitted {stage}"
            record["verified"] = True
        except (AssertionError, OSError, ValueError, subprocess.SubprocessError) as error:
            record["error"] = str(error)
        results.append(record)
        print(f"{fixture}: {'verified' if record['verified'] else 'FAILED'}", flush=True)
    (out / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    return 0 if all(result["verified"] for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
