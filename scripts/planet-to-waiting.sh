#!/usr/bin/env bash
# planet-to-waiting.sh — aim-by-rect drive from Planet to Waiting/Loading.
#
# Every click target is resolved at run time:
#   * control ids via `o2jam-dump rect` (native staged rects)
#   * the channel enter face via the pinned constants in
#     crates/o2jam-shell/src/screens/paint.rs (channel_enter_origin)
# No hand-read coordinates appear in this script. A click that does
# not advance the scene fails the run with a non-zero exit.
#
# Modes:
#   standalone (default): build the client, boot the session through
#     scripts/rig-run.sh, drive it, stop it by PID, print the scene sequence.
#   --attach OUT_DIR: drive a session that is already booted in OUT_DIR
#     (scripts/rig-afk.sh owns boot/stop there). No build, no boot, no stop.
#
# Usage:
#   scripts/planet-to-waiting.sh [OUT_DIR] [--display :N] [--sound|--no-sound]
#       [--music-id N] [--target loading|playing]
#   scripts/planet-to-waiting.sh --attach OUT_DIR [--display :N]
#       [--music-id N] [--target loading|playing]
#
# --music-id stages a host song pick: the first Waiting Confirm queues the
# 4000/4002/4030 select and stays in Waiting, the second sends START (the
# shell stages the pick first, so the host sees it). --target playing then
# Confirms at Loading (4012) and waits for Playing.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
POPUP_HELPER="$SCRIPT_DIR/lib/planet_popup.py"
read -r POPUP_OK_X POPUP_OK_Y <<<"$(python3 "$POPUP_HELPER" aim normal-mode-ok)"
# shellcheck source=lib/rig-session.sh
. "$SCRIPT_DIR/lib/rig-session.sh"

DISPLAY_NUM="${DISPLAY:-:99}"
OUT_DIR=""
ATTACH=0
SOUND=1
MUSIC_ID=""
TARGET="loading"

while [ $# -gt 0 ]; do
  case "$1" in
    --display) DISPLAY_NUM="$2"; shift 2 ;;
    --display=*) DISPLAY_NUM="${1#*=}"; shift ;;
    --attach) ATTACH=1; OUT_DIR="$2"; shift 2 ;;
    --attach=*) ATTACH=1; OUT_DIR="${1#*=}"; shift ;;
    --sound) SOUND=1; shift ;;
    --no-sound) SOUND=0; shift ;;
    --music-id) MUSIC_ID="$2"; shift 2 ;;
    --music-id=*) MUSIC_ID="${1#*=}"; shift ;;
    --target) TARGET="$2"; shift 2 ;;
    --target=*) TARGET="${1#*=}"; shift ;;
    *) OUT_DIR="$1"; shift ;;
  esac
done
[ -n "$OUT_DIR" ] || { echo "planet-to-waiting: output directory required" >&2; exit 2; }
case "$TARGET" in
  loading) ;;
  playing)
    [ -n "$MUSIC_ID" ] || { echo "planet-to-waiting: --target playing needs --music-id (Loading Confirm enters Playing only with a staged pick)" >&2; exit 2; }
    ;;
  *) echo "planet-to-waiting: --target must be loading or playing (got '$TARGET')" >&2; exit 2 ;;
esac

# Sanitize the environment: never leak a Wayland session onto the rig.
unset WAYLAND_DISPLAY XDG_SESSION_TYPE QT_QPA_PLATFORM 2>/dev/null || true
export WINIT_UNIX_BACKEND=x11
: "${PULSE_SINK:=wine_null}"
export PULSE_SINK

mkdir -p "$OUT_DIR"
LOG="$(rig_session_log "$OUT_DIR")"
FRAMES="$(rig_session_frames "$OUT_DIR")"
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

# The channel enter face: the pinned row-0 face of the channel bar
# (crates/o2jam-shell/src/screens/paint.rs, channel_enter_origin).
channel_enter_center() {
  python3 - <<'PY'
row_left, row_top = 216, 224
bar_inset = 1
face_local_x, face_local_y = 285, 11
face_w, face_h = 44, 21
print(row_left + bar_inset + face_local_x + face_w // 2,
      row_top + bar_inset + face_local_y + face_h // 2)
PY
}

wait_scene() {
  local target="$1"
  local limit="${2:-60}"
  local deadline=$((SECONDS + limit))
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

# The staging signal for a host song pick is the client's own log line: the
# select exchange (4000/4002/4030 + catalog chart fetch) runs over several
# seconds and the scene legitimately stays in Waiting while it does, so the
# second Confirm waits for the line instead of guessing a sleep.
wait_host_pick_staged() {
  local limit="${1:-120}"
  local deadline=$((SECONDS + limit))
  while [ "$SECONDS" -lt "$deadline" ]; do
    if grep -q "song staged" "$OUT_DIR/client.log" 2>/dev/null; then
      return 0
    fi
    sleep 0.5
  done
  echo "planet-to-waiting: FAIL: the host song pick never staged (no 'song staged' in $OUT_DIR/client.log)" >&2
  exit 1
}

# The session creates its own room at boot. On a shared Memoryer other
# rooms can precede it, so join the row named by the client's own room
# number instead of always taking row 0 (which can belong to another run).
created_room_row_control() {
  local line number
  line="$(sed -E 's/\x1B\[[0-9;]*[mK]//g' "$OUT_DIR/client.log" 2>/dev/null |
    grep -m1 'room created with number number=' || true)"
  number="${line##*number=}"
  case "$number" in
    '' | *[!0-9]*) die "no room number in $OUT_DIR/client.log; cannot pick the row" ;;
  esac
  if [ "$number" -gt 5 ]; then
    die "room number $number is outside the six staged room rows; pagination is OPEN"
  fi
  printf 'IDC_BUTTON_SROOM_EXBUTTON%d\n' "$((number + 1))"
}

