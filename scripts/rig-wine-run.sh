#!/usr/bin/env bash
# rig-wine-run.sh — one Wine-pair evidence pack, end to end (#240).
#
# Usage:
#   scripts/rig-wine-run.sh OUT_DIR [options]
#
# Options:
#   --display :N              private X display (default: first free in :160-:179)
#   --target SCENE            last scene the drive must reach (default loading)
#   --music-id ID             room music (default 757)
#   --join-mode MODE          dclick (qualified) or hover-press (#240 OPEN arm 1)
#   --row-repaint MODE        none or recreate (host-side 2005 repaint)
#   --repaint-delay S         seconds from room entry before the host rebuild
#   --row-dismiss MODE        auto clears the no-response modal; none keeps
#                             the session its OK button ends with FFF0
#   --wire-tap                log the exe's game-TCP frames pass-through
#   --observe-waiting-seconds S
#                             after a joined 0BBA, poll frames and wire for S
#                             seconds without claiming a Waiting scene
#                             (mint mirrors to :15012; hash the log in provenance)
#   --cue-discover-seconds S   after a proven Waiting capture, inventory controls,
#                             probe READY, then queue the host start command
#   --host-start-delay S      host's automatic start delay (default 20)
#   --user-index-id N         guest account (default 1 = tester)
#   --host-user-index-id N    host account (default 3 = t207probe; 0 = no host)
#   --cross DIR               an A/B partner run dir; verify checks the two
#                             packs hold pairwise-distinct screenshots
#   --no-drive                boot and package only (diagnostics)
#
# Phases, each bounded and each a kit call (nothing hand-rolled):
#   1. mint     scripts/rig-mint-launch-argv.sh — the #265 blob builder
#               writes <OUT>/argv.txt (blob + 5 mirrors) from the running
#               Memoryer database, read-only.
#   2. boot     scripts/rig-run.sh run — shared preflight (owned sink,
#               Memoryer, install-root, prefix claim), audio lane, Xvfb,
#               then `scripts/rig-wine.sh launch <argv.txt>`. A flock on
#               scripts/lib/rig-live-lock.sh is inherited by the client and
#               held until it exits, so no second lane can boot (#489).
#   3. drive    scripts/wine-scene.py --attach — SendInput clicks at aims
#               that cite pinned evidence, one capture per scene, scene log.
#   4. stop     scripts/rig-run.sh stop — recorded PIDs only.
#   5. package  run-manifest.sh write + rig-provenance.sh emit (producer
#               kind exe, version 7.55, the exe's sha256 pinned).
#   6. verify   run-manifest.sh verify [+ cross-pair], rig-audio.sh verify.
#
# Exit: 0 when every check passes, 1 when a check fails, 2 on usage error.
# A failed pack is kept whole: nothing is deleted.
#
# Required environment:
#   WINEPREFIX   a Korean-locale Wine prefix (docs/wine-boot.md, Attempt 5)
# Optional:
#   RIG_WINE_EXE         the 7.55 client (default ~/scratch/run-full/OTwo.exe)
#   RIG_WINE_EXE_SHA256  pinned hash the run refuses to violate
#   RIG_WINE_BOOT_TIMEOUT (default 60), RIG_WINE_DRIVE_TIMEOUT (default 600),
#   RIG_WINE_STOP_TIMEOUT (default 120)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/rig-preflight.sh
. "$SCRIPT_DIR/lib/rig-preflight.sh"
# shellcheck source=lib/rig-live-lock.sh
. "$SCRIPT_DIR/lib/rig-live-lock.sh"

die() { echo "rig-wine-run: $*" >&2; exit 2; }

OUT_DIR="${1:-}"
[ -n "$OUT_DIR" ] || die "usage: rig-wine-run.sh OUT_DIR [options]"
shift

