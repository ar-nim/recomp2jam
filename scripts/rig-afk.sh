#!/usr/bin/env bash
# rig-afk.sh — one unattended command: boot, drive, capture all three
# modalities, self-check the pack, write a report (issue #280).
#
# Usage:
#   scripts/rig-afk.sh OUT_DIR [options]
#   scripts/rig-afk.sh run OUT_DIR [options]
#   scripts/rig-afk.sh selftest
#
# Phases (each with a timeout; no infinite poll anywhere):
#   1. allocate: pick a free X display (:150-:199 by default) that no live
#      run claims and nothing answers on. The runbook's Xvfb is the workbench:
#      the runner never touches the owner's seat and never adopts a display it
#      did not start.
#   2. boot: `scripts/rig-run.sh run` — the shared preflight (display, owned
#      virtual sink, Memoryer, install-root, relay ports, Wine prefix), the
#      audio lane (monitor tap before the client, so the first cue is in the
#      artifact), then the client in its own process group.
#   3. drive: the scene script in `--attach` mode drives the booted session
#      (default scripts/planet-to-waiting.sh).
#   4. inject (optional): lane presses through the injector, which writes its
#      own epoch-stamped record (never the client's input log).
#   5. stop: `scripts/rig-run.sh stop` — recorded PIDs only, never a pattern
#      kill; the audio lane closes and measures its artifact.
#   6. package: stage frames into `artifacts/screenshots`, then
#      `run-manifest.sh write` + `rig-provenance.sh emit`.
#   7. self-check: scene reached, frame floor, judged count vs injector,
#      audio non-silent, `run-manifest.sh verify` green.
#
# Report: OUT_DIR/status (`PASS` or `FAILED`), OUT_DIR/report.md (the lines
# below, verbatim) and OUT_DIR/report.json. A failed check keeps the whole
# out-dir: nothing is deleted, nothing is cleaned up, so the evidence that
# proves the failure survives. Exit 0 on PASS, 1 on FAILED.
#
# Options:
#   --scene-script PATH   drive script (default scripts/planet-to-waiting.sh);
#                        must accept `--attach OUT_DIR --display :N`
#   --pair native         pair name (native is the implemented leg; the Wine
#                        leg waits on #240)
#   --target SCENE        scene the drive must reach, passed to the drive
#                        script (default loading)
#   --music-id ID         host song pick handed to the drive (gameplay)
#   --inject-keys LIST    lane keys injected after the drive, e.g.
#                        lane-0,lane-3,lane-6 (gameplay judged check)
#   --inject-count N      injection rounds (default 1)
#   --min-frames N        frame-PNG floor (default 4)
#   --display :N          pin the display instead of allocating one
#   --binary PATH         client binary (default target/debug/recomp2jam)
#
# Knobs: RIG_AFK_BOOT_TIMEOUT (90), RIG_AFK_DRIVE_TIMEOUT (300),
# RIG_AFK_INJECT_TIMEOUT (120), RIG_AFK_STOP_TIMEOUT (90),
# RIG_AFK_DISPLAY_FIRST (150), RIG_AFK_DISPLAY_LAST (199),
# RIG_AFK_INJECTOR (scripts/rig-inject.sh), RIG_AFK_SEED.
# RIG_AFK_FAULT is a selftest-only fault injection (see cmd_selftest); a real
# run leaves it unset.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/rig-preflight.sh
. "$SCRIPT_DIR/lib/rig-preflight.sh"
# shellcheck source=lib/rig-session.sh
. "$SCRIPT_DIR/lib/rig-session.sh"
# shellcheck source=lib/rig-stub-display.sh
. "$SCRIPT_DIR/lib/rig-stub-display.sh"
# shellcheck source=rig-audio.sh
. "$SCRIPT_DIR/rig-audio.sh"

die() { echo "rig-afk: $*" >&2; exit 2; }

CALLER_DISPLAY="${DISPLAY:-}"
BOOT_TIMEOUT="${RIG_AFK_BOOT_TIMEOUT:-90}"
DRIVE_TIMEOUT="${RIG_AFK_DRIVE_TIMEOUT:-300}"
INJECT_TIMEOUT="${RIG_AFK_INJECT_TIMEOUT:-120}"
STOP_TIMEOUT="${RIG_AFK_STOP_TIMEOUT:-90}"
DISPLAY_FIRST="${RIG_AFK_DISPLAY_FIRST:-150}"
DISPLAY_LAST="${RIG_AFK_DISPLAY_LAST:-199}"

# An AFK pack carries all three modalities, so the lane is on unless the
# caller turned it off. A switched-off lane is not refused here: it boots, the
# sidecar records the opt-out, and the audio self-check goes red on it (that is
# the run the owner must see in the report).
rig_afk_declare_audio() {
  export RIG_AUDIO_CAPTURE="${RIG_AUDIO_CAPTURE:-1}"
  if [ "$RIG_AUDIO_CAPTURE" != "1" ]; then
    echo "rig-afk: audio capture disabled by RIG_AUDIO_CAPTURE=$RIG_AUDIO_CAPTURE; the audio check will fail" >&2
  fi
}

