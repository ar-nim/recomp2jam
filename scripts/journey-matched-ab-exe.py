#!/usr/bin/env python3
"""Matched-scenario Wine leg for #339 (one Memoryer, one host room).

The script boots 7.55 through the rig kit, drives the qualified planet →
channel → room → waiting walk with the committed scene driver, then runs a
JSON state plan against the *external* host bot. Every step logs the host
commands, the wire frames that followed, and a native 800x600 capture.

The host room belongs to the scenario, not to this boot: pass the same
host directory used by every other leg so both clients see one room, one
song and one set of member states. No game bytes or token enter the repo.

Usage:
  scripts/matched-ab-exe.py --out DIR --display :N --host-dir DIR \
      --plan PATH [--wire-port 15012]
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import subprocess
import sys
import time
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
SCENE_PATH = SCRIPT_DIR / "wine-scene.py"
HOST_PATH = SCRIPT_DIR / "wine-room-host.py"

SETUP_DELAY_SECONDS = 2.0
WIRE_READY_TRIES = 50
STEP_TIMEOUT_SECONDS = 30.0


def load_module(name: str, path: Path):
    """Import a hyphenated script by path for the live leg."""
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


scene = load_module("matched_ab_wine_scene", SCENE_PATH)


def run(command: list[str]) -> None:
    """Run a rig command with its output attached to this leg."""
    subprocess.run(command, check=True)


def mint_launch_argv(out_dir: Path, wire_port: int) -> None:
    run([
        "bash", str(SCRIPT_DIR / "rig-mint-launch-argv.sh"),
        "--user-index-id", "1", "--out", str(out_dir / "argv.txt"),
        "--gateway-port", str(wire_port),
    ])


def start_wire_tap(out_dir: Path, wire_port: int) -> subprocess.Popen:
    log_path = out_dir / "wire" / "room-stage.log"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    process = subprocess.Popen([
        sys.executable, str(HOST_PATH), "--wire-tap",
        "--out-dir", str(out_dir / "wire"), "--wire-log", str(log_path),
        "--listen-port", str(wire_port), "--upstream-port", "15010",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
    for _ in range(WIRE_READY_TRIES):
        if log_path.is_file() and "wire-tap listening" in log_path.read_text():
            return process
        time.sleep(0.1)
    process.terminate()
    raise RuntimeError(f"wire tap did not listen on {wire_port}")


def boot(out_dir: Path, display: str) -> None:
    run([
        "bash", str(SCRIPT_DIR / "rig-run.sh"), "run", display, str(out_dir),
        "--", str(SCRIPT_DIR / "rig-wine.sh"), "launch", str(out_dir / "argv.txt"),
    ])


def stop(out_dir: Path, tap: subprocess.Popen) -> None:
    subprocess.run(
        ["bash", str(SCRIPT_DIR / "rig-run.sh"), "stop", str(out_dir)],
        check=False,
    )
    if tap.poll() is None:
        tap.terminate()
        try:
            tap.wait(timeout=10)
        except subprocess.TimeoutExpired:
            tap.kill()


def drive_to_waiting(out_dir: Path, display: str, wire_log: Path) -> scene.SceneLog:
    arguments = scene.build_argument_parser().parse_args([
        "--attach", str(out_dir), "--display", display,
        "--target", "waiting", "--host-user-index-id", "0",
        "--join-mode", "hover-press", "--row-dismiss", "auto",
        "--wire-log", str(wire_log), "--host-start-delay", "99999",
    ])
    log = scene.SceneLog(out_dir / "drive.log")
    frames = scene.Frames(display, Path("/tmp/matched-ab-exe-probe"))
    server = scene.ServerLog(
        Path.home() / "memoryer-local" / "server.log", arguments.account)
    wire = scene.WireTapLog(wire_log)
    injector = scene.Input(display,
                           scene.read_env(out_dir, arguments.wineprefix), log)
    reached = scene.drive(arguments, frames, server, injector, log,
                          out_dir / "artifacts" / "screenshots", wire)
    if "waiting" not in reached:
        raise RuntimeError(f"drive did not reach waiting: {reached}")
    return log


def saw_since(wire: scene.WireTapLog, opcode: int, baseline: int) -> bool:
    """Whether this state step produced the expected opcode on the wire."""
    wire.take()
    return any(value == opcode for _, _, value in wire.events[baseline:])


def execute_step(step: dict, host_dir: Path, wire: scene.WireTapLog,
                 injector: scene.Input, frames: scene.Frames,
                 shots: Path, log: scene.SceneLog) -> None:
    """Send one named state step and keep its wire and pixels together."""
    name = step["name"]
    wire.take()
    baseline = len(wire.events)
    for token in step.get("commands", []):
        with (host_dir / "host.cmd").open("a", encoding="utf-8") as handle:
            handle.write(token + "\n")
        deadline = time.time() + STEP_TIMEOUT_SECONDS
        while time.time() < deadline:
            if f"host scenario command {token}" in (host_dir / "host.log").read_text():
                break
            time.sleep(0.2)
        else:
            raise RuntimeError(f"host did not apply {token}")
        log.event(f"matched state={name} host command={token}")
    if "dclick" in step:
        x, y = step["dclick"]
        injector.send_dclick_xy(int(x), int(y))
        log.event(f"matched state={name} gate stimulus dclick=({x},{y})")
        time.sleep(SETUP_DELAY_SECONDS)
    wanted = step.get("await_opcode")
    if wanted is not None:
        deadline = time.time() + STEP_TIMEOUT_SECONDS
        while time.time() < deadline:
            if saw_since(wire, int(wanted), baseline):
                break
            time.sleep(0.2)
        else:
            raise RuntimeError(f"state {name} saw no opcode {wanted}")
    wire.take()
    after = wire.events[baseline:]
    for seconds, direction, value in after:
        log.event(f"matched state={name} wire {direction}:{value}@+{seconds:.2f}s")
    shot = shots / step.get("shot", f"{name}.png")
    frames.capture(shot)
    log.event(f"matched state captured name={name} shot={shot.name} "
              f"frames_after={len(after)}")


def package(out_dir: Path, display: str) -> None:
    run([
        "bash", str(SCRIPT_DIR / "run-manifest.sh"), "write", str(out_dir),
        "--pair", "wine-x11-sendinput-matched",
        "--build", subprocess.check_output(
            ["git", "-C", str(SCRIPT_DIR / ".."), "rev-parse", "HEAD"],
            text=True).strip(),
        "--script", "scripts/matched-ab-exe.py (matched #339 state plan)",
        "--params", f"display={display} host=external matched_scenario=339",
        "--input-log", str(out_dir / "drive.log"),
    ])
    run([
        "bash", str(SCRIPT_DIR / "rig-provenance.sh"), "emit", str(out_dir),
        "--producer", str(Path.home() / "scratch" / "run-full" / "OTwo.exe"),
        "--producer-kind", "exe", "--producer-version", "7.55",
        "--capture-tool", "ffmpeg", "--pair", "wine-x11-sendinput-matched",
        "--command", "scripts/rig-wine.sh launch argv.txt (matched host room)",
        "--artifact", "argv.txt", "--artifact", "argv.txt.fields",
        "--artifact", "audio/route-snapshot.txt",
        "--artifact", "audio/route-baseline.txt",
        "--artifact", "wire/room-stage.log",
    ])
    citation = subprocess.run(
        ["bash", str(SCRIPT_DIR / "rig-provenance.sh"), "cite", str(out_dir),
         "--claim", "exe"], text=True, capture_output=True)
    (out_dir / "citation.txt").write_text(citation.stdout + citation.stderr, encoding="utf-8")
    if citation.returncode != 0:
        print(f"journey-rig: citation unavailable rc={citation.returncode}; "
              "captures and wire kept", file=sys.stderr)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--host-dir", required=True)
    parser.add_argument("--plan", required=True)
    parser.add_argument("--wire-port", type=int, default=15012)
    args = parser.parse_args()

    out_dir = Path(args.out).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "artifacts" / "screenshots").mkdir(parents=True, exist_ok=True)
    host_dir = Path(args.host_dir).resolve()
    plan = json.loads(Path(args.plan).read_text())
    wire_log = out_dir / "wire" / "room-stage.log"
    tap = None
    try:
        mint_launch_argv(out_dir, args.wire_port)
        tap = start_wire_tap(out_dir, args.wire_port)
        boot(out_dir, args.display)
        log = drive_to_waiting(out_dir, args.display, wire_log)
        frames = scene.Frames(args.display, Path("/tmp/matched-ab-exe-probe"))
        wire = scene.WireTapLog(wire_log)
        injector = scene.Input(
            args.display,
            scene.read_env(out_dir, os.environ.get("WINEPREFIX", "")), log)
        shots = out_dir / "artifacts" / "screenshots"
        for step in plan["steps"]:
            execute_step(step, host_dir, wire, injector, frames, shots, log)
        package(out_dir, args.display)
    finally:
        stop(out_dir, tap) if tap is not None else None
    return 0


if __name__ == "__main__":
    sys.exit(main())
