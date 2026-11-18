#!/usr/bin/env python3
"""Matched-scenario Rust leg for #339 (same host room, same Memoryer).

The client is booted through the rig kit so audio is captured on a per-run
null sink, then the committed native replay driver injects one shared trace.
A companion controller sends the same host-bot state commands used by the
Wine leg and releases the trace only after the server's broadcast is on the
wire. No token or capture bytes enter the repository.

Usage:
  matched-ab-rust.py --out DIR --display :N --host-dir DIR --room N \
      --trace PATH [--wire-port 15013]
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sqlite3
import subprocess
import sys
import threading
import time
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
DRIVER_PATH = SCRIPT_DIR / "replay-trace-native.py"
TOKEN_DATABASE = Path.home() / "memoryer-local" / "O2JAM.db"
HOST_WAIT_SECONDS = 30.0
MARKER_WAIT_SECONDS = 60.0
WIRE_READY_TRIES = 50


def load_driver():
    spec = importlib.util.spec_from_file_location("matched_ab_replay", DRIVER_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def read_token() -> str:
    with sqlite3.connect(f"file:{TOKEN_DATABASE}?mode=ro", uri=True) as database:
        row = database.execute(
            "select TUSER_ID from t_o2jam_login where USER_INDEX_ID=? "
            "order by LOGIN_TIME desc limit 1", ("1",),
        ).fetchone()
    if row is None:
        raise RuntimeError("no session token for guest account 1")
    return str(row[0])


def write_client_wrapper(out_dir: Path, wire_port: int, room: int) -> Path:
    """Name the launcher `recomp2jam` so the audio lane attributes its stream."""
    binary = (SCRIPT_DIR / ".." / "target" / "release" / "recomp2jam").resolve()
    wrapper = out_dir / "bin" / "recomp2jam"
    wrapper.parent.mkdir(parents=True, exist_ok=True)
    wrapper.write_text(
        "#!/usr/bin/env bash\n"
        f"exec {binary} online --host 127.0.0.1 --port {wire_port} "
        f"--user-id 1 --token {read_token()} --gateway-id 0 --channel-id 0 "
        f"--join-room {room} "
        f"--frame-dump {out_dir}/frames --input-log {out_dir}/input.log\n",
        encoding="utf-8",
    )
    wrapper.chmod(0o700)
    return wrapper


def start_wire_tap(out_dir: Path, wire_port: int) -> subprocess.Popen:
    log_path = out_dir / "wire" / "room-stage.log"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    process = subprocess.Popen([
        sys.executable, str(SCRIPT_DIR / "wine-room-host.py"), "--wire-tap",
        "--out-dir", str(out_dir / "wire"), "--wire-log", str(log_path),
        "--listen-port", str(wire_port), "--upstream-port", "15010",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
    for _ in range(WIRE_READY_TRIES):
        if log_path.is_file() and "wire-tap listening" in log_path.read_text():
            return process
        time.sleep(0.1)
    process.terminate()
    raise RuntimeError(f"wire tap did not listen on {wire_port}")


def wait_for_text(path: Path, fragment: str, timeout: float) -> None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if path.is_file() and fragment in path.read_text(errors="replace"):
            return
        time.sleep(0.2)
    raise RuntimeError(f"no {fragment!r} in {path}")


def count_wire_opcode(log_path: Path, opcode: int) -> int:
    if not log_path.is_file():
        return 0
    return len(re.findall(rf"opcode={opcode} \(", log_path.read_text(errors="replace")))


class StateController:
    """Send host-bot commands only after the client reaches a named state."""

    def __init__(self, out_dir: Path, host_dir: Path, wire_log: Path) -> None:
        self.out_dir = out_dir
        self.host_dir = host_dir
        self.wire_log = wire_log
        self.applied_offset = 0

    def state_captured(self, name: str) -> None:
        """Wait for the trace's own capture marker, not a wall-clock guess."""
        path = self.out_dir / "applied.log"
        deadline = time.time() + MARKER_WAIT_SECONDS
        while time.time() < deadline:
            if path.is_file():
                with path.open("rb") as handle:
                    handle.seek(self.applied_offset)
                    fresh = handle.read().decode("utf-8", "replace")
                    self.applied_offset = handle.tell()
                wanted = f"control=state-shot action=capture"
                if wanted in fresh and f"name={name} status=captured" in fresh:
                    return
            time.sleep(0.2)
        raise RuntimeError(f"state-shot {name} was not captured")

    def send(self, tokens: list[str], opcode: int, marker: str) -> None:
        baseline = count_wire_opcode(self.wire_log, opcode)
        for token in tokens:
            with (self.host_dir / "host.cmd").open("a", encoding="utf-8") as handle:
                handle.write(token + "\n")
            wait_for_text(
                self.host_dir / "host.log",
                f"host scenario command {token}", HOST_WAIT_SECONDS)
        deadline = time.time() + HOST_WAIT_SECONDS
        while time.time() < deadline:
            if count_wire_opcode(self.wire_log, opcode) > baseline:
                break
            time.sleep(0.2)
        else:
            raise RuntimeError(f"server did not broadcast opcode {opcode}")
        (self.out_dir / f"{marker}.ready").write_text("ok\n", encoding="utf-8")

    def run(self) -> None:
        """Drive the downloaded → gate → base sequence behind the trace."""
        self.state_captured("base-ready")
        self.send(["music-list=100,200", "music-state=4", "sync-member=0"],
                  4037, "download-ready")
        self.state_captured("member-downloading")
        # The trace has already injected the double-click before this
        # marker; the master command closes the slot gate on the server.
        self.send(["update-slot=2"], 3009, "gate-ready")
        self.state_captured("gate-closed")
        self.send(["update-slot=2", "music-state=0", "sync-member=0"],
                  4037, "back-ready")


