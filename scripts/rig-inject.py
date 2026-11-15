#!/usr/bin/env python3
"""Scheduled XTEST injector for the O2Jam rig.

Delivers lane and menu keys through the XTEST extension and writes one
epoch-stamped record per press to its own record file. The record file is
deliberately never the client's input log: the client rewrites that log,
so injector records stay separate.

Key map (mirrors o2jam-shell lane_key/menu_of):
  lanes 0..6 -> s d f space j k l
  menu-confirm -> Return, menu-back -> Escape, menu-up -> Up,
  menu-down -> Down
"""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

LANE_KEYS = ["s", "d", "f", "space", "j", "k", "l"]
MENU_KEYS = {
    "menu-confirm": "Return",
    "menu-back": "Escape",
    "menu-up": "Up",
    "menu-down": "Down",
}

ALL_KEYS = [f"lane-{i}" for i in range(len(LANE_KEYS))] + list(MENU_KEYS)


def keysym_keycode(display, name):
    from Xlib import X, XK

    keysym = XK.string_to_keysym(name)
    if keysym == X.NoSymbol:
        raise ValueError(f"no keysym for {name!r}")
    code = display.keysym_to_keycode(keysym)
    if not code:
        raise ValueError(f"no keycode for {name!r}")
    return code


def find_window(display):
    from Xlib import X

    root = display.screen().root
    for window in root.query_tree().children:
        name = ""
        try:
            name = window.get_wm_name() or ""
        except Exception:
            pass
        geometry = window.get_geometry()
        if name == "recomp2jam" or (geometry.width in (800, 1024) and geometry.height in (600, 768)):
            return window, geometry
    return None, None


def inject_key(display, xtest, keycode, down):
    from Xlib import X

    xtest.fake_input(display, X.KeyPress if down else X.KeyRelease, keycode)
    display.sync()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--display", default=":99", help="X display to target")
    parser.add_argument("--record", default="", help="injector record file path")
    parser.add_argument("--out", default="", help="output dir; record becomes <out>/injector.log")
    parser.add_argument("--interval-ms", type=int, default=250, help="delay between presses")
    parser.add_argument("--count", type=int, default=1, help="number of full rounds")
    parser.add_argument("--keys", default=",".join(ALL_KEYS), help="comma list of keys to inject")
    parser.add_argument("--no-focus", action="store_true", help="skip the focus step")
    parser.add_argument("--print-keymap", action="store_true", help="print the key map and exit")
    args = parser.parse_args()

    if args.print_keymap:
        for i, key in enumerate(LANE_KEYS):
            print(f"lane {i} -> {key}")
        for name, key in MENU_KEYS.items():
            print(f"{name} -> {key}")
        return 0

    keys = [item.strip() for item in args.keys.split(",") if item.strip()]
    for key in keys:
        if key.startswith("lane-"):
            idx = int(key.split("-", 1)[1])
            if not (0 <= idx < len(LANE_KEYS)):
                print(f"unknown lane key: {key}", file=sys.stderr)
                return 2
        elif key not in MENU_KEYS:
            print(f"unknown key: {key}", file=sys.stderr)
            return 2

    if args.record:
        record_path = Path(args.record)
    elif args.out:
        record_path = Path(args.out) / "injector.log"
    else:
        record_path = Path("/tmp/rig-injector.log")
    record_path.parent.mkdir(parents=True, exist_ok=True)

    try:
        from Xlib import display as xdisplay
        from Xlib.ext import xtest
    except Exception as exc:  # pragma: no cover - environment dependent
        print(f"Xlib unavailable: {exc}", file=sys.stderr)
        return 2

    display = xdisplay.Display(args.display)

    if not args.no_focus:
        window, geometry = find_window(display)
        if window is not None:
            from Xlib import X

            display.set_input_focus(window, X.RevertToParent, X.CurrentTime)
            display.sync()
            xtest.fake_input(display, X.MotionNotify, x=geometry.x + 10, y=geometry.y + 10)
            display.sync()
            time.sleep(0.1)

    keycode_cache = {}

    def keycode_for(key):
        if key in keycode_cache:
            return keycode_cache[key]
        if key.startswith("lane-"):
            idx = int(key.split("-", 1)[1])
            keysym_name = LANE_KEYS[idx]
        else:
            keysym_name = MENU_KEYS[key]
        code = keysym_keycode(display, keysym_name)
        keycode_cache[key] = code
        return code

    with record_path.open("a", encoding="utf-8") as record:
        for _ in range(max(1, args.count)):
            for key in keys:
                epoch_ms = int(time.time() * 1000)
                code = keycode_for(key)
                inject_key(display, xtest, code, True)
                time.sleep(0.03)
                inject_key(display, xtest, code, False)
                display.sync()
                record.write(f"{epoch_ms} {key}\n")
                record.flush()
                time.sleep(max(0, args.interval_ms) / 1000.0)

    display.close()
    print(f"injected {len(keys) * max(1, args.count)} presses; record={record_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
