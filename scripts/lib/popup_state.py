#!/usr/bin/env python3
"""One popup-state query for the 7.55 rig (#747).

W6 freeze rule: NEVER blind-Escape a booting client. Bare Escape froze the
Wine desktop mid-boot; the NORMAL-mode box must be read from a frame and
dismissed at its own OK face (`docs/agents/environment.md:114`,
`docs/wine-boot.md`, `docs/rig-runbook.md`). This module is that read:
callers ask the query first, then press an aim published by a definition.

Before this seam, `lib/planet_popup.py` and `wine-scene.py` shared the box
and the 0.9 floor but used different blue tests
(`blue > red * 1.25 and blue > green * 1.15` versus `blue - red > 60`),
and aims were copied into drive scripts. Both are now consumers here.

Frame adapters:
  * `ImageFrameAdapter` — window-driver PNG captures (the old PIL path);
  * `ArrayFrameAdapter` — the x11grab ndarray held by `wine-scene.Frames`;
  * `WinedbgMemoryAdapter` — stopped-state framebuffer bytes read through
    winedbg; its core keeps the pinned X-plate NCC test
    (`docs/captures/t207/channel.md`, `~/scratch/planet-verify/common.py`).

Aims are declared once, beside the popup definitions. `lint-aims` fails on
a new hardcoded popup aim in a rig script; docs and Rust test fixtures keep
their coordinate citations as evidence, not as rig aims.
"""

from __future__ import annotations

import argparse
import math
import re
import sys
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Callable, Iterable, Protocol, Sequence

from PIL import Image

Box = tuple[int, int, int, int]
Pixel = tuple[int, int, int]


@dataclass(frozen=True)
class PopupAim:
    """A click face plus the evidence that placed it."""

    x: int
    y: int
    evidence: str

    def as_pair(self) -> tuple[int, int]:
        return self.x, self.y


POPUP_AIMS: dict[str, PopupAim] = {
    "normal-mode-ok": PopupAim(
        400,
        365,
        "docs/captures/t240/findings.md and ~/scratch/planet-verify/findings.md: "
        "the NORMAL-mode message box OK, which blocks canvas clicks",
    ),
    "crash-report-cancel": PopupAim(
        449,
        329,
        "docs/captures/ev-shops.md and "
        "docs/captures/journey-audit/313-tutorial-evidence-evtutorial.md: "
        "the .dmp report Cancel face; never substitute blind Escape",
    ),
    "channel-dialog-close": PopupAim(
        577,
        171,
        "~/scratch/planet-verify/findings.md: the verified X-close of the "
        "channel dialog (the earlier (557,195) probe was never valid)",
    ),
}


def aim(key: str) -> PopupAim:
    """Return the one published aim for a popup face."""
    try:
        return POPUP_AIMS[key]
    except KeyError as error:
        known = ", ".join(sorted(POPUP_AIMS))
        raise KeyError(f"unknown popup aim {key!r}; known aims: {known}") from error


class Detector(Enum):
    """How a frame decides that a popup is up."""

    BLUE_BODY = "blue-body"
    X_PLATE_NCC = "x-plate-ncc"


@dataclass(frozen=True)
class PopupDefinition:
    key: str
    detector: Detector
    box: Box
    floor: float
    aim: PopupAim | None = None


NORMAL_MODE_NOTICE = PopupDefinition(
    key="normal-mode-notice",
    detector=Detector.BLUE_BODY,
    box=(300, 240, 500, 340),
    floor=0.9,
    # 0.9, not 0.5: the promo carousel's blue page fills this box on the
    # bare board (measured 0.83) while a real message box measures 0.99
    # (docs/captures/t240/findings.md).
    aim=aim("normal-mode-ok"),
)

CHANNEL_DIALOG = PopupDefinition(
    key="channel-dialog",
    detector=Detector.X_PLATE_NCC,
    box=(548, 148, 615, 198),
    floor=0.7,
    aim=aim("channel-dialog-close"),
)