sanitize_session() {
  unset WAYLAND_DISPLAY XDG_SESSION_TYPE QT_QPA_PLATFORM 2>/dev/null || true
  export WINIT_UNIX_BACKEND=x11
}

# --- paths -----------------------------------------------------------------
out_path() { printf '%s/%s' "$OUT_DIR" "$1"; }
phase_log() { printf '%s/phases/%s.log' "$OUT_DIR" "$1"; }

# --- run state -------------------------------------------------------------
LAST_PHASE_CODE=0
LAST_PHASE_LOG=""
CHECK_NAMES=()
CHECK_STATUS=()
CHECK_DETAILS=()
OVERALL="PASS"
DISPLAY=""
SEED=""
BUILD=""

# --- 1. display ------------------------------------------------------------
# One registry serves every rig entry point: the shared allocator takes the
# display claim atomically, so two AFK runs cannot pick the same display in
# the gap between allocation and rig-run's preflight (#282).
allocate_display() {
  rig_allocate_display "$OUT_DIR" "rig-afk" "$DISPLAY_FIRST" "$DISPLAY_LAST"
}

# --- 2. phases -------------------------------------------------------------
# One phase per external command: output goes to OUT_DIR/phases/<name>.log,
# the call is bounded by a timeout, and the exit code is the phase verdict.
phase_run() {
  local name="$1"
  local limit="$2"
  shift 2
  local log
  log="$(phase_log "$name")"
  local code=0
  timeout "$limit" "$@" >"$log" 2>&1 || code=$?
  if [ "$code" -eq 124 ]; then
    echo "rig-afk: phase '$name' timed out after ${limit}s (see $log)" >&2
  fi
  LAST_PHASE_CODE="$code"
  LAST_PHASE_LOG="$log"
  return "$code"
}

phase_tail() {
  tail -n 3 "$LAST_PHASE_LOG" 2>/dev/null | tr '\n' ' '
}

# --- 3. self-checks --------------------------------------------------------
# Each check prints one detail line and returns 0 (pass), 77 (skipped) or 1
# (fail). The runner reports the detail verbatim.

check_scene() {
  local log probe
  log="$(rig_session_log "$OUT_DIR")"
  if [ ! -r "$log" ]; then
    echo "no input log at $log (the client never painted)"
    return 1
  fi
  if ! probe="$("$SCRIPT_DIR/scene-probe.sh" "$log" 2>/dev/null)"; then
    echo "scene probe found no frame line in $log"
    return 1
  fi
  if [ "$probe" != "$TARGET" ]; then
    echo "probe reports '$probe', expected '$TARGET'"
    return 1
  fi
  echo "reached '$probe'"
  return 0
}

check_frames() {
  local frames count
  frames="$(rig_session_frames "$OUT_DIR")"
  count="$(find "$frames" -maxdepth 1 -name '*.png' -type f 2>/dev/null | wc -l)"
  if [ "$count" -lt "$MIN_FRAMES" ]; then
    echo "$count frame PNG(s) in $frames, floor $MIN_FRAMES"
    return 1
  fi
  echo "$count frame PNG(s) >= floor $MIN_FRAMES"
  return 0
}

# A gameplay pack pairs the injector's own record with the client's judged
# lines (#237): counts must match per lane. A menu-only pack has no injector
# record; that is said out loud instead of passing silently.
check_judged() {
  local injector judged stats
  injector="$(out_path injector.log)"
  judged="$(rig_session_log "$OUT_DIR")"
  if [ ! -f "$injector" ]; then
    echo "no injector record (menu-only drive); judged lines not paired"
    return 77
  fi
  if ! stats="$("$SCRIPT_DIR/pair-stats.sh" "$injector" "$judged" 2>&1)"; then
    echo "injector/judged mismatch: $(printf '%s' "$stats" | tr '\n' ' ')"
    return 1
  fi
  echo "$(printf '%s' "$stats" | head -1)"
  return 0
}

# Three modalities means the lane proved sound, not that a recorder ran: the
# sidecar must declare sound, be captured, and measure above the floor.
check_audio() {
  local sidecar expect state
  sidecar="$(rig_audio_sidecar "$OUT_DIR")"
  if [ ! -f "$sidecar" ]; then
    echo "no audio lane in the pack (expected $sidecar)"
    return 1
  fi
  expect="$(rig_audio_field "$OUT_DIR" expect || true)"
  state="$(rig_audio_field "$OUT_DIR" state || true)"
  if [ "$expect" != "sound" ]; then
    echo "lane declares expect='$expect', an AFK pack needs 'sound'"
    return 1
  fi
  if [ "$state" != "captured" ]; then
    echo "lane state is '$state', expected 'captured'"
    return 1
  fi
  if ! rig_audio_verify "$OUT_DIR" >/dev/null 2>&1; then
    echo "run declared sound but the artifact is silent ($(rig_audio_field "$OUT_DIR" peak_dbfs 2>/dev/null || echo '?') dBFS)"
    return 1
  fi
  echo "$(rig_audio_field "$OUT_DIR" peak_dbfs 2>/dev/null || echo '?') dBFS peak, lane captured"
  return 0
}

