#!/usr/bin/env python3
"""Click through an InstallShield wizard running on a private X display.

The 7.55 installers (game, music) have no silent switch that works under
Wine, so extraction runs `setup.exe /a` on a throwaway display and this
driver advances the wizard.  It presses the middle bottom button
(Next / Finish) whenever the staging tree has stopped growing, and stops
once no wizard window is left.

Why the middle button: every page of the O2Jam wizards lays out
[< Back][Next>][Cancel], so the bottom row at 72% width is Next/Finish and
the Cancel button sits further right.  `--button-fraction` overrides it.

Usage:
  extract-installer-drive.py --display :90 --watch DIR
      [--settle-seconds 120] [--idle-seconds 20] [--max-seconds 3600]

Exit status: 0 when the wizard closed and the tree settled, 2 when
`--max-seconds` elapsed with a wizard still open (blocker).
"""

import argparse
import os
import sys
import time

from Xlib import X, display
from Xlib.ext import xtest


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--display", default=":90")
    parser.add_argument("--watch", required=True, help="staging directory to watch")
    parser.add_argument("--settle-seconds", type=float, default=120.0)
    parser.add_argument("--idle-seconds", type=float, default=20.0)
    parser.add_argument("--max-seconds", type=float, default=3600.0)
    parser.add_argument(
        "--button-offset",
        default="158x22",
        help="Next/Finish button centre as W x H pixels from the window's bottom-right corner",
    )
    parser.add_argument("--poll-seconds", type=float, default=5.0)
    parser.add_argument(
        "--fill-destination",
        default="",
        help="path to type into the installer's destination page (game installer)",
    )
    parser.add_argument(
        "--destination-fraction",
        default="0.5x0.474",
        help="destination field centre as a fraction of the wizard window",
    )
    return parser.parse_args()


def find_wizard(screen):
    """Return the smallest mapped dialog window.

    An InstallShield run shows two X windows: a full-screen backdrop that
    carries "- InstallShield Wizard" in its title, and the real dialog on
    top of it, whose title is the (Korean) product name only. Picking the
    smallest mapped window aims at the dialog, never at the backdrop's
    corner - the mistake that made every click miss.
    """
    candidates = []

    def walk(window, depth=0):
        if depth > 3:
            return
        name = window.get_wm_name()
        geometry = window.get_geometry()
        mapped = window.get_attributes().map_state == X.IsViewable
        if (
            isinstance(name, str)
            and name
            and mapped
            and geometry.width >= 300
            and geometry.height >= 200
        ):
            candidates.append((geometry.width * geometry.height, geometry))
        for child in window.query_tree().children:
            walk(child, depth + 1)

    walk(screen.root)
    if not candidates:
        return None
    return min(candidates, key=lambda candidate: candidate[0])


def tree_size(path):
    """Bytes plus entry count, so a copy that only creates files still counts."""
    total = 0
    entries = 0
    for dirpath, _dirnames, filenames in os.walk(path):
        for filename in filenames:
            try:
                total += os.path.getsize(os.path.join(dirpath, filename))
            except OSError:
                continue
            entries += 1
    return total, entries


def press(display_handle, x, y):
    xtest.fake_input(display_handle, X.MotionNotify, x=x, y=y)
    display_handle.sync()
    time.sleep(0.15)
    xtest.fake_input(display_handle, X.ButtonPress, 1)
    display_handle.sync()
    time.sleep(0.08)
    xtest.fake_input(display_handle, X.ButtonRelease, 1)
    display_handle.sync()
    return x, y


def click_button(display_handle, geometry, offset):
    """Click the Next/Finish button, placed a fixed distance from the corner."""
    dx, dy = (int(part) for part in offset.split("x"))
    return press(display_handle, geometry.x + geometry.width - dx, geometry.y + geometry.height - dy)


def click_destination_field(display_handle, geometry, fraction):
    fx, fy = (float(part) for part in fraction.split("x"))
    return press(
        display_handle,
        geometry.x + int(geometry.width * fx),
        geometry.y + int(geometry.height * fy),
    )


SHIFTED_KEYSYMS = {
    ":": "semicolon",
    "_": "minus",
}
PLAIN_KEYSYMS = {
    "\\": "backslash",
    "/": "slash",
    "-": "minus",
    ".": "period",
    ":": "semicolon",
    "_": "minus",
}


def type_text(display_handle, text):
    from Xlib import XK

    shift_code = display_handle.keysym_to_keycode(XK.XK_Shift_L)
    for character in text:
        needs_shift = character.isupper() or character in SHIFTED_KEYSYMS
        name = SHIFTED_KEYSYMS.get(character) or PLAIN_KEYSYMS.get(character) or character.lower()
        code = display_handle.keysym_to_keycode(XK.string_to_keysym(name))
        if needs_shift:
            xtest.fake_input(display_handle, X.KeyPress, shift_code)
        xtest.fake_input(display_handle, X.KeyPress, code)
        display_handle.sync()
        xtest.fake_input(display_handle, X.KeyRelease, code)
        if needs_shift:
            xtest.fake_input(display_handle, X.KeyRelease, shift_code)
        display_handle.sync()
        time.sleep(0.05)


def main():
    args = parse_args()
    handle = display.Display(args.display)
    screen = handle.screen()
    deadline = time.monotonic() + args.max_seconds
    last_size = None
    last_change = time.monotonic()
    last_seen = time.monotonic()
    destination_filled = False
    seen_pages = 0

    while time.monotonic() < deadline:
        wizard = find_wizard(screen)
        size = tree_size(args.watch)
        now = time.monotonic()
        if size != last_size:
            last_size = size
            last_change = now
        if wizard is None:
            if now - last_seen > args.settle_seconds:
                print(f"drive: wizard closed, staging settled at {size[1]} files")
                return 0
        else:
            last_seen = now
            _area, geometry = wizard
            if now - last_change > args.idle_seconds:
                if args.fill_destination and not destination_filled and seen_pages > 0:
                    # The destination page holds the wizard's only text field;
                    # an empty path leaves Next disabled, so fill it first.
                    click_destination_field(handle, geometry, args.destination_fraction)
                    time.sleep(0.4)
                    type_text(handle, args.fill_destination)
                    destination_filled = True
                    print(f"drive: typed destination {args.fill_destination}")
                else:
                    x, y = click_button(handle, geometry, args.button_offset)
                    print(
                        f"drive: clicked ({x},{y}) after {args.idle_seconds:g}s idle, "
                        f"{size[1]} files"
                    )
                    seen_pages += 1
                last_change = time.monotonic()
        time.sleep(args.poll_seconds)

    print("drive: timeout while a wizard window was still open", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
