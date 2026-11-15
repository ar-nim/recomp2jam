#!/usr/bin/env bash
# planet-to-waiting.sh — aim-by-rect drive from Planet to Waiting.
#
# Every click target is resolved at run time:
#   * control ids via `o2jam-dump rect` (native staged rects)
#   * the channel enter face via the pinned constants in
#     crates/o2jam-shell/src/screens/paint.rs (channel_enter_origin)
# No hand-read coordinates appear in this script. A click that does
# not advance the scene fails the run with a non-zero exit.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DISPLAY_NUM="${DISPLAY:-:99}"
OUT_DIR=""
while [ $# -gt 0 ]; do
  case "$1" in
    --display) DISPLAY_NUM="$2"; shift 2 ;;
    --display=*) DISPLAY_NUM="${1#*=}"; shift ;;
    *) OUT_DIR="$1"; shift ;;
  esac
done
[ -n "$OUT_DIR" ] || { echo "planet-to-waiting: output directory required" >&2; exit 2; }

# Sanitize the environment: never leak a Wayland session onto the rig.
unset WAYLAND_DISPLAY XDG_SESSION_TYPE QT_QPA_PLATFORM 2>/dev/null || true
export WINIT_UNIX_BACKEND=x11
: "${PULSE_SINK:=wine_null}"
export PULSE_SINK

mkdir -p "$OUT_DIR"
LOG="$OUT_DIR/input.log"
FRAMES="$OUT_DIR/frames"
mkdir -p "$FRAMES"

# --- helpers ---------------------------------------------------------------
die() { echo "planet-to-waiting: $*" >&2; exit 1; }

rect_center() {
  # Print "x y" center for a control id. Fails loudly on no hit.
  local id="$1"
  local line
  line="$(cd "$REPO_ROOT" && cargo run -q -p o2jam-dump -- rect "$id" "$REPO_ROOT/install-root/O2Jam/Image/Interface1.opi" 2>/dev/null)" \
    || die "rect query failed for $id"
  python3 - "$line" <<'PY'
import re, sys
line = sys.argv[1]
m = re.search(r"x=(\d+) y=(\d+) w=(\d+) h=(\d+)", line)
if not m:
    raise SystemExit("no rect in: " + line)
x, y, w, h = map(int, m.groups())
print(x + w // 2, y + h // 2)
PY
}

wait_scene() {
  local target="$1" timeout="${2:-60}"
  local deadline=$((SECONDS + timeout))
  local scene=""
  while [ "$SECONDS" -lt "$deadline" ]; do
    if [ -r "$LOG" ]; then
      scene="$("$REPO_ROOT/scripts/scene-probe.sh" "$LOG" 2>/dev/null || true)"
      if [ "$scene" = "$target" ]; then
        return 0
      fi
    fi
    sleep 0.5
  done
  echo "planet-to-waiting: FAIL: never reached scene '$target' (last='$scene')" >&2
  [ -r "$LOG" ] && tail -n 5 "$LOG" >&2 || true
  exit 1
}

click_at() {
  local x="$1" y="$2"
  "$REPO_ROOT/scripts/rig-drive.sh" --backend xtest --display "$DISPLAY_NUM" move "$OUT_DIR" "$x" "$y" >/dev/null
  "$REPO_ROOT/scripts/rig-drive.sh" --backend xtest --display "$DISPLAY_NUM" click "$OUT_DIR" 1 >/dev/null
}

# --- build -----------------------------------------------------------------
(cd "$REPO_ROOT" && cargo build -q -p recomp2jam >/dev/null 2>&1) || die "build failed"
BIN="$REPO_ROOT/target/debug/recomp2jam"
[ -x "$BIN" ] || die "no binary at $BIN"

# --- boot ------------------------------------------------------------------
"$REPO_ROOT/scripts/rig-run.sh" run "$DISPLAY_NUM" "$OUT_DIR" -- \
  "$BIN" online \
    --host 127.0.0.1 --port 15010 \
    --user-id 1 --token eebaccee-fde9-4c50-b941-fda6c7004376 \
    --gateway-id 0 --channel-id 0 \
    --create-room "DriveRoom" \
    --no-sound \
    --frame-dump "$FRAMES" \
    --input-log "$LOG" \
  >/dev/null 2>&1 || die "rig-run boot failed"

# --- drive -----------------------------------------------------------------
# 1. Splash -> Planet.
wait_scene planet 90

# 2. Planet emblem (control id) -> Channel.
read -r px py <<<"$(rect_center IDC_BUTTON_SPLANET_NORMAL)"
click_at "$px" "$py"
wait_scene channel 30

# 3. Channel enter face (pinned constants, channel_enter_origin row 0).
read -r cx cy <<<"$(python3 - <<'PY'
# Constants from crates/o2jam-shell/src/screens/paint.rs
row_left, row_top = 216, 224
bar_inset = 1
face_local_x, face_local_y = 285, 11
face_w, face_h = 44, 21
print(row_left + bar_inset + face_local_x + face_w // 2,
      row_top + bar_inset + face_local_y + face_h // 2)
PY
)"
click_at "$cx" "$cy"
wait_scene room 60

# 4. Room row (control id panel) -> Waiting.
read -r rx ry <<<"$(rect_center IDC_BUTTON_SROOM_EXBUTTON1)"
click_at "$rx" "$ry"
wait_scene waiting 60

# 5. Waiting confirm (control id) — press the chrome. START advances to
# loading, so the drive ends there: reaching loading proves the press.
read -r wx wy <<<"$(rect_center IDC_BUTTON_SWAIT_START)"
click_at "$wx" "$wy"
wait_scene loading 60

# --- stop ------------------------------------------------------------------
"$REPO_ROOT/scripts/rig-run.sh" stop "$OUT_DIR" >/dev/null 2>&1 || true

# --- report ----------------------------------------------------------------
echo "planet-to-waiting: OK scene sequence:"
"$REPO_ROOT/scripts/scene-probe.sh" "$LOG" --sequence