# The verifier names each gap on a `  missing: ...` / `  dupe: ...` line; the
# report quotes the first gap, not just the banner.
manifest_gap_line() {
  local verify_log="$1" gap
  gap="$(grep -m1 -E '^  (missing|dupe): ' "$verify_log" 2>/dev/null || true)"
  if [ -n "$gap" ]; then
    printf '%s' "${gap#  }"
    return 0
  fi
  tail -n 1 "$verify_log" 2>/dev/null
}

check_manifest() {
  local verify_log
  verify_log="$(phase_log verify-manifest)"
  if ! timeout 120 "$SCRIPT_DIR/run-manifest.sh" verify "$OUT_DIR" >"$verify_log" 2>&1; then
    echo "run-manifest verify failed: $(manifest_gap_line "$verify_log")"
    return 1
  fi
  echo "artifact set, provenance and audio verified"
  return 0
}

check_rows_file() { printf '%s/phases/checks.tsv' "$OUT_DIR"; }

# One row per check (`name<TAB>status<TAB>detail`) is the machine record the
# report reads back; the same line goes to stdout so an AFK log shows it.
run_check() {
  local name="$1"
  shift
  local detail code=0
  detail="$("$@" 2>&1)" || code=$?
  local status
  case "$code" in
    0) status="PASS" ;;
    77) status="SKIP" ;;
    *) status="FAIL" ;;
  esac
  if [ "$status" = "FAIL" ]; then
    OVERALL="FAILED"
  fi
  CHECK_NAMES+=("$name")
  CHECK_STATUS+=("$status")
  CHECK_DETAILS+=("$detail")
  printf '%s\t%s\t%s\n' "$name" "$status" "$detail" >>"$(check_rows_file)"
  echo "check $name: $status $detail"
}

run_self_checks() {
  mkdir -p "$OUT_DIR/phases"
  : >"$(check_rows_file)"
  run_check scene check_scene
  run_check frames check_frames
  run_check judged check_judged
  run_check audio check_audio
  run_check manifest check_manifest
}

# --- 4. packaging ----------------------------------------------------------
manifest_script_field() {
  local field
  field="--target $TARGET"
  if [ -n "$MUSIC_ID" ]; then
    field="$field --music-id $MUSIC_ID"
  fi
  printf '%s %s' "$(basename "$SCENE_SCRIPT")" "$field"
}

manifest_params_field() {
  local field
  field="display=$DISPLAY pair=$PAIR target=$TARGET"
  if [ -n "$MUSIC_ID" ]; then
    field="$field music-id=$MUSIC_ID"
  fi
  if [ -n "$INJECT_KEYS" ]; then
    field="$field inject=$INJECT_KEYS x$INJECT_COUNT"
  fi
  printf '%s' "$field"
}

