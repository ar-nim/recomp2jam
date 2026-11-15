#!/usr/bin/env python3
"""Replay a recomp2jam-input-v1 trace through the real O2Jam 7.55 exe.

This is the Wine-side counterpart of replay_window.py: the same trace
file drives the original exe running under Wine. Every press and release
is delivered through the SendInput helper (winegcc-built, running inside
Wine) — never through bare XTEST motion, which never reaches the
DirectDraw canvas (docs/wine-boot.md, Attempts 6-9).

The driver logs accepted/applied events in sequence (one line per event
in applied.log plus a manifest.json) so two runs of the same trace can
be compared byte-for-byte on the applied-event log.

Containment (rig-run.sh rules): a private Xvfb display, WAYLAND_DISPLAY
and XDG_SESSION_TYPE stripped on every boot, PULSE_SINK=wine_null always,
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
                except ValueError:
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



def intended_ms(event):
    """Microsecond authority (time_us) wins when time_ms is absent."""
    if "time_ms" in event and event.get("time_ms") is not None:
        return int(event["time_ms"])
    time_us = event.get("time_us")
    if time_us is not None:
        return int(time_us) // 1000
    return 0


def compile_helper(source, helper_path):
    helper_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["winegcc", str(source), "-o", str(helper_path)],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def run_helper(wine, prefix, helper, *args, env):
    cmd = [wine, str(helper)] + [str(a) for a in args]
    env = dict(env, WINEPREFIX=str(prefix))
    result = subprocess.run(cmd, capture_output=True, text=True, env=env)
    if result.returncode != 0:
        raise RuntimeError(f"helper failed: {result.stderr.strip() or result.stdout.strip()}")
    return result.stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace", required=True, help="trace JSON path")
    parser.add_argument("--exe", default="OTwo.exe", help="exe to run under Wine")
    parser.add_argument("--wine-prefix", default=os.path.expanduser("~/o2jam-prefix"))
    parser.add_argument("--out", required=True, help="evidence output dir")
    parser.add_argument("--display", default=":97")
    parser.add_argument("--helper", default="", help="compiled SendInput helper path")
    parser.add_argument("--helper-source", default="", help="helper C source path")
    parser.add_argument("--ready-delay", type=float, default=12.0,
                        help="seconds to wait for the exe window after launch")
    parser.add_argument("--await-timeout", type=float, default=120.0)
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

    script_dir = Path(__file__).resolve().parent
    helper_source = Path(args.helper_source) if args.helper_source else script_dir / "sendinput-helper.c"
    helper_path = Path(args.helper) if args.helper else out / "sendinput-helper.exe.so"
    if not helper_path.exists() or (helper_source.exists() and helper_source.stat().st_mtime > helper_path.stat().st_mtime):
        if not helper_source.exists():
            print(f"helper source missing: {helper_source}", file=sys.stderr)
            return 2
        try:
            compile_helper(helper_source, helper_path)
        except (subprocess.CalledProcessError, FileNotFoundError) as source:
            print(f"helper compile failed: {source}", file=sys.stderr)
            return 2

    wine = os.environ.get("WINE", "wine")
    prefix = Path(args.wine_prefix)
    exe = Path(args.exe)
    if not exe.is_file() and not exe.is_absolute():
        # resolve relative to cwd / prefix drive
        pass

    # Sanitised environment: no Wayland leak, null audio always.
    env = dict(os.environ)
    env.pop("WAYLAND_DISPLAY", None)
    env.pop("WAYLAND_SOCKET", None)
    env.pop("XDG_SESSION_TYPE", None)
    env["WINIT_UNIX_BACKEND"] = "x11"
    env["PULSE_SINK"] = "wine_null"
    env["WINEPREFIX"] = str(prefix)
    display_arg = args.display
    env["DISPLAY"] = display_arg

    # Start Xvfb if needed.
    lock = Path(f"/tmp/.X{display_arg[1:]}-lock")
    started_xvfb = not lock.exists()
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
    blob_file = Path(os.path.expanduser("~/scratch/blob.txt"))
    mirror = "|test|??|127.0.0.1|15010"
    if blob_file.is_file():
        blob = blob_file.read_text().strip()
        command = [wine, "explorer", "/desktop=o2jam,800x600", str(exe),
                   blob, mirror, mirror, mirror, mirror, mirror]
        if Path(os.path.expanduser("~/.local/share/wine-locales")).is_dir():
            env["LOCPATH"] = os.path.expanduser("~/.local/share/wine-locales")
            env["LANG"] = "ko_KR.UTF-8"
    else:
        command = [wine, str(exe)]
    client = subprocess.Popen(command, stdout=client_log, stderr=client_log, env=env)

    applied = []
    applied_log = out / "applied.log"
    applied_log.write_text("")
    manifest = {
        "trace": str(trace_path.resolve()),
        "exe": str(exe),
        "display": display_arg,
        "started_at": time.time(),
    }
    verdict = 0
    try:
        time.sleep(args.ready_delay)

        prev_time_ms = 0
        last_applied_wall = time.time()

        def helper_call(*helper_args):
            return run_helper(wine, prefix, helper_path, *helper_args, env=env)

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
                # The exe has no capture log; hold for a fixed settle.
                time.sleep(0.5)
                applied.append({"sequence": event["sequence"], "control": control, "action": action,
                                "time_ms": time_ms, "time_us": time_us})
                with applied_log.open("a") as handle:
                    handle.write(f"seq={event['sequence']} control={control} action={action} "
                                 f"intended_ms={time_ms} status=accepted\n")
                continue

            if control in MENU_KEYS:
                key_name = KEY_NAME[control]
                helper_call("key", key_name, "down" if action == "press" else "up")
            elif control.startswith("lane-"):
                lane_idx = int(control[5:])
                key_name = LANE_KEYS[lane_idx]
                if key_name == "space":
                    key_name = "space"
                helper_call("key", key_name, "down" if action == "press" else "up")
            elif control == "click":
                x, y = event["x"], event["y"]
                if action == "press":
                    helper_call("down", x, y)
                else:
                    helper_call("up", x, y)
            elif control == "text":
                helper_call("type", event["text"])
            else:
                raise RuntimeError(f"unhandled control {control!r}")

            applied.append({"sequence": event["sequence"], "control": control, "action": action,
                            "time_ms": time_ms, "time_us": time_us})
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
