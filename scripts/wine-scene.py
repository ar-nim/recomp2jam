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
The READY pill is retried after the Waiting capture: an early click is
silently swallowed by the exe, so only a C2S `4008` plus the guest's `0FA8`
server line arms the host's start command. A stage that times out is logged
as FAILED and the run keeps what it has: nothing is invented, and no capture
is written for a scene that did not appear.

Usage:
  scripts/wine-scene.py --attach OUT_DIR --display :N [options]

Options:
  --attach OUT_DIR      the run dir scripts/rig-run.sh booted
  --display :N          that run's X display
  --shots DIR           screenshot dir (default OUT_DIR/artifacts/screenshots)
  --target SCENE        last scene the drive must reach (default loading;
                        accepts planet|channel|room|waiting|loading|playing)
  --music-id ID         room music the host sets (default 757)
  --account NAME        guest account name in the server log (default tester)
  --server-log PATH     Memoryer log (default ~/memoryer-local/server.log)
  --host-user-index-id  host account (default 3); 0 disables the host bot
  --join-timeout S      seconds to wait for 0BBA after the row double-click
  --observe-waiting-seconds S
                        after a joined 0BBA, poll frames and wire for S
                        seconds without claiming a Waiting scene (cue leg)
  --cue-discover-seconds S
                        after a proven Waiting capture, inventory the room
                        affordances, probe the guest READY control, then queue
                        the host start command while polling frames and wire;
                        writes observation PNGs only, no Loading/Playing claim
  --host-start-delay S   host's automatic start delay after the guest joins
                        (default 20; the cue leg raises it so only the
                        explicit command can start the song)
  --no-host             drive without the host bot (room stage only)
"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import NamedTuple

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
    # #330 side scenes. Each aim was fired once in boot b1-explore on :260;
    # the cited frame is the resulting framebuffer capture (sha256 in the
    # aim string). Docs pin the control code and rect; the capture pins the
    # live coordinate. Existing AIMS strings above are unchanged.
    "option-gear": (
        729,
        580,
        "docs/planet.md OPTION gear rect (710,561)-(748,599); live "
        "~/scratch/side-scenes/b1-explore/explore/04-after-gear.png "
        "sha256=6d17a781e4d5e522da5ad72e62c26281ae0d7317b20bde9d7bad73421548043e",
    ),
    "exit-disc": (
        769,
        580,
        "docs/planet.md EXIT disc rect (750,561)-(788,599); live "
        "~/scratch/side-scenes/b1-explore/explore/08-after-exit-disc.png "
        "sha256=3dbcdf2f23c1069a9e7bc2e69cbc8aabc6cb4fa3f9ee343d98f1b4f2622e8553",
    ),
    "board-prime": (
        557,
        195,
        "live b1-explore/explore/02-after-x-557-195.png sha256="
        "5b38c0d05467e9280d6122f210899c97dd1946f158c86c41828aa0db439c3482; "
        "harmless board click that primes the canvas before the first control",
    ),
    "crash-cancel": (
        451,
        330,
        "live b1-explore/explore/no-board.png sha256="
        "24a76463e21dea812e230055a85ce0612816de6b048b9557ca5b17beef555d29; "
        "Cancel of the stale-dump crash report box",
    ),
    "crash-cancel-alt": (
        420,
        295,
        "docs/wine-boot.md Attempt 11: the same crash report Cancel sits at "
        "(420,295) when the box is centred higher",
    ),
    "room-musicshop": (
        350,
        16,
        "docs/hover.md IDC_BUTTON_SROOM_MUSICSHOP 0x13200037 rect "
        "317,2-384,31; live b1-explore/explore/h04-musicshop-long.png "
        "sha256=964be986ee1fa269306ad2115dfdb18efc1cc511e5dfa765e5845698bb8ee8aa",
    ),
    "room-myroom": (
        517,
        16,
        "docs/hover.md IDC_BUTTON_SROOM_MYROOM 0x13200009 rect 484,2-550,31; "
        "live b1-explore/explore/e01-myroom.png sha256="
        "0758c389ae49076f419a24e7d9e14a710808205c0b1a2dee84e661cbbc8d57eb",
    ),
    "room-firststep": (
        689,
        16,
        "docs/hover.md IDC_BUTTON_SROOM_FIRSTSTEP 0x13200032 rect 660,1-718,32; "
        "live b1-explore/explore/i03-firststep.png sha256="
        "2b1deb31a32de95df5b77520fd4f8a135f05c404820bf93e96852076b6c294ed",
    ),
    "room-messenger": (
        644,
        580,
        "docs/hover.md IDC_BUTTON_SROOM_MESSENGER 0x13200048 rect "
        "627,571-662,590; live b1-explore/explore/e04-messenger.png sha256="
        "946e503319b6fa18fcd28b2cf9230b9124b5177cc002567c07afac84631ad917",
    ),
    "options-exit": (
        472,
        532,
        "live b1-explore/explore/04-after-gear.png sha256="
        "6d17a781e4d5e522da5ad72e62c26281ae0d7317b20bde9d7bad73421548043e; "
        "Options dialog EXIT button (Escape is unreliable)",
    ),
    "side-dialog-ok": (
        400,
        368,
        "live b1-explore/explore/e04-messenger.png sha256="
        "946e503319b6fa18fcd28b2cf9230b9124b5177cc002567c07afac84631ad917; "
        "OK of the side-state MESSAGE popup",
    ),
    "side-back": (
        757,
        580,
        "live b1-explore/explore/h02-shop-back.png sha256="
        "118591bae1dc95b4a2c3dd3bbfeeb5fecea6a524c40eedb60168a1ba449ef170; "
        "BACK on MyRoom, Item Shop and Music Shop returns the room stage",
    ),
    "myroom-itemshop": (
        89,
        320,
        "live b1-explore/explore/f03-back-from-shop.png sha256="
        "9c898bfb357010d961eb3829fef738e5cb0e0f3ead83c285372b26ed8eb35b4b; "
        "MyRoom ITEM SHOP plate under the figure",
    ),
    "itemshop-cash-charge": (
        92,
        355,
        "live b1-explore/explore/g01-cash-charge.png sha256="
        "007e9ed7cb8d608f9bd877e65ebaf137d80c6be3593bdf0138e0ed09bc2265f2; "
        "Item Shop cash charge plate in the left panel",
    ),
}

