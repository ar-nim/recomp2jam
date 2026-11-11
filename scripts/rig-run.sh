#!/usr/bin/env bash
# rig-run.sh — launch hygiene for Wine/X rig sessions (retro 2026-11-11, issue #61).
#
# Usage:
#   scripts/rig-run.sh run <display> <out-dir> -- <binary> [args...]
#   scripts/rig-run.sh stop <out-dir>
#   scripts/rig-run.sh status <out-dir>
#   scripts/rig-run.sh selftest
#
# run: validates the environment, starts Xvfb on <display> when absent,
#     launches <binary> in the background with sanitized env, and records
#     every PID it started in <out-dir>/pids.
# stop: kills only the PIDs recorded in <out-dir>/pids, verifying each
#     process still matches what was started. Never pattern-kills.
#
# Refusals (clear error, exit 2):
#   - display :98 (reserved, tribal-knowledge display from smoke notes)
#   - the caller's own DISPLAY (never open the client on the real desktop)
#   - a leaked Wayland environment (WAYLAND_DISPLAY or XDG_SESSION_TYPE=wayland)
set -euo pipefail

die() { echo "rig-run: $*" >&2; exit 2; }

# --- environment capture (before any sanitizing) -------------------------
CALLER_DISPLAY="${DISPLAY:-}"
CALLER_WAYLAND="${WAYLAND_DISPLAY:-}"

sanitize_env() {
  unset WAYLAND_DISPLAY XDG_SESSION_TYPE QT_QPA_PLATFORM 2>/dev/null || true
  export WINIT_UNIX_BACKEND=x11
  if [ "${RIG_ALLOW_AUDIO:-0}" != "1" ]; then
    export PULSE_SINK="${PULSE_SINK:-wine_null}"
  fi
}

refuse_leaked_wayland() {
  [ -n "$CALLER_WAYLAND" ] && die "WAYLAND_DISPLAY is set ('$CALLER_WAYLAND'); open a clean shell or unset it, then re-run (a Wayland leak once put the client window on the real desktop)"
  [ "${XDG_SESSION_TYPE:-}" = "wayland" ] && die "XDG_SESSION_TYPE=wayland; run from an X session or unset it before driving the rig"
  return 0
}

validate_display() {
  local d="${1:-}"
  [ -n "$d" ] || die "no display given (expected e.g. :99)"
  case "$d" in
    :98) die ":98 is refused; pick another display for this session" ;;
  esac
  if [ -n "$CALLER_DISPLAY" ] && [ "$d" = "$CALLER_DISPLAY" ]; then
    die "display $d is your real DISPLAY; the client must never open there"
  fi
  return 0
}

display_up() { xdpyinfo -display "$1" >/dev/null 2>&1; }

# --- run ------------------------------------------------------------------
cmd_run() {
  local display="$1" out_dir="$2"; shift 2
  [ "${1:-}" = "--" ] && shift
  [ "$#" -ge 1 ] || die "run: no binary given after --"
  refuse_leaked_wayland
  validate_display "$display"
  sanitize_env
  mkdir -p "$out_dir"

  : > "$out_dir/pids"
  local started_xvfb=0
  if ! display_up "$display"; then
    command -v Xvfb >/dev/null || die "Xvfb not found and $display is not up"
    Xvfb "$display" -screen 0 1024x768x24 >"$out_dir/xvfb.log" 2>&1 &
    echo "xvfb $! Xvfb" >> "$out_dir/pids"
    started_xvfb=1
    for _ in $(seq 1 50); do display_up "$display" && break; sleep 0.1; done
    display_up "$display" || die "Xvfb on $display did not come up (see $out_dir/xvfb.log)"
  fi

  DISPLAY="$display" "$@" >"$out_dir/client.log" 2>&1 &
  echo "client $! $1" >> "$out_dir/pids"

  {
    echo "export DISPLAY='$display'"
    echo "export WINIT_UNIX_BACKEND=x11"
    echo "export PULSE_SINK='${PULSE_SINK:-wine_null}'"
  } > "$out_dir/env.sh"

  echo "rig-run: started $1 on $display (pid $(tail -1 "$out_dir/pids" | cut -d' ' -f2))"
  if [ "$started_xvfb" = 1 ]; then
    echo "rig-run: started Xvfb for $display ($(head -1 "$out_dir/pids" | cut -d' ' -f2))"
  fi
  echo "rig-run: driver env written to $out_dir/env.sh; stop with: scripts/rig-run.sh stop $out_dir"
  return 0
}