DISPLAY_NAME=""
TARGET="loading"
MUSIC_ID=757
JOIN_MODE="dclick"
ROW_REPAINT="none"
REPAINT_DELAY=0
ROW_DISMISS="auto"
WIRE_TAP=0
OBSERVE_WAITING_SECONDS=0
CUE_DISCOVER_SECONDS=0
HOST_START_DELAY=20
WIRE_TAP_PORT="${RIG_WIRE_TAP_PORT:-15012}"
GATEWAY_PORT=15010
USER_INDEX_ID=1
HOST_USER_INDEX_ID=3
CROSS_DIR=""
NO_DRIVE=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --display) DISPLAY_NAME="$2"; shift 2 ;;
    --target) TARGET="$2"; shift 2 ;;
    --music-id) MUSIC_ID="$2"; shift 2 ;;
    --join-mode) JOIN_MODE="$2"; shift 2 ;;
    --row-repaint) ROW_REPAINT="$2"; shift 2 ;;
    --repaint-delay) REPAINT_DELAY="$2"; shift 2 ;;
    --row-dismiss) ROW_DISMISS="$2"; shift 2 ;;
    --wire-tap) WIRE_TAP=1; shift ;;
    --observe-waiting-seconds) OBSERVE_WAITING_SECONDS="$2"; shift 2 ;;
    --cue-discover-seconds) CUE_DISCOVER_SECONDS="$2"; shift 2 ;;
    --host-start-delay) HOST_START_DELAY="$2"; shift 2 ;;
    --user-index-id) USER_INDEX_ID="$2"; shift 2 ;;
    --host-user-index-id) HOST_USER_INDEX_ID="$2"; shift 2 ;;
    --cross) CROSS_DIR="$2"; shift 2 ;;
    --no-drive) NO_DRIVE=1; shift ;;
    *) die "unknown option: $1" ;;
  esac
done

case "$JOIN_MODE" in
  dclick|hover-press) ;;
  *) die "unknown --join-mode '$JOIN_MODE' (want dclick|hover-press)" ;;
esac
case "$ROW_REPAINT" in
  none|recreate) ;;
  *) die "unknown --row-repaint '$ROW_REPAINT' (want none|recreate)" ;;
esac
case "$ROW_DISMISS" in
  auto|none) ;;
  *) die "unknown --row-dismiss '$ROW_DISMISS' (want auto|none)" ;;
esac
if [ "$WIRE_TAP" -eq 1 ]; then
  GATEWAY_PORT="$WIRE_TAP_PORT"
fi

RIG_WINE_EXE="${RIG_WINE_EXE:-$HOME/scratch/run-full/OTwo.exe}"
RIG_WINE_EXE_SHA256="${RIG_WINE_EXE_SHA256:-83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b}"
RIG_WINE_INJECTOR_DIR="${RIG_WINE_INJECTOR_DIR:-$HOME/scratch/240-fix/injector}"
RIG_WINE_BOOT_TIMEOUT="${RIG_WINE_BOOT_TIMEOUT:-60}"
RIG_WINE_DRIVE_TIMEOUT="${RIG_WINE_DRIVE_TIMEOUT:-600}"
RIG_WINE_STOP_TIMEOUT="${RIG_WINE_STOP_TIMEOUT:-120}"
PAIR="wine-x11-sendinput"

[ -n "${WINEPREFIX:-}" ] || die "WINEPREFIX is required (the Wine leg never uses the default prefix)"
[ -d "$WINEPREFIX" ] || die "WINEPREFIX '$WINEPREFIX' is not a directory"
export WINEPREFIX RIG_WINE_EXE RIG_WINE_INJECTOR_DIR

mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
# The relay-fastfail interposer appends its interception line here (the
# stderr copy lands in client.log either way); rig-wine.sh launch loads
# the .so through LD_PRELOAD, scoped to the Wine pair's own process tree.
RELAY_FASTFAIL_LOG="$OUT_DIR/relay-fastfail.log"
export RELAY_FASTFAIL_LOG
STATUS_FILE="$OUT_DIR/status"
REPORT_FILE="$OUT_DIR/report.md"
WIRE_LOG="$OUT_DIR/wire/room-stage.log"
WIRE_PID_FILE="$OUT_DIR/wire/tap.pid"
: > "$REPORT_FILE"
echo "FAILED" > "$STATUS_FILE"

note() { echo "rig-wine-run: $*"; echo "- $*" >> "$REPORT_FILE"; }

# The dead-relay interposer is part of every room boot: without it the
# exe's synchronous connect to 203.238.136.52:7788 blocks the UI thread
# 35.785 s (issue #323). Auto-build once from the committed source, the
# same scratch home the SendInput injector uses. RIG_RELAY_FASTFAIL=0
# keeps the falsification arm (b6 vs b8, issue #324) reachable.
ensure_relay_fastfail() {
  local so="$RIG_WINE_INJECTOR_DIR/relay-fastfail.so"
  if [ "${RIG_RELAY_FASTFAIL:-1}" = "0" ]; then
    note "relay-fastfail: disabled (RIG_RELAY_FASTFAIL=0, falsification arm)"
    return 0
  fi
  if [ ! -f "$so" ]; then
    note "relay-fastfail: building $so from scripts/rig-relay-fastfail.c"
    bash "$SCRIPT_DIR/rig-wine.sh" build-relay-fastfail "$RIG_WINE_INJECTOR_DIR" >> "$REPORT_FILE" 2>&1 \
      || die "relay-fastfail build failed"
  fi
  note "relay-fastfail: $(sha256sum "$so" | cut -c1-12) preloaded, log $RELAY_FASTFAIL_LOG"
}

