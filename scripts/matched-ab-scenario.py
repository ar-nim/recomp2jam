#!/usr/bin/env python3
"""One Memoryer session for both #339 A/B legs.

The host bot, the state commands and the proof extraction live here so the
Wine and Rust arms cannot drift: both are driven against the same room,
song and member states, and every pixel verdict carries a wire/server-state
diff. Captures and logs stay under the caller's scratch directory.

Usage:
  matched-ab-scenario.py host-start --dir DIR [--music-id ID] [--hold S]
  matched-ab-scenario.py host-command --dir DIR TOKEN [TOKEN ...]
  matched-ab-scenario.py host-stop --dir DIR
  matched-ab-scenario.py verify --exe DIR --rust DIR --server-start LINE \
      --out FILE
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
HOST_SCRIPT = SCRIPT_DIR / "wine-room-host.py"
SERVER_LOG = Path.home() / "memoryer-local" / "server.log"
WIRE_LINE = re.compile(
    r"(?P<clock>\d\d:\d\d:\d\d) (?P<direction>C2S|S2C) opcode=(?P<opcode>\d+) "
    r"\((?P<hex>[0-9A-F]{4})\) len=(?P<len>\d+) hex=(?P<body>[0-9a-f]*)"
)
HOST_READY_SECONDS = 30.0
STATE_WAIT_SECONDS = 30.0
# Auth endpoints, pings and catalog data vary by process or account; they
# stay in the raw traces but cannot be required to match byte for byte.
IGNORED_OPCODES = {1000, 1001, 6001, 5028, 5029, 2026, 4030, 4031}
SERVER_STATE_OPCODES = ("03EC", "07D2", "07DA", "07E8", "0BBA", "0FA8",
                        "0BC3", "0FBE", "0FA0", "0FC5", "0BC1")


@dataclass(frozen=True)
class WireFrame:
    """One length-prefixed game frame seen by a leg's wire tap."""

    direction: str
    opcode: int
    body: str
    clock: str


def read_wire(path: Path) -> list[WireFrame]:
    if not path.is_file():
        return []
    frames = []
    for line in path.read_text(errors="replace").splitlines():
        match = WIRE_LINE.search(line)
        if match is None:
            continue
        frames.append(WireFrame(match["direction"], int(match["opcode"]),
                                match["body"], match["clock"]))
    return frames


def scenario_trace(frames: list[WireFrame]) -> list[str]:
    """Direction/opcode sequence with process-specific frames removed."""
    return [
        f"{frame.direction}:{frame.opcode}"
        for frame in frames if frame.opcode not in IGNORED_OPCODES
    ]


def first_body_digest(frames: list[WireFrame], opcode: int) -> str:
    for frame in frames:
        if frame.opcode == opcode:
            return hashlib.sha256(bytes.fromhex(frame.body)).hexdigest()[:16]
    return "absent"


def wire_window(frames: list[WireFrame]) -> tuple[str, str]:
    """Clock bounds of one leg, used to slice the shared server log."""
    if not frames:
        return "00:00:00", "00:00:00"
    clocks = [frame.clock for frame in frames]
    return min(clocks), max(clocks)


def server_slice(start_line: int, window: tuple[str, str],
                 accounts: tuple[str, ...] = ("tester", "t207probe")) -> list[str]:
    """Server-log state lines inside one leg's wire window."""
    if not SERVER_LOG.is_file():
        return []
    lines = SERVER_LOG.read_text(errors="replace").splitlines()
    result = []
    for raw_line in lines[start_line:]:
        # ISO log prefix is YYYY-MM-DDTHH:MM:SS; the clock starts at 11.
        clock = raw_line[11:19]
        if not window[0] <= clock <= window[1]:
            continue
        line = re.sub(r"\x1b\[[0-9;]*m", "", raw_line)
        if not any(f"[ {account}" in line for account in accounts):
            continue
        if not any(f"  {opcode} " in line for opcode in SERVER_STATE_OPCODES):
            continue
        # Timestamps, session ports and local endpoints are provenance, not
        # state. Keep opcode, controller and message body.
        cleaned = re.sub(r"^\S+ ", "", line)
        cleaned = re.sub(r"\[127\.0\.0\.1:\d+\]", "[session]", cleaned)
        result.append(cleaned.strip())
    return result


