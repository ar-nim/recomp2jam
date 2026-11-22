#!/usr/bin/env python3
"""One blessed driver for an evidence wave, driven by a JSON recipe.

Live mode clicks through `rig-wine.sh send click` (the SendInput helper),
records frames through `lib/wire_frames.py`, and applies proxy rules from the
recipe. Replay mode consumes a cited tap log offline and reproduces the same
evidence strings without a Wine boot, Memoryer, or game data.

Usage:
  scripts/rig-wave.py --recipe scripts/recipes/row-vanish-582.json \
      --mode replay --frames TAP_LOG --out OUT_DIR
  scripts/rig-wave.py --recipe RECIPE --mode live --out OUT_DIR \
      --frames TAP_LOG --display :NNN [--proxy-listen PORT --proxy-upstream PORT]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import socket
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

from PIL import Image

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS / "lib"))

from popup_state import NORMAL_MODE_NOTICE, ImageFrameAdapter, query_popup  # noqa: E402
from wave_recipe import Aim, RecipeError, Step, WaveRecipe, load_recipe  # noqa: E402
from wire_frames import (  # noqa: E402
    Frame,
    FrameStream,
    InvalidFrameError,
    TruncatedFrameError,
    WireFrameError,
    WireLine,
    apply_rules,
    format_wire_line,
    parse_tap_lines,
)

CUE_TIMEOUT_SECONDS = 20.0
FRAME_SIZE = "1024x768"
TAP_LINE_PREFIX_LENGTH = 8


class WaveFailure(RuntimeError):
    """A recipe step could not be evidenced; no pack may be claimed."""


@dataclass(frozen=True)
class RunOutput:
    driver_lines: tuple[str, ...]
    evidence_lines: tuple[str, ...]
    captures: Mapping[str, object]


class SendInputClicker:
    """Canvas clicks go through the Wine SendInput helper path, never XTEST."""

    @staticmethod
    def command(aim: Aim) -> list[str]:
        x, y = aim.point()
        return [str(SCRIPTS / "rig-wine.sh"), "send", "click", str(x), str(y)]

    def click(self, aim: Aim) -> None:
        result = subprocess.run(
            self.command(aim), env=os.environ.copy(), capture_output=True,
            text=True, timeout=120, check=False,
        )
        if result.returncode != 0:
            raise WaveFailure(f"SendInput click {aim.key} failed: {result.stderr.strip()}")


class ReplayRig:
    """Replay a recipe against recorded frames without touching a rig."""

    def __init__(self, lines: list[WireLine]) -> None:
        self._lines = lines

    def probe(self, step: Step) -> str:
        return (
            f"{step.time} {step.label} {step.phase} "
            f"blue={step.blue:.3f} label={step.scene}"
        )

    def dismiss_popup(self, step: Step) -> str:
        return ""

    def click(self, step: Step) -> str:
        return f"{step.time} {step.text}"

    def cue(self, step: Step) -> str:
        if find_cue(self._lines, step) is None:
            raise WaveFailure(_missing_cue(step))
        return step.text

    @staticmethod
    def capture(step: Step) -> Mapping[str, object]:
        return dict(step.evidence)

    @staticmethod
    def note(step: Step) -> str:
        return f"{step.time} {step.text}"

    @staticmethod
    def wait(step: Step) -> None:
        return None


class LiveRig:
    """Drive the recipe against a real Wine display and a pass-through tap."""

    def __init__(self, out: Path, display: str, frames: Path) -> None:
        self._out = out
        self._display = display
        self._frames = frames
        self._clicker = SendInputClicker()

    def probe(self, step: Step) -> str:
        score = self._popup_score()
        return f"{_now()} {step.label} {step.phase} blue={score:.3f} label=live-frame"

    def dismiss_popup(self, step: Step) -> str:
        for _ in range(step.max_tries):
            if self._popup_score() <= NORMAL_MODE_NOTICE.floor:
                return ""
            self._clicker.click(self._required_aim(step))
        raise WaveFailure(f"popup still up at aim {step.aim.key}; BACK aborted")

    def click(self, step: Step) -> str:
        aim = self._required_aim(step)
        self._clicker.click(aim)
        return f"{_now()} {step.text} evidence={aim.evidence}"

    def cue(self, step: Step) -> str:
        deadline = time.monotonic() + CUE_TIMEOUT_SECONDS
        while time.monotonic() < deadline:
            lines = parse_tap_lines(self._frames.read_text(errors="replace"))
            found = find_cue(lines, step)
            if found is not None:
                return found.render()
            time.sleep(0.2)
        raise WaveFailure(_missing_cue(step))

    def capture(self, step: Step) -> Mapping[str, object]:
        full = self._out / "captures" / f"{step.name}.png"
        full.parent.mkdir(parents=True, exist_ok=True)
        _grab_frame(self._display, full)
        with Image.open(full) as image:
            frame = image.convert("RGB")
            crops = _crop_records(step, frame, full.parent)
        return {
            "name": step.name,
            "full": str(full),
            "full_sha256": _digest(full),
            "size": list(frame.size),
            "crops": crops,
        }

    @staticmethod
    def note(step: Step) -> str:
        return f"{_now()} {step.text}"

    @staticmethod
    def wait(step: Step) -> None:
        time.sleep(step.seconds)

    def _popup_score(self) -> float:
        probe = self._out / "captures" / "popup-probe.png"
        probe.parent.mkdir(parents=True, exist_ok=True)
        _grab_frame(self._display, probe)
        adapter = ImageFrameAdapter.from_path(probe)
        return query_popup(NORMAL_MODE_NOTICE, adapter).score

    def _required_aim(self, step: Step) -> Aim:
        if step.aim is None:
            raise WaveFailure(f"step {step.id} has no aim")
        return step.aim


class RewritingTap:
    """Pass-through TCP tap that rewrites S2C frames by recipe rule data."""

    def __init__(self, listen: int, upstream: int, rules: list[Mapping[str, object]],
                 log_path: Path) -> None:
        self._listen = listen
        self._upstream = upstream
        self._rules = rules
        self._log_path = log_path
        self._stop = threading.Event()
        self._server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._server.settimeout(0.5)

    def start(self) -> None:
        self._server.bind(("127.0.0.1", self._listen))
        self._server.listen(4)
        self._log_path.parent.mkdir(parents=True, exist_ok=True)
        threading.Thread(target=self._accept_clients, daemon=True).start()

    def stop(self) -> None:
        self._stop.set()
        self._server.close()

    def _accept_clients(self) -> None:
        while not self._stop.is_set():
            try:
                client, _ = self._server.accept()
            except socket.timeout:
                continue
            except OSError:
                return
            threading.Thread(target=self._serve, args=(client,), daemon=True).start()

    def _serve(self, client: socket.socket) -> None:
        upstream = socket.create_connection(("127.0.0.1", self._upstream), timeout=10)
        workers = [
            threading.Thread(target=self._pump, args=(client, upstream, "C2S"), daemon=True),
            threading.Thread(target=self._pump, args=(upstream, client, "S2C"), daemon=True),
        ]
        for worker in workers:
            worker.start()
        for worker in workers:
            worker.join()
        client.close()
        upstream.close()

    def _pump(self, source: socket.socket, target: socket.socket, direction: str) -> None:
        stream = FrameStream()
        source.settimeout(0.5)
        while not self._stop.is_set():
            try:
                chunk = source.recv(65536)
            except socket.timeout:
                continue
            if not chunk:
                break
            try:
                frames = stream.feed(chunk)
            except InvalidFrameError as error:
                print(f"rig-wave: tap {direction} refused frame: {error}", file=sys.stderr)
                return
            for frame in frames:
                rewritten = apply_rules(frame, direction, self._rules)
                self._record(direction, rewritten)
                target.sendall(rewritten.encoded())
        try:
            stream.finish()
        except TruncatedFrameError as error:
            print(f"rig-wave: tap {direction} refused partial frame: {error}", file=sys.stderr)

    def _record(self, direction: str, frame: Frame) -> None:
        line = format_wire_line(time.strftime("%H:%M:%S"), direction, frame)
        with self._log_path.open("a", encoding="utf-8") as handle:
            handle.write(line + "\n")


def _now() -> str:
    return time.strftime("%H:%M:%S")


def _missing_cue(step: Step) -> str:
    return f"missing wire cue {step.id} ({step.direction} {step.opcode})"


def find_cue(lines: list[WireLine], step: Step) -> WireLine | None:
    """Find the first frame that satisfies one recipe cue."""
    for line in lines:
        if line.direction != step.direction or line.frame.opcode != step.opcode:
            continue
        body = line.frame.body.hex()
        if step.body_contains and step.body_contains not in body:
            continue
        if step.body_absent and step.body_absent in body:
            continue
        if _is_tap_text(step.text) and line.render() != step.text:
            continue
        return line
    return None


def _is_tap_text(text: str) -> bool:
    return len(text) > TAP_LINE_PREFIX_LENGTH and text[2] == ":" and text[5] == ":"


def _grab_frame(display: str, path: Path) -> None:
    subprocess.run(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
         "-f", "x11grab", "-video_size", FRAME_SIZE, "-i", f"{display}+0,0",
         "-frames:v", "1", str(path)],
        check=True, capture_output=True,
    )


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _crop_records(step: Step, frame: Image.Image, directory: Path) -> Mapping[str, object]:
    records = {}
    for label, crop in step.evidence.get("crops", {}).items():
        path = directory / f"{step.name}-{label}.png"
        box = tuple(crop["box"])
        frame.crop(box).save(path, format="PNG")
        records[label] = {"path": str(path), "box": list(box), "sha256": _digest(path)}
    return records


def execute(recipe: WaveRecipe, rig: ReplayRig | LiveRig) -> RunOutput:
    """Run each recipe step; a failed cue aborts all evidence claims."""
    driver_lines: list[str] = []
    evidence_lines: list[str] = []
    captures: dict[str, object] = {}
    for step in recipe.steps:
        if step.action == "probe-popup":
            driver_lines.append(rig.probe(step))
        elif step.action == "dismiss-popup":
            line = rig.dismiss_popup(step)
            if line:
                driver_lines.append(line)
        elif step.action == "click":
            driver_lines.append(rig.click(step))
        elif step.action == "wire-cue":
            evidence_lines.append(rig.cue(step))
        elif step.action == "capture":
            captures[step.name] = rig.capture(step)
        elif step.action == "note":
            driver_lines.append(rig.note(step))
        elif step.action == "wait":
            rig.wait(step)
    return RunOutput(tuple(driver_lines), tuple(evidence_lines), captures)


def write_output(out: Path, result: RunOutput) -> None:
    out.mkdir(parents=True, exist_ok=True)
    (out / "driver.log").write_text("\n".join(result.driver_lines) + "\n", encoding="utf-8")
    (out / "evidence.log").write_text("\n".join(result.evidence_lines) + "\n", encoding="utf-8")
    (out / "captures.json").write_text(_json(result.captures), encoding="utf-8")


def _json(value: object) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--recipe", type=Path, required=True)
    parser.add_argument("--mode", choices=("replay", "live"), default="replay")
    parser.add_argument("--frames", type=Path, help="tap log (input for replay, output for live)")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--display", default=os.environ.get("DISPLAY", ":61"))
    parser.add_argument("--proxy-listen", type=int)
    parser.add_argument("--proxy-upstream", type=int)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.mode == "replay" and args.frames is None:
        parser.error("--mode replay needs --frames")
    if args.mode == "live" and args.frames is None:
        parser.error("--mode live needs --frames")
    if (args.proxy_listen is None) != (args.proxy_upstream is None):
        parser.error("--proxy-listen and --proxy-upstream must be given together")
    try:
        recipe = load_recipe(args.recipe)
        if args.mode == "replay":
            lines = parse_tap_lines(args.frames.read_text(errors="replace"))
            result = execute(recipe, ReplayRig(lines))
        else:
            result = run_live(recipe, args)
        write_output(args.out, result)
    except (WaveFailure, OSError, RecipeError, WireFrameError,
            subprocess.CalledProcessError) as error:
        print(f"rig-wave: {error}", file=sys.stderr)
        return 2
    return 0


def run_live(recipe: WaveRecipe, args: argparse.Namespace) -> RunOutput:
    """Start the rule-driven tap, drive the wave, then stop the tap."""
    tap = None
    if args.proxy_listen is not None and args.proxy_upstream is not None:
        args.frames.write_text("", encoding="utf-8")
        tap = RewritingTap(args.proxy_listen, args.proxy_upstream,
                           list(recipe.proxy_rules), args.frames)
        tap.start()
    try:
        rig = LiveRig(args.out, args.display, args.frames)
        return execute(recipe, rig)
    finally:
        if tap is not None:
            tap.stop()


if __name__ == "__main__":
    raise SystemExit(main())
