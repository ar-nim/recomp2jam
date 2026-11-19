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

from __future__ import annotations

import argparse
import json
import os
import sqlite3
import subprocess
import sys
import time
from dataclasses import dataclass
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
DEFAULT_GATEWAY_PORT = 15010
SESSION_DATABASE = Path.home() / "memoryer-local" / "O2JAM.db"
LIVE_LOCK_SCRIPT = Path(__file__).resolve().parent / "lib" / "rig-live-lock.sh"


@dataclass(frozen=True)
class ReplayOptions:
    """Everything needed to launch one client capture."""

    binary: Path
    online: bool
    host: str
    port: int
    user_id: int | None
    token: str
    gateway_id: int
    channel_id: int
    music_id: int | None
    frames_dir: Path
    input_log: Path
    silent: bool = True


def parse_gateway(endpoint: str) -> tuple[str, int]:
    """Split a host:port endpoint, defaulting an omitted port."""
    host, separator, port = endpoint.partition(":")
    return host, int(port) if separator and port else DEFAULT_GATEWAY_PORT


def resolve_binary(requested: str) -> Path:
    """Find the release client, then the debug client, unless told otherwise."""
    candidates = [Path(requested)] if requested else [
        Path("target/release/recomp2jam"),
        Path("target/debug/recomp2jam"),
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError(f"binary missing: {candidates[0]} (cargo build -p recomp2jam)")


def read_session_token(user_id: int, database: Path = SESSION_DATABASE) -> str:
    """Read the newest session token; the value never enters an artifact."""
    if not database.is_file():
        raise FileNotFoundError(f"token database missing: {database}")
    with sqlite3.connect(str(database)) as connection:
        row = connection.execute(
            "select TUSER_ID from t_o2jam_login where USER_INDEX_ID=? "
            "order by LOGIN_TIME desc limit 1", (str(user_id),),
        ).fetchone()
    if row is None:
        raise LookupError(f"no session token for user {user_id}")
    return str(row[0])


def child_environment(display: str) -> dict[str, str]:
    """Keep the capture on a private X11 display, never on the owner's seat."""
    env = dict(os.environ)
    env.pop("WAYLAND_DISPLAY", None)
    env.pop("WAYLAND_SOCKET", None)
    env["WINIT_UNIX_BACKEND"] = "x11"
    env["DISPLAY"] = display
    return env


def build_client_command(options: ReplayOptions) -> list[str]:
    """Build the client argv; both arms use null sound and native frame dumps."""
    command = [str(options.binary.resolve())]
    if options.online:
        command += [
            "online", "--host", options.host, "--port", str(options.port),
            "--user-id", str(options.user_id), "--token", options.token,
            "--gateway-id", str(options.gateway_id),
            "--channel-id", str(options.channel_id),
        ]
        if options.music_id is not None:
            command += ["--music-id", str(options.music_id)]
    else:
        command += ["play", "--window"]
    # A rig-owned boot routes audio to its own null sink; that boot
    # omits --no-sound so the lane can prove route=sink. Every driver-
    # owned boot stays silent by construction (#283 fence).
    if options.silent:
        command.append("--no-sound")
    command += [
        "--frame-dump", str(options.frames_dir),
        "--input-log", str(options.input_log),
    ]
    return command


def wrap_in_live_lock(command: list[str], display: str) -> list[str]:
    """Hold the shared client lane until this driver's client exits."""
    owner = f"replay-trace-native:{display.lstrip(':')}:pid={os.getpid()}"
    return [str(LIVE_LOCK_SCRIPT), "run", "--owner", owner, "--", *command]


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
        elif control == "pause-ms":
            if action != "wait" or not isinstance(event.get("ms"), int):
                errors.append(f"{at}: pause needs action wait and integer ms")
        elif control in ("state-shot", "await-file"):
            name = event.get("name", "")
            if not isinstance(name, str) or not name.replace("-", "").isalnum():
                errors.append(f"{at}: {control} needs a simple name")
            wanted = "capture" if control == "state-shot" else "exists"
            if action != wanted:
                errors.append(f"{at}: {control} needs action {wanted!r}")
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


def inject_key(display, xtest, keysym_name, down):
    """Send one real X11 key edge to the window."""
    from Xlib import X

    code = keysym_keycode(display, keysym_name)
    event = X.KeyPress if down else X.KeyRelease
    xtest.fake_input(display, event, code)
    display.sync()


def inject_click(display, xtest, geometry, x, y):
    """Send a native-frame-coordinate press through XTEST."""
    from Xlib import X

    xtest.fake_input(display, X.MotionNotify, x=geometry.x + x, y=geometry.y + y)
    display.sync()
    time.sleep(0.05)
    xtest.fake_input(display, X.ButtonPress, 1)
    display.sync()
    time.sleep(0.03)
    xtest.fake_input(display, X.ButtonRelease, 1)
    display.sync()


def inject_click_release(display, xtest, geometry, x, y):
    """Release a previously held mouse edge without re-pressing."""
    from Xlib import X

    xtest.fake_input(display, X.MotionNotify, x=geometry.x + x, y=geometry.y + y)
    display.sync()
    xtest.fake_input(display, X.ButtonRelease, 1)
    display.sync()


def inject_text(display, xtest, text):
    """Type printable ASCII as real key taps."""
    for character in text:
        key = "space" if character == " " else character
        inject_key(display, xtest, key, True)
        inject_key(display, xtest, key, False)
        time.sleep(0.03)


def injection_delay(event_time_ms, previous_time_ms, elapsed_wall_s):
    """Wait only the remaining trace time, never rewind the timeline."""
    return max(0.0, (event_time_ms - previous_time_ms) / 1000.0 - elapsed_wall_s)


def trace_has_lane_events(trace):
    """Report whether the trace needs the playing-scene prelude."""
    return any(
        isinstance(event.get("control"), str) and event["control"].startswith("lane-")
        for event in trace["events"]
    )


def capture_state_shot(display, geometry, path: Path) -> None:
    """Save one native 800x600 framebuffer at a named scenario state."""
    path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
            "-f", "x11grab", "-video_size", "800x600",
            "-i", f"{display}+{geometry.x},{geometry.y}",
            "-frames:v", "1", str(path),
        ],
        check=True,
        capture_output=True,
    )


