#!/usr/bin/env python3
"""Replay a recomp2jam-input-v1 trace through the real window.

The trace's events are injected as XTEST key/mouse events at the
running windowed binary (winit sees ordinary X11 input — no test-only
API bypass). Sync points (`await-screen`) poll the binary's capture
log, so server round trips never become fixed sleeps. Everything runs
under a private Xvfb display with null audio; full captures stay in
the --out directory (never committed: they may contain game artwork).

Preflight (batch-harness skill): gateway reachable, binary built,
trace valid. Fail fast before any display or server state is touched.
"""

import argparse
import json
import os
import select
import socket
import subprocess
import sys
import threading
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

# Wire frame: [u16 len][u16 id][body...] little-endian, len includes
# the length field itself (crates/o2jam-net/src/frame.rs). The proxy
# forwards raw bytes and scans this layout for the log only.


class WireProxy:
    """Logging TCP proxy: one client, frames recorded with direction."""

    def __init__(self, target_host, target_port):
        self.target = (target_host, target_port)
        self.listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen(1)
        self.port = self.listener.getsockname()[1]
        self.frames = []
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def start(self):
        self._thread.start()

    def _run(self):
        try:
            client, _ = self.listener.accept()
        except OSError:
            return
        try:
            server = socket.create_connection(self.target, timeout=10)
        except OSError:
            client.close()
            return
        sockets = [client, server]
        peers = {client: server, server: client}
        directions = {client: "C>S", server: "S>C"}
        buffers = {client: bytearray(), server: bytearray()}
        while sockets and not self._stop.is_set():
            ready, _, _ = select.select(sockets, [], [], 0.5)
            for conn in ready:
                try:
                    data = conn.recv(65_536)
                except OSError:
                    data = b""
                if not data:
                    if conn in sockets:
                        sockets.remove(conn)
                    continue
                try:
                    peers[conn].sendall(data)
                except OSError:
                    continue
                buffers[conn].extend(data)
                while len(buffers[conn]) >= 4:
                    total = int.from_bytes(buffers[conn][0:2], "little")
                    if len(buffers[conn]) < total:
                        break
                    frame_id = int.from_bytes(buffers[conn][2:4], "little")
                    self.frames.append((time.time(), directions[conn], frame_id))
                    del buffers[conn][:total]
        for conn in (client, server):
            conn.close()

    def stop(self):
        self._stop.set()
        try:
            self.listener.close()
        except OSError:
            pass


