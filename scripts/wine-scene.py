#!/usr/bin/env python3
"""Drive the Wine pair from the notice board to the loading screen (#240).

The Wine-side counterpart of `scripts/planet-to-waiting.sh`: it attaches to
a session `scripts/rig-run.sh` already booted, drives the 7.55 exe through
planet -> channel -> room -> waiting -> loading, captures one framebuffer
PNG per scene, and writes a scene log in the shape the kit's probes and
manifests already read (`frame N screen=<scene> ...`).

Two input paths, both documented (never a bare XTEST click on the canvas):
  * canvas buttons go through `scripts/rig-wine.sh send` — Win32 SendInput
    from inside Wine, the only path the DirectDraw canvas polls
    (`docs/wine-boot.md`, Attempts 7-9);
  * chrome that answers the message queue may also use XTEST, but this
    drive keeps one path per click and records which it used.

The `hover-press` join mode follows the #240 OPEN arm 1 evidence step:
warp the X pointer onto the painted room row, capture the exe's own
hover highlight, then ONE SendInput press with no SetCursorPos. That is
the mechanical difference from the proven `siclick3` shape
(`~/scratch/button-unblock/siclick3.c`) and from this drive's default
`dclick` path, which calls SetCursorPos inside Wine before sending.

Every aim point carries the pinned evidence that placed it. The drive
prints one `aim <name> at (x,y) evidence=<...>` line per target before the
first click, so a reviewer can check the target without reading pixels.

Stage transitions are logged with the cue that established them: the exe's
own wire frames observed in the Memoryer server log (`03EA` channel list
answer, `03EC` room-list request, `0BBA` join-waiting, `4008` ready,
`4012` confirm-music-loaded) plus a mean-luma sample of the captured frame.
A stage that times out is logged as FAILED and the run keeps what it has:
nothing is invented, and no capture is written for a scene that did not
appear.

Usage:
  scripts/wine-scene.py --attach OUT_DIR --display :N [options]

Options:
  --attach OUT_DIR      the run dir scripts/rig-run.sh booted
  --display :N          that run's X display
  --shots DIR           screenshot dir (default OUT_DIR/artifacts/screenshots)
  --target SCENE        last scene the drive must reach (default loading;
                        accepts planet|channel|room|waiting|loading)
  --music-id ID         room music the host sets (default 757)
  --account NAME        guest account name in the server log (default tester)
  --server-log PATH     Memoryer log (default ~/memoryer-local/server.log)
  --host-user-index-id  host account (default 3); 0 disables the host bot
  --join-timeout S      seconds to wait for 0BBA after the row double-click
  --no-host             drive without the host bot (room stage only)
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import numpy as np
from PIL import Image

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
CANVAS = (800, 600)

# Aim points, each with the evidence that placed it. Coordinates are native
# 800x600 canvas pixels (the Wine desktop is that size at the X screen's
# origin, so XTEST coordinates and canvas coordinates agree).
AIMS = {
    "easy-emblem": (
        136,
        88,
        "docs/captures: planet-verify findings.md 'reopen: siclick (136,88)'; "
        "the H/EASY plate opens the Channel dialog",
    ),
    "fallback-ok": (
        400,
        365,
        "planet-verify findings.md: the NORMAL-mode message box OK sits at "
        "(400,365) and blocks canvas clicks until dismissed",
    ),
    "channel-enter": (
        526,
        248,
        "docs/channel-fill.md: bnd-pinned enter face (502,236) 44x21 on row 0, "
        "centre (524,246); the live Wine aim (526,248) is the same face",
    ),
    "room-row-title": (
        100,
        100,
        "docs/wine-boot.md Attempt 13: the row TITLE bar is the hot zone "
        "(100,100 joins first try; the panel middle never emits 0BBA). "
        "The #240 leg saw no 0BBA from this aim, so hover-press verifies "
        "the hotspot from the exe's own highlight before pressing.",
    ),
    "normal-emblem": (
        255,
        88,
        "docs/captures: planet-verify findings.md 'NORMAL (255,88)' — the "
        "family switch forces a reconnect and a fresh channel list",
    ),
    "ready-pill": (
        680,
        143,
        "docs/wine-boot.md Attempt 11: the waiting-room READY pill sits at "
        "(680,143) and sends 4008",
    ),
}

# Probe boxes, measured from this leg's own live frames (the captures that
# fed them are cited in docs/captures/t240/findings.md).
MENU_BAND = (555, 595)          # dark navy MENU bar under the planet board
TAB_BAND = (50, 140)            # emblem/tab strip along the board's head
PLATE_BOX = (300, 160, 500, 185)  # channel dialog's grey title plate
POPUP_BOX = (300, 240, 500, 340)  # blue message-box body
ROW_BANNER_BOX = (40, 120, 330, 148)  # hosted row's green music banner
ROW_TITLE_BOX = (40, 85, 330, 120)    # title strip; hover highlight lands here

# Scene signatures, measured from this leg's own frames (captures cited in
# docs/captures/t240/findings.md). A capture is labelled only when its own
# pixels match; a stage that is not on screen is reported, never labelled.
BOARD_MENU_LUMA = 90.0      # board's navy MENU bar 53.9; room stage 101.4
BOARD_TAB_BRIGHT = 0.10     # board emblem strip 0.133
ROOM_TAB_BRIGHT = 0.55      # room stage 0.694-0.709
CHANNEL_PLATE_SAT = 6.0     # channel title plate 1.9; board promo area 30-61
CHANNEL_PLATE_LUMA = (100.0, 145.0)


class SceneLog:
    """Writes `frame N ... screen=<scene>` lines plus plain event lines."""

    def __init__(self, path: Path) -> None:
        self.handle = open(path, "w", buffering=1, encoding="utf-8")
        self.index = 0

    def event(self, message: str) -> None:
        line = f"{time.strftime('%H:%M:%S')} {message}"
        print(line, flush=True)
        self.handle.write(line + "\n")

    def scene(self, name: str, mean_luma: float, cue: str, shot: str) -> None:
        # The scene parsers (`run-manifest.sh write`, `scene-probe.sh`) key on
        # a line that STARTS with "frame ", so the clock goes last.
        self.index += 1
        line = (
            f"frame {self.index} screen={name} mean_luma={mean_luma:.1f} "
            f"cue={cue} shot={shot} t={time.strftime('%H:%M:%S')}"
        )
        print(line, flush=True)
        self.handle.write(line + "\n")


class Frames:
    """Framebuffer captures and the pixel probes built on them."""

    def __init__(self, display: str, work_dir: Path) -> None:
        self.display = display
        self.work_dir = work_dir
        self.work_dir.mkdir(parents=True, exist_ok=True)
        self.last: np.ndarray | None = None

    def grab(self) -> np.ndarray:
        target = self.work_dir / "probe.png"
        command = [
            "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
            "-f", "x11grab", "-video_size", f"{CANVAS[0]}x{CANVAS[1]}",
            "-i", f"{self.display}+0,0", "-frames:v", "1", str(target),
        ]
        subprocess.run(command, check=True, capture_output=True)
        self.last = np.asarray(Image.open(target).convert("RGB")).astype(np.int16)
        return self.last

    def capture(self, path: Path) -> float:
        frame = self.grab()
        Image.fromarray(frame.astype(np.uint8)).save(path, format="PNG")
        return float(frame.mean())

    def mean_luma(self) -> float:
        frame = self.last if self.last is not None else self.grab()
        luma = frame @ np.array([0.299, 0.587, 0.114])
        return float(luma.mean())

    def board_is_up(self) -> bool:
        frame = self.last if self.last is not None else self.grab()
        luma = frame @ np.array([0.299, 0.587, 0.114])
        menu = luma[MENU_BAND[0]:MENU_BAND[1], :]
        tabs = luma[TAB_BAND[0]:TAB_BAND[1], :]
        return bool(menu.mean() < 90 and (tabs > 100).mean() > 0.10)

    def popup_fraction(self) -> float:
        """Share of the message-box box that is the blue body (0..1)."""
        frame = self.last if self.last is not None else self.grab()
        left, top, right, bottom = POPUP_BOX
        box = frame[top:bottom, left:right]
        red, _, blue = box[:, :, 0], box[:, :, 1], box[:, :, 2]
        blue_body = (blue > 120) & (blue - red > 60)
        return float(blue_body.mean())

    def popup_is_up(self) -> bool:
        # 0.9, not 0.5: the promo carousel's blue page fills this box on the
        # bare board (measured 0.83) while a real message box measures 0.99.
        return self.popup_fraction() > 0.9

    def scene_label(self) -> str:
        """Which scene these pixels are, by this leg's own signatures."""
        frame = self.last if self.last is not None else self.grab()
        luma = frame @ np.array([0.299, 0.587, 0.114])
        menu = float(luma[MENU_BAND[0]:MENU_BAND[1], :].mean())
        tabs = float((luma[TAB_BAND[0]:TAB_BAND[1], :] > 100).mean())
        left, top, right, bottom = PLATE_BOX
        plate = frame[top:bottom, left:right]
        plate_luma = float(plate.mean())
        plate_sat = float((plate.max(axis=2) - plate.min(axis=2)).mean())
        if menu > BOARD_MENU_LUMA and tabs > ROOM_TAB_BRIGHT:
            return "room"
        if menu <= BOARD_MENU_LUMA:
            if plate_sat < CHANNEL_PLATE_SAT and (
                CHANNEL_PLATE_LUMA[0] <= plate_luma <= CHANNEL_PLATE_LUMA[1]
            ):
                return "channel"
            if tabs > BOARD_TAB_BRIGHT:
                return "planet"
        return "unknown"

    def row_painted(self) -> bool:
        frame = self.last if self.last is not None else self.grab()
        left, top, right, bottom = ROW_BANNER_BOX
        box = frame[top:bottom, left:right]
        red, green, blue = box[:, :, 0], box[:, :, 1], box[:, :, 2]
        banner = (green > 110) & (green - blue > 25) & (green - red > 15)
        return bool(banner.mean() > 0.04)

    def row_highlight_delta(self, rest: np.ndarray) -> float:
        """Mean absolute pixel change in the title strip after the X warp."""
        frame = self.last if self.last is not None else self.grab()
        left, top, right, bottom = ROW_TITLE_BOX
        changed = np.abs(frame[top:bottom, left:right] -
                         rest[top:bottom, left:right])
        return float(changed.mean())


