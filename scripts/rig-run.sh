#!/usr/bin/env bash
# rig-run.sh — launch hygiene for Wine/X rig sessions (retro 2026-11-11, issue #61).
#
# Usage:
#   scripts/rig-run.sh run <display> <out-dir> -- <binary> [args...]
#   scripts/rig-run.sh stop <out-dir>
#   scripts/rig-run.sh status <out-dir>
#   scripts/rig-run.sh selftest
#
# run: enforces the shared preflight (scripts/lib/rig-preflight.sh) — no
#     conflicting display/port/prefix claims, no occupied relay ports,
#     virtual/null audio sink present and selected (NEVER the speakers),
#     Memoryer up and answering (unless RIG_EXPECT_MEMORYER=0), install-root
#     readable — then starts Xvfb on <display> when absent, launches <binary>
#     in the background with sanitized env, and records every PID it started
#     in <out-dir>/pids and every claim it owns in <out-dir>/claims.
# stop: kills only the PIDs recorded in <out-dir>/pids, verifying each
#     process still matches what was started. Never pattern-kills. Cleanup
#     failures are recorded in <out-dir>/cleanup-failures.log, the evidence
#     (pidfile, logs, claims) is kept, and stop exits 16.
#
# Refusals:
#   - malformed invocations exit 2 (usage errors)
#   - display conflicts exit 10 (:98 reserved; the caller's own DISPLAY;
#     a leaked Wayland environment; a display claimed by a live run)
#   - port/audio/server/data/prefix rejections exit 11/12/13/14/15
#     (see scripts/lib/rig-preflight.sh)
set -euo pipefail

RIG_LIB="$(cd "$(dirname "$0")/lib" && pwd)/rig-preflight.sh"
# shellcheck source=lib/rig-preflight.sh
. "$RIG_LIB"

die() { echo "rig-run: $*" >&2; exit 2; }

fail_with() { local code="$1"; shift; echo "rig-run: $*" >&2; exit "$code"; }

# --- environment capture (before any sanitizing) -------------------------
CALLER_DISPLAY="${DISPLAY:-}"
CALLER_WAYLAND="${WAYLAND_DISPLAY:-}"

sanitize_env() {
  unset WAYLAND_DISPLAY XDG_SESSION_TYPE QT_QPA_PLATFORM 2>/dev/null || true
  export WINIT_UNIX_BACKEND=x11
  # Containment default: automated audio never reaches the speakers. An
  # explicitly chosen sink still passes through the null-sink preflight.
  : "${PULSE_SINK:=wine_null}"
  export PULSE_SINK
}

refuse_leaked_wayland() {
  [ -n "$CALLER_WAYLAND" ] && fail_with "$RIG_E_DISPLAY" "WAYLAND_DISPLAY is set ('$CALLER_WAYLAND'); open a clean shell or unset it, then re-run (a Wayland leak once put the client window on the real desktop)"
  [ "${XDG_SESSION_TYPE:-}" = "wayland" ] && fail_with "$RIG_E_DISPLAY" "XDG_SESSION_TYPE=wayland; run from an X session or unset it before driving the rig"
  return 0
}

validate_display() {
  local d="${1:-}"
  [ -n "$d" ] || die "no display given (expected e.g. :99)"
  case "$d" in
    :98) fail_with "$RIG_E_DISPLAY" ":98 is refused; pick another display for this session" ;;
  esac
  if [ -n "$CALLER_DISPLAY" ] && [ "$d" = "$CALLER_DISPLAY" ]; then
    fail_with "$RIG_E_DISPLAY" "display $d is your real DISPLAY; the client must never open there"
  fi
  return 0
}

display_up() { xdpyinfo -display "$1" >/dev/null 2>&1; }

# --- run ------------------------------------------------------------------
preflight_release() {
  local out_dir="$1"
  rig_release_sink "$out_dir" "$out_dir/cleanup-failures.log" 2>/dev/null || true
  rig_release_claims "$out_dir" 2>/dev/null || true
}

