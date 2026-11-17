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
#               then `scripts/rig-wine.sh launch <argv.txt>`.
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

die() { echo "rig-wine-run: $*" >&2; exit 2; }

OUT_DIR="${1:-}"
[ -n "$OUT_DIR" ] || die "usage: rig-wine-run.sh OUT_DIR [options]"
shift

DISPLAY_NAME=""
TARGET="loading"
MUSIC_ID=757
JOIN_MODE="dclick"
ROW_REPAINT="none"
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
STATUS_FILE="$OUT_DIR/status"
REPORT_FILE="$OUT_DIR/report.md"
: > "$REPORT_FILE"
echo "FAILED" > "$STATUS_FILE"

note() { echo "rig-wine-run: $*"; echo "- $*" >> "$REPORT_FILE"; }

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
  note "phase mint"
  bash "$SCRIPT_DIR/rig-mint-launch-argv.sh" \
    --user-index-id "$USER_INDEX_ID" --out "$OUT_DIR/argv.txt" >> "$REPORT_FILE" 2>&1 \
    || { note "mint FAILED"; return 1; }
  return 0
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
    --row-repaint "$ROW_REPAINT" \
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
  return 0
}

phase_package() {
  note "phase package"
  bash "$SCRIPT_DIR/run-manifest.sh" write "$OUT_DIR" \
    --pair "$PAIR" \
    --build "$(git -C "$REPO_ROOT" rev-parse HEAD)" \
    --script "scripts/wine-scene.py (target $TARGET, music $MUSIC_ID, join $JOIN_MODE, repaint $ROW_REPAINT)" \
    --params "display=$DISPLAY_NAME sink=$(sed -n "s/^export PULSE_SINK='\([^']*\)'$/\1/p" "$OUT_DIR/env.sh" | head -1) exe_sha256=$RIG_WINE_EXE_SHA256" \
    --input-log "$OUT_DIR/drive.log" >> "$REPORT_FILE" 2>&1 \
    || { note "manifest write FAILED"; return 1; }
  local injector="$RIG_WINE_INJECTOR_DIR/sendinput-helper.exe.so"
  local -a injector_args=()
  [ -f "$injector" ] && injector_args=(--injector "$injector")
  bash "$SCRIPT_DIR/rig-provenance.sh" emit "$OUT_DIR" \
    --producer "$RIG_WINE_EXE" --producer-kind exe --producer-version 7.55 \
    --expect-producer-sha256 "$RIG_WINE_EXE_SHA256" \
    --capture-tool ffmpeg --pair "$PAIR" \
    --command "scripts/rig-wine.sh launch $OUT_DIR/argv.txt (wine explorer /desktop=o2jam,800x600 $RIG_WINE_EXE, argv = AuthParameters blob + 5 |test|??|addr|port mirrors from argv.txt)" \
    --artifact argv.txt --artifact argv.txt.fields \
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
  verify_pins
  if [ -z "$DISPLAY_NAME" ]; then
    DISPLAY_NAME="$(allocate_display)" || die "no free display in :160-:179"
  fi
  note "run dir $OUT_DIR, display $DISPLAY_NAME, pair $PAIR"
  local failed=0 booted=0
  phase_mint || failed=1
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
