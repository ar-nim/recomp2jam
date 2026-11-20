#!/usr/bin/env bash
# rig-client-display.sh — X11 display pin for boots of OUR Rust client (#719).
#
# Source this file from a launcher, or run it directly for hermetic checks:
#   scripts/lib/rig-client-display.sh selftest
#   scripts/lib/rig-client-display.sh probe        # print the pinned env
#   scripts/lib/rig-client-display.sh clear-prefix -- CLIENT [ARGS...]
#
# The #719 leak: play-local.sh used to re-add WAYLAND_DISPLAY=wayland-1 when
# an X11 rig had cleared it, so the Rust client opened on the owner's KWin
# session instead of the rig's Xvfb display. A rig boot must always carry an
# explicit :NNN display, no Wayland socket, and no Wine prefix (prefixes are
# Wine-only state; an inherited one also makes rig-run.sh's preflight refuse
# the native peer with exit 15).
set -euo pipefail

RIG_E_DISPLAY="${RIG_E_DISPLAY:-10}"
# Rig Xvfb displays are three digits (:150-:199 by default). The owner's
# session is :0; one- and two-digit displays are never rig boots.
RIG_RUST_DISPLAY_PATTERN=':[1-9][0-9][0-9]'

rig_requested_client_display() {
  printf '%s' "${RIG_DISPLAY:-${DISPLAY:-}}"
}

# Query: prints the validated rig display, or refuses before any client boot.
rig_validated_client_display() {
  local requested caller_note=""
  requested="$(rig_requested_client_display)"
  case "$requested" in
    $RIG_RUST_DISPLAY_PATTERN) ;;
    *)
      echo "rig-client-display: REFUSED: no rig display: RIG_DISPLAY=${RIG_DISPLAY:-<unset>}, DISPLAY=${DISPLAY:-<unset>}; expected $RIG_RUST_DISPLAY_PATTERN (never the owner's :0)" >&2
      return "$RIG_E_DISPLAY"
      ;;
  esac
  if [ -n "${RIG_DISPLAY:-}" ] && [ -n "${DISPLAY:-}" ] && [ "$RIG_DISPLAY" != "$DISPLAY" ]; then
    caller_note=" (overrides caller DISPLAY=$DISPLAY)"
  fi
  if [ "${RIG_CLIENT_DISPLAY_SKIP_X_PROBE:-0}" != "1" ] && command -v xdpyinfo >/dev/null 2>&1; then
    if ! xdpyinfo -display "$requested" >/dev/null 2>&1; then
      echo "rig-client-display: REFUSED: X display $requested is not up" >&2
      return "$RIG_E_DISPLAY"
    fi
  fi
  printf 'rig-client-display: rig display %s%s\n' "$requested" "$caller_note" >&2
  printf '%s\n' "$requested"
}

# Command: export the pinned display and strip owner-session/Wine state.
rig_pin_rust_client_display() {
  local rig_display
  rig_display="$(rig_validated_client_display)" || return "$?"
  export DISPLAY="$rig_display" RIG_DISPLAY="$rig_display"
  unset WAYLAND_DISPLAY WAYLAND_SOCKET XDG_SESSION_TYPE QT_QPA_PLATFORM
  unset WINEPREFIX RIG_WINEPREFIX
  export WINIT_UNIX_BACKEND=x11
  echo "rig-client-display: pin DISPLAY=$DISPLAY RIG_DISPLAY=$RIG_DISPLAY WINIT_UNIX_BACKEND=$WINIT_UNIX_BACKEND (WAYLAND_DISPLAY unset, WINEPREFIX unset)"
}

rig_client_is_native() {
  local client="${1:-}"
  case "${RIG_CLIENT_KIND:-}" in
    native) return 0 ;;
    wine) return 1 ;;
  esac
  case "${client##*/}" in
    recomp2jam | recomp2jam.exe) return 0 ;;
    *) return 1 ;;
  esac
}

# Command: inherited Wine state is meaningless for a native client and makes
# rig-run.sh's rig_check_prefix claim a prefix owned by an earlier Wine leg.
rig_clear_inherited_wine_prefix() {
  local client="${1:-}"
  rig_client_is_native "$client" || return 0
  if [ -n "${WINEPREFIX:-}" ] || [ -n "${RIG_WINEPREFIX:-}" ]; then
    echo "rig-client-display: native client ${client##*/}: clearing inherited WINEPREFIX=${WINEPREFIX:-<unset>} RIG_WINEPREFIX=${RIG_WINEPREFIX:-<unset>}" >&2
  fi
  unset WINEPREFIX RIG_WINEPREFIX
}

