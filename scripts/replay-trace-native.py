#!/usr/bin/env python3
"""Replay a recomp2jam-input-v1 trace through the native (Rust) client.

Native-side counterpart of scripts/replay-trace-exe.py: the same trace
file drives the recomp2jam binary running under Xvfb. Every press and
release is delivered through XTEST (never through the snapshot seam,
which is reserved for in-process tests). The driver logs accepted and
applied events in sequence (applied.log plus manifest.json) so two runs
of the same trace can be compared on the applied-event log.

Containment: a private Xvfb display, WAYLAND_DISPLAY/WAYLAND_SOCKET
stripped on every boot, WINIT_UNIX_BACKEND=x11, null audio throughout,
and only the child processes this script starts are ever terminated.
"""

import argparse
import json
import os
import socket
import subprocess
import sys
import time
from pathlib import Path

SCHEMA_VERSION = "recomp2jam-input-v1"
KNOWN_SCREENS = {
    "avi", "planet", "channel", "room", "waiting", "loading",
    "playing", "result", "shop", "musicshop", "myroom", "lanking",
}
MENU_KEYS = {
    "menu-confirm": "Return",
    "menu-back": "Escape",
    "menu-up": "Up",
    "menu-down": "Down",
}
LANE_KEYS = ["s", "d", "f", "space", "j", "k", "l"]
KEY_NAME = {
    "menu-confirm": "return",
    "menu-back": "escape",
    "menu-up": "up",
    "menu-down": "down",
}


def intended_ms(event):
    if "time_ms" in event and event.get("time_ms") is not None:
        return int(event["time_ms"])
    time_us = event.get("time_us")
    if time_us is not None:
        return int(time_us) // 1000
    return 0


def validate(trace):
    errors = []
    if trace.get("schema_version") != SCHEMA_VERSION:
        errors.append(f"schema_version != {SCHEMA_VERSION!r}")
    events = trace.get("events") or []
    if not events:
        errors.append("no events")
    held = set()
    last_seq = last_time = None
    for event in events:
        seq = event.get("sequence")
        control = event.get("control")
        action = event.get("action")
        time_ms = intended_ms(event)
        if last_seq is not None and seq <= last_seq:
            errors.append(f"event {seq}: sequence not increasing")
        last_seq = seq
        if last_time is not None and time_ms < last_time:
            errors.append(f"event {seq}: time goes backwards")
        last_time = time_ms
        at = f"event {seq} ({control}/{action})"
        if control in MENU_KEYS or (isinstance(control, str) and control.startswith("lane-")):
            if control.startswith("lane-"):
                try:
                    lane = int(control[5:])
                except (TypeError, ValueError):
                    lane = 99
                if not (0 <= lane <= 6):
                    errors.append(f"{at}: lane out of 0..=6")
            if action == "press":
                if control in held:
                    errors.append(f"{at}: double press")
                held.add(control)
            elif action == "release":
                if control not in held:
                    errors.append(f"{at}: release without press")
                held.discard(control)
            else:
                errors.append(f"{at}: unknown action {action!r}")
        elif control == "click":
            if action not in ("press", "release"):
                errors.append(f"{at}: click presses or releases")
            x, y = event.get("x"), event.get("y")
            if x is None or y is None or not (0 <= x < 800 and 0 <= y < 600):
                errors.append(f"{at}: click outside 800x600")
        elif control == "text":
            if action != "type" or not event.get("text"):
                errors.append(f"{at}: text needs action type + text")
            text = event.get("text", "")
            if not text.isascii() or not text.isprintable():
                errors.append(f"{at}: text must be printable ASCII")
        elif control == "await-screen":
            if action != "reach" or event.get("screen") not in KNOWN_SCREENS:
                errors.append(f"{at}: unknown screen {event.get('screen')!r}")
        else:
            errors.append(f"{at}: unknown control {control!r}")
    for control in held:
        errors.append(f"control {control!r} never released")
    return errors


class CaptureLog:
    def __init__(self, path):
        self.path = path
        self.offset = 0

    def lines(self):
        if not self.path.exists():
            return []
        with self.path.open("r", errors="replace") as handle:
            handle.seek(self.offset)
            fresh = handle.readlines()
            self.offset += sum(len(line.encode("utf-8", "replace")) for line in fresh)
        return [line.rstrip("\n") for line in fresh]

    def current_screen(self):
        screen = None
        for line in self.lines():
            if line.startswith("frame "):
                for field in line.split():
                    if field.startswith("screen="):
                        screen = field[len("screen="):]
        return screen


def wait_for(condition, timeout_s, what):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        value = condition()
        if value:
            return value
        time.sleep(0.25)
    raise TimeoutError(f"timed out after {timeout_s:.0f}s waiting for {what}")