# One layout contract (#302): manifests declare artifacts/screenshots, and
# the client writes frames/. Copy each frame into the declared path before
# the manifest is written, and keep frames/ as the client's own output.
stage_screenshots() {
  local frames screenshots frame copied=0
  frames="$(rig_session_frames "$OUT_DIR")"
  screenshots="$(out_path artifacts/screenshots)"
  mkdir -p "$screenshots"
  [ -d "$frames" ] || return 0
  for frame in "$frames"/*.png; do
    [ -f "$frame" ] || continue
    cp -f "$frame" "$screenshots/$(basename "$frame")"
    copied=$((copied + 1))
  done
  echo "rig-afk: staged $copied frame(s) into artifacts/screenshots" \
    >>"$(phase_log package)"
}

write_manifest() {
  timeout 120 "$SCRIPT_DIR/run-manifest.sh" write "$OUT_DIR" \
    --pair "$PAIR" \
    --build "$BUILD" \
    --script "$(manifest_script_field)" \
    --params "$(manifest_params_field)" \
    --seed "$SEED" \
    --input-log "$(rig_session_log "$OUT_DIR")" \
    >>"$(phase_log package)" 2>&1
}

# The exe cannot dump its own pixels, but a client build can: the capture tool
# is the in-process frame dump, and the command line comes from the runner's
# own record because `rig-run.sh stop` reclaims its pidfile.
emit_provenance() {
  local -a args=(
    --producer "$BINARY" --capture-tool client-frame-dump
    --command "$(cat "$(out_path launch.txt)")"
  )
  if [ -f "$(out_path injector.log)" ]; then
    args+=(--injector "$SCRIPT_DIR/rig-inject.sh")
  fi
  timeout 120 "$SCRIPT_DIR/rig-provenance.sh" emit "$OUT_DIR" "${args[@]}" \
    >>"$(phase_log package)" 2>&1
}

package_pack() {
  local code=0
  : >"$(phase_log package)"
  stage_screenshots || code=$?
  if [ "$code" -eq 0 ]; then
    write_manifest || code=$?
  fi
  if [ "$code" -eq 0 ] && [ -r "$(rig_session_log "$OUT_DIR")" ]; then
    if [ "${RIG_AFK_FAULT:-}" = "drop-provenance" ]; then
      # Selftest-only: the manifest self-check must go red when a capture
      # arrives without provenance (the #278 refusal).
      rm -f "$OUT_DIR/provenance.json"
    else
      emit_provenance || code=$?
    fi
  fi
  return "$code"
}

# --- 5. report -------------------------------------------------------------
report_status_file() { printf '%s/%s' "$OUT_DIR" "status"; }

write_report() {
  local status_file report_md report_json index
  status_file="$(report_status_file)"
  report_md="$(out_path report.md)"
  report_json="$(out_path report.json)"
  printf '%s\n' "$OVERALL" >"$status_file"

  {
    echo "# Rig AFK run — $OVERALL"
    echo
    echo "- out-dir: \`$OUT_DIR\`"
    echo "- pair: $PAIR (display ${DISPLAY:-none}, build ${BUILD:-unknown})"
    echo "- drive: $(manifest_script_field)"
    echo "- audio: $(rig_audio_field "$OUT_DIR" peak_dbfs 2>/dev/null || echo 'none') dBFS peak"
    echo "- started: ${STARTED_AT:-unknown}"
    echo "- finished: $(date -Is)"
    echo
    echo "## Self-checks"
    echo
    for index in "${!CHECK_NAMES[@]}"; do
      echo "- \`${CHECK_NAMES[index]}\`: ${CHECK_STATUS[index]} — ${CHECK_DETAILS[index]}"
    done
    echo
    echo "## Artifacts"
    echo
    local relative
    while IFS= read -r relative; do
      echo "- \`$relative\`"
    done < <(cd "$OUT_DIR" && find . -type f ! -name report.json ! -name report.md | sed 's|^\./||' | sort)
    if [ "$OVERALL" = "FAILED" ]; then
      echo
      echo "## Next step"
      echo
      echo "The pack is kept as the failure record: read \`phases/\` for the"
      echo "phase that went red, then fix the cause and re-run the same command."
    fi
  } >"$report_md"

  python3 - "$report_json" "$OUT_DIR" "$PAIR" "$DISPLAY" \
    "$BUILD" "$(manifest_script_field)" "$OVERALL" "$(check_rows_file)" <<'PY'
import json, sys

(path, out_dir, pair, display, build, script, overall, rows) = sys.argv[1:9]
checks = []
with open(rows, encoding="utf-8") as handle:
    for row in handle:
        name, status, detail = row.rstrip("\n").split("\t", 2)
        checks.append({"name": name, "status": status, "detail": detail})
document = {
    "ticket": 280,
    "status": overall,
    "out_dir": out_dir,
    "pair": pair,
    "display": display,
    "build": build,
    "script": script,
    "checks": checks,
}
with open(path, "w", encoding="utf-8") as handle:
    json.dump(document, handle, indent=2)
    handle.write("\n")
PY
}

# --- 6. the run ------------------------------------------------------------
start_phase_dir() {
  mkdir -p "$OUT_DIR/phases"
  : >"$(out_path launch.txt)"
}

# The session options this run carries (one per line, for `mapfile -t`), so
# the recorded command line and the booted command line cannot drift.
session_options() {
  if [ -n "$MUSIC_ID" ]; then
    printf '%s\n' --music-id "$MUSIC_ID"
  fi
}

record_launch() {
  local -a options=()
  mapfile -t options < <(session_options)
  rig_session_command_line "$OUT_DIR" "${options[@]}" >"$(out_path launch.txt)"
}

boot_session() {
  local -a argv=() options=()
  mapfile -t options < <(session_options)
  mapfile -t argv < <(rig_session_argv "$OUT_DIR" "${options[@]}")
  # DISPLAY is this runner's private display, not a caller display. Rig-run
  # reads its inherited DISPLAY as CALLER_DISPLAY and would refuse the run.
  phase_run boot "$BOOT_TIMEOUT" env -u DISPLAY "$SCRIPT_DIR/rig-run.sh" run \
    "$DISPLAY" "$OUT_DIR" -- "$BINARY" "${argv[@]}"
}

drive_scenes() {
  local -a options=(--attach "$OUT_DIR" --display "$DISPLAY" --target "$TARGET")
  local session=()
  mapfile -t session < <(session_options)
  phase_run drive "$DRIVE_TIMEOUT" "$SCENE_SCRIPT" "${options[@]}" "${session[@]}"
}

inject_presses() {
  local injector="${RIG_AFK_INJECTOR:-$SCRIPT_DIR/rig-inject.sh}"
  phase_run inject "$INJECT_TIMEOUT" "$injector" --display "$DISPLAY" \
    --out "$OUT_DIR" --keys "$INJECT_KEYS" --count "$INJECT_COUNT"
}

stop_session() {
  if [ ! -f "$(out_path pids)" ]; then
    # A display may already be claimed when boot fails before rig-run starts.
    rig_release_claims "$OUT_DIR" 2>/dev/null || true
    echo "rig-afk: no pidfile in $OUT_DIR; nothing to stop" >&2
    return 0
  fi
  phase_run stop "$STOP_TIMEOUT" "$SCRIPT_DIR/rig-run.sh" stop "$OUT_DIR"
}

# A failed phase does not abort the run: the pack still gets packaged and
# self-checked, so the report names every red line at once and keeps the
# evidence. Only the report decides the exit code.
report_phase_failure() {
  local name="$1"
  echo "rig-afk: phase '$name' failed (exit $LAST_PHASE_CODE): $(phase_tail)" >&2
  OVERALL="FAILED"
}

# A run never writes into a directory that already holds evidence: a mixed
# pack cannot be cited, and deleting the old one would destroy the record of
# the run that failed.
refuse_occupied_out_dir() {
  if [ -d "$OUT_DIR" ] && [ -n "$(ls -A "$OUT_DIR" 2>/dev/null)" ]; then
    die "$OUT_DIR already holds files; pick a fresh out-dir (this runner never overwrites evidence)"
  fi
}

run_afk() {
  rig_afk_declare_audio
  sanitize_session
  refuse_occupied_out_dir
  STARTED_AT="$(date -Is)"
  BUILD="$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
  SEED="${RIG_AFK_SEED:-$RANDOM$RANDOM}"
  start_phase_dir

  # The runner never adopts the ambient DISPLAY (that is the owner's seat):
  # a display comes from the option or from the allocator, never from the env.
  if [ -n "$PINNED_DISPLAY" ]; then
    DISPLAY="$PINNED_DISPLAY"
  else
    DISPLAY="$(allocate_display)" || die "no free display in $DISPLAY_FIRST..$DISPLAY_LAST (each is up or claimed by a live run)"
  fi
  echo "rig-afk: run $OUT_DIR on $DISPLAY (pair $PAIR, target $TARGET)"
  record_launch

  if ! boot_session; then
    report_phase_failure boot
  else
    if ! drive_scenes; then
      report_phase_failure drive
    elif [ -n "$INJECT_KEYS" ] && ! inject_presses; then
      report_phase_failure inject
    fi
  fi
  if ! stop_session; then
    report_phase_failure stop
  fi
  if ! package_pack; then
    report_phase_failure package
  fi
  run_self_checks
  write_report

  if [ "$OVERALL" = "PASS" ]; then
    echo "rig-afk: PASS $OUT_DIR (report $(out_path report.md))"
    return 0
  fi
  echo "rig-afk: FAILED $OUT_DIR (evidence kept; report $(out_path report.md))" >&2
  return 1
}

# An interrupted AFK run must not leak a booted client or a stray Xvfb: stop
# through the pidfile (never a pattern kill) and keep the pack.
on_interrupt() {
  echo "rig-afk: interrupted; stopping the run by PID (evidence kept in $OUT_DIR)" >&2
  stop_session || true
  OVERALL="FAILED"
  write_report 2>/dev/null || true
  exit 130
}

# --- 7. selftest ------------------------------------------------------------
# Hermetic: stub Xvfb/xdpyinfo, stub PipeWire tools (the audio lane's own
# stubs), a stand-in client binary, a stand-in drive script and a stand-in
# injector. The legs prove the happy pack and red-prove every self-check by
# injecting exactly the failure that check exists for.
selftest_fail() { echo "rig-afk: selftest leg $1 FAIL: $2" >&2; exit 1; }

selftest_leg_pass() { echo "selftest: leg $1 PASS ($2)"; }

write_drive_stub() {  # path
  cat >"$1" <<'STUBEOF'
#!/usr/bin/env bash
# Stand-in drive script: accepts the runner's --attach/--display contract and
# writes the two capture streams a real drive writes (input log + frames).
set -euo pipefail
out=""
while [ $# -gt 0 ]; do
  case "$1" in
    --attach) out="$2"; shift 2 ;;
    *) shift ;;
  esac
done
[ -n "$out" ] || { echo "drive-stub: no --attach out-dir" >&2; exit 2; }
mkdir -p "$out/frames"
log="$out/input.log"
: >"$log"
index=0
for scene in planet channel room "${STUB_FINAL_SCENE:-loading}"; do
  index=$((index + 1))
  printf 'frame %d wall_ms=%d screen=%s state=aaaa content=bbbb\n' \
    "$index" "$((index * 1000))" "$scene" >>"$log"
  printf 'frame-%d' "$index" >"$out/frames/000$index.png"
done
for round in $(seq 1 "${STUB_JUDGED_LINES:-0}"); do
  printf 'judged %d lane %d\n' "$((round * 100))" "$((round % 7))" >>"$log"
done
printf 'event 1 click-up 100 100\n' >>"$log"
STUBEOF
  chmod +x "$1"
}

write_inject_stub() {  # path
  cat >"$1" <<'STUBEOF'
#!/usr/bin/env bash
# Stand-in injector: writes the injector's own epoch-stamped record.
set -euo pipefail
out=""
keys="lane-0"
count=1
while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --keys) keys="$2"; shift 2 ;;
    --count) count="$2"; shift 2 ;;
    *) shift ;;
  esac
done
[ -n "$out" ] || { echo "inject-stub: no --out out-dir" >&2; exit 2; }
: >"$out/injector.log"
epoch=1000
for _ in $(seq 1 "$count"); do
  for key in ${keys//,/ }; do
    epoch=$((epoch + 250))
    printf '%s %s\n' "$epoch" "$key" >>"$out/injector.log"
  done
done
STUBEOF
  chmod +x "$1"
}

write_client_stub() {  # path
  cat >"$1" <<'STUBEOF'
#!/bin/sh
exec sleep 600
STUBEOF
  chmod +x "$1"
}

# The stub world: no Xvfb, no PipeWire, no game, no owner seat, no speakers.
selftest_stub_world() {
  local tmp="$1"
  mkdir -p "$tmp/bin" "$tmp/state" "$tmp/fake-root"
  rig_display_stub_tools "$tmp/bin" "$tmp/state"
  rig_audio_stub_tools "$tmp/bin" "$tmp/state"
  # The stand-in client sits under a target/<profile> path, like the real
  # build: provenance infers the cargo profile from the producer path.
  mkdir -p "$tmp/target/debug"
  write_client_stub "$tmp/target/debug/fake-client"
  write_drive_stub "$tmp/bin/drive-stub"
  write_inject_stub "$tmp/bin/inject-stub"
  SENV=(env -u WAYLAND_DISPLAY -u DISPLAY XDG_SESSION_TYPE=
    "PATH=$tmp/bin:/usr/bin:/bin" "STUBSTATE=$tmp/state"
    "RIG_STATE_DIR=$tmp/rig-state" "RIG_INSTALL_ROOT=$tmp/fake-root"
    "RIG_RELAY_PORTS=59941 59942" "RIG_EXPECT_MEMORYER=0"
    "RIG_AFK_INJECTOR=$tmp/bin/inject-stub")
}

selftest_run() {  # <suffix> [extra KEY=VALUE ...] -- [runner argv ...]
  local suffix="$1"
  shift
  local -a env_pairs=()
  while [ "$#" -gt 0 ] && [ "$1" != "--" ]; do
    env_pairs+=("$1")
    shift
  done
  [ "${1:-}" = "--" ] && shift
  LEG_OUT="$TMP/$suffix"
  LEG_LOG="$TMP/$suffix.out"
  LEG_EXIT=0
  "${SENV[@]}" "${env_pairs[@]}" bash "$0" run "$LEG_OUT" \
    --binary "$TMP/target/debug/fake-client" --scene-script "$TMP/bin/drive-stub" \
    "$@" >"$LEG_LOG" 2>&1 || LEG_EXIT=$?
}

assert_status() {  # <leg> <want>
  local got
  got="$(cat "$LEG_OUT/status" 2>/dev/null || echo missing)"
  if [ "$got" != "$2" ]; then
    sed -n '1,40p' "$LEG_LOG" >&2
    selftest_fail "$1" "status file says '$got', want '$2'"
  fi
}

assert_line() {  # <leg> <needle> — the line must appear verbatim in the run log
  if ! grep -qF -- "$2" "$LEG_LOG"; then
    sed -n '1,40p' "$LEG_LOG" >&2
    selftest_fail "$1" "check line missing from $LEG_LOG: $2"
  fi
}

assert_kept() {  # <leg> <relative path> — evidence survives a failed run
  if [ ! -e "$LEG_OUT/$2" ]; then
    selftest_fail "$1" "evidence not kept: $LEG_OUT/$2"
  fi
}

selftest_leg_happy() {
  local leg="happy-path"
  selftest_run happy STUB_FINAL_SCENE=playing STUB_JUDGED_LINES=7 -- \
    --target playing --min-frames 4 \
    --inject-keys lane-0,lane-1,lane-2,lane-3,lane-4,lane-5,lane-6 || true
  [ "$LEG_EXIT" -eq 0 ] || selftest_fail "$leg" "runner exited $LEG_EXIT (want 0)"
  assert_status "$leg" PASS
  assert_line "$leg" "check scene: PASS reached 'playing'"
  assert_line "$leg" "check frames: PASS 4 frame PNG(s) >= floor 4"
  assert_line "$leg" "check judged: PASS injections=7 judged=7 lanes=7/7"
  assert_line "$leg" "check audio: PASS"
  assert_line "$leg" "check manifest: PASS"
  for kept in report.md report.json provenance.json manifest.json input.log \
    phases/boot.log audio/audio.wav; do
    assert_kept "$leg" "$kept"
  done
  selftest_leg_pass "$leg" "complete pack, five green checks"
}

selftest_leg_ambient_display() {
  local leg="ambient-display"
  selftest_run ambient DISPLAY=:0 -- --target loading || true
  if [ "$LEG_EXIT" -ne 0 ]; then
    sed -n '1,30p' "$LEG_LOG" >&2
    selftest_fail "$leg" "runner exited $LEG_EXIT (want 0)"
  fi
  assert_status "$leg" PASS
  assert_kept "$leg" phases/boot.log
  assert_line "$leg" "check scene: PASS reached 'loading'"
  selftest_leg_pass "$leg" "ambient DISPLAY sanitized; the run booted on a private display"
}

selftest_leg_ambient_display_pinned() {
  local leg="ambient-display-pinned"
  selftest_run ambient-pin DISPLAY=:0 -- --display :0 --target loading || true
  if [ "$LEG_EXIT" -eq 0 ]; then
    sed -n '1,30p' "$LEG_LOG" >&2
    selftest_fail "$leg" "runner exited 0; the owner's display was accepted"
  fi
  assert_line "$leg" "rig-afk: --display :0 is the caller's DISPLAY"
  if [ -e "$LEG_OUT" ]; then
    selftest_fail "$leg" "a refused run created $LEG_OUT (no phase may start)"
  fi
  selftest_leg_pass "$leg" "--display :0 refused before boot when DISPLAY=:0 is ambient"
}

selftest_leg_allocator_isolation() {
  local leg="allocator-isolation"
  local out_a="$TMP/alloc-a" out_b="$TMP/alloc-b"
  "${SENV[@]}" bash "$0" run "$out_a" \
    --binary "$TMP/target/debug/fake-client" --scene-script "$TMP/bin/drive-stub" \
    --target loading >"$TMP/alloc-a.out" 2>&1 &
  local pid_a=$!
  "${SENV[@]}" bash "$0" run "$out_b" \
    --binary "$TMP/target/debug/fake-client" --scene-script "$TMP/bin/drive-stub" \
    --target loading >"$TMP/alloc-b.out" 2>&1 &
  local pid_b=$!
  if ! wait "$pid_a"; then
    sed -n '1,40p' "$TMP/alloc-a.out" >&2
    selftest_fail "$leg" "first concurrent runner failed"
  fi
  if ! wait "$pid_b"; then
    sed -n '1,40p' "$TMP/alloc-b.out" >&2
    selftest_fail "$leg" "second concurrent runner failed"
  fi
  local display_a display_b
  display_a="$(sed -n 's/^rig-afk: run [^ ]* on \(:[0-9]*\).*/\1/p' "$TMP/alloc-a.out" | head -1)"
  display_b="$(sed -n 's/^rig-afk: run [^ ]* on \(:[0-9]*\).*/\1/p' "$TMP/alloc-b.out" | head -1)"
  [ -n "$display_a" ] && [ -n "$display_b" ] ||
    selftest_fail "$leg" "a runner did not report its allocated display"
  [ "$display_a" != "$display_b" ] ||
    selftest_fail "$leg" "both runners allocated $display_a"
  selftest_leg_pass "$leg" "two concurrent AFK runners got $display_a and $display_b"
}