exe_sha256() { sha256sum "$RIG_WINE_EXE" | cut -d' ' -f1; }

verify_pins() {
  [ -f "$RIG_WINE_EXE" ] || die "client exe not found: $RIG_WINE_EXE"
  local actual
  actual="$(exe_sha256)"
  [ "$actual" = "$RIG_WINE_EXE_SHA256" ] || \
    die "client exe sha256 $actual is not the pinned $RIG_WINE_EXE_SHA256"
  note "pins: exe $RIG_WINE_EXE sha256 $actual, wine $(wine --version 2>/dev/null || echo 'unknown')"
}

allocate_display() {
  local candidate
  for candidate in $(seq "${RIG_WINE_DISPLAY_FIRST:-160}" "${RIG_WINE_DISPLAY_LAST:-179}"); do
    if ! xdpyinfo -display ":$candidate" >/dev/null 2>&1; then
      echo ":$candidate"
      return 0
    fi
  done
  return 1
}

phase_mint() {
  note "phase mint (gateway port $GATEWAY_PORT)"
  bash "$SCRIPT_DIR/rig-mint-launch-argv.sh" \
    --user-index-id "$USER_INDEX_ID" --out "$OUT_DIR/argv.txt" \
    --gateway-port "$GATEWAY_PORT" >> "$REPORT_FILE" 2>&1 \
    || { note "mint FAILED"; return 1; }
  return 0
}

phase_wire_tap() {
  [ "$WIRE_TAP" -eq 1 ] || return 0
  note "phase wire-tap on 127.0.0.1:$WIRE_TAP_PORT -> $GATEWAY_PORT"
  mkdir -p "$OUT_DIR/wire"
  python3 "$SCRIPT_DIR/wine-room-host.py" --wire-tap \
    --out-dir "$OUT_DIR/wire" --wire-log "$WIRE_LOG" \
    --listen-port "$WIRE_TAP_PORT" --upstream-port 15010 \
    > "$OUT_DIR/wire/tap.log" 2>&1 &
  echo "$!" > "$WIRE_PID_FILE"
  local tries=0
  while [ "$tries" -lt 50 ]; do
    if grep -q 'wire-tap listening' "$WIRE_LOG" 2>/dev/null; then
      note "wire-tap ready pid $(cat "$WIRE_PID_FILE") log $WIRE_LOG"
      return 0
    fi
    sleep 0.2
    tries=$((tries + 1))
  done
  note "wire-tap FAILED to become ready (see wire/tap.log)"
  return 1
}

# The join verdict belongs in the manifest, not only in drive.log. The
# wire log is the oracle: 0BBA is opcode 3002 decimal (0x0BBA on the wire).
join_outcome_fields() {
  local opcode=absent
  local press_count=0
  if [ -s "$WIRE_LOG" ] && grep -Eq 'opcode=(3002|0BBA) ' "$WIRE_LOG"; then
    opcode=present
  fi
  if [ -s "$OUT_DIR/drive.log" ]; then
    press_count="$(grep -c 'inject send-press ' "$OUT_DIR/drive.log" || true)"
  fi
  printf 'join_0BBA=%s join_press_count=%s' "$opcode" "$press_count"
}

stop_wire_tap() {
  [ -f "$WIRE_PID_FILE" ] || return 0
  local tap_pid
  tap_pid="$(cat "$WIRE_PID_FILE")"
  if kill -0 "$tap_pid" 2>/dev/null; then
    kill -TERM "$tap_pid"
  fi
  note "wire-tap stopped pid $tap_pid"
}

phase_boot() {
  note "phase boot on $DISPLAY_NAME"
  local boot_log="$OUT_DIR/phases-boot.log"
  if ! timeout "$RIG_WINE_BOOT_TIMEOUT" bash "$SCRIPT_DIR/rig-run.sh" run \
      "$DISPLAY_NAME" "$OUT_DIR" -- scripts/rig-wine.sh launch "$OUT_DIR/argv.txt" \
      > "$boot_log" 2>&1; then
    note "boot FAILED (see phases-boot.log); last lines:"
    tail -5 "$boot_log" >> "$REPORT_FILE" || true
    return 1
  fi
  note "boot ok: $(grep -c . "$boot_log") log lines, pidfile $(grep -c . "$OUT_DIR/pids") entries"
  return 0
}