POPUPS: dict[str, PopupDefinition] = {
    popup.key: popup for popup in (NORMAL_MODE_NOTICE, CHANNEL_DIALOG)
}


def is_blue_body(pixel: Pixel) -> bool:
    """One blue predicate for every adapter (the wine-scene pin)."""
    red, _green, blue = pixel
    return blue > 120 and blue - red > 60


def blue_fraction(pixels: Iterable[Pixel]) -> float:
    samples = list(pixels)
    if not samples:
        return 0.0
    return sum(is_blue_body(pixel) for pixel in samples) / len(samples)


def normalized_correlation(sample: Sequence[int], template: Sequence[int]) -> float:
    """Zero-mean, unit-variance correlation; 1.0 is a template match."""
    if not template or len(sample) != len(template):
        return 0.0
    sample_mean = sum(sample) / len(sample)
    template_mean = sum(template) / len(template)
    sample_deviation = math.sqrt(sum((value - sample_mean) ** 2 for value in sample))
    template_deviation = math.sqrt(
        sum((value - template_mean) ** 2 for value in template)
    )
    if not sample_deviation or not template_deviation:
        return 0.0
    covariance = sum(
        (left - sample_mean) * (right - template_mean)
        for left, right in zip(sample, template)
    )
    return covariance / (sample_deviation * template_deviation)


def rgb_triplets(raw: bytes) -> Iterable[Pixel]:
    """Unpack packed RGB bytes; a short read is an error, not a low score."""
    if len(raw) % 3:
        raise ValueError(f"RGB memory read has {len(raw)} bytes, want a multiple of 3")
    return zip(raw[0::3], raw[1::3], raw[2::3])


def pixels_of(frame: Image.Image) -> list[object]:
    """Read pixels across Pillow versions without the deprecated getdata."""
    data = frame.get_flattened_data() if hasattr(frame, "get_flattened_data") else frame.getdata()
    return list(data)


def _luma(pixel: object) -> int:
    if isinstance(pixel, int):
        return pixel
    red, green, blue = pixel[:3]
    return int(0.299 * red + 0.587 * green + 0.114 * blue)


class FrameAdapter(Protocol):
    def score(self, popup: PopupDefinition) -> float:
        """Return this popup's detector score for the wrapped frame."""
        ...


class ImageFrameAdapter:
    """Window-driver PNG capture (the planet_popup.py path)."""

    def __init__(self, frame: Image.Image, template: bytes | None = None) -> None:
        self._rgb = frame.convert("RGB")
        self._gray = frame.convert("L")
        self._template = template

    @classmethod
    def from_path(cls, path: Path, template: bytes | None = None) -> ImageFrameAdapter:
        with Image.open(path) as image:
            return cls(image.copy(), template)

    def score(self, popup: PopupDefinition) -> float:
        if popup.detector is Detector.BLUE_BODY:
            return blue_fraction(pixels_of(self._rgb.crop(popup.box)))
        samples = pixels_of(self._gray.crop(popup.box))
        return _template_score(samples, self._template)


class ArrayFrameAdapter:
    """The ndarray frame held by `wine-scene.Frames.last`."""

    def __init__(self, frame: object, template: bytes | None = None) -> None:
        self._frame = frame
        self._template = template

    def score(self, popup: PopupDefinition) -> float:
        left, top, right, bottom = popup.box
        crop = self._frame[top:bottom, left:right]
        if popup.detector is Detector.BLUE_BODY:
            return blue_fraction(tuple(pixel[:3]) for row in crop for pixel in row)
        samples = [_luma(pixel) for row in crop for pixel in row]
        return _template_score(samples, self._template)