def validate(trace):
    """Mirror of recomp2jam validate-trace: reject before any replay."""
    errors = []
    if trace.get("schema_version") != SCHEMA_VERSION:
        errors.append(f"schema_version != {SCHEMA_VERSION!r}")
    events = trace.get("events") or []
    if not events:
        errors.append("no events")
    held = set()
    last_seq = last_time = None
    for event in events:
        seq, control, action = event.get("sequence"), event.get("control"), event.get("action")
        time_ms = event.get("time_ms")
        if last_seq is not None and seq <= last_seq:
            errors.append(f"event {seq}: sequence not increasing")
        last_seq = seq
        if last_time is not None and time_ms < last_time:
            errors.append(f"event {seq}: time goes backwards")
        last_time = time_ms
        at = f"event {seq} ({control}/{action})"
        if control in MENU_KEYS or (isinstance(control, str) and control.startswith("lane-")):
            if control.startswith("lane-") and not (0 <= int(control[5:]) <= 6):
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
            if action != "press":
                errors.append(f"{at}: click only presses")
            x, y = event.get("x"), event.get("y")
            if x is None or y is None or not (0 <= x < 800 and 0 <= y < 600):
                errors.append(f"{at}: click outside 800x600")
        elif control == "text":
            if action != "type" or not event.get("text"):
                errors.append(f"{at}: text needs action type + text")
            elif not event["text"].isascii() or not event["text"].isprintable():
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
    """Incremental reader for the binary's applied-event log."""

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
    from Xlib import display as xdisplay

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
    parser.add_argument("--token", required=True, help="gateway session token")
    parser.add_argument("--user-id", type=int, default=1)
    parser.add_argument("--gateway", default="127.0.0.1:15010")
    parser.add_argument("--gateway-id", type=int, default=0)
    parser.add_argument("--channel-id", type=int, default=0)
    parser.add_argument("--binary", default="target/release/recomp2jam")
    parser.add_argument("--out", required=True, help="evidence output dir")
    parser.add_argument("--create-room", default="U1 Trace")
    parser.add_argument("--music-id", type=int, default=757)
    parser.add_argument("--fixed-clock", type=int, default=25)
    parser.add_argument("--display", default=":97")
    parser.add_argument("--await-timeout", type=float, default=600.0,
                        help="per-await-screen wall timeout (the playing segment "
                             "runs at fixed-clock pace; debug builds are ~4x slower)")
    args = parser.parse_args()

    trace = json.loads(Path(args.trace).read_text())
    errors = validate(trace)
    if errors:
        for error in errors:
            print(f"trace invalid: {error}", file=sys.stderr)
        return 2

    gateway_host, _, gateway_port = args.gateway.partition(":")
    gateway_port = int(gateway_port or 15010)

    # Preflight: gateway reachable, binary built.
    try:
        socket.create_connection((gateway_host, gateway_port), timeout=3).close()
    except OSError as source:
        print(f"preflight failed: gateway {args.gateway} unreachable ({source})", file=sys.stderr)
        return 2
    binary = Path(args.binary)
    if not binary.is_file():
        print(f"preflight failed: binary {binary} missing (cargo build -p recomp2jam)", file=sys.stderr)
        return 2

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    proxy = WireProxy(gateway_host, gateway_port)
    proxy.start()

    # Private Xvfb display (never the user's desktop). The child env
    # drops Wayland and pins winit to X11 — without this a
    # WAYLAND_DISPLAY inherited from the caller puts the window on the
    # user's real session (containment breach, run 3 of U1).
    display_arg = args.display
    env = dict(os.environ, DISPLAY=display_arg)
    env.pop("WAYLAND_DISPLAY", None)
    env.pop("WAYLAND_SOCKET", None)
    env["WINIT_UNIX_BACKEND"] = "x11"

    lock = Path(f"/tmp/.X{display_arg[1:]}-lock")
    started_xvfb = lock.exists() is False
    xvfb = None
    if started_xvfb:
        xvfb = subprocess.Popen(
            ["Xvfb", display_arg, "-screen", "0", "800x600x24"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env,
        )
        time.sleep(0.7)

    client_log = (out / "client.log").open("w")
    input_log = out / "input.log"
    frames_dir = out / "frames"
    command = [
        str(binary.resolve()),
        "online",
        "--host", "127.0.0.1",
        "--port", str(proxy.port),
        "--user-id", str(args.user_id),
        "--token", args.token,
        "--gateway-id", str(args.gateway_id),
        "--channel-id", str(args.channel_id),
        "--create-room", args.create_room,
        "--music-id", str(args.music_id),
        "--fixed-clock", str(args.fixed_clock),
        "--frame-dump", str(frames_dir),
        "--input-log", str(input_log),
    ]
    client = subprocess.Popen(command, stdout=client_log, stderr=client_log, env=env)

    manifest = {
        "trace": str(Path(args.trace).resolve()),
        "binary": str(binary.resolve()),
        "command": command,
        "display": display_arg,
        "started_at": time.time(),
    }
    applied = []
    try:
        # The first frame line proves the window is up and painting;
        # events injected earlier would land on the root window and die.
        def first_frame_seen():
            if not input_log.exists():
                return False
            return "screen=" in input_log.read_text(errors="replace")

        wait_for(first_frame_seen, 60, "the first painted frame")

        from Xlib import X, XK, display as xdisplay
        from Xlib.ext import xtest

        display = xdisplay.Display(display_arg)

        def window_appeared():
            found = find_window(display)
            return found if found[0] is not None else False

        window, geometry = wait_for(window_appeared, 30, "the recomp2jam window")
        display.set_input_focus(window, X.RevertToParent, X.CurrentTime)
        display.sync()
        xtest.fake_input(display, X.MotionNotify, x=geometry.x + 400, y=geometry.y + 300)
        display.sync()
        time.sleep(0.2)
        manifest["window"] = {"x": geometry.x, "y": geometry.y}

        log = CaptureLog(input_log)
        log.lines()  # catch up past the boot lines
        last_applied_wall = time.time()
        prev_time_ms = 0

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
            control, action = event["control"], event["action"]
            time_ms = event["time_ms"]
            delay = (time_ms - prev_time_ms) / 1000.0 - (time.time() - last_applied_wall)
            if delay > 0:
                time.sleep(delay)
            prev_time_ms = time_ms
            last_applied_wall = time.time()

            if control == "await-screen":
                target = event["screen"]
                wait_for(
                    lambda: log.current_screen() == target or None,
                    args.await_timeout,
                    f"screen {target!r}",
                )
                applied.append({"sequence": event["sequence"], "control": control, "screen": target})
                continue
            if control in MENU_KEYS:
                inject_key(MENU_KEYS[control], action == "press")
            elif control.startswith("lane-"):
                inject_key(LANE_KEYS[int(control[5:])], action == "press")
            elif control == "click":
                inject_click(event["x"], event["y"])
            elif control == "text":
                for char in event["text"]:
                    if char == " ":
                        inject_key("space", True)
                        inject_key("space", False)
                    else:
                        inject_key(char, True)
                        inject_key(char, False)
                    time.sleep(0.03)
            applied.append({"sequence": event["sequence"], "control": control, "action": action})

        time.sleep(2.0)
        all_lines = input_log.read_text(errors="replace").splitlines()
        state_hash = None
        content_hash = None
        final_screen = None
        for line in all_lines:
            if line.startswith("frame "):
                fields = dict(field.split("=", 1) for field in line.split() if "=" in field)
                # A truncated tail line (client killed mid-write) must
                # never clobber values parsed from complete lines.
                if "screen" in fields:
                    final_screen = fields["screen"]
                if "state" in fields:
                    state_hash = fields["state"]
                if "content" in fields:
                    content_hash = fields["content"]
        manifest["final"] = {
            "screen": final_screen,
            "state_hash": state_hash,
            "content_hash": content_hash,
        }
        manifest["applied_events"] = applied
        manifest["applied_event_log"] = [line for line in all_lines if line.startswith("event ")]
        verdict = 0
    except Exception as source:  # evidence first: record, then fail
        manifest["failure"] = repr(source)
        verdict = 1
    finally:
        client.terminate()
        try:
            client.wait(timeout=10)
        except subprocess.TimeoutExpired:
            client.kill()
        client_log.close()
        proxy.stop()
        manifest["wire_frames"] = [
            {"t": round(stamp - manifest["started_at"], 3), "dir": direction, "id": frame_id}
            for stamp, direction, frame_id in proxy.frames
        ]
        captures = sorted(frames_dir.glob("*.png")) if frames_dir.exists() else []
        manifest["captures"] = [path.name for path in captures]
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
        final = manifest.get("final", {})
        print(f"final screen={final.get('screen')} state_hash={final.get('state_hash')}")
        print(f"captures={len(manifest['captures'])} wire_frames={len(manifest['wire_frames'])}")
    else:
        print(f"FAILED: {manifest.get('failure')}", file=sys.stderr)
    return verdict


if __name__ == "__main__":
    sys.exit(main())