# --- hermetic scenario checks -------------------------------------------------
# Caller chain: play-local.sh -> rig_pin_rust_client_display ->
# rig_validated_client_display, and rig-run.sh cmd_run ->
# rig_clear_inherited_wine_prefix. These legs drive the same exported functions
# with no Xvfb, audio sink, or client, so env stripping is pinned in isolation;
# the live boot under the rig display proves the full chain end to end.
selftest_pin_strips_owner_session() {
  local output status=0
  output="$(env -u RIG_DISPLAY RIG_CLIENT_DISPLAY_SKIP_X_PROBE=1 \
    DISPLAY=:163 WAYLAND_DISPLAY=wayland-1 WINEPREFIX=/tmp/owner-prefix \
    bash "$0" probe 2>&1)" || status=$?
  [ "$status" -eq 0 ] || { echo "$output"; return 1; }
  grep -qF 'DISPLAY=:163' <<<"$output" || { echo "$output"; return 1; }
  grep -qF 'WAYLAND_DISPLAY=<unset>' <<<"$output" || { echo "$output"; return 1; }
  grep -qF 'WINEPREFIX=<unset>' <<<"$output" || { echo "$output"; return 1; }
  grep -qF 'WINIT_UNIX_BACKEND=x11' <<<"$output" || { echo "$output"; return 1; }
  echo "rig-client-display: PASS pin-strips-owner-session"
}

selftest_refuses_owner_display() {
  local output status
  set +e
  output="$(env -u RIG_DISPLAY RIG_CLIENT_DISPLAY_SKIP_X_PROBE=1 \
    DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 bash "$0" probe 2>&1)"
  status=$?
  set -e
  [ "$status" -eq 0 ] && { echo "$output"; echo "owner display was accepted" >&2; return 1; }
  [ "$status" -eq "$RIG_E_DISPLAY" ] || { echo "$output"; return 1; }
  grep -qF 'REFUSED: no rig display' <<<"$output" || { echo "$output"; return 1; }
  echo "rig-client-display: PASS refuses-owner-display-before-boot"
}

selftest_native_clears_inherited_prefix() {
  local output
  output="$(RIG_CLIENT_KIND=native WINEPREFIX=/tmp/wine-leg \
    RIG_WINEPREFIX=/tmp/wine-leg bash "$0" clear-prefix -- target/debug/recomp2jam 2>&1)"
  grep -qF 'WINEPREFIX=<unset>' <<<"$output" || { echo "$output"; return 1; }
  output="$(WINEPREFIX=/tmp/wine-leg bash "$0" clear-prefix -- scripts/rig-wine.sh 2>&1)"
  grep -qF 'WINEPREFIX=/tmp/wine-leg' <<<"$output" || { echo "$output"; return 1; }
  echo "rig-client-display: PASS native-clears-inherited-prefix-wine-keeps-it"
}

cmd_probe() {
  rig_pin_rust_client_display
  printf 'DISPLAY=%s\nRIG_DISPLAY=%s\nWAYLAND_DISPLAY=%s\nWINEPREFIX=%s\nWINIT_UNIX_BACKEND=%s\n' \
    "$DISPLAY" "$RIG_DISPLAY" "${WAYLAND_DISPLAY:-<unset>}" \
    "${WINEPREFIX:-<unset>}" "$WINIT_UNIX_BACKEND"
}

cmd_clear_prefix() {
  [ "${1:-}" = "--" ] && shift
  [ "$#" -ge 1 ] || { echo "rig-client-display: no client given" >&2; return 2; }
  rig_clear_inherited_wine_prefix "$1"
  printf 'WINEPREFIX=%s\nRIG_WINEPREFIX=%s\n' \
    "${WINEPREFIX:-<unset>}" "${RIG_WINEPREFIX:-<unset>}"
}

main() {
  case "${1:-}" in
    probe) shift; cmd_probe "$@" ;;
    clear-prefix) shift; cmd_clear_prefix "$@" ;;
    selftest)
      selftest_pin_strips_owner_session
      selftest_refuses_owner_display
      selftest_native_clears_inherited_prefix
      echo "rig-client-display: selftest OK" ;;
    *) echo "usage: rig-client-display.sh {probe|clear-prefix -- CLIENT|selftest}" >&2; return 2 ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