click_at() {
  local x="$1" y="$2"
  "$REPO_ROOT/scripts/rig-drive.sh" --backend xtest --display "$DISPLAY_NUM" move "$OUT_DIR" "$x" "$y" >/dev/null
  "$REPO_ROOT/scripts/rig-drive.sh" --backend xtest --display "$DISPLAY_NUM" click "$OUT_DIR" 1 >/dev/null
}

press_key() {
  "$REPO_ROOT/scripts/rig-drive.sh" --backend xtest --display "$DISPLAY_NUM" key "$OUT_DIR" "$1" >/dev/null
}

latest_frame() {
  find "$FRAMES" -maxdepth 1 -name '*.png' -type f -printf '%T@ %p\n' 2>/dev/null |
    sort -nr | head -1 | cut -d' ' -f2-
}

popup_is_up() {
  local frame="$1"
  [ -n "$frame" ] && [ -r "$frame" ] || return 1
  [ "$(python3 "$POPUP_HELPER" check "$frame" 2>/dev/null)" = "up" ]
}

# The NORMAL-mode box blocks the Planet emblem. The aim comes from the popup
# definition in lib/popup_state.py (#747); the frame test alone decides
# whether that face needs pressing (W6: never blind-Escape).
dismiss_startup_popup() {
  local frame deadline
  frame="$(latest_frame)"
  if ! popup_is_up "$frame"; then
    return 0
  fi
  echo "planet-to-waiting: dismiss NORMAL-mode box at ($POPUP_OK_X,$POPUP_OK_Y) (docs/captures/t240/findings.md)"
  click_at "$POPUP_OK_X" "$POPUP_OK_Y"
  deadline=$((SECONDS + 10))
  while [ "$SECONDS" -lt "$deadline" ]; do
    frame="$(latest_frame)"
    if ! popup_is_up "$frame"; then
      return 0
    fi
    sleep 0.5
  done
  die "NORMAL-mode box is still up after the OK click"
}

# --- session lifecycle (standalone mode) -----------------------------------
build_client() {
  (cd "$REPO_ROOT" && cargo build -q -p recomp2jam >/dev/null 2>&1) || die "build failed"
  [ -x "$(rig_session_binary)" ] || die "no binary at $(rig_session_binary)"
}

boot_session() {
  local -a argv=() options=()
  if [ -n "$MUSIC_ID" ]; then
    options+=(--music-id "$MUSIC_ID")
  fi
  if [ "$SOUND" = "0" ]; then
    options+=(--no-sound)
  fi
  mapfile -t argv < <(rig_session_argv "$OUT_DIR" "${options[@]}")
  "$REPO_ROOT/scripts/rig-run.sh" run "$DISPLAY_NUM" "$OUT_DIR" -- \
    "$(rig_session_binary)" "${argv[@]}" >/dev/null 2>&1 || die "rig-run boot failed"
}

stop_session() {
  "$REPO_ROOT/scripts/rig-run.sh" stop "$OUT_DIR" >/dev/null 2>&1 || true
}

# --- the drive itself -------------------------------------------------------
# 1. Splash -> Planet.
drive_to_waiting() {
  wait_scene planet 90
  dismiss_startup_popup

  # 2. Planet emblem (control id) -> Channel.
  local px py
  read -r px py <<<"$(rect_center IDC_BUTTON_SPLANET_NORMAL)"
  click_at "$px" "$py"
  wait_scene channel 30

  # 3. Channel enter face (pinned constants, channel_enter_origin row 0).
  local cx cy
  read -r cx cy <<<"$(channel_enter_center)"
  click_at "$cx" "$cy"
  wait_scene room 60

  # 4. The room this session created -> Waiting.
  local rx ry
  read -r rx ry <<<"$(rect_center "$(created_room_row_control)")"
  click_at "$rx" "$ry"
  wait_scene waiting 60
}

# 5. Waiting confirm (control id): START. With a host pick staged the first
# press queues the select and stays in Waiting, so the second press is the
# one that sends START and reaches Loading.
drive_to_loading() {
  local wx wy
  read -r wx wy <<<"$(rect_center IDC_BUTTON_SWAIT_START)"
  click_at "$wx" "$wy"
  if [ -n "$MUSIC_ID" ]; then
    wait_host_pick_staged 120
    click_at "$wx" "$wy"
  fi
  wait_scene loading 60
}

# 6. Loading confirm (Return) sends ConfirmMusicLoaded 4012 -> Playing.
drive_to_playing() {
  press_key Return
  wait_scene playing 60
}

report_sequence() {
  echo "planet-to-waiting: OK scene sequence:"
  "$REPO_ROOT/scripts/scene-probe.sh" "$LOG" --sequence
}

drive_scenes() {
  drive_to_waiting
  drive_to_loading
  if [ "$TARGET" = "playing" ]; then
    drive_to_playing
  fi
}

main() {
  if [ "$ATTACH" = "1" ]; then
    drive_scenes
  else
    build_client
    boot_session
    drive_scenes
    stop_session
  fi
  report_sequence
}

main "$@"
