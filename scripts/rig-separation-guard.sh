#!/usr/bin/env bash
# rig-separation-guard.sh — #282 guard for the two-path separation.
#
# Usage: scripts/rig-separation-guard.sh
#
# The guard drives the *real* rig-run entry point through a hermetic stub
# world (stub Xvfb/xdpyinfo and stub pactl/parec). It never opens a real
# display, loads a real sink, or kills a process by pattern.
#
# Caller chain under test:
#   rig-afk.sh run -> (strip ambient session env) -> rig-run.sh run-auto
#   -> rig_allocate_display (registry claim: display -> pid + purpose)
#   -> sanitize_env -> refuse_leaked_wayland -> preflight_run_checks
#   -> rig_check_audio_sink (default sink refused without owner flag)
#   -> write_env_sh (sanitized child env) -> cmd_stop -> rig_release_claims.
#
# Legs:
#   1. leaked Wayland session is refused before any display/sink is touched;
#   2. an explicit default-sink target is refused without the owner flag;
#   3. an agent boot with the owner seat armed uses a private display and a
#      virtual sink; its env.sh and registry entry prove both;
#   4. two concurrent run-auto boots get different displays;
#   5. a pinned display collision fails loudly (10), never kills anything.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUN="$SCRIPT_DIR/rig-run.sh"
# shellcheck source=lib/rig-preflight.sh
. "$SCRIPT_DIR/lib/rig-preflight.sh"
# shellcheck source=rig-audio.sh
. "$SCRIPT_DIR/rig-audio.sh"
# shellcheck source=lib/rig-stub-display.sh
. "$SCRIPT_DIR/lib/rig-stub-display.sh"

guard_fail() { echo "separation-guard: FAIL $*" >&2; exit 1; }
guard_pass() { echo "separation-guard: PASS $*"; }

TMP="$(mktemp -d)"
BIN="$TMP/bin"
STATE="$TMP/state"
OWNER_OUT="$TMP/owner"
export RIG_STATE_DIR="$TMP/rig-state"
export STUBSTATE="$STATE"
mkdir -p "$BIN" "$STATE" "$OWNER_OUT" "$TMP/fake-root" "$RIG_STATE_DIR"
rig_display_stub_tools "$BIN" "$STATE"
rig_audio_stub_tools "$BIN" "$STATE"

# Every rig-run child sees only the stub tools and an isolated claim root.
# The appended KEY=VALUE pairs restore one ambient fact per leg.
SENV=(env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE -u QT_QPA_PLATFORM
  -u PULSE_SINK -u PIPEWIRE_NODE
  "PATH=$BIN:/usr/bin:/bin" "STUBSTATE=$STATE"
  "RIG_STATE_DIR=$RIG_STATE_DIR" "RIG_INSTALL_ROOT=$TMP/fake-root"
  "RIG_RELAY_PORTS=59891 59892" "RIG_EXPECT_MEMORYER=0"
  "RIG_AUDIO_ROUTE_TRIES=5" "RIG_DISPLAY_FIRST=160" "RIG_DISPLAY_LAST=170"
  "RIG_AUDIO_TEST_STREAM_PIDS=4242" "RIG_AUDIO_SKIP_STREAM_BASELINE=1")

