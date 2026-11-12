#!/usr/bin/env bash
# wayland-loop-smoke.sh — L1 harness case (Refs: #118 #119):
#   "wayland frame-callback starvation keeps the loop idle"
#
# Red->green evidence for the 100%-CPU Wayland wedge fixed in 7c3fcfd
# (winit 0.30.13 arms its frame-callback gate only via
# window.pre_present_notify()). Method:
#
#   1. Start a nested KWin on the current session
#      (kwin_wayland --socket wl-smoke-<pid>), 800x600.
#   2. Start the game binary on that socket with X11 unset, so winit must
#      use the Wayland backend (login failure against the local Memoryer
#      is fine — the loop still runs and paints the splash).
#   3. Minimize the nested-compositor window through the primary session's
#      KWin scripting API (org.kde.kwin.Scripting.loadScript + start).
#      Minimized => the game is starved of frame callbacks: the exact
#      condition that wedged the pre-fix build at ~100% CPU.
#   4. Sample /proc/<game-pid>/stat utime+stime over 5 windows of 6 s.
#      PASS: every window < 20% CPU. FAIL (exit 1): any window > 60%.
#
# Cleanup is PID-recorded only (never pkill -f): the trap verifies
# /proc/<pid>/cmdline still matches before signalling, per
# scripts/rig-run.sh discipline. Total runtime is ~60 s per run.
#
# Usage: scripts/wayland-loop-smoke.sh [--scratch DIR] [GAME-BINARY]
#   GAME-BINARY defaults to <repo>/target/debug/recomp2jam; built with
#   `cargo build` when missing.
#
# Requires: kwin_wayland and qdbus6 on the PATH, XDG_RUNTIME_DIR set, a
# KDE session bus (org.kde.KWin) for the minimize step.
set -euo pipefail

# --- tunables (issue #118 acceptance bands) ---------------------------------
MAP_WAIT_SECONDS=20     # let the game finish startup churn and map before occluding
SETTLE_SECONDS=3        # let the minimize propagate before sampling
WINDOW_COUNT=5          # CPU sampling windows
WINDOW_SECONDS=6        # per-window delta length (5 x 6 s = 30 s sampled)
PASS_MAX_PERCENT=20     # every window below this => PASS
FAIL_MIN_PERCENT=60     # any window at/above this => FAIL
JOURNAL_SINCE="2 minutes ago"

SOCKET_NAME="wl-smoke-$$"
MARKER="wl-smoke-$$"    # KWin script console.log prefix; unique per run

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SCRATCH="$(mktemp -d /tmp/wayland-loop-smoke.XXXXXX)"
GAME_BIN="${1:-"$REPO_ROOT/target/debug/recomp2jam"}"
GAME_PID=""
KWIN_PID=""
NULL_SINK_MODULE=""

log() { echo "wayland-loop-smoke: $*"; }
die() { echo "wayland-loop-smoke: $*" >&2; exit 2; }

# --- cleanup (trap EXIT): signal only recorded PIDs whose /proc cmdline ----
# still matches what we started — PID-reuse discipline of scripts/rig-run.sh.
signal_if_matching() {
  local pid="$1" fragment="$2" tag="$3"
  [ -n "$pid" ] || return 0
  if [ -r "/proc/$pid/cmdline" ] && tr '\0' ' ' < "/proc/$pid/cmdline" | grep -qF "$fragment"; then
    kill "$pid" 2>/dev/null || true
    for _ in $(seq 1 30); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
    kill -9 "$pid" 2>/dev/null || true
    log "stopped $tag (pid $pid)"
  else
    log "skipped $tag (pid $pid no longer matches '$fragment' — not signalling)"
  fi
}

cleanup() {
  signal_if_matching "$GAME_PID" "recomp2jam" "game"
  signal_if_matching "$KWIN_PID" "kwin_wayland" "nested kwin"
  if [ -n "$NULL_SINK_MODULE" ]; then
    pactl unload-module "$NULL_SINK_MODULE" 2>/dev/null || true
  fi
  rm -f "${XDG_RUNTIME_DIR:?}/$SOCKET_NAME"
  rm -rf "$SCRATCH"
}
trap cleanup EXIT