class WireTapLog:
    """Read-only view of the run's pass-through wire tap.

    The writer lives in `wine-room-host.py --wire-tap`. A downstream 2005
    line is the room-created event whose arrival opens the row-paint window;
    counting it here ties each pixel probe to a wire event in drive.log.
    """

    ROOM_CREATED = 2005

    def __init__(self, path: Path) -> None:
        self.path = path
        self.offset = 0
        self.room_created_count = 0

    def take(self) -> list[str]:
        if not self.path.is_file():
            return []
        with open(self.path, "rb") as handle:
            handle.seek(self.offset)
            text = handle.read().decode("utf-8", "replace")
            self.offset = handle.tell()
        fresh = []
        for line in text.splitlines():
            if "S2C" in line and f"opcode={self.ROOM_CREATED} " in line:
                self.room_created_count += 1
                fresh.append(line)
        return fresh

    def note(self, log: SceneLog, label: str) -> None:
        events = self.take()
        first = f" first={events[0]}" if events else ""
        log.event(f"wire {label} S2C 2005 count={self.room_created_count}{first}")


class RoomPaintWait:
    """Watches for the hosted row while keeping the session alive.

    `row_dismiss=none` is the fuse hypothesis arm: the no-response modal
    is left up because its OK button sends FFF0 and ends the session before
    the row can paint. `repaint` is called once at the scheduled time so a
    late host rebuild can be tested inside the same window.
    """

    def __init__(self, frames: Frames, injector: Input, wire: WireTapLog,
                 log: SceneLog, row_dismiss: str) -> None:
        self.frames = frames
        self.injector = injector
        self.wire = wire
        self.log = log
        self.row_dismiss = row_dismiss
        self.popup_was_up = False

    def observe_popup(self) -> None:
        is_up = self.frames.popup_is_up()
        if is_up and self.row_dismiss == "auto":
            self.injector.send_click("fallback-ok")
            time.sleep(1.0)
        elif is_up != self.popup_was_up:
            self.log.event(f"paint wait popup_up={is_up} row_dismiss={self.row_dismiss}")
        self.popup_was_up = is_up

    def run(self, timeout: float, repaint=None, repaint_after: float = 0.0) -> bool:
        started = time.time()
        deadline = started + timeout
        repaint_sent = False
        while time.time() < deadline:
            if (repaint is not None and not repaint_sent
                    and time.time() - started >= repaint_after):
                repaint()
                repaint_sent = True
            self.frames.grab()
            self.observe_popup()
            if self.frames.row_painted():
                self.wire.note(self.log, "row painted")
                return True
            time.sleep(0.5)
        self.wire.note(self.log, "row window closed")
        return False


ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*m")