selftest_leg_menu_only() {
  local leg="menu-only-skip"
  selftest_run menu -- --target loading || true
  [ "$LEG_EXIT" -eq 0 ] || selftest_fail "$leg" "runner exited $LEG_EXIT (want 0)"
  assert_line "$leg" "check judged: SKIP no injector record"
  selftest_leg_pass "$leg" "judged check declares the menu-only skip"
}

selftest_leg_scene() {
  local leg="scene-red"
  selftest_run scene STUB_FINAL_SCENE=planet -- --target loading || true
  [ "$LEG_EXIT" -eq 1 ] || selftest_fail "$leg" "runner exited $LEG_EXIT (want 1)"
  assert_status "$leg" FAILED
  assert_line "$leg" "check scene: FAIL probe reports 'planet', expected 'loading'"
  assert_kept "$leg" input.log
  assert_kept "$leg" report.md
  selftest_leg_pass "$leg" "check scene: FAIL probe reports 'planet', expected 'loading'"
}

selftest_leg_frames() {
  local leg="frames-red"
  selftest_run frames -- --target loading --min-frames 99 || true
  [ "$LEG_EXIT" -eq 1 ] || selftest_fail "$leg" "runner exited $LEG_EXIT (want 1)"
  assert_line "$leg" "check frames: FAIL 4 frame PNG(s)"
  assert_line "$leg" "floor 99"
  selftest_leg_pass "$leg" "check frames: FAIL 4 frame PNG(s) in <out>/frames, floor 99"
}

