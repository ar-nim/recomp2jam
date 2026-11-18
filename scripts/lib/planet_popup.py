#!/usr/bin/env python3
"""Detect the NORMAL-mode message box on a native Planet frame.

The box blocks canvas clicks until its OK face is pressed. Its blue body
occupies the pinned rectangle used by the Wine drive
(``scripts/wine-scene.py``, evidence ``docs/captures/t240/findings.md``).
Only the pixel test lives here; the shell drive owns the click.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from PIL import Image

POPUP_BOX = (300, 240, 500, 340)
BLUE_FRACTION_FLOOR = 0.9


def blue_fraction(frame: Path) -> float:
    """Return the share of blue pixels inside the message-box body."""
    with Image.open(frame) as image:
        body = image.convert("RGB").crop(POPUP_BOX)
        read_pixels = getattr(body, "get_flattened_data", body.getdata)
        pixels = list(read_pixels())
    if not pixels:
        return 0.0
    blue = sum(
        1 for red, green, channel_blue in pixels
        if channel_blue > 120
        and channel_blue > red * 1.25
        and channel_blue > green * 1.15
    )
    return blue / len(pixels)


def popup_is_up(frame: Path) -> bool:
    return blue_fraction(frame) > BLUE_FRACTION_FLOOR


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("frame", type=Path, help="native frame PNG to inspect")
    argv = sys.argv[1:]
    if argv[:1] == ["check"]:
        argv = argv[1:]
    args = parser.parse_args(argv)
    print("up" if popup_is_up(args.frame) else "down")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