# The exe swallows a READY click until the pill is live. e2-playing-qualified
# clicked ~1 s after 0BBA with no 4008; c1-start-cue/drive.log:52-63 pins the
# successful click and its C2S 4008 / S2C 4009 pair. Retry on a short cadence,
# but never queue the host start on an unconfirmed guest.
READY_RETRY_SECONDS = 3.0
READY_RETRY_WINDOW_SECONDS = 30.0
READY_CONFIRM_SECONDS = 20.0
JOIN_WIRE_OPCODE = 3002          # 0BBA, the wire clock's zero point
C2S_READY_OPCODE = 4008          # 0FA8, the guest's READY press

# Probe boxes, measured from this leg's own live frames (the captures that
# fed them are cited in docs/captures/t240/findings.md).
MENU_BAND = (555, 595)          # dark navy MENU bar under the planet board
TAB_BAND = (50, 140)            # emblem/tab strip along the board's head
PLATE_BOX = (300, 160, 500, 185)  # channel dialog's grey title plate
POPUP_BOX = (300, 240, 500, 340)  # blue message-box body
ROW_BANNER_BOX = (40, 120, 330, 148)  # hosted row's green music banner
ROW_TITLE_BOX = (40, 85, 330, 120)    # title strip; hover highlight lands here

# Waiting-room signature, measured on build 9b4b0ea from the c1-waiting-observe
# pack: drive.log:36 and wire/room-stage.log:21-22 pin the frame to 0BBA/0BBB,
# and artifacts/screenshots/waiting-observe-001.png measures
# 0.615 / 0.298 / 0.681. The pack's room.png measures 0.000 / 0.032 / 0.273.
# All three boxes are needed: the room and waiting stages share the bright
# top strip that the older scene_label rule used as its room cue.
WAITING_GREEN_TOP_BOX = (570, 15, 780, 40)
WAITING_READY_PILL_BOX = (625, 125, 780, 165)
WAITING_SKILL_PANEL_BOX = (625, 190, 780, 420)
WAITING_GREEN_TOP_FRACTION = 0.35
WAITING_READY_PILL_FRACTION = 0.15
WAITING_SKILL_PANEL_FRACTION = 0.45

# Probe boxes for the #326 cue leg, read off c3-waiting-qualified's
# artifacts/screenshots/waiting.png (native 800x600 canvas). These name
# controls to inspect; they are NOT a scene classifier.
WAITING_AFFORDANCE_BOXES = {
    "settings_bar": (570, 15, 780, 40),
    "ready_pill": (625, 125, 780, 165),
    "team_grid": (635, 55, 775, 130),
    "skill_panel": (625, 190, 780, 355),
    "speed_panel": (625, 350, 780, 420),
    "song_slots": (610, 445, 790, 565),
    "back_button": (700, 570, 790, 598),
}
CASH_TITLE_BOX = (195, 5, 375, 45)   # Cash Charge dialog's blue title bar
CASH_TITLE_BLUE_FRACTION = 0.30

CUE_QUIET_SECONDS = 6.0     # watch the joined room before any stimulus
CUE_HOST_START_SECONDS = 14.0  # after the READY probe has been read

# Loading signature, measured on the c1-start-cue pack's own pixels:
# drive.log:63 pins the frame to the 4011/4012 start gate
# (wire/room-stage.log:28-31), and
# artifacts/screenshots/waiting-cue-012.png measures 0.284 / 0.278 / 54.0.
# The same boxes on planet measure 0.044 / 0.067 / 10.9; on channel
# 0.018 / 0.067 / 10.9. The old planet rule mislabelled this art.
LOADING_WARM_ART_BOX = (100, 100, 350, 500)   # song art's warm centre-left
LOADING_BADGE_BOX = (700, 410, 800, 490)      # white 3KEY VERSION badge
LOADING_LOGO_BOX = (700, 0, 800, 40)          # O2Jam logo at top right
LOADING_WARM_ART_FRACTION = 0.15
LOADING_BADGE_BRIGHT_FRACTION = 0.15
LOADING_LOGO_SATURATION = 30.0

# Provisional Playing boxes, taken from the lane/HUD geometry in
# docs/geometry-audit.md:31 and the native design reference
# ~/scratch/smoke-finish/05-playing-03.png (that frame is NOT exe evidence).
# The boot's own pack must name the measured pixels before this is cited.
PLAYING_LANE_BOX = (0, 0, 205, 520)
PLAYING_HUD_BOX = (200, 520, 800, 600)
PLAYING_LANE_DARK_FRACTION = 0.60
PLAYING_HUD_BLUE_FRACTION = 0.20
PLAYING_TIMEOUT_SECONDS = 90.0

SCENE_TARGETS = ["planet", "channel", "room", "waiting", "loading", "playing"]


class SideStep(NamedTuple):
    """One click, its settle time, and the scene its pixels must prove."""

    scene: str
    aim: str
    settle: float
    close_key: str = ""
    close_aim: str = ""


class SidePlan(NamedTuple):
    """A side-scene walk: whether it starts from the room stage and its steps."""

    needs_room: bool
    steps: tuple[SideStep, ...]
    prime_aim: str = ""


# #330 side scenes. Each coordinate is cited in AIMS; each guard threshold
# is measured on the boot-1 frame named there. A target's last step names
# the scene reported by scene-probe.sh and by the manifest.
SIDE_PLANS = {
    "options": SidePlan(False, (
        SideStep("options", "option-gear", 4.0, "escape"),
    ), "board-prime"),
    "quit-confirm": SidePlan(False, (
        SideStep("options", "option-gear", 4.0, "", "options-exit"),
        SideStep("quit-confirm", "exit-disc", 4.0),
    ), "board-prime"),
    "myroom": SidePlan(True, (
        SideStep("myroom", "room-myroom", 8.0),
    )),
    "itemshop": SidePlan(True, (
        SideStep("myroom", "room-myroom", 8.0),
        SideStep("itemshop", "myroom-itemshop", 12.0),
    )),
    "cash-charge": SidePlan(True, (
        SideStep("myroom", "room-myroom", 8.0),
        SideStep("messenger", "room-messenger", 8.0,
                 close_aim="side-dialog-ok"),
        SideStep("itemshop", "myroom-itemshop", 12.0),
        SideStep("cash-charge", "itemshop-cash-charge", 8.0),
    )),
    "musicshop": SidePlan(True, (
        SideStep("musicshop", "room-musicshop", 12.0),
    )),
    "tutorial": SidePlan(True, (
        SideStep("musicshop", "room-musicshop", 12.0, close_aim="side-back"),
        SideStep("tutorial", "room-firststep", 8.0),
    )),
    "messenger": SidePlan(True, (
        SideStep("myroom", "room-myroom", 8.0),
        SideStep("messenger", "room-messenger", 8.0),
    )),
}
SIDE_TARGETS = tuple(SIDE_PLANS)