class ServerLog:
    """Follows the Memoryer log for one account's opcodes."""

    def __init__(self, path: Path, account: str) -> None:
        self.path = path
        self.account = account
        self.offset = path.stat().st_size if path.is_file() else 0

    def take(self) -> list[str]:
        if not self.path.is_file():
            return []
        # Byte offsets: the log carries multi-byte UTF-8 (Korean) and ANSI
        # colouring, so a text-mode seek can land mid-character.
        with open(self.path, "rb") as handle:
            handle.seek(self.offset)
            text = handle.read().decode("utf-8", "replace")
            self.offset = handle.tell()
        opcodes = []
        for raw_line in text.splitlines():
            # The console log carries ANSI colouring around every field.
            line = ANSI_ESCAPE.sub("", raw_line)
            # Log field layout: time level OPCODE --- [ controller ] [ account ]
            if " --- [" not in line:
                continue
            if f"[ {self.account} " not in line and f"[ {self.account}" not in line:
                continue
            for token in line.split():
                if len(token) == 4 and all(c in "0123456789ABCDEF" for c in token):
                    opcodes.append(token)
                    break
        return opcodes

    def wait_for(self, opcode: str, timeout: float) -> bool:
        deadline = time.time() + timeout
        while time.time() < deadline:
            if opcode in self.take():
                return True
            time.sleep(0.5)
        return False

