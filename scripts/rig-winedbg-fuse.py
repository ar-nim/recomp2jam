#!/usr/bin/env python3
"""Capture one stopped-state read of the Room fuse globals under winedbg.

Usage: rig-winedbg-fuse.py --argv FILE --exe PATH --cwd DIR --log FILE
       --trigger FILE --marker TEXT [--winedbg NAME]

The debugger starts the game directly (never through explorer). It sends one
initial `cont`, waits for the drive log to show the marker, sends ONE Ctrl-C,
and queues all memory reads while stopped. It never continues after a stop.
"""

import argparse
import os
import re
import subprocess
import threading
import time
from pathlib import Path


def wait_for_text(log_path: Path, needle: bytes, base: int, timeout: float) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        data = log_path.read_bytes()
        if data.count(needle) > base:
            return True
        time.sleep(0.25)
    return False


def wait_for_marker(path: Path, marker: str, timeout: float) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists() and marker in path.read_text(errors="replace"):
            return True
        time.sleep(0.5)
    return False


def send_command(master_fd: int, log_path: Path, command: str, timeout: float) -> None:
    base = log_path.read_bytes().count(b"Wine-dbg>")
    os.write(master_fd, command.encode() + b"\r")
    if not wait_for_text(log_path, b"Wine-dbg>", base, timeout):
        print(f"warning: no prompt after {command!r}")


def parse_pointer(log_path: Path, address: str) -> str | None:
    text = log_path.read_text(errors="replace")
    pattern = re.compile(
        r"0x0*" + re.escape(address.lstrip("0x")) + r"\s+\w+\+0x[0-9a-f]+:\s+([0-9a-f]{8})"
    )
    matches = pattern.findall(text)
    return matches[-1] if matches else None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--argv", type=Path, required=True)
    parser.add_argument("--exe", type=Path, required=True)
    parser.add_argument("--cwd", type=Path, required=True)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--trigger", type=Path, required=True)
    parser.add_argument("--marker", required=True)
    parser.add_argument("--winedbg", default="winedbg")
    parser.add_argument("--trigger-timeout", type=float, default=300.0)
    parser.add_argument("--stop-timeout", type=float, default=240.0)
    args = parser.parse_args()

    argv_words = [line.strip() for line in args.argv.read_text().splitlines() if line.strip()]
    master_fd, slave_fd = os.openpty()
    process = subprocess.Popen(
        [args.winedbg, str(args.exe), *argv_words],
        stdin=slave_fd,
        stdout=slave_fd,
        stderr=subprocess.STDOUT,
        cwd=str(args.cwd),
        env=os.environ.copy(),
    )
    os.close(slave_fd)
    args.log.parent.mkdir(parents=True, exist_ok=True)
    log_file = args.log.open("wb", buffering=0)

    def pump() -> None:
        while True:
            try:
                chunk = os.read(master_fd, 4096)
            except OSError:
                break
            if not chunk:
                break
            log_file.write(chunk)

    reader = threading.Thread(target=pump, daemon=True)
    reader.start()

    time.sleep(5)
    os.write(master_fd, b"cont\r")
    print("controller: debugger continued once", flush=True)

    if not wait_for_marker(args.trigger, args.marker, args.trigger_timeout):
        print("controller: TRIGGER NOT SEEN", flush=True)
    else:
        print("controller: marker seen, sending ONE Ctrl-C", flush=True)
        base = args.log.read_bytes().count(b"Wine-dbg>")
        os.write(master_fd, b"\x03")
        if not wait_for_text(args.log, b"Wine-dbg>", base, args.stop_timeout):
            print("controller: STOP DID NOT FINISH", flush=True)
        else:
            print("controller: stopped", flush=True)
            send_command(master_fd, args.log, "x/4x 0x76835c", 30.0)
            owner = parse_pointer(args.log, "76835c")
            print(f"controller: network object = {owner}", flush=True)
            if owner:
                send_command(master_fd, args.log, f"x/16x 0x{owner}+0x432c", 30.0)
                send_command(master_fd, args.log, f"x/8x 0x{owner}+0x4348", 30.0)
            send_command(master_fd, args.log, "x/4x 0x768120", 30.0)
            send_command(master_fd, args.log, "x/4x 0x7684d8", 30.0)

    send_command(master_fd, args.log, "quit", 20.0)
    try:
        process.wait(timeout=20)
    except subprocess.TimeoutExpired:
        process.terminate()
    log_file.close()
    print("controller: done", flush=True)


if __name__ == "__main__":
    main()