# The owner seat is armed in the stub world: display :0 answers, its holder
# is alive, and `pactl get-default-sink` names the owner's speakers.
OWNER_XVFB_PID=""
cleanup() {
  [ -z "$OWNER_XVFB_PID" ] || kill "$OWNER_XVFB_PID" 2>/dev/null || true
  rig_release_claims "$OWNER_OUT" 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT

arm_owner_seat() {
  ( exec env "PATH=$BIN:$PATH" "STUBSTATE=$STATE" Xvfb :0 >/dev/null 2>&1 ) &
  OWNER_XVFB_PID=$!
  printf 'xvfb %s Xvfb\n' "$OWNER_XVFB_PID" > "$OWNER_OUT/pids"
  for _ in $(seq 1 50); do
    [ -f "$STATE/up-:0" ] && break
    sleep 0.1
  done
  [ -f "$STATE/up-:0" ] || guard_fail "owner display :0 did not come up in the stub world"
  rig_claim display :0 "$OWNER_OUT" "$RIG_E_DISPLAY" owner-playtest ||
    guard_fail "cannot record the owner's display claim"
  [ "$(cat "$STATE/default")" = "alsa_output.default" ] ||
    guard_fail "the stub default sink is not the owner's speakers"
}

expect_rejection() {
  local leg="$1" want="$2" fragment="$3" errfile="$4" code=0
  shift 4
  set +e
  "$@" >"$errfile.out" 2>"$errfile.err" || code=$?
  set -e
  [ "$code" -eq "$want" ] || {
    echo "--- stderr ---" >&2; cat "$errfile.err" >&2
    guard_fail "$leg exited $code, want $want"
  }
  grep -qF "$fragment" "$errfile.err" || {
    echo "--- stderr ---" >&2; cat "$errfile.err" >&2
    guard_fail "$leg message missing '$fragment'"
  }
  [ ! -f "$errfile.out/env.sh" ] || guard_fail "$leg wrote env.sh after refusal"
  [ ! -f "$errfile.out/pids" ] || guard_fail "$leg wrote a pidfile after refusal"
  guard_pass "$leg (exit $code: $fragment)"
}

assert_env_sh() {
  local leg="$1" out_dir="$2" display="$3"
  local env_sh="$out_dir/env.sh"
  [ -f "$env_sh" ] || guard_fail "$leg: no env.sh"
  grep -qF "export DISPLAY='$display'" "$env_sh" ||
    guard_fail "$leg: child display is not the private display $display"
  for variable in WAYLAND_DISPLAY XDG_SESSION_TYPE QT_QPA_PLATFORM RIG_OWNER_PATH; do
    grep -qF "unset $variable" "$env_sh" ||
      guard_fail "$leg: env.sh does not unset $variable"
  done
  grep -qF "export PULSE_SINK='rig_null_guard'" "$env_sh" ||
    guard_fail "$leg: child audio is not on the virtual sink"
  grep -qF "export PIPEWIRE_NODE='rig_null_guard'" "$env_sh" ||
    guard_fail "$leg: ALSA child audio is not on the virtual sink"
}

assert_route() {
  local leg="$1" out_dir="$2"
  local route source_sink on_default
  route="$(rig_audio_field "$out_dir" route 2>/dev/null || echo missing)"
  source_sink="$(rig_audio_field "$out_dir" default_sink 2>/dev/null || echo missing)"
  on_default="$(rig_audio_field "$out_dir" on_default_sink 2>/dev/null || echo missing)"
  [ "$route" = "sink" ] || guard_fail "$leg: route is '$route', want 'sink'"
  [ "$source_sink" = "alsa_output.default" ] ||
    guard_fail "$leg: default sink is '$source_sink', want the owner's speakers"
  [ "$on_default" = "0" ] || guard_fail "$leg: $on_default stream(s) on the default sink"
}

# The owner path is play-local.sh -> rig_check_audio_sink(attended). Exercise
# that exact shared seam with the flag; no game or speaker is involved.
leg_owner_flag_seam() {
  local code=0
  set +e
  "${SENV[@]}" RIG_OWNER_PATH=1 bash -c '
    . "$1"
    rig_check_audio_sink attended alsa_output.default
  ' _ "$SCRIPT_DIR/lib/rig-preflight.sh" >"$TMP/owner-flag.out" 2>&1 || code=$?
  set -e
  [ "$code" -eq 0 ] || {
    cat "$TMP/owner-flag.out" >&2
    guard_fail "owner flag did not permit the default sink on the attended path (exit $code)"
  }
  guard_pass "owner flag permits the default sink on the attended owner path (seam)"
}

agent_display() { sed -n "s/^export DISPLAY='\(:[0-9]*\)'$/\1/p" "$1/env.sh" | head -1; }

leg_wayland_leak_refused() {
  expect_rejection "wayland-leak" "$RIG_E_DISPLAY" "WAYLAND_DISPLAY" "$TMP/leak" \
    "${SENV[@]}" DISPLAY=:0 WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland \
    bash "$RUN" run-auto "$TMP/leak" -- sleep 30
}

leg_default_sink_refused() {
  expect_rejection "default-sink" "$RIG_E_AUDIO" "default sink" "$TMP/default" \
    "${SENV[@]}" PULSE_SINK=alsa_output.default \
    bash "$RUN" run-auto "$TMP/default" -- sleep 30
}

leg_agent_boot_sanitized() {
  local out="$TMP/agent" display
  # The AFK boundary strips the ambient session vars (the same boundary that
  # strips DISPLAY for #344); the owner seat and default sink stay armed.
  "${SENV[@]}" DISPLAY=:0 WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland \
    QT_QPA_PLATFORM=wayland env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE \
    -u QT_QPA_PLATFORM RIG_SINK_NAME=rig_null_guard \
    STUB_SINKINPUT_NAME=sleep STUB_SINKINPUT_SINK=rig_null_guard \
    bash "$RUN" run-auto "$out" -- sleep 30 >"$TMP/agent.out" 2>"$TMP/agent.err" ||
    { cat "$TMP/agent.err" >&2; guard_fail "agent boot on an armed owner seat failed"; }
  display="$(agent_display "$out")"
  [ "$display" != ":0" ] || guard_fail "agent boot used the owner's display :0"
  assert_env_sh "agent-boot" "$out" "$display"
  assert_route "agent-boot" "$out"
  grep -qF "display $display" <("$RUN" registry) ||
    guard_fail "agent display $display is absent from the registry"
  grep -qF "purpose rig-run" <("$RUN" registry) ||
    guard_fail "agent display carries no purpose in the registry"
  "${SENV[@]}" bash "$RUN" stop "$out" >/dev/null
  if "$RUN" registry | grep -qF "display $display"; then
    guard_fail "stop did not release display $display from the registry"
  fi
  guard_pass "agent boot on armed owner seat used $display, not :0 or the default sink"
}

leg_concurrent_allocations() {
  local first="$TMP/c1" second="$TMP/c2" d1 d2
  "${SENV[@]}" bash "$RUN" run-auto "$first" -- sleep 30 >"$TMP/c1.out" 2>"$TMP/c1.err" &
  local first_pid=$!
  "${SENV[@]}" bash "$RUN" run-auto "$second" -- sleep 30 >"$TMP/c2.out" 2>"$TMP/c2.err" &
  local second_pid=$!
  wait "$first_pid" || { cat "$TMP/c1.err" >&2; guard_fail "concurrent run 1 failed"; }
  wait "$second_pid" || { cat "$TMP/c2.err" >&2; guard_fail "concurrent run 2 failed"; }
  d1="$(agent_display "$first")"
  d2="$(agent_display "$second")"
  [ -n "$d1" ] && [ -n "$d2" ] || guard_fail "concurrent runs did not record displays"
  [ "$d1" != "$d2" ] || guard_fail "two concurrent runs both got $d1"
  "${SENV[@]}" bash "$RUN" stop "$first" >/dev/null
  "${SENV[@]}" bash "$RUN" stop "$second" >/dev/null
  guard_pass "two concurrent runs got $d1 and $d2"
}

leg_pinned_collision() {
  local first="$TMP/p1" second="$TMP/p2"
  "${SENV[@]}" bash "$RUN" run :165 "$first" -- sleep 30 >"$TMP/p1.out" 2>"$TMP/p1.err" ||
    { cat "$TMP/p1.err" >&2; guard_fail "pinned setup run failed"; }
  expect_rejection "pinned-collision" "$RIG_E_DISPLAY" "already claimed" "$TMP/p2" \
    "${SENV[@]}" bash "$RUN" run :165 "$second" -- sleep 30
  "${SENV[@]}" bash "$RUN" stop "$first" >/dev/null
  guard_pass "a pinned display collision failed loudly without killing the first run"
}

main() {
  arm_owner_seat
  leg_wayland_leak_refused
  leg_default_sink_refused
  leg_owner_flag_seam
  leg_agent_boot_sanitized
  leg_concurrent_allocations
  leg_pinned_collision
  echo "separation-guard: all legs green"
}

main "$@"