# (box, colour predicate, minimum share) measured on boot b1-explore.
# A click cue alone can mislabel a frozen frame; the guard refuses pixels
# that do not show the named screen. Classifiers are optional for #330, so
# these only gate the label, they do not claim a general scene classifier.
SIDE_SIGNATURES = {
    "options": ((240, 50, 480, 90), "orange", 0.40),
    "quit-confirm": ((250, 200, 400, 230), "green", 0.30),
    "myroom": ((100, 10, 200, 45), "green", 0.10),
    "itemshop": ((250, 70, 600, 350), "neutral_dark", 0.50),
    "cash-charge": ((220, 70, 600, 500), "uniform_panel", 0.80),
    "musicshop": ((250, 70, 600, 350), "blue", 0.40),
    "tutorial": ((320, 40, 720, 210), "white", 0.20),
    "messenger": ((260, 235, 530, 375), "blue", 0.70),
}

# Scene signatures, measured from this leg's own frames (captures cited in
# docs/captures/t240/findings.md). A capture is labelled only when its own
# pixels match; a stage that is not on screen is reported, never labelled.
BOARD_MENU_LUMA = 90.0      # board's navy MENU bar 53.9; room stage 101.4
BOARD_TAB_BRIGHT = 0.10     # board emblem strip 0.133
ROOM_TAB_BRIGHT = 0.55      # room stage 0.694-0.709
CHANNEL_PLATE_SAT = 6.0     # channel title plate 1.9; board promo area 30-61
CHANNEL_PLATE_LUMA = (100.0, 145.0)


def green_top_fraction(frame: np.ndarray, box: tuple[int, int, int, int]) -> float:
    """Share of a box painted with the room-settings bar's green fill."""
    left, top, right, bottom = box
    red, green, blue = (frame[top:bottom, left:right, channel]
                        for channel in range(3))
    return float(((green > 130) & (green - red > 35) &
                  (green - blue > 15)).mean())


def orange_pill_fraction(frame: np.ndarray, box: tuple[int, int, int, int]) -> float:
    """Share of a box painted with the READY pill's orange fill."""
    left, top, right, bottom = box
    red, green, blue = (frame[top:bottom, left:right, channel]
                        for channel in range(3))
    return float(((red > 170) & (green > 80) & (green < 190) &
                  (blue < 120) & (red - blue > 80)).mean())


def box_color_observation(frame: np.ndarray,
                          box: tuple[int, int, int, int]) -> tuple[str, float]:
    """Mean RGB and bright-pixel share of one probed control box."""
    left, top, right, bottom = box
    pixels = frame[top:bottom, left:right]
    mean_rgb = tuple(float(value) for value in pixels.mean(axis=(0, 1)))
    bright = float((pixels.mean(axis=2) > 120).mean())
    text = "(" + ", ".join(f"{value:.0f}" for value in mean_rgb) + ")"
    return text, bright


def dark_fraction(frame: np.ndarray, box: tuple[int, int, int, int]) -> float:
    """Share of a box darker than the play highway's navy lane fill."""
    left, top, right, bottom = box
    pixels = frame[top:bottom, left:right]
    return float((pixels.mean(axis=2) < 60).mean())


def blue_fraction(frame: np.ndarray, box: tuple[int, int, int, int]) -> float:
    """Share of a box painted blue, as in the play HUD's gauge panel."""
    left, top, right, bottom = box
    red, green, blue = (frame[top:bottom, left:right, channel]
                        for channel in range(3))
    return float(((blue > red + 20) & (blue > green + 10)).mean())


def warm_art_fraction(frame: np.ndarray, box: tuple[int, int, int, int]) -> float:
    """Share of a box painted with the song art's warm ochre/skin tones."""
    left, top, right, bottom = box
    red, green, blue = (frame[top:bottom, left:right, channel]
                        for channel in range(3))
    warm = ((red > 120) & (red - blue > 25) &
            (green > 80) & (green - blue > 20))
    return float(warm.mean())


def mean_saturation(frame: np.ndarray, box: tuple[int, int, int, int]) -> float:
    """Mean per-pixel channel spread, high for the O2Jam logo's colours."""
    left, top, right, bottom = box
    pixels = frame[top:bottom, left:right]
    return float((pixels.max(axis=2) - pixels.min(axis=2)).mean())


def region_fraction(frame: np.ndarray, box: tuple[int, int, int, int],
                    predicate: str) -> float:
    """Share of a box painted with one of the side-scene's signature colours."""
    left, top, right, bottom = box
    pixels = frame[top:bottom, left:right]
    red, green, blue = (pixels[:, :, index] for index in range(3))
    brightness = pixels.mean(axis=2)
    tests = {
        "orange": (red > 180) & (green > 100) & (green < 200) & (blue < 100),
        "green": (green > 150) & (green > red + 25) & (green > blue + 25),
        "blue": (blue > 110) & (blue > red + 50) & (blue > green + 20),
        "white": brightness > 220,
        "uniform_panel": (brightness > 220) | (brightness < 45),
        "dark": (brightness > 10) & (brightness < 45),
        "neutral_dark": (brightness > 20) & (brightness < 130) &
                        (abs(red - blue) < 25),
    }
    return float(tests[predicate].mean())


def waiting_affordance_lines(frames: Frames, log: SceneLog) -> None:
    """Inventory the controls visible in the joined room, without a claim."""
    frame = frames.last if frames.last is not None else frames.grab()
    for name, box in WAITING_AFFORDANCE_BOXES.items():
        mean_rgb, bright = box_color_observation(frame, box)
        coordinates = ",".join(str(value) for value in box)
        log.event(f"cue affordance {name} box={coordinates} "
                  f"mean_rgb={mean_rgb} bright={bright:.3f}")