def host_start(args: argparse.Namespace) -> int:
    directory = Path(args.dir)
    directory.mkdir(parents=True, exist_ok=True)
    pid_file = directory / "pid"
    process = subprocess.Popen([
        sys.executable, str(HOST_SCRIPT), "--out-dir", str(directory),
        "--user-index-id", "3", "--music-id", str(args.music_id),
        "--hold", str(args.hold), "--start-delay", "99999",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
    pid_file.write_text(f"{process.pid}\n", encoding="utf-8")
    deadline = time.time() + HOST_READY_SECONDS
    log_path = directory / "host.log"
    while time.time() < deadline:
        if log_path.is_file() and "host room created" in log_path.read_text():
            for line in log_path.read_text().splitlines():
                if "host room created" in line:
                    print(line)
            return 0
        time.sleep(0.2)
    print("host did not create the room", file=sys.stderr)
    return 1


def host_command(args: argparse.Namespace) -> int:
    cmd_file = Path(args.dir) / "host.cmd"
    cmd_file.parent.mkdir(parents=True, exist_ok=True)
    with cmd_file.open("a", encoding="utf-8") as handle:
        for token in args.tokens:
            handle.write(token + "\n")
    deadline = time.time() + STATE_WAIT_SECONDS
    log_path = Path(args.dir) / "host.log"
    for token in args.tokens:
        while time.time() < deadline:
            if f"host scenario command {token}" in log_path.read_text(errors="replace"):
                break
            time.sleep(0.2)
        else:
            print(f"host did not apply {token}", file=sys.stderr)
            return 1
    return 0


def host_stop(args: argparse.Namespace) -> int:
    pid_file = Path(args.dir) / "pid"
    if pid_file.is_file():
        pid = int(pid_file.read_text().strip())
        subprocess.run(["kill", "-TERM", str(pid)], check=False)
    return 0


def verify(args: argparse.Namespace) -> int:
    exe_frames = read_wire(Path(args.exe) / "wire" / "room-stage.log")
    rust_frames = read_wire(Path(args.rust) / "wire" / "room-stage.log")
    exe_trace = scenario_trace(exe_frames)
    rust_trace = scenario_trace(rust_frames)
    proof = {
        "exe_frames": len(exe_frames),
        "rust_frames": len(rust_frames),
        "wire_sequence_equal": exe_trace == rust_trace,
        "exe_wire_sequence": exe_trace,
        "rust_wire_sequence": rust_trace,
        "exe_server_window": wire_window(exe_frames),
        "rust_server_window": wire_window(rust_frames),
        "exe_server_slice": server_slice(args.server_start,
                                          wire_window(exe_frames)),
        "rust_server_slice": server_slice(args.server_start,
                                           wire_window(rust_frames)),
        "server_slice_equal": (
            server_slice(args.server_start, wire_window(exe_frames))
            == server_slice(args.server_start, wire_window(rust_frames))
        ),
        "room_list_body_equal": (
            first_body_digest(exe_frames, 2003) == first_body_digest(rust_frames, 2003)
        ),
        "channel_list_body_equal": (
            first_body_digest(exe_frames, 1003) == first_body_digest(rust_frames, 1003)
        ),
        "exe_music_list_body": first_body_digest(exe_frames, 2024),
        "rust_music_list_body": first_body_digest(rust_frames, 2024),
    }
    Path(args.out).write_text(json.dumps(proof, indent=2), encoding="utf-8")
    print(json.dumps({key: value for key, value in proof.items()
                      if not key.endswith("sequence")}, indent=2))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)

    start = commands.add_parser("host-start")
    start.add_argument("--dir", required=True)
    start.add_argument("--music-id", type=int, default=757)
    start.add_argument("--hold", type=float, default=1800.0)
    start.set_defaults(handler=host_start)

    command = commands.add_parser("host-command")
    command.add_argument("--dir", required=True)
    command.add_argument("tokens", nargs="+")
    command.set_defaults(handler=host_command)

    stop = commands.add_parser("host-stop")
    stop.add_argument("--dir", required=True)
    stop.set_defaults(handler=host_stop)

    compare = commands.add_parser("verify")
    compare.add_argument("--exe", required=True)
    compare.add_argument("--rust", required=True)
    compare.add_argument("--server-start", type=int, required=True)
    compare.add_argument("--out", required=True)
    compare.set_defaults(handler=verify)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    return args.handler(args)


if __name__ == "__main__":
    sys.exit(main())
