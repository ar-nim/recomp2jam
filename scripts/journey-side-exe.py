#!/usr/bin/env python3
"""Scratch rig harness: boot the exe on a wire tap and drive one side scene."""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent


def run(command: list[str]) -> None:
    subprocess.run(command, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--wire-port", type=int, required=True)
    args = parser.parse_args()
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    (out / "artifacts/screenshots").mkdir(parents=True, exist_ok=True)
    wire_log = out / "wire" / "room-stage.log"
    tap = None
    try:
        run([
            "bash", str(SCRIPT_DIR / "rig-mint-launch-argv.sh"),
            "--user-index-id", "1", "--out", str(out / "argv.txt"),
            "--gateway-port", str(args.wire_port),
        ])
        (out / "wire").mkdir(parents=True, exist_ok=True)
        tap = subprocess.Popen([
            sys.executable, str(SCRIPT_DIR / "wine-room-host.py"), "--wire-tap",
            "--out-dir", str(out / "wire"), "--wire-log", str(wire_log),
            "--listen-port", str(args.wire_port), "--upstream-port", "15010",
        ], stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
        import time
        for _ in range(80):
            if wire_log.is_file() and "wire-tap listening" in wire_log.read_text():
                break
            time.sleep(0.2)
        else:
            raise RuntimeError("wire tap did not listen")
        run([
            "bash", str(SCRIPT_DIR / "rig-run.sh"), "run", args.display, str(out),
            "--", str(SCRIPT_DIR / "rig-wine.sh"), "launch", str(out / "argv.txt"),
        ])
        run([
            sys.executable, str(SCRIPT_DIR / "wine-scene.py"), "--attach", str(out),
            "--display", args.display, "--target", args.target,
            "--host-user-index-id", "0", "--wire-log", str(wire_log),
        ])
    finally:
        if (out / "pids").is_file():
            subprocess.run(["bash", str(SCRIPT_DIR / "rig-run.sh"), "stop", str(out)],
                           check=False)
        if tap is not None and tap.poll() is None:
            tap.terminate()
            try:
                tap.wait(timeout=10)
            except subprocess.TimeoutExpired:
                tap.kill()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