preflight_run_checks() {
  local display="$1" out_dir="$2" owner code sink repo_root
  repo_root="$(cd "$(dirname "$0")/.." && pwd)"
  RIG_INSTALL_ROOT="${RIG_INSTALL_ROOT:-$repo_root/install-root/O2Jam}"
  export RIG_INSTALL_ROOT
  if display_up "$display"; then
    if owner="$(rig_claimed_by display "$display")"; then
      rig_fail "$RIG_E_DISPLAY" "display-claim" "display $display already claimed by $owner"
      return "$RIG_E_DISPLAY"
    fi
    rig_note "display $display already up with no live owner; adopting"
  fi
  rig_claim "display" "$display" "$out_dir" "$RIG_E_DISPLAY" || return "$?"
  rig_check_install_root || { code=$?; preflight_release "$out_dir"; return "$code"; }
  rig_check_memoryer || { code=$?; preflight_release "$out_dir"; return "$code"; }
  rig_check_relay_ports_free || { code=$?; preflight_release "$out_dir"; return "$code"; }
  sink="${RIG_SINK_NAME:-$PULSE_SINK}"
  export PULSE_SINK="$sink"
  rig_check_audio_sink null "$sink" || { code=$?; preflight_release "$out_dir"; return "$code"; }
  rig_check_prefix "$out_dir" || { code=$?; preflight_release "$out_dir"; return "$code"; }
  rig_claim_listen_ports "$out_dir" || { code=$?; preflight_release "$out_dir"; return "$code"; }
  return 0
}

cmd_run() {
  local display="$1" out_dir="$2"; shift 2
  [ "${1:-}" = "--" ] && shift
  [ "$#" -ge 1 ] || die "run: no binary given after --"
  refuse_leaked_wayland
  validate_display "$display"
  sanitize_env
  mkdir -p "$out_dir"
  : > "$out_dir/claims"
  export RIG_CLAIMS_FILE="$out_dir/claims"
  preflight_run_checks "$display" "$out_dir" || exit "$?"

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
process_gone() {
  local pid="$1" state
  kill -0 "$pid" 2>/dev/null || return 0
  state="$(awk '{print $3}' "/proc/$pid/stat" 2>/dev/null || true)"
  [ "$state" = "Z" ] || [ -z "$state" ]
}

kill_recorded() {
  local out_dir="$1" failures="$2" failed=0
  [ -f "$out_dir/pids" ] || die "no pidfile at $out_dir/pids"
  while read -r tag pid frag; do
    [ -n "${pid:-}" ] || continue
    if [ -r "/proc/$pid/cmdline" ] && tr '\0' ' ' < "/proc/$pid/cmdline" | grep -qF "$frag"; then
      kill "$pid" 2>/dev/null || true
      for _ in $(seq 1 20); do process_gone "$pid" && break; sleep 0.1; done
      kill -9 "$pid" 2>/dev/null || true
      sleep 0.2
      if process_gone "$pid"; then
        echo "rig-run: stopped $tag (pid $pid)"
      else
        echo "process $tag (pid $pid) survived SIGKILL" >> "$failures"
        echo "rig-run: FAILED to stop $tag (pid $pid) — recorded, evidence kept"
        failed=1
      fi
    else
      echo "rig-run: skipped $tag (pid $pid no longer matches '$frag' — not signalling)"
    fi
  done < "$out_dir/pids"
  return "$failed"
}

cmd_stop() {
  local out_dir="$1" code=0
  local failures="$out_dir/.cleanup-failures.tmp"
  : > "$failures"
  if ! kill_recorded "$out_dir" "$failures"; then code="$RIG_E_CLEANUP"; fi
  rig_release_sink "$out_dir" "$failures"
  if [ -s "$failures" ]; then code="$RIG_E_CLEANUP"; fi
  if [ "$code" -ne 0 ]; then
    mv -f "$failures" "$out_dir/cleanup-failures.log"
    echo "rig-run: cleanup FAILED (exit $code); pidfile, logs, and claims kept in $out_dir" >&2
    return "$code"
  fi
  rm -f "$failures" "$out_dir/pids"
  rig_release_claims "$out_dir" || true
  return 0
}

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
  preflight_legs "$tmp"
  rm -rf "$tmp"
  echo "rig-run: selftest OK"
}

# --- #206 preflight legs -----------------------------------------------------
# Hermetic stubs under $1 (no real Xvfb/audio/server): each leg drives `run`
# with one bad fixture and asserts a distinct rejection exit code. These legs
# fail until run() calls the shared preflight (failing-first, TDD red).
attempt() {
  local errfile="$1"; shift
  ATTEMPT_EXIT=0
  "$@" 2>"$errfile" || ATTEMPT_EXIT=$?
  return 0
}

# stop_quiet stops with the hermetic stub env from preflight_legs (dynamic
# scope: senv is local to preflight_legs, the only caller; the pactl stub
# owns the sinks, so the real audio state is never touched).
stop_quiet() { "${senv[@]}" bash "$0" stop "$1" >/dev/null 2>&1 || true; }

assert_reject() {
  local leg="$1" want="$2" frag="$3" errfile="$4"
  if [ "$ATTEMPT_EXIT" -ne "$want" ]; then
    echo "selftest: leg $leg FAIL: exit $ATTEMPT_EXIT, want $want" >&2
    cat "$errfile" >&2
    return 1
  fi
  if ! grep -qF "$frag" "$errfile"; then
    echo "selftest: leg $leg FAIL: message missing '$frag'" >&2
    cat "$errfile" >&2
    return 1
  fi
  echo "selftest: leg $leg PASS (rejected exit $ATTEMPT_EXIT)"
  return 0
}