def bright_panel_fraction(frame: np.ndarray, box: tuple[int, int, int, int]) -> float:
    """Share of a box brighter than the room stage's dark blue side panel."""
    left, top, right, bottom = box
    panel = frame[top:bottom, left:right].mean(axis=2)
    return float((panel > 120).mean())


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

    def playing_metrics(self) -> tuple[float, float]:
        """Dark-lane and blue-HUD shares for this frame."""
        frame = self.last if self.last is not None else self.grab()
        return (
            dark_fraction(frame, PLAYING_LANE_BOX),
            blue_fraction(frame, PLAYING_HUD_BOX),
        )

    def playing_is_up(self) -> bool:
        lane, hud = self.playing_metrics()
        return (lane > PLAYING_LANE_DARK_FRACTION
                and hud > PLAYING_HUD_BLUE_FRACTION)

    def loading_metrics(self) -> tuple[float, float, float]:
        """Warm art, bright badge and saturated logo fractions for this frame."""
        frame = self.last if self.last is not None else self.grab()
        badge = frame[LOADING_BADGE_BOX[1]:LOADING_BADGE_BOX[3],
                      LOADING_BADGE_BOX[0]:LOADING_BADGE_BOX[2]]
        badge_bright = float((badge.mean(axis=2) > 150).mean())
        return (
            warm_art_fraction(frame, LOADING_WARM_ART_BOX),
            badge_bright,
            mean_saturation(frame, LOADING_LOGO_BOX),
        )

    def loading_is_up(self) -> bool:
        warm, badge, logo = self.loading_metrics()
        return (warm > LOADING_WARM_ART_FRACTION
                and badge > LOADING_BADGE_BRIGHT_FRACTION
                and logo > LOADING_LOGO_SATURATION)

    def waiting_metrics(self) -> tuple[float, float, float]:
        """Green bar, READY pill and side panel fractions for this frame."""
        frame = self.last if self.last is not None else self.grab()
        return (
            green_top_fraction(frame, WAITING_GREEN_TOP_BOX),
            orange_pill_fraction(frame, WAITING_READY_PILL_BOX),
            bright_panel_fraction(frame, WAITING_SKILL_PANEL_BOX),
        )

    def waiting_is_up(self) -> bool:
        green, orange, bright = self.waiting_metrics()
        return (green > WAITING_GREEN_TOP_FRACTION
                and orange > WAITING_READY_PILL_FRACTION
                and bright > WAITING_SKILL_PANEL_FRACTION)

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
        if self.playing_is_up():
            return "playing"
        if self.loading_is_up():
            return "loading"
        if self.waiting_is_up():
            return "waiting"
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

    def side_guard_fraction(self, name: str) -> float:
        """Measured signature share for one #330 side scene."""
        box, predicate, _ = SIDE_SIGNATURES[name]
        frame = self.last if self.last is not None else self.grab()
        return region_fraction(frame, box, predicate)

    def cash_title_is_up(self) -> bool:
        """The billing web view is blank in both states; its title is not."""
        frame = self.last if self.last is not None else self.grab()
        return region_fraction(frame, CASH_TITLE_BOX, "blue") > \
            CASH_TITLE_BLUE_FRACTION

    def side_scene_is_up(self, name: str) -> bool:
        _, _, minimum = SIDE_SIGNATURES[name]
        if self.side_guard_fraction(name) <= minimum:
            return False
        if name == "cash-charge":
            return self.cash_title_is_up()
        return True

    def crash_report_is_up(self) -> bool:
        """Stale-dump report box: blue title bar over a light grey body."""
        frame = self.last if self.last is not None else self.grab()
        title = frame[250, 400]
        body = frame[290, 400]
        return bool(title[2] > 200 and title[1] > 100 and title[0] < 100
                    and body.mean() > 230)


class WireTapLog:
    """Read-only view of the run's pass-through wire tap.

    The writer lives in `wine-room-host.py --wire-tap`. A downstream 2005
    line is the room-created event whose arrival opens the row-paint window;
    counting it here ties each pixel probe to a wire event in drive.log.
    """

    ROOM_CREATED = 2005
    BOUNDARY_OPCODES = (
        (1004, "03EC"), (2005, "2005"), (2010, "07DA"),
        (2024, "07E8"), (3002, "0BBA"), (65520, "FFF0"),
    )

    def __init__(self, path: Path) -> None:
        self.path = path
        self.offset = 0
        self.room_created_count = 0
        self.events: list[tuple[float, str, int]] = []

    def take(self) -> list[str]:
        if not self.path.is_file():
            return []
        with open(self.path, "rb") as handle:
            handle.seek(self.offset)
            text = handle.read().decode("utf-8", "replace")
            self.offset = handle.tell()
        fresh = []
        for line in text.splitlines():
            seconds = wire_seconds(line)
            match = WIRE_OPCODE.search(line)
            if seconds is not None and match is not None:
                self.events.append((seconds, match.group(1), int(match.group(2))))
            if "S2C" in line and f"opcode={self.ROOM_CREATED} " in line:
                self.room_created_count += 1
                fresh.append(line)
        return fresh

    def note(self, log: SceneLog, label: str) -> None:
        events = self.take()
        first = f" first={events[0]}" if events else ""
        log.event(f"wire {label} S2C 2005 count={self.room_created_count}{first}")

    def opcodes_since(self, opcode: int) -> list[str]:
        """Name every frame from the first matching opcode, in wire order."""
        self.take()
        start = next(
            (seconds for seconds, direction, value in self.events
             if direction == "C2S" and value == opcode),
            None,
        )
        if start is None:
            return []
        return [
            f"{direction}:{value}@+{seconds - start:.1f}s"
            for seconds, direction, value in self.events
            if seconds >= start
        ]

    def saw(self, opcode: int) -> bool:
        """Whether the tap has logged this opcode in either direction."""
        self.take()
        return any(value == opcode for _, _, value in self.events)

    def saw_direction(self, direction: str, opcode: int) -> bool:
        """Whether the tap has logged this opcode from one side of the pair."""
        self.take()
        return any(direction == side and opcode == value
                   for _, side, value in self.events)

    def note_boundaries(self, log: SceneLog, label: str) -> None:
        """Log the frame timeline since 03EC at one exe-visible boundary."""
        self.take()
        if not self.events:
            log.event(f"wire boundary {label} no frames")
            return
        channel_enter = next(
            (seconds for seconds, direction, opcode in self.events
             if direction == "C2S" and opcode == 1004),
            self.events[0][0],
        )
        markers = []
        for opcode, name in self.BOUNDARY_OPCODES:
            seen = next(
                (seconds for seconds, _, value in self.events if value == opcode),
                None,
            )
            if seen is not None:
                markers.append(f"{name}@+{seen - channel_enter:.1f}s")
        last = self.events[-1]
        c2s = sum(1 for _, direction, _ in self.events if direction == "C2S")
        s2c = len(self.events) - c2s
        log.event(
            f"wire boundary {label} {' '.join(markers)} "
            f"last={last[1]}:{last[2]}@+{last[0] - channel_enter:.1f}s "
            f"frames=C2S:{c2s}/S2C:{s2c}"
        )