# --- preflight --------------------------------------------------------------
preflight() {
  [ -n "${XDG_RUNTIME_DIR:-}" ] || die "XDG_RUNTIME_DIR is not set"
  command -v kwin_wayland >/dev/null || die "kwin_wayland not found"
  command -v qdbus6 >/dev/null || die "qdbus6 not found (KWin scripting driver)"
  qdbus6 org.kde.KWin /Scripting org.freedesktop.DBus.Introspectable.Introspect >/dev/null 2>&1 \
    || die "org.kde.KWin scripting bus not reachable; this case needs a KDE session"
  if [ ! -x "$GAME_BIN" ]; then
    log "game binary missing; building (cargo build)…"
    cargo build --manifest-path "$REPO_ROOT/Cargo.toml" -p recomp2jam >/dev/null
  fi
  [ -x "$GAME_BIN" ] || die "game binary not found at $GAME_BIN"
}

# --- nested compositor + game ----------------------------------------------
start_nested_kwin() {
  # Nested on the session compositor: keep XDG_RUNTIME_DIR/WAYLAND_DISPLAY,
  # drop DISPLAY so KWin cannot pick the X11 backend.
  env -u DISPLAY kwin_wayland --socket "$SOCKET_NAME" --width 800 --height 600 \
    > "$SCRATCH/kwin.log" 2>&1 &
  KWIN_PID=$!
  echo "$KWIN_PID" > "$SCRATCH/kwin.pid"
  for _ in $(seq 1 50); do
    [ -S "${XDG_RUNTIME_DIR}/$SOCKET_NAME" ] && break
    kill -0 "$KWIN_PID" 2>/dev/null || die "nested kwin died; see $SCRATCH/kwin.log"
    sleep 0.2
  done
  [ -S "${XDG_RUNTIME_DIR}/$SOCKET_NAME" ] || die "nested kwin socket did not appear"
  log "nested kwin pid $KWIN_PID on $SOCKET_NAME"
}

start_game() {
  # Containment: no DISPLAY (force winit's Wayland backend), no
  # WINIT_UNIX_BACKEND override. No --sound: the binary's audio backend is
  # the silent null sink by default (automated runs must never sound).
  env -u DISPLAY -u WINIT_UNIX_BACKEND "WAYLAND_DISPLAY=$SOCKET_NAME" \
    "$GAME_BIN" online \
    --host 127.0.0.1 --port 15010 \
    --user-id 2 --token 77b0a3e3-32e6-4a67-b3d8-563a0df4c08c \
    > "$SCRATCH/game.log" 2>&1 &
  GAME_PID=$!
  echo "$GAME_PID" > "$SCRATCH/game.pid"
  kill -0 "$GAME_PID" 2>/dev/null || die "game died at launch; see $SCRATCH/game.log"
  log "game pid $GAME_PID (login failure against Memoryer is fine; loop still runs)"
  sleep "$MAP_WAIT_SECONDS"
  kill -0 "$GAME_PID" 2>/dev/null || die "game exited during map wait; see $SCRATCH/game.log"
}

# --- occlusion via primary-session KWin scripting ----------------------------
# KWin 6 removed the script-console helper writeFile, but script console.log
# output lands in the user journal, which is our read-back channel. The
# nested compositor window has an EMPTY resourceClass (observed on KWin
# 6.8.2; caption "KDE Wayland Compositor …"), so match by its client PID
# (workspace window `.pid` equals the nested kwin PID we recorded), with
# the caption as belt and braces. A verify pass then reads the minimized
# state back. Fallback if nothing matched: keepBelow + raise another
# window above it.
run_kwin_script() {
  local name="$1" js="$2"
  local file="$SCRATCH/$name.js"
  printf '%s\n' "$js" > "$file"
  qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.loadScript "$file" >/dev/null
  qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.start >/dev/null
}

journal_since_marker() {
  journalctl --user --no-pager -o cat --since "$JOURNAL_SINCE" 2>/dev/null | grep "$MARKER"
}

wait_for_marker_lines() {
  local pattern="$1" tries=40
  while [ "$tries" -gt 0 ]; do
    if journal_since_marker | grep -q "$pattern"; then return 0; fi
    sleep 0.5; tries=$((tries - 1))
  done
  return 1
}

# The nested compositor window has an EMPTY resourceClass on KWin 6.8
# (observed; caption "KDE Wayland Compositor …"), so the scripts below
# match it by PID (w.pid === our recorded kwin pid) or caption. $KWIN_PID
# expands inside the heredocs at generation time, after kwin has started.

minimize_js() {
  cat <<EOF
for (const w of workspace.windowList()) {
  if (w.pid === $KWIN_PID || (w.resourceClass || "").toLowerCase().includes("kwin")
      || (w.caption || "").includes("KDE Wayland Compositor")) {
    w.minimized = true;
    console.log("$MARKER min pid=" + w.pid + " cap=" + w.caption + " minimized=" + w.minimized);
  }
}
EOF
}