class Input:
    """One home per input path: SendInput for canvas, XTEST for the record."""

    def __init__(self, display: str, env: dict, log: SceneLog) -> None:
        self.display = display
        self.env = env
        self.log = log
        self.cited: set[str] = set()

    def cite(self, aim: str) -> tuple[int, int]:
        x, y, evidence = AIMS[aim]
        if aim not in self.cited:
            self.log.event(f"aim {aim} at ({x},{y}) evidence={evidence}")
            self.cited.add(aim)
        return x, y

    def send_click(self, aim: str) -> None:
        x, y = self.cite(aim)
        result = subprocess.run(
            [str(SCRIPT_DIR / "rig-wine.sh"), "send", "click", str(x), str(y)],
            env=self.env, capture_output=True, text=True, timeout=120,
        )
        detail = " ".join(
            line for line in result.stdout.splitlines()
            if line.startswith(("opendesktop", "setthreaddesktop", "click"))
        )
        self.log.event(f"inject send-click {aim} ({x},{y}) {detail}")
        if result.returncode != 0:
            self.log.event(f"inject send-click {aim} FAILED rc={result.returncode}")

    def send_double_click(self, aim: str) -> None:
        # One helper process for both pairs: two `send click` invocations are
        # ~1 s apart (Wine startup each), far outside the client's
        # double-click time, so the pair must come from one process.
        x, y = self.cite(aim)
        result = subprocess.run(
            [str(SCRIPT_DIR / "rig-wine.sh"), "send", "dclick", str(x), str(y)],
            env=self.env, capture_output=True, text=True, timeout=120,
        )
        detail = " ".join(
            line for line in result.stdout.splitlines()
            if line.startswith(("opendesktop", "setthreaddesktop", "dclick"))
        )
        self.log.event(f"inject send-double-click {aim} ({x},{y}) {detail}")
        if result.returncode != 0:
            self.log.event(f"inject send-double-click {aim} FAILED rc={result.returncode}")

    def warp_pointer(self, x: int, y: int) -> None:
        """Move the X pointer with XTEST motion; the canvas polls motion."""
        result = subprocess.run(
            ["python3", str(SCRIPT_DIR / "rig-xtest.py"),
             "--display", self.display, "move", str(x), str(y)],
            env=self.env, capture_output=True, text=True, timeout=60,
        )
        detail = result.stdout.strip().splitlines()
        self.log.event(f"inject x-warp ({x},{y}) rc={result.returncode} "
                       f"{detail[-1] if detail else ''}")
        if result.returncode != 0:
            self.log.event(f"inject x-warp ({x},{y}) FAILED")

    def send_press(self) -> None:
        """One SendInput press at the current pointer, with no SetCursorPos."""
        result = subprocess.run(
            [str(SCRIPT_DIR / "rig-wine.sh"), "send", "press"],
            env=self.env, capture_output=True, text=True, timeout=120,
        )
        detail = " ".join(
            line for line in result.stdout.splitlines()
            if line.startswith(("opendesktop", "setthreaddesktop", "press"))
        )
        self.log.event(f"inject send-press {detail}")
        if result.returncode != 0:
            self.log.event(f"inject send-press FAILED rc={result.returncode}")

    def send_click_xy(self, x: int, y: int) -> None:
        result = subprocess.run(
            [str(SCRIPT_DIR / "rig-wine.sh"), "send", "click", str(x), str(y)],
            env=self.env, capture_output=True, text=True, timeout=120,
        )
        self.log.event(f"inject send-click ({x},{y}) rc={result.returncode}")

    def send_dclick_xy(self, x: int, y: int) -> None:
        result = subprocess.run(
            [str(SCRIPT_DIR / "rig-wine.sh"), "send", "dclick", str(x), str(y)],
            env=self.env, capture_output=True, text=True, timeout=120,
        )
        self.log.event(f"inject send-dclick ({x},{y}) rc={result.returncode}")

    def send_key(self, name: str, direction: str) -> None:
        subprocess.run(
            [str(SCRIPT_DIR / "rig-wine.sh"), "send", "key", name, direction],
            env=self.env, capture_output=True, text=True, timeout=120,
        )
        self.log.event(f"inject send-key {name} {direction}")

    def xtest_click(self, aim: str) -> None:
        x, y = self.cite(aim)
        result = subprocess.run(
            [sys.executable, str(SCRIPT_DIR / "rig-xtest.py"),
             "--display", self.display, "click", str(x), str(y)],
            capture_output=True, text=True, timeout=60,
        )
        self.log.event(f"inject xtest-click {aim} ({x},{y}) rc={result.returncode}")


