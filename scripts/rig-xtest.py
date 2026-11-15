#!/usr/bin/env python3
"""XTEST input backend for the rig driver (python-xlib).

Drives a bare Xvfb display through XTEST. Clicks work on an unfocused
window, but keys do not, so every `key` command runs a focus step first.
"""
import argparse
import sys
import time

from Xlib import X, display, XK
from Xlib.ext import xtest


def connect(display_name):
    d = display.Display(display_name)
    if d is None:
        raise SystemExit("rig-xtest: cannot open display %s" % display_name)
    return d


def find_window(d, needle):
    root = d.screen().root
    stack = [root]
    while stack:
        w = stack.pop()
        try:
            name = w.get_wm_name()
        except Exception:
            name = None
        if isinstance(name, bytes):
            name = name.decode("utf-8", "replace")
        if name and needle.lower() in str(name).lower():
            return w
        try:
            stack.extend(w.query_tree().children)
        except Exception:
            pass
    return None


def cmd_focus(d, needle):
    w = find_window(d, needle)
    if w is None:
        raise SystemExit("rig-xtest: no window matching %r" % needle)
    d.set_input_focus(w.id, X.RevertToPointerRoot, X.CurrentTime)
    d.sync()
    print("rig-xtest: focused %s (%s)" % (w.get_wm_name(), hex(w.id)))
    return 0


def cmd_move(d, x, y):
    root = d.screen().root
    root.warp_pointer(int(x), int(y))
    d.sync()
    xtest.fake_input(d, X.MotionNotify, 0, 0, root.id, int(x), int(y))
    d.sync()
    print("rig-xtest: moved to %s,%s" % (x, y))
    return 0


def resolve_button(value):
    value = value.strip()
    if value.lower().startswith("0x"):
        return int(value, 16)
    try:
        return int(value)
    except ValueError:
        names = {"left": 1, "middle": 2, "right": 3}
        return names.get(value.lower(), 1)


def cmd_click(d, button, x=None, y=None):
    root = d.screen().root
    if x is not None and y is not None:
        root.warp_pointer(int(x), int(y))
        d.sync()
    btn = resolve_button(button)
    xtest.fake_input(d, X.ButtonPress, btn, 0, root.id, int(x or 0), int(y or 0))
    d.sync()
    time.sleep(0.05)
    xtest.fake_input(d, X.ButtonRelease, btn, 0, root.id, int(x or 0), int(y or 0))
    d.sync()
    print("rig-xtest: clicked %s" % button)
    return 0


def resolve_keycode(d, name):
    name = name.strip()
    if name.isdigit():
        return int(name)
    ks = XK.string_to_keysym(name)
    if ks == 0:
        # try capitalised single char
        if len(name) == 1:
            ks = XK.string_to_keysym(name)
        if ks == 0:
            raise SystemExit("rig-xtest: unknown key %r" % name)
    kc = d.keysym_to_keycode(ks)
    if kc == 0:
        raise SystemExit("rig-xtest: no keycode for key %r" % name)
    return kc


def cmd_key(d, specs, focus_needle):
    # focus step is mandatory on bare Xvfb before key delivery
    w = find_window(d, focus_needle)
    if w is None:
        raise SystemExit("rig-xtest: no window matching %r" % focus_needle)
    d.set_input_focus(w.id, X.RevertToPointerRoot, X.CurrentTime)
    d.sync()
    print("rig-xtest: focused %s for keys" % (w.get_wm_name(),))
    root = d.screen().root
    for spec in specs:
        state = None
        code = spec
        if ":" in spec:
            code, state = spec.split(":", 1)
        kc = resolve_keycode(d, code)
        if state is None or state == "1" or state.lower() in ("down", "press"):
            xtest.fake_input(d, X.KeyPress, kc, 0, root.id, 0, 0)
            d.sync()
            print("rig-xtest: key %s down" % spec)
        if state is None or state == "0" or state.lower() in ("up", "release"):
            if state is None:
                time.sleep(0.05)
            xtest.fake_input(d, X.KeyRelease, kc, 0, root.id, 0, 0)
            d.sync()
            print("rig-xtest: key %s up" % spec)
    return 0


def main():
    parser = argparse.ArgumentParser(prog="rig-xtest")
    parser.add_argument("--display", default=None)
    parser.add_argument("command")
    parser.add_argument("args", nargs="*")
    opts = parser.parse_args()

    display_name = opts.display
    if not display_name:
        import os
        display_name = os.environ.get("DISPLAY")
    if not display_name:
        raise SystemExit("rig-xtest: DISPLAY is not set")

    d = connect(display_name)

    cmd = opts.command
    args = opts.args
    if cmd == "wait-device":
        # any successful connect is readiness
        print("rig-xtest: device ready on %s" % display_name)
        return 0
    if cmd == "focus":
        needle = args[0] if args else "recomp2jam"
        return cmd_focus(d, needle)
    if cmd == "move":
        if len(args) < 2:
            raise SystemExit("usage: move <x> <y>")
        return cmd_move(d, args[0], args[1])
    if cmd == "click":
        btn = args[0] if args else "1"
        x = args[1] if len(args) > 1 else None
        y = args[2] if len(args) > 2 else None
        return cmd_click(d, btn, x, y)
    if cmd == "key":
        if not args:
            raise SystemExit("usage: key <code[:state]>...")
        return cmd_key(d, args, "recomp2jam")
    raise SystemExit("rig-xtest: unknown command %r" % cmd)


if __name__ == "__main__":
    sys.exit(main())