selftest_leg_judged() {
  local leg="judged-red"
  selftest_run judged STUB_FINAL_SCENE=playing STUB_JUDGED_LINES=3 -- \
    --target playing --inject-keys lane-0,lane-1,lane-2,lane-3,lane-4,lane-5,lane-6 || true
  [ "$LEG_EXIT" -eq 1 ] || selftest_fail "$leg" "runner exited $LEG_EXIT (want 1)"
  assert_line "$leg" "check judged: FAIL injector/judged mismatch: FAIL: count mismatch: injections=7 judged=3"
  selftest_leg_pass "$leg" "check judged: FAIL injector/judged mismatch: FAIL: count mismatch: injections=7 judged=3"
}

selftest_leg_audio_disabled() {
  local leg="audio-disabled-red"
  selftest_run audio-off RIG_AUDIO_CAPTURE=0 -- --target loading || true
  [ "$LEG_EXIT" -eq 1 ] || selftest_fail "$leg" "runner exited $LEG_EXIT (want 1)"
  assert_line "$leg" "check audio: FAIL lane declares expect='disabled', an AFK pack needs 'sound'"
  assert_kept "$leg" audio/manifest
  selftest_leg_pass "$leg" "check audio: FAIL lane declares expect='disabled', an AFK pack needs 'sound'"
}