class RoomPaintWait:
    """Watches for the hosted row while keeping the session alive.

    `row_dismiss=none` is the fuse hypothesis arm: the no-response modal
    is left up because its OK button sends FFF0 and ends the session before
    the row can paint. `repaint` is called once at the scheduled time so a
    late host rebuild can be tested inside the same window.
    """

    def __init__(self, frames: Frames, injector: Input, wire: WireTapLog,
                 log: SceneLog, row_dismiss: str, server: ServerLog | None = None) -> None:
        self.frames = frames
        self.injector = injector
        self.wire = wire
        self.log = log
        self.row_dismiss = row_dismiss
        self.server = server
        self.popup_was_up = False

    def log_boundaries(self, label: str) -> None:
        self.wire.note_boundaries(self.log, label)
        if self.server is not None:
            self.server.note_boundaries(self.log, label)

    def observe_popup(self) -> None:
        is_up = self.frames.popup_is_up()
        if is_up and self.row_dismiss == "auto":
            self.injector.send_click("fallback-ok")
            time.sleep(1.0)
        elif is_up != self.popup_was_up:
            self.log.event(f"paint wait popup_up={is_up} row_dismiss={self.row_dismiss}")
            if is_up:
                self.log_boundaries("popup-up")
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
                self.log_boundaries("row-painted")
                return True
            time.sleep(0.5)
        self.wire.note(self.log, "row window closed")
        self.log_boundaries("row-window-closed")
        return False


ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*m")
WIRE_CLOCK = re.compile(r"^(\d\d):(\d\d):(\d\d) ")
WIRE_OPCODE = re.compile(r"(C2S|S2C) opcode=(\d+) ")
SERVER_OPCODES = {
    "03EC", "0FBE", "07D0", "07D2", "13A4", "07DA", "07E8",
    "07D4", "0BBA", "FFF0",
}


def wire_seconds(line: str) -> float | None:
    """Clock seconds for a wire line, or None for a non-frame line."""
    match = WIRE_CLOCK.match(line)
    if match is None:
        return None
    hours, minutes, seconds = (int(part) for part in match.groups())
    return float(hours * 3600 + minutes * 60 + seconds)


class ServerLog:
    """Follows the Memoryer log for one account's opcodes."""

    def __init__(self, path: Path, account: str) -> None:
        self.path = path
        self.account = account
        self.offset = path.stat().st_size if path.is_file() else 0
        self.trace: list[str] = []

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
                    if token in SERVER_OPCODES:
                        self.trace.append(line)
                    break
        return opcodes

    def note_boundaries(self, log: SceneLog, label: str) -> None:
        """Write the server's opcode trace into drive.log for this run."""
        self.take()
        for line in self.trace[-24:]:
            log.event(f"server boundary {label} {line}")

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


def dismiss_popup(frames: Frames, injector: Input, log: SceneLog,
                  tries: int = 4, shots: Path | None = None) -> bool:
    """Click the message box's OK while its blue body is up.

    Two samples per look: the box's blue body animates (a pulsing ring), so a
    single frame can read low while the box is on screen and swallow the
    next click. A caller that keeps evidence captures the first up frame so
    the dialog's text and single OK can be cited (#244).
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
        if shots is not None:
            popup_shot = shots / "message-box.png"
            if not popup_shot.exists():
                mean = frames.capture(popup_shot)
                log.event(f"popup captured mean={mean:.1f} shot=message-box.png")
        injector.send_click("fallback-ok")
        time.sleep(1.5)
    frames.grab()
    still_up = frames.popup_is_up()
    log.event(f"popup dismiss {'FAILED' if still_up else 'ok'}")
    return not still_up


def capture_scene(frames: Frames, log: SceneLog, shots: Path, name: str,
                  cue: str) -> bool:
    """Write <name>.png only when the frame's own pixels show that scene."""
    frames.grab()
    label = frames.scene_label()
    if label != name:
        log.event(f"stage {name} NOT captured: screen shows '{label}' cue={cue}")
        return False
    mean = frames.capture(shots / f"{name}.png")
    log.scene(name, mean, cue, f"{name}.png")
    return True


def side_evidence_line(name: str, fraction: float, cue: str) -> str:
    """Name the guard, its threshold, and the boot-1 frame that measured it."""
    box, predicate, minimum = SIDE_SIGNATURES[name]
    coordinates = ",".join(str(value) for value in box)
    return (
        f"{name} signature {predicate}={fraction:.3f} "
        f"box={coordinates} min={minimum:.2f} "
        f"evidence=~/scratch/side-scenes/b1-explore/explore "
        f"aim={cue} (boot-1 frame sha256 in that aim's AIMS entry)"
    )


def capture_side_scene(frames: Frames, log: SceneLog, shots: Path, name: str,
                       cue: str) -> bool:
    """Write a side-scene PNG only when its measured guard passes.

    The click names the stimulus; the pixel guard stops a frozen or wrong
    scene from being relabelled. This is a label guard, not a general
    classifier (#330 makes signatures optional).
    """
    frames.grab()
    if not frames.side_scene_is_up(name):
        fraction = frames.side_guard_fraction(name)
        log.event(f"stage {name} NOT captured: guard={fraction:.3f} cue={cue}")
        return False
    mean = frames.capture(shots / f"{name}.png")
    log.event(side_evidence_line(name, frames.side_guard_fraction(name), cue))
    log.scene(name, mean, cue, f"{name}.png")
    return True


def activate_aim(injector: Input, aim: str) -> None:
    """Fire the same aim twice: the first click after a scene change is swallowed.

    Pinned by boot b5: MyRoom's ITEM SHOP hovered under a single click while
    the frame stayed MyRoom for 12 s. The repeat lands after focus settles.
    """
    injector.send_click(aim)
    time.sleep(1.2)
    injector.send_click(aim)