def find_window(display):
    root = display.screen().root
    for window in root.query_tree().children:
        name = ""
        try:
            name = window.get_wm_name() or ""
        except Exception:
            pass
        geometry = window.get_geometry()
        if name == "recomp2jam" or (geometry.width == 800 and geometry.height == 600):
            return window, geometry
    return None, None


def keysym_keycode(display, keysym_name):
    from Xlib import X, XK

    keysym = XK.string_to_keysym(keysym_name)
    if keysym == X.NoSymbol:
        raise ValueError(f"no keysym for {keysym_name!r}")
    return display.keysym_to_keycode(keysym)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace", required=True, help="trace JSON path")
    parser.add_argument("--binary", default="", help="recomp2jam binary path")
    parser.add_argument("--out", required=True, help="evidence output dir")
    parser.add_argument("--display", default=":97")
    parser.add_argument("--user-id", type=int, default=None, help="gateway account id (omit for offline)")
    parser.add_argument("--token", default="", help="gateway session token (omit for offline)")
    parser.add_argument("--gateway", default="", help="gateway host:port (omit for offline)")
    parser.add_argument("--gateway-id", type=int, default=0)
    parser.add_argument("--channel-id", type=int, default=0)
    parser.add_argument("--music-id", type=int, default=757)
    parser.add_argument("--ready-delay", type=float, default=8.0)
    parser.add_argument("--await-timeout", type=float, default=120.0)
    parser.add_argument("--no-sound", action="store_true")
    args = parser.parse_args()

    trace_path = Path(args.trace)
    trace = json.loads(trace_path.read_text())
    errors = validate(trace)
    if errors:
        for error in errors:
            print(f"trace invalid: {error}", file=sys.stderr)
        return 2

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    binary = Path(args.binary) if args.binary else None
    if binary is None:
        for candidate in (Path("target/release/recomp2jam"), Path("target/debug/recomp2jam")):
            if candidate.is_file():
                binary = candidate
                break
    if binary is None or not binary.is_file():
        print("binary missing; cargo build -p recomp2jam first", file=sys.stderr)
        return 2

    online = bool(args.gateway and args.user_id is not None)
    token = args.token
    if online and not token:
        db_path = Path.home() / "memoryer-local" / "O2JAM.db"
        if db_path.is_file():
            import sqlite3
            conn = sqlite3.connect(str(db_path))
            row = conn.execute(
                "select TUSER_ID from t_o2jam_login where USER_INDEX_ID=? "
                "order by LOGIN_TIME desc limit 1", (str(args.user_id),)
            ).fetchone()
            if row:
                token = row[0]
        if not token:
            print("token missing and not found in Memoryer DB", file=sys.stderr)
            return 2
    if online:
        gateway_host, _, gateway_port = args.gateway.partition(":")
        gateway_port = int(gateway_port or 15010)

    env = dict(os.environ)
    env.pop("WAYLAND_DISPLAY", None)
    env.pop("WAYLAND_SOCKET", None)
    env["WINIT_UNIX_BACKEND"] = "x11"
    env["DISPLAY"] = args.display

    lock = Path(f"/tmp/.X{args.display[1:]}-lock")
    started_xvfb = not lock.exists()
    xvfb = None
    if started_xvfb:
        xvfb = subprocess.Popen(
            ["Xvfb", args.display, "-screen", "0", "800x600x24"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env,
        )
        time.sleep(0.7)

    input_log = out / "input.log"
    frames_dir = out / "frames"
    frames_dir.mkdir(parents=True, exist_ok=True)
    client_log = (out / "client.log").open("w")
    if online:
        command = [
            str(binary.resolve()),
            "online",
            "--host", gateway_host,
            "--port", str(gateway_port),
            "--user-id", str(args.user_id),
            "--token", token,
            "--gateway-id", str(args.gateway_id),
            "--channel-id", str(args.channel_id),
        ]
        if args.music_id is not None:
            command += ["--music-id", str(args.music_id)]
        command += [
            "--no-sound",
            "--frame-dump", str(frames_dir),
            "--input-log", str(input_log),
        ]
    else:
        command = [
            str(binary.resolve()),
            "play",
            "--window",
            "--no-sound",
            "--frame-dump", str(frames_dir),
            "--input-log", str(input_log),
        ]
    client = subprocess.Popen(command, stdout=client_log, stderr=client_log, env=env)

    applied = []
    applied_log = out / "applied.log"
    applied_log.write_text("")
    manifest = {
        "trace": str(trace_path.resolve()),
        "binary": str(binary.resolve()),
        "display": args.display,
        "started_at": time.time(),
    }
    verdict = 0
    try:
        # Wait for the window to map and the first frame to land.
        from Xlib import X, display as xdisplay
        from Xlib.ext import xtest

        display = xdisplay.Display(args.display)

        def first_frame_seen():
            if not input_log.exists():
                return False
            return "screen=" in input_log.read_text(errors="replace")

        wait_for(first_frame_seen, 60, "the first painted frame")

        def window_appeared():
            found = find_window(display)
            return found if found[0] is not None else False

        window, geometry = wait_for(window_appeared, 30, "the recomp2jam window")
        display.set_input_focus(window, X.RevertToParent, X.CurrentTime)
        display.sync()
        xtest.fake_input(display, X.MotionNotify, x=geometry.x + 400, y=geometry.y + 300)
        display.sync()
        time.sleep(0.2)

        log = CaptureLog(input_log)
        log.lines()

        # Wait until the client reaches the playing scene before
        # injecting gameplay (lane) events; menu-only traces inject
        # at whatever scene the client is in.
        is_gameplay = any(
            isinstance(e.get("control"), str) and e["control"].startswith("lane-")
            for e in trace["events"]
        )
        if is_gameplay:
            def playing_reached():
                return log.current_screen() == "playing"
            try:
                wait_for(playing_reached, min(15.0, args.await_timeout), "the playing screen")
            except TimeoutError:
                pass

        prev_time_ms = 0
        last_applied_wall = time.time()

        def inject_key(keysym_name, down):
            keycode = keysym_keycode(display, keysym_name)
            xtest.fake_input(display, X.KeyPress if down else X.KeyRelease, keycode)
            display.sync()

        def inject_click(x, y):
            xtest.fake_input(display, X.MotionNotify, x=geometry.x + x, y=geometry.y + y)
            display.sync()
            time.sleep(0.05)
            xtest.fake_input(display, X.ButtonPress, 1)
            display.sync()
            time.sleep(0.03)
            xtest.fake_input(display, X.ButtonRelease, 1)
            display.sync()

        for event in trace["events"]:
            control = event["control"]
            action = event["action"]
            time_ms = intended_ms(event)
            time_us = event.get("time_us")
            delay = (time_ms - prev_time_ms) / 1000.0 - (time.time() - last_applied_wall)
            if delay > 0:
                time.sleep(delay)
            prev_time_ms = time_ms
            last_applied_wall = time.time()

            if control == "await-screen":
                target = event["screen"]
                def reached():
                    return log.current_screen() == target
                wait_for(reached, args.await_timeout, f"screen {target!r}")
                applied.append({"sequence": event["sequence"], "control": control,
                                "action": action, "time_ms": time_ms, "time_us": time_us})
                with applied_log.open("a") as handle:
                    handle.write(f"seq={event['sequence']} control={control} action={action} "
                                 f"intended_ms={time_ms} status=accepted\n")
                continue

            if control in MENU_KEYS:
                inject_key(MENU_KEYS[control], action == "press")
            elif control.startswith("lane-"):
                lane_idx = int(control[5:])
                key_name = LANE_KEYS[lane_idx]
                inject_key(key_name, action == "press")
            elif control == "click":
                x, y = event["x"], event["y"]
                if action == "press":
                    inject_click(x, y)
                else:
                    # release-only click: motion then button release
                    xtest.fake_input(display, X.MotionNotify, x=geometry.x + x, y=geometry.y + y)
                    display.sync()
                    xtest.fake_input(display, X.ButtonRelease, 1)
                    display.sync()
            elif control == "text":
                for char in event["text"]:
                    if char == " ":
                        inject_key("space", True)
                        inject_key("space", False)
                    else:
                        inject_key(char, True)
                        inject_key(char, False)
                    time.sleep(0.03)
            else:
                raise RuntimeError(f"unhandled control {control!r}")

            applied.append({"sequence": event["sequence"], "control": control,
                            "action": action, "time_ms": time_ms, "time_us": time_us})
            with applied_log.open("a") as handle:
                handle.write(f"seq={event['sequence']} control={control} action={action} "
                             f"intended_ms={time_ms} status=applied\n")

        time.sleep(2.0)
        verdict = 0
    except Exception as source:
        manifest["failure"] = repr(source)
        verdict = 1
    finally:
        client.terminate()
        try:
            client.wait(timeout=10)
        except subprocess.TimeoutExpired:
            client.kill()
        client_log.close()
        manifest["applied_events"] = applied
        manifest["applied_event_log"] = applied_log.read_text().splitlines()
        manifest["finished_at"] = time.time()
        (out / "manifest.json").write_text(json.dumps(manifest, indent=2))
        if xvfb is not None:
            xvfb.terminate()
            try:
                xvfb.wait(timeout=5)
            except subprocess.TimeoutExpired:
                xvfb.kill()

    print(f"manifest: {out / 'manifest.json'}")
    if verdict == 0:
        print(f"applied_events={len(applied)}")
    else:
        print(f"FAILED: {manifest.get('failure')}", file=sys.stderr)
    return verdict


if __name__ == "__main__":
    sys.exit(main())