selftest_leg_audio_silent() {
  local leg="audio-silent-red"
  selftest_run audio-silent STUB_AUDIO_SILENT=1 -- --target loading || true
  [ "$LEG_EXIT" -eq 1 ] || selftest_fail "$leg" "runner exited $LEG_EXIT (want 1)"
  assert_line "$leg" "check audio: FAIL run declared sound but the artifact is silent"
  assert_kept "$leg" audio/audio.wav
  selftest_leg_pass "$leg" "check audio: FAIL run declared sound but the artifact is silent (peak ... dBFS)"
}

selftest_leg_manifest() {
  local leg="manifest-red"
  selftest_run manifest RIG_AFK_FAULT=drop-provenance -- --target loading || true
  [ "$LEG_EXIT" -eq 1 ] || selftest_fail "$leg" "runner exited $LEG_EXIT (want 1)"
  assert_line "$leg" "check manifest: FAIL run-manifest verify failed: missing: manifest: no provenance.json"
  assert_kept "$leg" manifest.json
  local line
  line="$(grep -m1 'check manifest: FAIL' "$LEG_LOG")"
  selftest_leg_pass "$leg" "$line"
}

cmd_selftest() {
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT
  selftest_stub_world "$TMP"
  selftest_leg_happy
  selftest_leg_ambient_display
  selftest_leg_ambient_display_pinned
  selftest_leg_allocator_isolation
  selftest_leg_menu_only
  selftest_leg_scene
  selftest_leg_frames
  selftest_leg_judged
  selftest_leg_audio_disabled
  selftest_leg_audio_silent
  selftest_leg_manifest
  echo "rig-afk: selftest OK"
}