def dismiss_popup(frames: Frames, injector: Input, log: SceneLog, tries: int = 4) -> bool:
    """Click the message box's OK while its blue body is up.

    Two samples per look: the box's blue body animates (a pulsing ring), so a
    single frame can read low while the box is on screen and swallow the
    next click.
    """
    for attempt in range(1, tries + 1):
        frames.grab()
        first = frames.popup_fraction()
        time.sleep(0.4)
        frames.grab()
        highest = max(first, frames.popup_fraction())
        log.event(f"popup probe attempt={attempt} max_blue={highest:.3f}")
        if highest <= 0.9:
            return True
        injector.send_click("fallback-ok")
        time.sleep(1.5)
    frames.grab()
    still_up = frames.popup_is_up()
    log.event(f"popup dismiss {'FAILED' if still_up else 'ok'}")
    return not still_up


def capture_scene(frames: Frames, log: SceneLog, shots: Path, name: str,
                  cue: str, accept: tuple[str, ...] = ()) -> bool:
    """Write <name>.png only when the frame's own pixels show that scene.

    `accept` lists additional pixel labels that count as this scene. The
    waiting room and the loading gate have no measured signature yet, so
    they are accepted as "unknown" — a screen that is none of the three
    calibrated scenes — and only ever on their own wire cue (0BBA / 4012),
    which the caller has already matched. The gap is named in the findings.
    """
    frames.grab()
    label = frames.scene_label()
    if label != name and label not in accept:
        log.event(f"stage {name} NOT captured: screen shows '{label}' cue={cue}")
        return False
    mean = frames.capture(shots / f"{name}.png")
    log.scene(name, mean, cue, f"{name}.png")
    return True