phase_drive() {
  note "phase drive (target $TARGET)"
  timeout "$RIG_WINE_DRIVE_TIMEOUT" python3 "$SCRIPT_DIR/wine-scene.py" \
    --attach "$OUT_DIR" --display "$DISPLAY_NAME" --target "$TARGET" \
    --music-id "$MUSIC_ID" --join-mode "$JOIN_MODE" \
    --row-repaint "$ROW_REPAINT" --repaint-delay "$REPAINT_DELAY" \
    --row-dismiss "$ROW_DISMISS" --wire-log "$WIRE_LOG" \
    --observe-waiting-seconds "$OBSERVE_WAITING_SECONDS" \
    --cue-discover-seconds "$CUE_DISCOVER_SECONDS" \
    --host-start-delay "$HOST_START_DELAY" \
    --host-user-index-id "$HOST_USER_INDEX_ID" \
    --wineprefix "$WINEPREFIX" > "$OUT_DIR/phases-drive.log" 2>&1 || true
  local reached
  reached="$(bash "$SCRIPT_DIR/scene-probe.sh" "$OUT_DIR/drive.log" 2>/dev/null || echo none)"
  note "drive reached '$reached'"
  if [ "$reached" = "$TARGET" ]; then
    return 0
  fi
  note "drive FAILED to reach '$TARGET' (see drive.log)"
  return 1
}

phase_stop() {
  note "phase stop"
  # A clean stop removes the pidfile; keep the record of what ran.
  [ -f "$OUT_DIR/pids" ] && cp -f "$OUT_DIR/pids" "$OUT_DIR/pids.txt"
  timeout "$RIG_WINE_STOP_TIMEOUT" bash "$SCRIPT_DIR/rig-run.sh" stop "$OUT_DIR" \
    >> "$REPORT_FILE" 2>&1 || note "stop reported a problem (evidence kept)"
  stop_wire_tap
  return 0
}

phase_package() {
  note "phase package"
  # A --no-drive diagnostic boot has no drive.log. Omitting --input-log
  # keeps the manifest honest: the pack declares no input log rather than
  # inventing one.
  local -a input_log_args=()
  if [ "$NO_DRIVE" -eq 0 ]; then
    input_log_args=(--input-log "$OUT_DIR/drive.log")
  fi
  local relay_sha=off
  if [ "${RIG_RELAY_FASTFAIL:-1}" != "0" ] && [ -f "$RIG_WINE_INJECTOR_DIR/relay-fastfail.so" ]; then
    relay_sha="$(sha256sum "$RIG_WINE_INJECTOR_DIR/relay-fastfail.so" | cut -c1-12)"
  fi
  bash "$SCRIPT_DIR/run-manifest.sh" write "$OUT_DIR" \
    --pair "$PAIR" \
    --build "$(git -C "$REPO_ROOT" rev-parse HEAD)" \
    --script "scripts/wine-scene.py (target $TARGET, music $MUSIC_ID, join $JOIN_MODE, repaint $ROW_REPAINT, dismiss $ROW_DISMISS)" \
    --params "display=$DISPLAY_NAME sink=$(sed -n "s/^export PULSE_SINK='\([^']*\)'$/\1/p" "$OUT_DIR/env.sh" | head -1) exe_sha256=$RIG_WINE_EXE_SHA256 wire_tap=$WIRE_TAP gateway_port=$GATEWAY_PORT relay_fastfail=$relay_sha $(join_outcome_fields) $(rig_server_identity_params "$OUT_DIR")" \
    "${input_log_args[@]}" >> "$REPORT_FILE" 2>&1 \
    || { note "manifest write FAILED"; return 1; }
  local injector="$RIG_WINE_INJECTOR_DIR/sendinput-helper.exe.so"
  local -a injector_args=()
  [ -f "$injector" ] && injector_args=(--injector "$injector")
  local -a wire_args=()
  if [ "$WIRE_TAP" -eq 1 ]; then
    [ -s "$WIRE_LOG" ] || { note "wire log missing or empty: $WIRE_LOG"; return 1; }
    wire_args=(--artifact wire/room-stage.log)
  fi
  local -a relay_args=()
  [ -s "$OUT_DIR/relay-fastfail.log" ] && relay_args=(--artifact relay-fastfail.log)
  bash "$SCRIPT_DIR/rig-provenance.sh" emit "$OUT_DIR" \
    --producer "$RIG_WINE_EXE" --producer-kind exe --producer-version 7.55 \
    --expect-producer-sha256 "$RIG_WINE_EXE_SHA256" \
    --capture-tool ffmpeg --pair "$PAIR" \
    --command "scripts/rig-wine.sh launch $OUT_DIR/argv.txt (wine explorer /desktop=o2jam,800x600 $RIG_WINE_EXE, argv = AuthParameters blob + 5 |test|??|addr|port mirrors from argv.txt)" \
    --artifact argv.txt --artifact argv.txt.fields \
    --artifact audio/route-snapshot.txt \
    --artifact audio/route-baseline.txt \
    "${wire_args[@]}" \
    "${relay_args[@]}" \
    "${injector_args[@]}" >> "$REPORT_FILE" 2>&1 \
    || { note "provenance emit FAILED"; return 1; }
  return 0
}