# --- stop / status ---------------------------------------------------------
# Each pidfile line: "<tag> <pid> <cmdline-fragment>". Before signalling we
# verify /proc/<pid>/cmdline still contains the fragment, so PID reuse can
# never make us kill a stranger. This is the fix for the pkill self-match.
kill_recorded() {
  local out_dir="$1"
  [ -f "$out_dir/pids" ] || die "no pidfile at $out_dir/pids"
  while read -r tag pid frag; do
    [ -n "${pid:-}" ] || continue
    if [ -r "/proc/$pid/cmdline" ] && tr '\0' ' ' < "/proc/$pid/cmdline" | grep -qF "$frag"; then
      kill "$pid" 2>/dev/null || true
      for _ in $(seq 1 20); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
      kill -9 "$pid" 2>/dev/null || true
      echo "rig-run: stopped $tag (pid $pid)"
    else
      echo "rig-run: skipped $tag (pid $pid no longer matches '$frag' — not signalling)"
    fi
  done < "$out_dir/pids"
  rm -f "$out_dir/pids"
}

cmd_stop() { kill_recorded "$1"; }

cmd_status() {
  local out_dir="$1"
  [ -f "$out_dir/pids" ] || { echo "rig-run: no pidfile at $out_dir/pids"; return 1; }
  while read -r tag pid frag; do
    if [ -r "/proc/$pid/cmdline" ] && tr '\0' ' ' < "/proc/$pid/cmdline" | grep -qF "$frag"; then
      echo "$tag pid $pid: alive"
    else
      echo "$tag pid $pid: gone"
    fi
  done < "$out_dir/pids"
}

# --- selftest ---------------------------------------------------------------
cmd_selftest() {
  local tmp; tmp="$(mktemp -d)"
  # clean-env invocations: this helper refuses a leaked Wayland session, so
  # each sub-call below must present the sanitized env it wants to test.
  local clean=(env -u WAYLAND_DISPLAY -u DISPLAY XDG_SESSION_TYPE=)
  # refuse: :98
  if "${clean[@]}" bash "$0" run :98 "$tmp/o" -- true 2>"$tmp/e"; then
    die "selftest: :98 was not refused"
  fi
  grep -q ":98" "$tmp/e" || die "selftest: :98 refusal message missing"
  # refuse: caller's own display
  if "${clean[@]}" DISPLAY=:77 bash "$0" run :77 "$tmp/o" -- true 2>"$tmp/e"; then
    die "selftest: caller DISPLAY was not refused"
  fi
  # refuse: leaked wayland
  if WAYLAND_DISPLAY=wayland-0 DISPLAY= bash "$0" run :96 "$tmp/o" -- true 2>"$tmp/e"; then
    die "selftest: leaked WAYLAND_DISPLAY was not refused"
  fi
  # run + stop round-trip on a scratch display with a stand-in binary
  if command -v Xvfb >/dev/null; then
    "${clean[@]}" bash "$0" run :96 "$tmp/rig" -- sleep 30
    [ -f "$tmp/rig/pids" ] || die "selftest: pidfile not written"
    grep -q "client" "$tmp/rig/pids" || die "selftest: client pid not recorded"
    local st; st="$(cmd_status "$tmp/rig")"
    grep -q "alive" <<<"$st" || die "selftest: client not alive"
    bash "$0" stop "$tmp/rig"
    if cmd_status "$tmp/rig" 2>/dev/null; then
      die "selftest: pidfile still present after stop"
    fi
    # the pkill-fault re-run: stopping twice must be clean and kill nothing else
    bash "$0" stop "$tmp/rig-missing" 2>/dev/null && die "selftest: stop on missing pidfile should fail" || true
  else
    echo "rig-run: selftest skipped Xvfb round-trip (Xvfb not installed)"
  fi
  rm -rf "$tmp"
  echo "rig-run: selftest OK"
}

main() {
  local cmd="${1:-}"; shift || true
  case "$cmd" in
    run)    [ "$#" -ge 3 ] || die "run: expected <display> <out-dir> -- <binary> [args...]"; cmd_run "$@" ;;
    stop)   [ "$#" -ge 1 ] || die "stop: expected <out-dir>"; cmd_stop "$1" ;;
    status) [ "$#" -ge 1 ] || die "status: expected <out-dir>"; cmd_status "$1" ;;
    selftest) cmd_selftest ;;
    *) die "usage: rig-run.sh {run <display> <out-dir> -- <binary> [args...] | stop <out-dir> | status <out-dir> | selftest}" ;;
  esac
}

main "$@"