class WinedbgMemoryAdapter:
    """Stopped-state framebuffer bytes read under winedbg.

    `read_bytes(box)` returns packed RGB for a blue-body popup, or one
    grayscale byte per pixel for the X-plate NCC template. The caller owns
    the debugger PTY and the one-Ctrl-C stop discipline
    (`docs/wine-boot.md`); this adapter only interprets the bytes.
    """

    def __init__(
        self,
        read_bytes: Callable[[Box], bytes],
        template: bytes | None = None,
    ) -> None:
        self._read_bytes = read_bytes
        self._template = template

    def score(self, popup: PopupDefinition) -> float:
        raw = self._read_bytes(popup.box)
        if popup.detector is Detector.BLUE_BODY:
            return blue_fraction(rgb_triplets(raw))
        return _template_score(list(raw), self._template)


def _template_score(samples: Sequence[int], template: bytes | None) -> float:
    if template is None:
        raise ValueError("X-plate NCC needs a template; none was supplied")
    return normalized_correlation(samples, template)


@dataclass(frozen=True)
class PopupState:
    is_up: bool
    score: float


def query_popup(popup: PopupDefinition, adapter: FrameAdapter) -> PopupState:
    """The one popup-state query: score the frame against the definition."""
    score = adapter.score(popup)
    return PopupState(is_up=score > popup.floor, score=score)


def popup_is_up(popup: PopupDefinition, adapter: FrameAdapter) -> bool:
    return query_popup(popup, adapter).is_up


AIM_LINT_SUFFIXES = {".py", ".sh", ".rs"}
AIM_LINT_SKIP_DIRS = {"tests", "__pycache__", ".git"}
_AIM_LINT_CLICK = re.compile(r"\bclick_at\s+\d{2,4}\s+\d{2,4}\b")


def lint_aims(root: Path) -> list[str]:
    """Find popup aims copied into rig code instead of read from definitions."""
    definition = Path(__file__).resolve()
    violations: list[str] = []
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.suffix not in AIM_LINT_SUFFIXES:
            continue
        if any(part in AIM_LINT_SKIP_DIRS for part in path.parts):
            continue
        if path.resolve() == definition:
            continue
        text = path.read_text(errors="replace")
        for key, popup_aim in POPUP_AIMS.items():
            literal = rf"(?<!\d){popup_aim.x}[,\s]+{popup_aim.y}(?!\d)"
            for match in re.finditer(literal, text):
                number = text.count("\n", 0, match.start()) + 1
                violations.append(
                    f"{path}:{number}: hardcoded {key} aim "
                    f"({popup_aim.x}, {popup_aim.y})"
                )
        for number, line in enumerate(text.splitlines(), 1):
            if _AIM_LINT_CLICK.search(line):
                violations.append(f"{path}:{number}: hardcoded click_at aim")
    return violations


def print_aim(key: str) -> int:
    popup_aim = aim(key)
    print(f"{popup_aim.x} {popup_aim.y}")
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    words = list(sys.argv[1:] if argv is None else argv)
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check", help="print up/down for a PNG frame")
    check.add_argument("frame", type=Path)
    check.add_argument("--popup", default=NORMAL_MODE_NOTICE.key, choices=POPUPS)
    aim_command = commands.add_parser("aim", help="print one aim as 'x y'")
    aim_command.add_argument("key", choices=POPUP_AIMS)
    lint = commands.add_parser("lint-aims", help="fail on hardcoded popup aims")
    lint.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parent)
    args = parser.parse_args(words)

    if args.command == "check":
        adapter = ImageFrameAdapter.from_path(args.frame)
        print("up" if popup_is_up(POPUPS[args.popup], adapter) else "down")
        return 0
    if args.command == "aim":
        return print_aim(args.key)

    violations = lint_aims(args.root)
    for violation in violations:
        print(violation, file=sys.stderr)
    if violations:
        print(
            f"popup-state: {len(violations)} hardcoded aim(s); "
            "read them from lib/popup_state.py",
            file=sys.stderr,
        )
        return 1
    print("popup-state: no hardcoded aims")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