def execute_side_plan(plan: SidePlan, frames: Frames, injector: Input,
                      log: SceneLog, shots: Path) -> list[str]:
    """Click each planned aim and capture only the scenes that appear."""
    reached: list[str] = []
    for step in plan.steps:
        activate_aim(injector, step.aim)
        time.sleep(step.settle)
        if not capture_side_scene(frames, log, shots, step.scene,
                                  f"click:{step.aim}"):
            break
        reached.append(step.scene)
        if step.close_key:
            injector.send_key(step.close_key, "down")
            injector.send_key(step.close_key, "up")
            time.sleep(1.0)
        if step.close_aim:
            injector.send_click(step.close_aim)
            time.sleep(3.0)
    return reached


def dismiss_stale_crash_report(frames: Frames, injector: Input,
                               log: SceneLog) -> bool:
    """Clear the stale-dump report box that can cover the boot board."""
    frames.grab()
    if not frames.crash_report_is_up():
        return False
    log.event("stale crash report box up; dismiss before the board")
    injector.send_click("crash-cancel")
    time.sleep(2)
    frames.grab()
    if frames.crash_report_is_up():
        log.event("crash report still up; retry the higher Cancel aim")
        injector.send_click("crash-cancel-alt")
        time.sleep(2)
    return True


def board_ready(frames: Frames) -> bool:
    frames.grab()
    return frames.board_is_up()


def observe_waiting_transition(frames: Frames, wire: WireTapLog,
                               log: SceneLog, shots: Path, timeout: float,
                               interval: float = 1.0) -> None:
    """Poll frames and wire after 0BBA, without claiming a scene.

    Each distinct frame is saved as a hash-pinned observation capture; a
    repeated frame is not saved twice. The wire view names every frame since
    the join so a later transition can be timed against the exe's own traffic.
    """
    started = time.time()
    last_digest = ""
    capture_count = 0
    while time.time() - started < timeout:
        frame = frames.grab()
        elapsed = time.time() - started
        digest = hashlib.sha256(frame.tobytes()).hexdigest()
        shot = ""
        if digest != last_digest:
            capture_count += 1
            name = f"waiting-observe-{capture_count:03}.png"
            frames.capture(shots / name)
            shot = f" shot={name}"
            # capture() re-grabs: hash the pixels actually saved, so a
            # byte-identical PNG is never written twice into the pack.
            last_digest = hashlib.sha256(frames.last.tobytes()).hexdigest()
        opcodes = ",".join(wire.opcodes_since(3002))
        log.event(
            f"waiting observe t=+{elapsed:.1f}s label={frames.scene_label()} "
            f"mean={frames.mean_luma():.1f} wire={opcodes}{shot}"
        )
        time.sleep(interval)


def playing_evidence_line(metrics: tuple[float, float]) -> str:
    """Name the measured play pixels and the live wire cue for this frame."""
    lane, hud = metrics
    return (
        f"playing signature lane_dark={lane:.3f} hud_blue={hud:.3f} "
        f"cue=server:4014 evidence=this pack's wire/room-stage.log "
        f"and artifacts/screenshots/playing.png"
    )


def capture_probe_failure(frames: Frames, shots: Path) -> None:
    """Keep the failing frame unless the pack already holds its bytes.

    A wedged client freezes on the last painted scene (f1's probe-failure
    was byte-identical to loading.png), and the manifest requires distinct
    screenshots — a duplicate turns a citable negative pack into a
    packaging FAIL. The scene shot already proves the frozen frame, so
    the duplicate is dropped, not kept.
    """
    target = shots / "probe-failure.png"
    frames.capture(target)
    frozen = target.read_bytes()
    for sibling in shots.glob("*.png"):
        if sibling != target and sibling.read_bytes() == frozen:
            target.unlink()
            return


def loading_evidence_line(metrics: tuple[float, float, float]) -> str:
    """Name the measured Loading pixels and the pack lines that pinned them."""
    warm, badge, logo = metrics
    return (
        f"loading signature warm={warm:.3f} badge={badge:.3f} "
        f"logo_sat={logo:.1f} "
        f"evidence=c1-start-cue/drive.log:63 "
        f"c1-start-cue/wire/room-stage.log:28-31 "
        f"c1-start-cue/artifacts/screenshots/waiting-cue-012.png"
    )


def waiting_evidence_line(metrics: tuple[float, float, float]) -> str:
    """Name the measured pixels and the pack lines that pinned them."""
    green, orange, bright = metrics
    return (
        f"waiting signature green={green:.3f} orange={orange:.3f} "
        f"bright={bright:.3f} evidence=c1-waiting-observe/drive.log:36 "
        f"c1-waiting-observe/wire/room-stage.log:21-22"
    )


def wait_until(probe, timeout: float, interval: float = 1.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if probe():
            return True
        time.sleep(interval)
    return False


class ReadyPillGate:
    """Press the waiting-room pill until the guest's 4008 reaches the wire.

    Live caller chain: rig-wine-run.sh phase_drive -> wine-scene.py drive ->
    arm_start_gate -> ReadyPillGate.run. Each miss is logged with the wire
    state so an early-click failure can be cited without guessing.
    """

    def __init__(self, wire: WireTapLog, injector: Input, log: SceneLog) -> None:
        self.wire = wire
        self.injector = injector
        self.log = log

    def wire_state(self) -> str:
        return ",".join(self.wire.opcodes_since(JOIN_WIRE_OPCODE)) or "none"

    def run(self) -> bool:
        started = time.time()
        deadline = started + READY_RETRY_WINDOW_SECONDS
        attempt = 0
        while time.time() < deadline:
            attempt += 1
            self.injector.send_click("ready-pill")
            if self.wire.saw_direction("C2S", C2S_READY_OPCODE):
                self.log.event(
                    f"ready attempt={attempt} C2S 4008 confirmed "
                    f"t=+{time.time() - started:.1f}s wire={self.wire_state()}")
                return True
            remaining = max(0.0, deadline - time.time())
            wait = min(remaining, READY_RETRY_SECONDS)
            self.log.event(
                f"ready retry attempt={attempt} "
                f"t=+{time.time() - started:.1f}s c2s_4008=no "
                f"next_retry_in={wait:.1f}s wire={self.wire_state()}")
            if remaining <= 0.0:
                break
            time.sleep(wait)
        self.log.event(
            f"ready FAILED reason=no C2S 4008 after {attempt} attempts "
            f"in {READY_RETRY_WINDOW_SECONDS:.0f}s wire={self.wire_state()}")
        return False


def arm_start_gate(wire: WireTapLog, injector: Input, server: ServerLog,
                   host: Host | None, log: SceneLog) -> bool:
    """Only the ready guest may receive the host's explicit start command."""
    if not ReadyPillGate(wire, injector, log).run():
        return False
    if not server.wait_for("0FA8", READY_CONFIRM_SECONDS):
        log.event("ready FAILED reason=no 0FA8 after C2S 4008")
        return False
    log.event("ready guest 0FA8 confirmed (guest is ready)")
    if host is None:
        log.event("host start FAILED reason=no host bot")
        return False
    host.command("start")
    return True


class Host:
    """The room host bot: created before entry, driven by a command file."""

    def __init__(self, out_dir: Path, music_id: int, user_index_id: int,
                 log: SceneLog, start_delay: float = 20.0):
        self.out_dir = out_dir
        self.music_id = music_id
        self.user_index_id = user_index_id
        self.log = log
        self.start_delay = start_delay
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
             "--hold", "600", "--start-delay", str(self.start_delay)],
            stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        (self.dir / "host.pid").write_text(f"{self.process.pid}\n", encoding="utf-8")
        self.log.event(f"host started pid={self.process.pid}")

    def command(self, token: str) -> None:
        with open(self.cmd_file, "a", encoding="utf-8") as handle:
            handle.write(token + "\n")
        self.log.event(f"host command queued {token}")

    def note_boundaries(self, log: SceneLog, label: str) -> None:
        """Log the host bot's pulse and keepalive witness for this run."""
        host_log = self.dir / "host.log"
        if not host_log.is_file():
            log.event(f"host boundary {label} no host.log")
            return
        lines = [
            line for line in host_log.read_text(encoding="utf-8").splitlines()
            if any(marker in line for marker in (
                "recreate", "keepalive", "guest JOINED", "room created",
            ))
        ]
        for line in lines[-12:]:
            log.event(f"host boundary {label} {line}")

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


