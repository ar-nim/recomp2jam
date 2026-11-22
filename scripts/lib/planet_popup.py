#!/usr/bin/env python3
"""CLI consumer of the NORMAL-mode popup query (ticket #747).

The box, blue predicate, floor and OK aim live in `lib/popup_state.py`,
beside the popup definition. This helper keeps the old CLI and the two
compatibility names used by tests and the shell drive; it owns no pixels.
The W6 freeze rule (never blind-Escape a booting client) is why the shell
asks this helper for state before it presses the OK face.
"""

from __future__ import annotations

import sys
from pathlib import Path

import popup_state
from popup_state import NORMAL_MODE_NOTICE, ImageFrameAdapter, query_popup

POPUP_BOX = NORMAL_MODE_NOTICE.box
BLUE_FRACTION_FLOOR = NORMAL_MODE_NOTICE.floor


def blue_fraction(frame: Path) -> float:
    """Return the share of blue pixels inside the message-box body."""
    return ImageFrameAdapter.from_path(frame).score(NORMAL_MODE_NOTICE)


def popup_is_up(frame: Path) -> bool:
    return query_popup(NORMAL_MODE_NOTICE, ImageFrameAdapter.from_path(frame)).is_up


def main() -> int:
    words = sys.argv[1:]
    if words[:1] == ["check"]:
        words = words[1:]
    if words[:1] == ["aim"]:
        return popup_state.main(words)
    return popup_state.main(["check", *words])


if __name__ == "__main__":
    raise SystemExit(main())