def wait_until(probe, timeout: float, interval: float = 1.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if probe():
            return True
        time.sleep(interval)
    return False


class Host:
    """The room host bot: created before entry, driven by a command file."""

    def __init__(self, out_dir: Path, music_id: int, user_index_id: int, log: SceneLog):
        self.out_dir = out_dir
        self.music_id = music_id
        self.user_index_id = user_index_id
        self.log = log
        self.dir = out_dir / "host"
        self.dir.mkdir(parents=True, exist_ok=True)
        self.cmd_file = self.dir / "host.cmd"
        self.process: subprocess.Popen | None = None

    def start(self) -> None:
        self.cmd_file.write_text("", encoding="utf-8")
        self.process = subprocess.Popen(
            [sys.executable, str(SCRIPT_DIR / "wine-room-host.py"),
             "--out-dir", str(self.dir),
             "--user-index-id", str(self.user_index_id),
             "--music-id", str(self.music_id),
             "--hold", "600", "--start-delay", "20"],
            stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        (self.dir / "host.pid").write_text(f"{self.process.pid}\n", encoding="utf-8")
        self.log.event(f"host started pid={self.process.pid}")

    def command(self, token: str) -> None:
        with open(self.cmd_file, "a", encoding="utf-8") as handle:
            handle.write(token + "\n")
        self.log.event(f"host command queued {token}")

    def wait_for_room(self, timeout: float = 30.0) -> bool:
        deadline = time.time() + timeout
        log_file = self.dir / "host.log"
        while time.time() < deadline:
            if log_file.is_file() and "host room created" in log_file.read_text(encoding="utf-8"):
                return True
            time.sleep(0.5)
        return False

    def stop(self) -> None:
        if self.process is None:
            return
        subprocess.run(
            ["kill", "-TERM", str(self.process.pid)], capture_output=True, timeout=10
        )
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            subprocess.run(["kill", "-KILL", str(self.process.pid)], capture_output=True)
        self.log.event("host stopped")


def hover_press_join(frames: Frames, injector: Input, server: ServerLog,
                     log: SceneLog, shots: Path, join_timeout: float) -> bool:
    """One #240 OPEN arm 1 stimulus: hover, capture the highlight, press.

    One stimulus per boot. The X warp settles the exe's own row highlight;
    the capture is the evidence that the hot zone is under the pointer.
    The press carries no SetCursorPos, so it lands on that same hovered
    control instead of a Wine-relative cursor position.
    """
    x, y = injector.cite("room-row-title")
    rest = frames.grab()
    injector.warp_pointer(x, y)
    time.sleep(0.6)
    frames.grab()
    delta = frames.row_highlight_delta(rest)
    frames.capture(shots / "room-row-hover.png")
    log.event(f"room row hover delta={delta:.2f} at ({x},{y})")
    injector.send_press()
    return server.wait_for("0BBA", join_timeout)


def no_join_failure_message(join_mode: str) -> str:
    """Name the stimulus count that this join mode can actually send."""
    if join_mode == "hover-press":
        return "stage waiting FAILED reason=no 0BBA after one hover-press"
    return "stage waiting FAILED reason=no 0BBA after two joins"


def drive(arguments, frames: Frames, server: ServerLog, injector: Input,
          log: SceneLog, shots: Path, wire: WireTapLog) -> list[str]:
    reached: list[str] = []
    targets = ["planet", "channel", "room", "waiting", "loading"]
    wanted = targets[: targets.index(arguments.target) + 1]

    def board_ready() -> bool:
        frames.grab()
        return frames.board_is_up()

    if not wait_until(board_ready, 150.0, 2.0):
        # Keep the failing framebuffer: a blind failure cannot be triaged.
        frames.capture(shots / "probe-failure.png")
        log.event("stage planet FAILED reason=board never rendered shot=probe-failure.png")
        return reached
    if not capture_scene(frames, log, shots, "planet", "probe:board"):
        frames.capture(shots / "probe-failure.png")
        return reached
    reached.append("planet")

    opened = False
    for attempt in range(1, 4):
        dismiss_popup(frames, injector, log)
        injector.send_click("easy-emblem")
        if server.wait_for("03EA", 30.0):
            opened = True
            break
        log.event(f"retry easy-emblem attempt={attempt} reason=no 03EA")
    if not opened:
        log.event("stage channel FAILED reason=no 03EA after three emblem clicks")
        frames.capture(shots / "probe-failure.png")
        return reached
    dismiss_popup(frames, injector, log)
    if not capture_scene(frames, log, shots, "channel", "server:03EA"):
        frames.capture(shots / "probe-failure.png")
        return reached
    reached.append("channel")
    if "room" not in wanted:
        return reached

    host = None
    if arguments.host_user_index_id:
        host = Host(Path(arguments.attach), arguments.music_id,
                    arguments.host_user_index_id, log)
        host.start()
        # The room must exist BEFORE the guest asks for the list: a room
        # created afterwards needs another 2005 event to reach the client
        # (docs/wine-boot.md, Attempt 12: entries land on fresh 2005 data).
        if not host.wait_for_room(40.0):
            log.event("host FAILED reason=no room created")
        else:
            log.event("host room created")
        time.sleep(2)
    try:
        def reopen_live_list() -> bool:
            """Force a fresh channel list after a dead dialog.

            A dialog that reopened without asking for the list again has
            blank rows and a dead enter face (docs/captures: planet-verify
            findings.md, "Reopen works cleanly only after a real family
            switch"). Switching family reconnects and re-requests, then the
            EASY emblem brings the list back.
            """
            log.event("reopen live channel list via family switch")
            injector.send_click("normal-emblem")
            time.sleep(3)
            injector.send_click("easy-emblem")
            if not server.wait_for("03EA", 25.0):
                return False
            dismiss_popup(frames, injector, log)
            return True

        def enter_room_stage() -> bool:
            for attempt in range(1, 4):
                dismiss_popup(frames, injector, log)
                injector.send_click("channel-enter")
                if server.wait_for("03EC", 15.0):
                    return True
                log.event(f"enter attempt {attempt} got no 03EC")
                if attempt < 3 and not reopen_live_list():
                    log.event("reopen_live_list failed")
            return False

        def attempt_join() -> bool:
            if arguments.row_dismiss == "auto":
                if not dismiss_popup(frames, injector, log, tries=2):
                    log.event("join attempt with the message box still up")
            else:
                log.event("join attempt with row-dismiss=none (message box left up)")
            if arguments.join_mode == "hover-press":
                return hover_press_join(frames, injector, server, log, shots,
                                        arguments.join_timeout)
            injector.send_double_click("room-row-title")
            return server.wait_for("0BBA", arguments.join_timeout)

        entered = enter_room_stage()
        if not entered:
            log.event("stage room FAILED reason=no 03EC after two enter clicks")
            frames.capture(shots / "probe-failure.png")
            return reached

        # Capture the room stage on arrival: the stage itself is the scene
        # (its chrome replaces the board), while the hosted row inside it
        # needs a member/music poll (07DA/07E8) that can be ~36 s away or,
        # as this leg measured, may not come before the idle fuse returns
        # the client to the board.
        time.sleep(2)
        if not capture_scene(frames, log, shots, "room", "server:03EC"):
            frames.capture(shots / "probe-failure.png")
            return reached
        reached.append("room")
        if "waiting" not in wanted:
            return reached

        wire.note(log, "room stage entered")
        repaint = None
        if arguments.row_repaint == "recreate" and host is not None:
            # The idle fuse can return the client to the board before its
            # member/music poll paints the row. A host rebuild sends fresh
            # 2005 data while the guest sits on the room stage (Attempt 12).
            def repaint() -> None:
                host.command("recreate")
        room_wait = RoomPaintWait(frames, injector, wire, log, arguments.row_dismiss)
        painted = room_wait.run(40.0, repaint, arguments.repaint_delay)
        log.event(f"room row painted={painted}")
        joined = painted and attempt_join()
        log.event(f"join attempt 1 joined={joined}")
        # The #240 OPEN arm 1 calls for one stimulus per boot. The warm
        # re-entry is the qualified dclick path's second attempt; the
        # hover-press experiment must not send a second press in one boot.
        if not joined and arguments.join_mode == "dclick":
            # Warm re-entry (docs/wine-boot.md, Attempt 13 "reliable loop"):
            # the idle stage fuse makes the client send FFF0 and authorise
            # again by itself, and the next room entry paints its row at
            # once from the cached room detail instead of waiting a fresh
            # 36 s poll.
            if server.wait_for("03EA", 150.0):
                dismiss_popup(frames, injector, log)
                warm_wait = RoomPaintWait(frames, injector, wire, log, "auto")
                if enter_room_stage() and warm_wait.run(25.0):
                    joined = attempt_join()
                    log.event(f"join attempt 2 joined={joined}")
        if not joined:
            log.event(no_join_failure_message(arguments.join_mode))
            frames.capture(shots / "probe-failure.png")
            return reached
        time.sleep(3)
        dismiss_popup(frames, injector, log)
        if not capture_scene(frames, log, shots, "waiting", "server:0BBA",
                             accept=("unknown",)):
            frames.capture(shots / "probe-failure.png")
            return reached
        reached.append("waiting")
        if "loading" not in wanted:
            return reached

        injector.send_click("ready-pill")
        if not server.wait_for("4008", 20.0):
            log.event("ready FAILED reason=no 4008 (the start may still arrive)")
        if host is not None:
            host.command("start")
        if not server.wait_for("4012", 30.0):
            log.event("stage loading FAILED reason=no 4012 confirm")
            return reached
        time.sleep(2)
        # No pixel signature for the loading gate yet: the scene is claimed
        # by the client's own 4012 ConfirmMusicLoaded, and the capture is
        # recorded as cue-free pixels under that claim (docs note the gap).
        mean = frames.capture(shots / "loading.png")
        log.scene("loading", mean, "server:4012", "loading.png")
        reached.append("loading")
    finally:
        if host is not None:
            host.stop()
    return reached


def read_env(out_dir: Path, wine_prefix: str) -> dict:
    """The run's env (from rig-run.sh's env.sh) plus this process's own."""
    merged = dict(os.environ)
    env_file = out_dir / "env.sh"
    if env_file.is_file():
        for line in env_file.read_text(encoding="utf-8").splitlines():
            if line.startswith("export ") and "=" in line:
                key, value = line[len("export "):].split("=", 1)
                merged[key] = value.strip("'")
    if wine_prefix:
        merged["WINEPREFIX"] = wine_prefix
    return merged


def main() -> int:
    parser = argparse.ArgumentParser(prog="wine-scene")
    parser.add_argument("--attach", required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--shots", default="")
    parser.add_argument("--target", default="loading",
                        choices=["planet", "channel", "room", "waiting", "loading"])
    parser.add_argument("--music-id", type=int, default=757)
    parser.add_argument("--account", default="tester")
    parser.add_argument("--server-log",
                        default=str(Path.home() / "memoryer-local" / "server.log"))
    parser.add_argument("--host-user-index-id", type=int, default=3)
    parser.add_argument("--join-timeout", type=float, default=12.0)
    parser.add_argument("--row-dismiss", choices=["auto", "none"],
                        default="auto",
                        help="auto clears the no-response modal; none keeps "
                             "the session whose OK button sends FFF0")
    parser.add_argument("--repaint-delay", type=float, default=0.0,
                        help="seconds from room entry before a host recreate")
    parser.add_argument("--wire-log", default="",
                        help="the run's pass-through wire log (defaults to "
                             "OUT_DIR/wire/room-stage.log)")
    parser.add_argument("--join-mode", choices=["dclick", "hover-press"],
                        default="dclick",
                        help="join stimulus: the qualified double-click or the "
                             "#240 OPEN arm 1 X-warp hover then one press")
    parser.add_argument("--row-repaint", choices=["none", "recreate"],
                        default="none",
                        help="host-side room rebuild that repaints the list from "
                             "fresh 2005 data (docs/wine-boot.md Attempt 12)")
    parser.add_argument("--wineprefix", default=os.environ.get("WINEPREFIX", ""))
    parser.add_argument("--no-host", action="store_true")
    arguments = parser.parse_args()
    if arguments.no_host:
        arguments.host_user_index_id = 0

    out_dir = Path(arguments.attach).resolve()
    shots = Path(arguments.shots) if arguments.shots else out_dir / "artifacts" / "screenshots"
    shots.mkdir(parents=True, exist_ok=True)
    log = SceneLog(out_dir / "drive.log")
    log.event(f"drive start display={arguments.display} target={arguments.target} "
              f"account={arguments.account} host_user={arguments.host_user_index_id}")
    # Probe frames are working pixels, not evidence: keep them out of the
    # run dir so the provenance manifest lists only the scene captures.
    probe_dir = Path(tempfile.mkdtemp(prefix="wine-scene-probe-"))
    frames = Frames(arguments.display, probe_dir)
    server = ServerLog(Path(arguments.server_log), arguments.account)
    wire = WireTapLog(Path(arguments.wire_log) if arguments.wire_log
                      else out_dir / "wire" / "room-stage.log")
    injector = Input(arguments.display,
                     read_env(out_dir, arguments.wineprefix), log)
    reached: list[str] = []
    try:
        reached = drive(arguments, frames, server, injector, log, shots, wire)
    finally:
        log.event(f"drive done reached={','.join(reached)} probe_dir={probe_dir}")
    return 0 if reached else 1


if __name__ == "__main__":
    sys.exit(main())