class StartCueObserver:
    """Frames and wire around the first song start, with no scene claim.

    The cue leg first inventories the joined room's controls, then probes
    the guest READY pill, then queues the host's explicit start command.
    Every poll names the wire frames since 0BBA; the distinct frame PNGs
    are observations for the later signature legs, not Loading/Playing
    evidence by themselves.
    """

    def __init__(self, frames: Frames, wire: WireTapLog, injector: Input,
                 host: Host, log: SceneLog, shots: Path) -> None:
        self.frames = frames
        self.wire = wire
        self.injector = injector
        self.host = host
        self.log = log
        self.shots = shots
        self.started_at = time.time()
        self.last_digest = ""
        self.captures = 0
        self.ready_sent = False
        self.host_started = False
        self.gate_logged = False

    def poll(self, elapsed: float) -> None:
        frame = self.frames.grab()
        digest = hashlib.sha256(frame.tobytes()).hexdigest()
        shot = ""
        if digest != self.last_digest:
            self.captures += 1
            name = f"waiting-cue-{self.captures:03}.png"
            self.frames.capture(self.shots / name)
            shot = f" shot={name}"
            # capture() re-grabs, so hash the bytes actually saved.
            self.last_digest = hashlib.sha256(
                self.frames.last.tobytes()).hexdigest()
        opcodes = ",".join(self.wire.opcodes_since(3002))
        self.log.event(
            f"cue t=+{elapsed:.1f}s label={self.frames.scene_label()} "
            f"mean={self.frames.mean_luma():.1f} wire={opcodes}{shot}"
        )
        if not self.gate_logged and self.wire.saw(4011):
            self.log.event("cue wire S2C 4011 seen (start gate answered)")
            self.gate_logged = True

    def run(self, seconds: float) -> None:
        self.log.event(
            f"cue discovery begin seconds={seconds:.1f}s "
            f"quiet={CUE_QUIET_SECONDS:.1f}s "
            f"host_start_at={CUE_HOST_START_SECONDS:.1f}s"
        )
        while time.time() - self.started_at < seconds:
            elapsed = time.time() - self.started_at
            if not self.ready_sent and elapsed >= CUE_QUIET_SECONDS:
                self.log.event("cue stimulus guest ready-pill (visible control)")
                self.injector.send_click("ready-pill")
                self.ready_sent = True
            if not self.host_started and elapsed >= CUE_HOST_START_SECONDS:
                self.log.event(
                    "cue stimulus host start command (host owns 4010 StartGame)")
                self.host.command("start")
                self.host_started = True
            self.poll(elapsed)
            time.sleep(1.0)
        opcodes = ",".join(self.wire.opcodes_since(3002))
        self.log.event(
            f"cue discovery end frames={self.captures} wire={opcodes}")
        self.host.note_boundaries(self.log, "start-cue")


def observe_start_cue(frames: Frames, wire: WireTapLog, injector: Input,
                      host: Host, log: SceneLog, shots: Path,
                      seconds: float) -> None:
    """Inventory the Waiting controls, then time the first start stimuli."""
    waiting_affordance_lines(frames, log)
    StartCueObserver(frames, wire, injector, host, log, shots).run(seconds)


def no_join_failure_message(join_mode: str) -> str:
    """Name the stimulus count that this join mode can actually send."""
    if join_mode == "hover-press":
        return "stage waiting FAILED reason=no 0BBA after one hover-press"
    return "stage waiting FAILED reason=no 0BBA after two joins"


