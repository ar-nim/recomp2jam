#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$REPO_ROOT/.ci-rig"
rm -rf "$OUT"; mkdir -p "$OUT"
DISPLAY_NUM=":99"
if ! command -v Xvfb >/dev/null; then echo "FAIL: Xvfb not found" >&2; exit 1; fi
if ! command -v xdpyinfo >/dev/null; then echo "FAIL: xdpyinfo not found (install x11-utils)" >&2; exit 1; fi
# Never touch the live session: a leaked WAYLAND_DISPLAY makes winit open
# the window on the owner's compositor instead of the rig display (seen
# 2026-11-14: client windows popped onto the live Plasma session).
# rig-run.sh sanitizes the same way.
unset WAYLAND_DISPLAY XDG_SESSION_TYPE
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
# Cold boots stage art, BGM, and the offline catalog before the window
# maps (15 s observed); a fixed sleep races the boot. Poll for the
# named client window instead, up to 60 s, then fail with the log tail.
WINDOW_UP=0
if command -v xdotool >/dev/null; then
  for _ in $(seq 1 120); do
    xdotool search --onlyvisible --name recomp2jam >/dev/null 2>&1 && { WINDOW_UP=1; break; }
    sleep 0.5
  done
else
  if python3 - "$DISPLAY_NUM" <<'PY'
import sys, time
from Xlib import display
deadline = time.time() + 60
while time.time() < deadline:
    try:
        d = display.Display(sys.argv[1])
        for w in d.screen().root.query_tree().children:
            try:
                if w.get_wm_name() == 'recomp2jam':
                    print('window up')
                    sys.exit(0)
            except Exception:
                pass
    except Exception:
        pass
    time.sleep(0.5)
print('window never appeared')
sys.exit(1)
PY
  then
    WINDOW_UP=1
  fi
fi
if [ "$WINDOW_UP" -ne 1 ]; then
  echo "FAIL: client window never appeared" >&2
  tail -20 "$OUT/client.log" >&2
  kill "$CLIENT_PID" 2>/dev/null || true
  kill "$XVFB_PID" 2>/dev/null || true
  exit 1
fi
if command -v xdotool >/dev/null; then
  xdotool mousemove 120 80
  xdotool click 1
else
  python3 - "$DISPLAY_NUM" <<'PY'
import sys, time
from Xlib import X, display
from Xlib.ext import xtest
d = display.Display(sys.argv[1])
root = d.screen().root
# Warp first: fake MotionNotify alone does not move the pointer,
# so an unwarped click lands outside the client and logs nothing.
root.warp_pointer(120, 80)
d.sync(); time.sleep(0.3)
xtest.fake_input(d, X.ButtonPress, 1)
d.sync()
time.sleep(0.1)
xtest.fake_input(d, X.ButtonRelease, 1)
d.sync()
PY
fi
sleep 2
kill "$CLIENT_PID" 2>/dev/null || true
wait "$CLIENT_PID" 2>/dev/null || true
kill "$XVFB_PID" 2>/dev/null || true
if [ ! -f "$OUT/input.log" ]; then echo "FAIL: no input log" >&2; exit 1; fi
if ! grep -qE "click 120 80$" "$OUT/input.log"; then echo "FAIL: click not logged" >&2; exit 1; fi
if ! grep -qE "click-up 120 80$" "$OUT/input.log"; then echo "FAIL: click-up not logged" >&2; exit 1; fi
echo "PASS: injected move+click arrived"