phase_verify() {
  local ok=0
  note "phase verify"
  local shots
  shots="$(find "$OUT_DIR/artifacts/screenshots" -name '*.png' 2>/dev/null | wc -l)"
  note "screenshots: $shots captured, $(find "$OUT_DIR/artifacts/screenshots" -name '*.png' -exec md5sum {} \; 2>/dev/null | awk '{print $1}' | sort -u | wc -l) distinct"
  if [ -n "$CROSS_DIR" ]; then
    if bash "$SCRIPT_DIR/run-manifest.sh" verify "$OUT_DIR/manifest.json" "$CROSS_DIR/manifest.json" >> "$REPORT_FILE" 2>&1; then
      note "check manifest: PASS (A/B with $CROSS_DIR: no shared screenshot)"
    else
      note "check manifest: FAIL (A/B with $CROSS_DIR)"
      ok=1
    fi
  else
    if bash "$SCRIPT_DIR/run-manifest.sh" verify "$OUT_DIR/manifest.json" >> "$REPORT_FILE" 2>&1; then
      note "check manifest: PASS (artifact set, distinct screenshots, provenance, audio)"
    else
      note "check manifest: FAIL"
      ok=1
    fi
  fi
  if bash "$SCRIPT_DIR/rig-audio.sh" verify "$OUT_DIR" >> "$REPORT_FILE" 2>&1; then
    note "check audio: PASS ($(sed -n 's/^peak_dbfs=//p' "$OUT_DIR/audio/manifest" | head -1) dBFS peak, lane captured)"
  else
    note "check audio: FAIL"
    ok=1
  fi
  if bash "$SCRIPT_DIR/rig-provenance.sh" cite "$OUT_DIR" --claim exe >> "$OUT_DIR/citation.txt" 2>&1; then
    note "check cite: PASS (exe claim allowed; citation.txt written)"
  else
    note "check cite: FAIL (the pack may not be cited as exe evidence)"
    ok=1
  fi
  return "$ok"
}

main() {
  rig_live_lock_acquire "rig-wine-run:$OUT_DIR pid=$$" || {
    note "live lane lock unavailable; no client booted"
    return 1
  }
  rig_live_lock_install_release_trap
  verify_pins
  ensure_relay_fastfail
  if [ -z "$DISPLAY_NAME" ]; then
    DISPLAY_NAME="$(allocate_display)" || die "no free display in :160-:179"
  fi
  note "run dir $OUT_DIR, display $DISPLAY_NAME, pair $PAIR"
  local failed=0 booted=0
  phase_mint || failed=1
  if [ "$failed" -eq 0 ]; then phase_wire_tap || failed=1; fi
  if [ "$failed" -eq 0 ]; then
    if phase_boot; then booted=1; else failed=1; fi
  fi
  if [ "$failed" -eq 0 ] && [ "$NO_DRIVE" -eq 0 ]; then phase_drive || failed=1; fi
  phase_stop || true
  # A drive that stopped short is still evidence: package and verify it so
  # the negative result carries a manifest instead of an unlabelled pack.
  if [ "$booted" -eq 1 ]; then
    phase_package || failed=1
    phase_verify || failed=1
  fi
  if [ "$failed" -eq 0 ]; then
    echo "PASS" > "$STATUS_FILE"
    note "status PASS"
    return 0
  fi
  note "status FAILED (pack kept: $OUT_DIR)"
  return 1
}

main
