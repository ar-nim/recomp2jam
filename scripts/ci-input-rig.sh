#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$REPO_ROOT/.ci-rig"
rm -rf "$OUT"; mkdir -p "$OUT"
DISPLAY_NUM=":99"
if ! command -v Xvfb >/dev/null; then echo "FAIL: Xvfb not found" >&2; exit 1; fi
Xvfb "$DISPLAY_NUM" -screen 0 1024x768x24 >"$OUT/xvfb.log" 2>&1 &
XVFB_PID=$!
for _ in $(seq 1 50); do xdpyinfo -display "$DISPLAY_NUM" >/dev/null 2>&1 && break; sleep 0.2; done
if ! xdpyinfo -display "$DISPLAY_NUM" >/dev/null 2>&1; then
  echo "FAIL: display $DISPLAY_NUM did not answer" >&2
  cat "$OUT/xvfb.log" >&2
  kill "$XVFB_PID" 2>/dev/null || true
  exit 1
fi
export DISPLAY="$DISPLAY_NUM"
export WINIT_UNIX_BACKEND=x11
"$REPO_ROOT/target/debug/recomp2jam" play --window --no-sound --input-log "$OUT/input.log" >"$OUT/client.log" 2>&1 &
CLIENT_PID=$!
sleep 6
if command -v xdotool >/dev/null; then
  xdotool mousemove 120 80
  xdotool click 1
else
  python3 - "$DISPLAY_NUM" <<'PY'
import sys
from Xlib import X, display
from Xlib.ext import xtest
d = display.Display(sys.argv[1])
xtest.fake_input(d, X.MotionNotify, x=120, y=80)
d.sync()
xtest.fake_input(d, X.ButtonPress, 1)
d.sync()
xtest.fake_input(d, X.ButtonRelease, 1)
d.sync()
PY
fi
sleep 2
kill "$CLIENT_PID" 2>/dev/null || true
wait "$CLIENT_PID" 2>/dev/null || true
kill "$XVFB_PID" 2>/dev/null || true
if [ ! -f "$OUT/input.log" ]; then echo "FAIL: no input log" >&2; exit 1; fi
if ! grep -qE '^click [0-9]+ [0-9]+$' "$OUT/input.log"; then echo "FAIL: click not logged" >&2; exit 1; fi
if ! grep -qE '^click-up [0-9]+ [0-9]+$' "$OUT/input.log"; then echo "FAIL: click-up not logged" >&2; exit 1; fi
echo "PASS: injected move+click arrived"