def replay(out_dir: Path, display: str, trace: Path) -> None:
    subprocess.run([
        sys.executable, str(DRIVER_PATH), "--trace", str(trace),
        "--out", str(out_dir), "--display", display,
        "--attach-rig", "--sound", "--await-timeout", "120",
    ], check=True)


def stop(out_dir: Path, tap: subprocess.Popen) -> None:
    subprocess.run(["bash", str(SCRIPT_DIR / "rig-run.sh"), "stop", str(out_dir)],
                   check=False)
    if tap.poll() is None:
        tap.terminate()
        try:
            tap.wait(timeout=10)
        except subprocess.TimeoutExpired:
            tap.kill()


def package(out_dir: Path, display: str, trace: Path) -> None:
    subprocess.run([
        "bash", str(SCRIPT_DIR / "run-manifest.sh"), "write", str(out_dir),
        "--pair", "native-x11-matched",
        "--build", subprocess.check_output(
            ["git", "-C", str(SCRIPT_DIR / ".."), "rev-parse", "HEAD"],
            text=True).strip(),
        "--script", "scripts/matched-ab-rust.py (matched #339 state plan)",
        "--params", f"display={display} trace={trace.name} matched_scenario=339",
        "--input-log", str(out_dir / "input.log"),
    ], check=True)
    subprocess.run([
        "bash", str(SCRIPT_DIR / "rig-provenance.sh"), "emit", str(out_dir),
        "--producer", str((SCRIPT_DIR / ".." / "target" / "release" /
                           "recomp2jam").resolve()),
        "--producer-kind", "client-build", "--producer-version", "0.1.0",
        "--command", "rig-run.sh run -- bin/recomp2jam (matched host room)",
        "--pair", "native-x11-matched",
        "--artifact", "audio/route-snapshot.txt",
        "--artifact", "audio/route-baseline.txt",
        "--artifact", "wire/room-stage.log",
    ], check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--host-dir", required=True)
    parser.add_argument("--room", type=int, required=True)
    parser.add_argument("--trace", required=True)
    parser.add_argument("--wire-port", type=int, default=15013)
    args = parser.parse_args()

    out_dir = Path(args.out).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "artifacts" / "screenshots").mkdir(parents=True, exist_ok=True)
    wrapper = write_client_wrapper(out_dir, args.wire_port, args.room)
    tap = None
    try:
        tap = start_wire_tap(out_dir, args.wire_port)
        subprocess.run([
            "bash", str(SCRIPT_DIR / "rig-run.sh"), "run", args.display,
            str(out_dir), "--", str(wrapper),
        ], check=True)
        controller = StateController(
            out_dir, Path(args.host_dir).resolve(),
            out_dir / "wire" / "room-stage.log")
        # The controller waits on the trace's own applied-event log, so no
        # fixed sleep can desynchronise the two clients' server state.
        worker = threading.Thread(target=controller.run, daemon=True)
        worker.start()
        replay(out_dir, args.display, Path(args.trace).resolve())
        worker.join(timeout=HOST_WAIT_SECONDS)
        if worker.is_alive():
            raise RuntimeError("state controller did not finish")
        package(out_dir, args.display, Path(args.trace).resolve())
    finally:
        stop(out_dir, tap) if tap is not None else None
    return 0


if __name__ == "__main__":
    sys.exit(main())