write_stubs() {
  local stubbin="$1" stubstate="$2"
  mkdir -p "$stubbin" "$stubstate"
  cat >"$stubbin/xdpyinfo" <<'STUBEOF'
#!/bin/sh
if [ -f "$STUBSTATE/up-$2" ]; then exit 0; else exit 1; fi
STUBEOF
  cat >"$stubbin/Xvfb" <<'STUBEOF'
#!/bin/sh
touch "$STUBSTATE/up-$1"
exec sleep 120
STUBEOF
  cat >"$stubbin/pactl" <<'STUBEOF'
#!/bin/sh
extra="$STUBSTATE/sinks-extra"
if [ "$1" = "list" ]; then
  if [ "$STUB_SINK_MISSING" != "1" ]; then
    printf '7\twine_null\tPipeWire\ts16le 2ch 44100Hz\tIDLE\n'
  else
    printf '64\talsa_output.usb-ADAPTER\tPipeWire\ts24le 2ch 48000Hz\tSUSPENDED\n'
  fi
  [ -f "$extra" ] && cat "$extra"
elif [ "$1" = "load-module" ]; then
  name=""; for a in "$@"; do case "$a" in sink_name=*) name="${a#sink_name=}";; esac; done
  printf '99\t%s\tPipeWire\ts16le 2ch 44100Hz\tIDLE\n' "$name" >> "$extra"
  echo 99
elif [ "$1" = "unload-module" ]; then
  if [ "$STUB_UNLOAD_FAIL" = "1" ]; then exit 1; fi
  touch "$STUBSTATE/unloaded-$2"
elif [ "$1" = "get-default-sink" ]; then
  echo alsa_output.usb-ADAPTER
fi
exit 0
STUBEOF
  chmod +x "$stubbin/xdpyinfo" "$stubbin/Xvfb" "$stubbin/pactl"
}