verify_js() {
  cat <<EOF
for (const w of workspace.windowList()) {
  if (w.pid === $KWIN_PID || (w.resourceClass || "").toLowerCase().includes("kwin")
      || (w.caption || "").includes("KDE Wayland Compositor")) {
    console.log("$MARKER verify pid=" + w.pid + " minimized=" + w.minimized
                + " keepBelow=" + w.keepBelow);
  }
}
EOF
}

lower_js() {
  cat <<EOF
const nested = w => w.pid === $KWIN_PID || (w.resourceClass || "").toLowerCase().includes("kwin")
  || (w.caption || "").includes("KDE Wayland Compositor");
const kwin = workspace.windowList().filter(nested);
for (const w of kwin) { w.keepBelow = true; }
const rest = workspace.windowList().filter(w => !nested(w) && !w.minimized);
if (rest.length > 0) { workspace.raiseWindow ? workspace.raiseWindow(rest[0]) : rest[0].active = true; }
for (const w of kwin) { console.log("$MARKER lower pid=" + w.pid + " keepBelow=" + w.keepBelow); }
EOF
}

occlude_game_window() {
  run_kwin_script minimize "$(minimize_js)"
  if wait_for_marker_lines "$MARKER min" && wait_for_marker_lines "minimized=true"; then
    run_kwin_script verify "$(verify_js)"
    wait_for_marker_lines "verify pid=.*minimized=true" || die "minimize verify pass printed nothing"
    log "occluded: nested kwin window minimized via KWin scripting"
    journal_since_marker | grep "verify pid" | tail -5 | sed 's/^/wayland-loop-smoke:   /'
    return 0
  fi
  log "minimize found no kwin window (or scripting is a no-op); falling back to keepBelow"
  run_kwin_script lower "$(lower_js)"
  wait_for_marker_lines "$MARKER lower" || die "no kwin window reachable via scripting; see journal for $MARKER"
  log "occluded: nested kwin window lowered via KWin scripting"
}

# --- CPU sampling ------------------------------------------------------------
cpu_ticks() { # utime+stime of $1 in clock ticks (stat fields 14+15)
  awk '{ print $14 + $15 }' "/proc/$1/stat"
}

sample_window() { # percent over $WINDOW_SECONDS
  local pid="$1" start end
  start="$(cpu_ticks "$pid")"
  sleep "$WINDOW_SECONDS"
  end="$(cpu_ticks "$pid")"
  awk -v d=$((end - start)) -v s="$WINDOW_SECONDS" \
    'BEGIN { printf "%.1f", 100.0 * d / ('"$(getconf CLK_TCK)"' * s) }'
}

sample_cpu() {
  SAMPLES=()
  local window
  log "settling ${SETTLE_SECONDS}s after occlusion, then sampling"
  sleep "$SETTLE_SECONDS"
  for i in $(seq 1 "$WINDOW_COUNT"); do
    if [ ! -r "/proc/$GAME_PID/stat" ]; then
      die "game pid $GAME_PID vanished during sampling; see $SCRATCH/game.log"
    fi
    window="$(sample_window "$GAME_PID")"
    SAMPLES+=("$window")
    log "sample $i/$WINDOW_COUNT: ${window}% CPU over ${WINDOW_SECONDS}s"
  done
}

verdict_from_samples() {
  local sample
  # any window in the fail band (>= FAIL_MIN_PERCENT) => FAIL outright
  for sample in "${SAMPLES[@]}"; do
    if awk -v s="$sample" -v f="$FAIL_MIN_PERCENT" 'BEGIN { exit !(s >= f) }'; then
      echo FAIL; return
    fi
  done
  # every window must also be under the pass band (< PASS_MAX_PERCENT);
  # samples in between are an inconclusive band and do not pass
  for sample in "${SAMPLES[@]}"; do
    if ! awk -v s="$sample" -v p="$PASS_MAX_PERCENT" 'BEGIN { exit !(s < p) }'; then
      echo FAIL; return
    fi
  done
  echo PASS
}

# --- main --------------------------------------------------------------------
preflight
start_nested_kwin
start_game
occlude_game_window
sample_cpu
VERDICT="$(verdict_from_samples)"
log "verdict: $VERDICT (samples: ${SAMPLES[*]}% | pass < ${PASS_MAX_PERCENT}% every window, fail >= ${FAIL_MIN_PERCENT}% any window)"
[ "$VERDICT" = PASS ]
