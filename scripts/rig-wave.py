#!/usr/bin/env python3
"""One blessed driver for an evidence wave, driven by a JSON recipe.

Live mode clicks through `rig-wine.sh send click` (the SendInput helper),
records frames through `lib/wire_frames.py`, and applies proxy rules from the
recipe. Replay mode consumes a cited tap log offline and reproduces the same
evidence strings without a Wine boot, Memoryer, or game data. A boot-attach
leg writes its wire log to OUT_DIR/wire/room-stage.log; pass that path as
--frames so cue and host-marker steps read the same log.

Usage:
  scripts/rig-wave.py --recipe scripts/recipes/row-vanish-582.json \
      --mode replay --frames TAP_LOG --out OUT_DIR
  scripts/rig-wave.py --recipe RECIPE --mode live --out OUT_DIR \
      --frames OUT_DIR/wire/room-stage.log --display :NNN \
      [--host-dir HOST_DIR] [--proxy-listen PORT --proxy-upstream PORT]
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import socket
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from collections.abc import Callable
from pathlib import Path
from typing import Mapping

from PIL import Image

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS / "lib"))

from popup_state import NORMAL_MODE_NOTICE, ImageFrameAdapter, query_popup  # noqa: E402
from wave_recipe import (  # noqa: E402
    INJECTORS,
    Aim,
    RecipeError,
    Step,
    WaveRecipe,
    load_recipe,
)
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
RIG_POLL_SECONDS = 0.2
PROCESS_TIMEOUT_SECONDS = 180
TAP_STOP_TIMEOUT_SECONDS = 10


class WaveFailure(RuntimeError):
    """A recipe step could not be evidenced; no pack may be claimed."""


@dataclass(frozen=True)
class RunOutput:
    driver_lines: tuple[str, ...]
    evidence_lines: tuple[str, ...]
    captures: Mapping[str, object]


class RigProcesses:
    """The live process seam; tests replace it with a recording fake."""

    @staticmethod
    def run(argv: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            argv, env=os.environ.copy(), capture_output=True, text=True,
            timeout=PROCESS_TIMEOUT_SECONDS, check=False,
        )

    @staticmethod
    def spawn(argv: list[str]) -> subprocess.Popen[str]:
        return subprocess.Popen(
            argv, env=os.environ.copy(), stdout=subprocess.DEVNULL,
            stderr=subprocess.STDOUT, text=True,
        )

    @staticmethod
    def wait_for_text(path: Path, text: str, timeout: float) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if path.is_file() and text in path.read_text(errors="replace"):
                return
            time.sleep(RIG_POLL_SECONDS)
        raise WaveFailure(f"{path} never printed {text!r}")


@dataclass(frozen=True)
class RigCommand:
    """One step of a boot/attach leg, with its readiness cue."""

    label: str
    argv: tuple[str, ...]
    ready_text: str = ""
    ready_log: Path | None = None


def boot_attach_commands(step: Step, out: Path, display: str) -> tuple[RigCommand, ...]:
    """Translate one boot/attach leg into the pinned rig command chain."""
    leg = step.leg
    client = str(leg["client"])
    wire_log = out / "wire" / "room-stage.log"
    commands: list[RigCommand] = []
    if leg["mode"] == "boot":
        wire_port = str(leg["wire_port"])
        upstream_port = str(leg["upstream_port"])
        if client == "exe":
            commands.append(RigCommand(
                "mint-login-argv",
                ("bash", str(SCRIPTS / "rig-mint-launch-argv.sh"),
                 "--user-index-id", str(leg["user_index_id"]),
                 "--out", str(out / "argv.txt"),
                 "--gateway-port", wire_port),
            ))
        commands.append(RigCommand(
            "wire-tap",
            (sys.executable, str(SCRIPTS / "wine-room-host.py"), "--wire-tap",
             "--out-dir", str(out / "wire"), "--wire-log", str(wire_log),
             "--listen-port", wire_port, "--upstream-port", upstream_port),
            ready_text="wire-tap listening", ready_log=wire_log,
        ))
        if client == "exe":
            launch = ("bash", str(SCRIPTS / "rig-wine.sh"), "launch", str(out / "argv.txt"))
        else:
            launch = tuple(str(word) for word in leg["launch"])
        commands.append(RigCommand(
            "client-boot",
            ("bash", str(SCRIPTS / "rig-run.sh"), "run", display, str(out),
             "--", *launch),
        ))
    if client == "exe":
        login = (sys.executable, str(SCRIPTS / "wine-scene.py"),
                 "--attach", str(out), "--display", display,
                 "--target", str(leg["target"]), "--no-host",
                 "--wire-log", str(wire_log))
    else:
        login = tuple(str(word) for word in leg["login"])
    commands.append(RigCommand("login-flow", login))
    return tuple(commands)


class BootSession:
    """Owns the processes a boot/attach leg started, so cleanup is exact."""

    def __init__(self, out: Path) -> None:
        self.out = out
        self.tap: subprocess.Popen[str] | None = None
        self.booted = False

    def stop(self, processes: RigProcesses) -> None:
        if self.booted:
            processes.run(["bash", str(SCRIPTS / "rig-run.sh"), "stop", str(self.out)])
            self.booted = False
        if self.tap is not None and self.tap.poll() is None:
            self.tap.terminate()
            self.tap.wait(timeout=TAP_STOP_TIMEOUT_SECONDS)
            self.tap = None


def launch_boot_attach(step: Step, out: Path, display: str,
                       processes: RigProcesses,
                       session: BootSession | None = None) -> BootSession:
    """Run the chain; a refused preflight stops before the login flow."""
    session = session or BootSession(out)
    timeout = float(step.leg["login_timeout_seconds"])
    for command in boot_attach_commands(step, out, display):
        if command.ready_text:
            session.tap = processes.spawn(list(command.argv))
            if session.tap.poll() is not None:
                raise WaveFailure(f"{command.label} exited before {command.ready_text}")
            processes.wait_for_text(command.ready_log, command.ready_text, timeout)
            continue
        result = processes.run(list(command.argv))
        if result.returncode != 0:
            raise WaveFailure(
                f"{command.label} failed (exit {result.returncode}): "
                f"{result.stderr.strip() or 'standard preflight refused the run'}")
        if command.label == "client-boot":
            session.booted = True
    return session


class SceneSync:
    """Wait for a painted screen or a state marker; never guess with a sleep."""

    def __init__(self, step: Step, out: Path,
                 probe: Callable[[], str] | None,
                 now: Callable[[], float] | None = None,
                 sleep: Callable[[float], None] | None = None) -> None:
        self.step = step
        self.out = out
        self.probe = probe
        self.now = now or time.monotonic
        self.sleep = sleep or time.sleep

    def wait(self) -> str:
        timeout = float(self.step.leg["timeout_seconds"])
        deadline = self.now() + timeout
        while True:
            if self._reached():
                return self._line()
            if self.now() >= deadline:
                wanted = self.step.leg.get("scene") or self.step.leg.get("state_text")
                raise WaveFailure(f"scene-sync {self.step.id} timed out waiting for {wanted}")
            self.sleep(RIG_POLL_SECONDS)

    def _reached(self) -> bool:
        if self.step.leg["mode"] == "screen":
            return self.probe() == self.step.leg["scene"]
        path = Path(str(self.step.leg["state_file"]))
        if not path.is_absolute():
            path = self.out / path
        if not path.is_file():
            return False
        return str(self.step.leg["state_text"]) in path.read_text(errors="replace")

    def _line(self) -> str:
        if self.step.leg["mode"] == "screen":
            return (f"scene-sync {self.step.id} screen={self.step.leg['scene']} "
                    f"label={self.step.leg['scene']}")
        return (f"scene-sync {self.step.id} "
                f"state={_state_name(str(self.step.leg['state_text']))}")


class HostMarker:
    """Queue a host command only after the guest opcode reaches the tap."""

    def __init__(self, step: Step, host_dir: Path, out: Path,
                 lines_provider: Callable[[], list[WireLine]],
                 now: Callable[[], float] | None = None,
                 sleep: Callable[[float], None] | None = None) -> None:
        self.step = step
        self.host_dir = host_dir
        self.out = out
        self.lines_provider = lines_provider
        self.now = now or time.monotonic
        self.sleep = sleep or time.sleep

    def run(self) -> str:
        self._await_guest()
        self._queue_command()
        return (f"host-marker {self.step.id} gate={self.step.leg['gate_opcode']} "
                f"command={self.step.leg['command']}")

    def _await_guest(self) -> None:
        gate = _cue_step(self.step.id, int(self.step.leg["gate_opcode"]))
        deadline = self.now() + float(self.step.leg["timeout_seconds"])
        while True:
            if find_cue(self.lines_provider(), gate) is not None:
                return
            if self.now() >= deadline:
                raise WaveFailure(
                    f"host-marker {self.step.id}: guest {gate.opcode} never reached the tap")
            self.sleep(RIG_POLL_SECONDS)

    def _queue_command(self) -> None:
        self.host_dir.mkdir(parents=True, exist_ok=True)
        with (self.host_dir / "host.cmd").open("a", encoding="utf-8") as handle:
            handle.write(str(self.step.leg["command"]) + "\n")
        ack = str(self.step.leg["ack_text"])
        log = self.host_dir / "host.log"
        deadline = self.now() + float(self.step.leg["timeout_seconds"])
        while True:
            if log.is_file() and ack in log.read_text(errors="replace"):
                break
            if self.now() >= deadline:
                raise WaveFailure(f"host-marker {self.step.id}: host did not ack {ack!r}")
            self.sleep(RIG_POLL_SECONDS)
        marker = self.out / f"{self.step.leg['marker']}.ready"
        marker.parent.mkdir(parents=True, exist_ok=True)
        marker.write_text("ok\n", encoding="utf-8")


class SendInputCommand:
    """Build Wine SendInput clicks through the siclick helper path."""

    @staticmethod
    def command(aim: Aim) -> list[str]:
        x, y = aim.point()
        return [str(SCRIPTS / "rig-wine.sh"), "send", "click", str(x), str(y)]


class ClickDispatcher:
    """One injector seam with two adapters: Wine SendInput and native XTEST."""

    def __init__(self, display: str, processes: RigProcesses | None = None) -> None:
        self.display = display
        self.processes = processes or RigProcesses()
        self.name = "sendinput"

    def select(self, name: str) -> None:
        if name not in INJECTORS:
            raise WaveFailure(f"injector {name!r} is not published: {sorted(INJECTORS)}")
        self.name = name

    def command(self, aim: Aim) -> list[str]:
        if self.name == "sendinput":
            return SendInputCommand.command(aim)
        x, y = aim.point()
        return [str(SCRIPTS / "rig-xtest.py"), "--display", self.display,
                "click", "1", str(x), str(y)]

    def click(self, aim: Aim) -> str:
        command = self.command(aim)
        result = self.processes.run(command)
        if result.returncode != 0:
            raise WaveFailure(
                f"{self.name} click {aim.key} failed: {result.stderr.strip()}")
        x, y = aim.point()
        return f"{self.name} click {aim.key} at ({x},{y})"


class ReplayRig:
    """Replay a recipe against recorded frames without touching a rig."""

    def __init__(self, lines: list[WireLine], out: Path | None = None) -> None:
        self._lines = lines
        self._out = out

    def boot_attach(self, step: Step) -> str:
        return (f"boot-attach {step.leg['client']} -> {step.leg['target']} "
                f"evidence={step.evidence}")

    def scene_sync(self, step: Step) -> str:
        if step.leg["mode"] == "screen":
            observed = step.evidence.get("observed_scene")
            if observed != step.leg["scene"]:
                raise WaveFailure(
                    f"scene-sync {step.id} replay saw {observed!r}, "
                    f"expected {step.leg['scene']!r}")
            return f"scene-sync {step.id} screen={observed}"
        path = Path(str(step.leg["state_file"]))
        if not path.is_absolute() and self._out is not None:
            path = self._out / path
        if not path.is_file() or str(step.leg["state_text"]) not in path.read_text(
                errors="replace"):
            raise WaveFailure(f"scene-sync {step.id} replay has no {step.leg['state_text']!r}")
        return f"scene-sync {step.id} state={_state_name(str(step.leg['state_text']))}"

    def host_marker(self, step: Step) -> str:
        gate = _cue_step(step.id, int(step.leg["gate_opcode"]))
        if find_cue(self._lines, gate) is None:
            raise WaveFailure(
                f"host-marker {step.id}: guest {gate.opcode} never reached the tap")
        return (f"host-marker {step.id} gate={gate.opcode} "
                f"command={step.leg['command']} (replay)")

    def set_injector(self, step: Step) -> str:
        return f"injector {step.leg['name']} (replay)"

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

    def __init__(self, out: Path, display: str, frames: Path,
                 processes: RigProcesses | None = None, scene_probe=None,
                 host_dir: Path | None = None) -> None:
        self._out = out
        self._display = display
        self._frames = frames
        self._processes = processes or RigProcesses()
        self._clicks = ClickDispatcher(display, self._processes)
        self._scene_probe = scene_probe
        self._host_dir = host_dir
        self._session: BootSession | None = None

    def boot_attach(self, step: Step) -> str:
        self._session = BootSession(self._out)
        launch_boot_attach(step, self._out, self._display,
                           self._processes, self._session)
        return (f"boot-attach {step.leg['client']} -> {step.leg['target']} "
                f"via rig-run.sh (standard preflight)")

    def scene_sync(self, step: Step) -> str:
        probe = self._scene_probe or _wine_scene_label(self._display, self._out)
        return SceneSync(step, self._out, probe).wait()

    def host_marker(self, step: Step) -> str:
        if self._host_dir is None:
            raise WaveFailure(f"host-marker {step.id} needs --host-dir")
        return HostMarker(
            step, self._host_dir, self._out, self._tap_lines).run()

    def set_injector(self, step: Step) -> str:
        self._clicks.select(str(step.leg["name"]))
        return f"injector {self._clicks.name}"

    def stop(self) -> None:
        if self._session is not None:
            self._session.stop(self._processes)
            self._session = None

    def _tap_lines(self) -> list[WireLine]:
        try:
            return parse_tap_lines(self._frames.read_text(errors="replace"))
        except OSError:
            return []

    def probe(self, step: Step) -> str:
        score = self._popup_score()
        return f"{_now()} {step.label} {step.phase} blue={score:.3f} label=live-frame"

    def dismiss_popup(self, step: Step) -> str:
        for _ in range(step.max_tries):
            if self._popup_score() <= NORMAL_MODE_NOTICE.floor:
                return ""
            self._clicks.click(self._required_aim(step))
        raise WaveFailure(f"popup still up at aim {step.aim.key}; BACK aborted")

    def click(self, step: Step) -> str:
        aim = self._required_aim(step)
        click_line = self._clicks.click(aim)
        return f"{_now()} {step.text} evidence={aim.evidence} via={click_line}"

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


def _cue_step(step_id: str, opcode: int) -> Step:
    return Step(id=step_id, action="wire-cue", direction="C2S", opcode=opcode)


def _state_name(state_text: str) -> str:
    if "name=" not in state_text:
        return "marker"
    return state_text.split("name=", 1)[1].split()[0]


def _wine_scene_label(display: str, out: Path):
    """Borrow the pinned scene classifier; do not duplicate its pixel rules."""
    path = SCRIPTS / "wine-scene.py"
    spec = importlib.util.spec_from_file_location("rig_wave_wine_scene", path)
    if spec is None or spec.loader is None:
        raise WaveFailure(f"cannot load scene classifier {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.Frames(display, out / "scene-probe").scene_label


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
        elif step.action == "boot-attach":
            driver_lines.append(rig.boot_attach(step))
        elif step.action == "scene-sync":
            driver_lines.append(rig.scene_sync(step))
        elif step.action == "host-marker":
            driver_lines.append(rig.host_marker(step))
        elif step.action == "set-injector":
            driver_lines.append(rig.set_injector(step))
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
    parser.add_argument("--host-dir", type=Path, help="external host-bot command/log dir")
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
            result = execute(recipe, ReplayRig(lines, args.out))
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
    rig = LiveRig(args.out, args.display, args.frames, host_dir=args.host_dir)
    try:
        return execute(recipe, rig)
    finally:
        rig.stop()
        if tap is not None:
            tap.stop()


if __name__ == "__main__":
    raise SystemExit(main())