def record_applied_event(applied, applied_log, event, status):
    """Append one applied-event record and its plain-text log line."""
    entry = {
        "sequence": event["sequence"],
        "control": event["control"],
        "action": event["action"],
        "time_ms": intended_ms(event),
        "time_us": event.get("time_us"),
    }
    if "name" in event:
        entry["name"] = event["name"]
    applied.append(entry)
    with applied_log.open("a") as handle:
        suffix = f"name={entry['name']} " if "name" in entry else ""
        handle.write(
            f"seq={entry['sequence']} control={entry['control']} "
            f"action={entry['action']} intended_ms={entry['time_ms']} "
            f"{suffix}status={status}\n"
        )
    return entry


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
    parser.add_argument(
        "--no-sound", action="store_true",
        help="accepted for CLI compatibility; driver-owned boots are silent",
    )
    parser.add_argument(
        "--sound", action="store_true",
        help="let a rig-owned boot open its per-run null sink instead of --no-sound",
    )
    parser.add_argument(
        "--attach-rig", action="store_true",
        help="attach to a client booted by rig-run.sh; never spawn or stop one",
    )
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
    input_log = out / "input.log"
    frames_dir = out / "frames"
    frames_dir.mkdir(parents=True, exist_ok=True)

    try:
        binary = resolve_binary(args.binary)
    except FileNotFoundError as source:
        print(f"binary missing: {source}", file=sys.stderr)
        return 2

    online = bool(args.gateway and args.user_id is not None)
    try:
        token = args.token or (read_session_token(args.user_id) if online else "")
    except (FileNotFoundError, LookupError) as source:
        print(f"token lookup failed: {source}", file=sys.stderr)
        return 2
    host, port = parse_gateway(args.gateway) if online else ("", 0)
    options = ReplayOptions(
        binary=binary,
        online=online,
        host=host,
        port=port,
        user_id=args.user_id,
        token=token,
        gateway_id=args.gateway_id,
        channel_id=args.channel_id,
        music_id=args.music_id,
        frames_dir=frames_dir,
        input_log=input_log,
        silent=not args.sound,
    )
    env = child_environment(args.display)
    attached = args.attach_rig
    if attached and not (out / "env.sh").is_file():
        print(f"rig env missing: {out / 'env.sh'}", file=sys.stderr)
        return 2

    lock = Path(f"/tmp/.X{args.display[1:]}-lock")
    started_xvfb = not attached and not lock.exists()
    xvfb = None
    if started_xvfb:
        xvfb = subprocess.Popen(
            ["Xvfb", args.display, "-screen", "0", "800x600x24"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env,
        )
        time.sleep(0.7)

    command = build_client_command(options)
    client_log = None
    client = None
    if not attached:
        client_log = (out / "client.log").open("w")
        client = subprocess.Popen(
            wrap_in_live_lock(command, args.display),
            stdout=client_log,
            stderr=client_log,
            env=env,
        )

    applied = []
    applied_log = out / "applied.log"
    applied_log.write_text("")
    manifest = {
        "trace": str(trace_path.resolve()),
        "binary": str(binary.resolve()),
        "command": command,
        "display": args.display,
        "attached_rig": attached,
        "live_lock": "inherited from rig-run.sh" if attached else f"replay-trace-native:{args.display.lstrip(':')}",
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
        if trace_has_lane_events(trace):
            def playing_reached():
                return log.current_screen() == "playing"
            try:
                wait_for(playing_reached, min(15.0, args.await_timeout), "the playing screen")
            except TimeoutError:
                pass

        prev_time_ms = 0
        last_applied_wall = time.time()

        for event in trace["events"]:
            control = event["control"]
            time_ms = intended_ms(event)
            delay = injection_delay(time_ms, prev_time_ms, time.time() - last_applied_wall)
            if delay > 0:
                time.sleep(delay)
            prev_time_ms = time_ms
            last_applied_wall = time.time()

            if control == "await-screen":
                target = event["screen"]
                wait_for(
                    lambda: log.current_screen() == target,
                    args.await_timeout,
                    f"screen {target!r}",
                )
                record_applied_event(applied, applied_log, event, "accepted")
                continue
            if control == "await-file":
                marker = out / f"{event['name']}.ready"
                wait_for(marker.is_file, args.await_timeout, f"marker {marker}")
                record_applied_event(applied, applied_log, event, "accepted")
                continue
            if control == "pause-ms":
                time.sleep(event["ms"] / 1000.0)
                record_applied_event(applied, applied_log, event, "accepted")
                continue
            if control == "state-shot":
                capture_state_shot(args.display, geometry,
                                   out / "states" / f"{event['name']}.png")
                record_applied_event(applied, applied_log, event, "captured")
                continue

            action = event["action"]
            if control in MENU_KEYS:
                inject_key(display, xtest, MENU_KEYS[control], action == "press")
            elif control.startswith("lane-"):
                inject_key(display, xtest, LANE_KEYS[int(control[5:])], action == "press")
            elif control == "click":
                if action == "press":
                    inject_click(display, xtest, geometry, event["x"], event["y"])
                else:
                    inject_click_release(display, xtest, geometry, event["x"], event["y"])
            elif control == "text":
                inject_text(display, xtest, event["text"])
            else:
                raise RuntimeError(f"unhandled control {control!r}")
            record_applied_event(applied, applied_log, event, "applied")

        time.sleep(2.0)
        verdict = 0
    except Exception as source:
        manifest["failure"] = repr(source)
        verdict = 1
    finally:
        if client is not None:
            client.terminate()
            try:
                client.wait(timeout=10)
            except subprocess.TimeoutExpired:
                client.kill()
        if client_log is not None:
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