def drive(arguments, frames: Frames, server: ServerLog, injector: Input,
          log: SceneLog, shots: Path, wire: WireTapLog) -> list[str]:
    reached: list[str] = []
    targets = SCENE_TARGETS
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
        dismiss_popup(frames, injector, log, shots=shots)
        injector.send_click("easy-emblem")
        if server.wait_for("03EA", 30.0):
            opened = True
            break
        log.event(f"retry easy-emblem attempt={attempt} reason=no 03EA")
    if not opened:
        log.event("stage channel FAILED reason=no 03EA after three emblem clicks")
        frames.capture(shots / "probe-failure.png")
        return reached
    dismiss_popup(frames, injector, log, shots=shots)
    if not capture_scene(frames, log, shots, "channel", "server:03EA"):
        frames.capture(shots / "probe-failure.png")
        return reached
    reached.append("channel")
    if "room" not in wanted:
        return reached

    host = None
    if arguments.host_user_index_id:
        host = Host(Path(arguments.attach), arguments.music_id,
                    arguments.host_user_index_id, log,
                    arguments.host_start_delay)
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
                if attempt == 1:
                    # The 07D2 room list can still be painting when the
                    # channel capture lands; repeat the same aim before
                    # forcing a family switch (which re-logs the session).
                    log.event("channel row settle retry (no family switch)")
                    time.sleep(3.0)
                    continue
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
        room_wait = RoomPaintWait(
            frames, injector, wire, log, arguments.row_dismiss, server)
        painted = room_wait.run(40.0, repaint, arguments.repaint_delay)
        log.event(f"room row painted={painted}")
        if host is not None:
            host.note_boundaries(log, "row-paint")
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
                warm_wait = RoomPaintWait(
                    frames, injector, wire, log, "auto", server)
                if enter_room_stage() and warm_wait.run(25.0):
                    joined = attempt_join()
                    log.event(f"join attempt 2 joined={joined}")
        if not joined:
            log.event(no_join_failure_message(arguments.join_mode))
            frames.capture(shots / "probe-failure.png")
            return reached
        if arguments.observe_waiting_seconds > 0:
            observe_waiting_transition(
                frames, wire, log, shots, arguments.observe_waiting_seconds)
            log.event("stage waiting NOT claimed reason=observation-only")
            return reached
        time.sleep(3)
        dismiss_popup(frames, injector, log)
        log.event(waiting_evidence_line(frames.waiting_metrics()))
        if not capture_scene(frames, log, shots, "waiting", "server:0BBA"):
            frames.capture(shots / "probe-failure.png")
            return reached
        reached.append("waiting")
        if arguments.cue_discover_seconds > 0:
            if host is None:
                log.event("cue discovery FAILED reason=no host bot")
            else:
                observe_start_cue(frames, wire, injector, host, log, shots,
                                  arguments.cue_discover_seconds)
            return reached
        if "loading" not in wanted:
            return reached

        if not arm_start_gate(wire, injector, server, host, log):
            frames.capture(shots / "probe-failure.png")
            return reached
        if not server.wait_for("0FAC", 30.0):
            log.event("stage loading FAILED reason=no 0FAC confirm")
            return reached
        def loading_frame_up() -> bool:
            frames.grab()
            return frames.loading_is_up()

        if not wait_until(loading_frame_up, 20.0, 1.0):
            log.event("stage loading FAILED reason=4012 answered but pixels "
                      "did not match the measured signature")
            frames.capture(shots / "probe-failure.png")
            return reached
        log.event(loading_evidence_line(frames.loading_metrics()))
        if not capture_scene(frames, log, shots, "loading", "server:4012"):
            frames.capture(shots / "probe-failure.png")
            return reached
        reached.append("loading")
        if "playing" not in wanted:
            return reached

        def poll_play() -> bool:
            frames.grab()
            lane, hud = frames.playing_metrics()
            opcodes = ",".join(wire.opcodes_since(3002))
            log.event(
                f"play observe lane={lane:.3f} hud={hud:.3f} "
                f"label={frames.scene_label()} wire={opcodes}")
            return frames.playing_is_up() and wire.saw(4014)

        if not wait_until(poll_play, PLAYING_TIMEOUT_SECONDS, 2.0):
            log.event("stage playing FAILED reason=no 4014 with play pixels")
            capture_probe_failure(frames, shots)
            return reached
        log.event(playing_evidence_line(frames.playing_metrics()))
        if not capture_scene(frames, log, shots, "playing", "server:4014"):
            frames.capture(shots / "probe-failure.png")
            return reached
        reached.append("playing")
    finally:
        if host is not None:
            host.stop()
    return reached


def drive_side_scenes(arguments, frames: Frames, server: ServerLog,
                      injector: Input, log: SceneLog, shots: Path,
                      wire: WireTapLog) -> list[str]:
    """Reach a #330 side scene; reuse the pinned planet->room walk if needed.

    The plan's last step names the reported scene. A room-side boot disables
    the host bot: these scenes need no room host, only the pinned lobby walk.
    """
    dismiss_stale_crash_report(frames, injector, log)
    plan = SIDE_PLANS[arguments.target]
    if plan.needs_room:
        room_arguments = argparse.Namespace(**vars(arguments))
        room_arguments.target = "room"
        room_arguments.host_user_index_id = 0
        reached = drive(room_arguments, frames, server, injector, log,
                        shots, wire)
        if "room" not in reached:
            return reached
    else:
        reached = []
        if not wait_until(lambda: board_ready(frames), 150.0, 2.0):
            frames.capture(shots / "probe-failure.png")
            log.event("stage planet FAILED reason=board never rendered")
            return reached
        if not capture_scene(frames, log, shots, "planet", "probe:board"):
            frames.capture(shots / "probe-failure.png")
            return reached
        reached.append("planet")
    if plan.prime_aim:
        injector.send_click(plan.prime_aim)
        time.sleep(1.0)
    reached.extend(execute_side_plan(plan, frames, injector, log, shots))
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


def build_argument_parser() -> argparse.ArgumentParser:
    """One parser for the CLI and the seam tests that check its targets."""
    parser = argparse.ArgumentParser(prog="wine-scene")
    parser.add_argument("--attach", required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--shots", default="")
    parser.add_argument("--target", default="loading",
                        choices=SCENE_TARGETS + list(SIDE_TARGETS))
    parser.add_argument("--music-id", type=int, default=757)
    parser.add_argument("--account", default="tester")
    parser.add_argument("--server-log",
                        default=str(Path.home() / "memoryer-local" / "server.log"))
    parser.add_argument("--host-user-index-id", type=int, default=3)
    parser.add_argument("--join-timeout", type=float, default=12.0)
    parser.add_argument("--observe-waiting-seconds", type=float, default=0.0,
                        help="poll frames and wire after a joined 0BBA without "
                             "claiming a Waiting scene (cue-discovery leg)")
    parser.add_argument("--cue-discover-seconds", type=float, default=0.0,
                        help="after a proven Waiting capture, inventory the "
                             "controls and observe the first song start")
    parser.add_argument("--host-start-delay", type=float, default=20.0,
                        help="host's automatic start delay after JOINED")
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
    return parser


def main() -> int:
    parser = build_argument_parser()
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
        if arguments.target in SIDE_TARGETS:
            reached = drive_side_scenes(
                arguments, frames, server, injector, log, shots, wire)
        else:
            reached = drive(
                arguments, frames, server, injector, log, shots, wire)
    finally:
        log.event(f"drive done reached={','.join(reached)} probe_dir={probe_dir}")
    return 0 if reached else 1


if __name__ == "__main__":
    sys.exit(main())