# --- 8. CLI ----------------------------------------------------------------
parse_args() {
  OUT_DIR=""
  SCENE_SCRIPT="$SCRIPT_DIR/planet-to-waiting.sh"
  PAIR=""
  TARGET="loading"
  MUSIC_ID=""
  INJECT_KEYS=""
  INJECT_COUNT=1
  MIN_FRAMES=4
  BINARY=""
  PINNED_DISPLAY=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --scene-script) SCENE_SCRIPT="$2"; shift 2 ;;
      --pair) PAIR="$2"; shift 2 ;;
      --target) TARGET="$2"; shift 2 ;;
      --music-id) MUSIC_ID="$2"; shift 2 ;;
      --inject-keys) INJECT_KEYS="$2"; shift 2 ;;
      --inject-count) INJECT_COUNT="$2"; shift 2 ;;
      --min-frames) MIN_FRAMES="$2"; shift 2 ;;
      --display) PINNED_DISPLAY="$2"; shift 2 ;;
      --binary) BINARY="$2"; shift 2 ;;
      -*) die "unknown option: $1" ;;
      *)
        if [ -n "$OUT_DIR" ]; then
          die "one output directory per run (got '$OUT_DIR' and '$1')"
        fi
        OUT_DIR="$1"
        shift
        ;;
    esac
  done
  [ -n "$OUT_DIR" ] || die "usage: rig-afk.sh OUT_DIR [options] (see the header)"
  # The boot call strips DISPLAY for rig-run, so this runner keeps the
  # owner-seat refusal for an explicitly pinned display.
  if [ -n "$PINNED_DISPLAY" ] && [ "$PINNED_DISPLAY" = "$CALLER_DISPLAY" ]; then
    die "--display $PINNED_DISPLAY is the caller's DISPLAY; the client must never open there"
  fi
  [ -f "$SCENE_SCRIPT" ] && [ -x "$SCENE_SCRIPT" ] || die "scene script not executable: $SCENE_SCRIPT"
  case "${PAIR:-native}" in
    native | native-xvfb-xtest) PAIR="native-xvfb-xtest" ;;
    wine | *) die "pair '${PAIR}' is not implemented; the Wine leg waits on #240 (pass --pair native)" ;;
  esac
  if [ -z "$BINARY" ]; then
    BINARY="$(rig_session_binary)"
  fi
  [ -x "$BINARY" ] || die "client binary not executable: $BINARY"
  case "$TARGET" in
    *[!a-z]*) die "--target must be a scene word like loading or playing (got '$TARGET')" ;;
  esac
  case "$MIN_FRAMES" in
    '' | *[!0-9]*) die "--min-frames must be a number (got '$MIN_FRAMES')" ;;
  esac
  case "$INJECT_COUNT" in
    '' | *[!0-9]*) die "--inject-count must be a number (got '$INJECT_COUNT')" ;;
  esac
  if [ -n "$INJECT_KEYS" ] && [ "$TARGET" != "playing" ]; then
    die "--inject-keys needs --target playing (lane presses are only judged while Playing)"
  fi
}

main() {
  local command="${1:-}"
  case "$command" in
    selftest) shift; cmd_selftest ;;
    run) shift; parse_args "$@"; trap on_interrupt INT TERM; run_afk ;;
    "") die "usage: rig-afk.sh OUT_DIR [options] | rig-afk.sh selftest" ;;
    *) parse_args "$@"; trap on_interrupt INT TERM; run_afk ;;
  esac
}

main "$@"