preflight_legs() {
  local tmp="$1" stubbin="$tmp/stubbin" stubstate="$tmp/stubstate"
  write_stubs "$stubbin" "$stubstate"
  mkdir -p "$tmp/fake-root"
  local senv=(env -u WAYLAND_DISPLAY -u DISPLAY XDG_SESSION_TYPE=
    "PATH=$stubbin:/usr/bin:/bin" "STUBSTATE=$stubstate"
    "RIG_STATE_DIR=$tmp/rig-state" "RIG_INSTALL_ROOT=$tmp/fake-root"
    "RIG_RELAY_PORTS=59891 59892" "RIG_EXPECT_MEMORYER=0")

  # Occupied relay port must reject with 11 (fixture: python listener).
  python3 - "$stubstate/relay-ready" 59891 <<'PYEOF' &
import socket, sys, time
ready, port = sys.argv[1], int(sys.argv[2])
sock = socket.socket()
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.bind(("127.0.0.1", port))
sock.listen(1)
open(ready, "w").write("ok")
time.sleep(30)
PYEOF
  local relay_pid=$!
  for _ in $(seq 1 50); do [ -f "$stubstate/relay-ready" ] && break; sleep 0.1; done
  [ -f "$stubstate/relay-ready" ] || die "selftest: relay stub did not start"
  attempt "$tmp/err-relay" "${senv[@]}" bash "$0" run :188 "$tmp/o-relay" -- sleep 30
  kill "$relay_pid" 2>/dev/null || true; wait "$relay_pid" 2>/dev/null || true
  stop_quiet "$tmp/o-relay"
  assert_reject "relay-occupied" 11 "relay port" "$tmp/err-relay"

  # Dead Memoryer must reject with 13 (nothing listens on 59998).
  attempt "$tmp/err-server" "${senv[@]}" RIG_EXPECT_MEMORYER=1 RIG_MEMORYER_PORT=59998 \
    bash "$0" run :187 "$tmp/o-server" -- sleep 30
  stop_quiet "$tmp/o-server"
  assert_reject "dead-server" 13 "Memoryer" "$tmp/err-server"

  # Missing null sink with creation off must reject with 12.
  attempt "$tmp/err-sink" "${senv[@]}" STUB_SINK_MISSING=1 RIG_CREATE_SINK=0 \
    bash "$0" run :186 "$tmp/o-sink" -- sleep 30
  stop_quiet "$tmp/o-sink"
  assert_reject "missing-sink" 12 "sink" "$tmp/err-sink"

  # Unreadable install-root must reject with 14.
  attempt "$tmp/err-data" "${senv[@]}" RIG_INSTALL_ROOT="$tmp/does-not-exist" \
    bash "$0" run :185 "$tmp/o-data" -- sleep 30
  stop_quiet "$tmp/o-data"
  assert_reject "bad-data" 14 "install-root" "$tmp/err-data"

  # Second run on a claimed display must reject with 10.
  attempt "$tmp/err-dsp-a" "${senv[@]}" bash "$0" run :192 "$tmp/o-dsp-a" -- sleep 30
  [ "$ATTEMPT_EXIT" -eq 0 ] || die "selftest: leg display-claim setup run failed"
  attempt "$tmp/err-dsp-b" "${senv[@]}" bash "$0" run :192 "$tmp/o-dsp-b" -- sleep 30
  stop_quiet "$tmp/o-dsp-a"
  stop_quiet "$tmp/o-dsp-b"
  assert_reject "display-claim" 10 "already claimed" "$tmp/err-dsp-b"

  # Second run on a claimed Wine prefix must reject with 15.
  attempt "$tmp/err-pfx-a" "${senv[@]}" RIG_WINEPREFIX="$tmp/p1" \
    bash "$0" run :193 "$tmp/o-pfx-a" -- sleep 30
  [ "$ATTEMPT_EXIT" -eq 0 ] || die "selftest: leg prefix-claim setup run failed"
  attempt "$tmp/err-pfx-b" "${senv[@]}" RIG_WINEPREFIX="$tmp/p1" \
    bash "$0" run :194 "$tmp/o-pfx-b" -- sleep 30
  stop_quiet "$tmp/o-pfx-a"
  stop_quiet "$tmp/o-pfx-b"
  assert_reject "prefix-claim" 15 "already claimed" "$tmp/err-pfx-b"

  # Second run claiming the same listen port must reject with 11.
  attempt "$tmp/err-port-a" "${senv[@]}" RIG_LISTEN_PORTS=59401 \
    bash "$0" run :195 "$tmp/o-port-a" -- sleep 30
  [ "$ATTEMPT_EXIT" -eq 0 ] || die "selftest: leg port-claim setup run failed"
  attempt "$tmp/err-port-b" "${senv[@]}" RIG_LISTEN_PORTS=59401 \
    bash "$0" run :196 "$tmp/o-port-b" -- sleep 30
  stop_quiet "$tmp/o-port-a"
  stop_quiet "$tmp/o-port-b"
  assert_reject "port-claim" 11 "already claimed" "$tmp/err-port-b"

  # Owned per-run sink lifecycle: created on run, unloaded on stop.
  attempt "$tmp/err-lc" "${senv[@]}" STUB_SINK_MISSING=1 RIG_SINK_NAME=rig_null_197 \
    bash "$0" run :197 "$tmp/o-lc" -- sleep 30
  local lc_exit=$ATTEMPT_EXIT
  stop_quiet "$tmp/o-lc"
  [ "$lc_exit" -eq 0 ] || { cat "$tmp/err-lc" >&2; die "selftest: leg sink-create-unload setup run failed"; }
  grep -q "sinkmod" "$tmp/o-lc/claims" || die "selftest: leg sink-create-unload FAILED: no sinkmod claim"
  [ -f "$stubstate/unloaded-99" ] || die "selftest: leg sink-create-unload FAILED: owned sink not unloaded"
  echo "selftest: leg sink-create-unload PASS"

  # Cleanup failure must exit 16 and keep the evidence (pidfile + log).
  attempt "$tmp/err-cl-a" "${senv[@]}" STUB_SINK_MISSING=1 RIG_SINK_NAME=rig_null_198 \
    bash "$0" run :198 "$tmp/o-clean" -- sleep 30
  [ "$ATTEMPT_EXIT" -eq 0 ] || { cat "$tmp/err-cl-a" >&2; die "selftest: leg cleanup-failure setup run failed"; }
  attempt "$tmp/err-clean" "${senv[@]}" STUB_SINK_MISSING=1 STUB_UNLOAD_FAIL=1 \
    bash "$0" stop "$tmp/o-clean"
  local kept=0 logged=0
  [ -f "$tmp/o-clean/pids" ] && kept=1
  [ -f "$tmp/o-clean/cleanup-failures.log" ] && logged=1
  grep -q "unload" "$tmp/o-clean/cleanup-failures.log" 2>/dev/null || logged=0
  stop_quiet "$tmp/o-clean"
  [ "$kept" = 1 ] || die "selftest: leg cleanup-failure FAILED: pidfile deleted despite failure"
  [ "$logged" = 1 ] || die "selftest: leg cleanup-failure FAILED: no cleanup-failures.log"
  assert_reject "cleanup-failure" 16 "cleanup" "$tmp/err-clean"
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
