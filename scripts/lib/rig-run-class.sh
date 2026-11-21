#!/usr/bin/env bash
# rig-run-class.sh — the one query for "what kind of boot is this?" (#745).
#
# Source this file from a launcher, or run it directly for hermetic checks:
#   scripts/lib/rig-run-class.sh classify [CLIENT]
#   scripts/lib/rig-run-class.sh selftest
#
# The three classes are exhaustive. `owner-session` is the attended Wayland
# + :0 run: no display pin, no live-lane flock. `rig-x11` is a client boot on
# an Xvfb display (--rig, RIG_DISPLAY, or a :NNN DISPLAY): pin and flock.
# `rig-wine` is a Wine boot on a non-owner path (a wine client name,
# RIG_CLIENT_KIND=wine, or WINEPREFIX/RIG_WINEPREFIX): it keeps its prefix
# and uses the rig lane. The owner's EXE path stays `owner-session`.
#
# Caller chains: play-local.sh -> rig_run_class -> display pin + flock;
# rig-client-display.sh -> rig_run_class -> pin validation;
# rig-client-identity.sh -> rig_run_class -> 1000 client-id policy;
# rig-run.sh cmd_run -> rig_run_class -> inherited-prefix policy.
set -euo pipefail

RIG_RUN_CLASS_OWNER='owner-session'
RIG_RUN_CLASS_X11='rig-x11'
RIG_RUN_CLASS_WINE='rig-wine'
# Rig Xvfb displays are three digits (:150-:199 by default). The owner's
# session is :0; one- and two-digit displays are never rig boots.
RIG_RUST_DISPLAY_PATTERN=':[1-9][0-9][0-9]'

RIG_RUN_CLASS_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/rig-run-class.sh"

rig_run_class_is_owner_session() {
  [ "${RIG_RUN:-0}" != 1 ] && [ -z "${RIG_DISPLAY:-}" ] &&
    [ -n "${WAYLAND_DISPLAY:-}" ] && [ "${DISPLAY:-}" = :0 ]
}

rig_run_class_is_native_client() {
  case "${RIG_CLIENT_KIND:-}" in
    native) return 0 ;;
    wine) return 1 ;;
  esac
  case "${1:-}" in
    */recomp2jam | */recomp2jam.exe | recomp2jam | recomp2jam.exe) return 0 ;;
  esac
  return 1
}

rig_run_class_has_wine_client_name() {
  case "${1:-}" in
    */wine | */wine64 | */rig-wine.sh | *.exe | wine | wine64 | rig-wine.sh) return 0 ;;
  esac
  return 1
}

rig_run_class_is_wine_boot() {
  local client="${1:-}"
  case "${RIG_CLIENT_KIND:-}" in
    native) return 1 ;;
    wine) return 0 ;;
  esac
  if rig_run_class_has_wine_client_name "$client"; then
    return 0
  fi
  if rig_run_class_is_native_client "$client" || rig_run_class_is_owner_session; then
    return 1
  fi
  [ -n "${WINEPREFIX:-}" ] || [ -n "${RIG_WINEPREFIX:-}" ]
}

rig_run_class_is_x11_boot() {
  local display_pattern="^${RIG_RUST_DISPLAY_PATTERN}$"
  [ "${RIG_RUN:-0}" = 1 ] && return 0
  [ -n "${RIG_DISPLAY:-}" ] && return 0
  [[ "${DISPLAY:-}" =~ $display_pattern ]]
}

# The one query: print exactly one class name on stdout, nothing else.
rig_run_class() {
  local client="${1:-}"
  if rig_run_class_is_wine_boot "$client"; then
    printf '%s\n' "$RIG_RUN_CLASS_WINE"
  elif rig_run_class_is_x11_boot; then
    printf '%s\n' "$RIG_RUN_CLASS_X11"
  else
    printf '%s\n' "$RIG_RUN_CLASS_OWNER"
  fi
}

# --- hermetic scenario checks ------------------------------------------------
# Each leg runs the same public query in a fresh env. No Xvfb, audio, server,
# or client is touched. A live rig run still proves the caller chain end to end.
assert_run_class() {
  local expected="$1" scenario="$2" actual
  shift 2
  actual="$("$@")" || { echo "rig-run-class: FAIL $scenario (query failed)" >&2; return 1; }
  if [ "$actual" != "$expected" ]; then
    echo "rig-run-class: FAIL $scenario: expected '$expected', got '$actual'" >&2
    return 1
  fi
  echo "rig-run-class: PASS $scenario"
}

selftest_owner_wayland_zero_display_is_owner_session() {
  assert_run_class "$RIG_RUN_CLASS_OWNER" owner-wayland-zero-display \
    env -u RIG_RUN -u RIG_DISPLAY -u WINEPREFIX -u RIG_WINEPREFIX -u RIG_CLIENT_KIND \
    DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 bash "$RIG_RUN_CLASS_LIB" classify
}

selftest_rig_display_is_rig_x11() {
  assert_run_class "$RIG_RUN_CLASS_X11" rig-display-is-rig-x11 \
    env -u RIG_RUN -u WAYLAND_DISPLAY -u WINEPREFIX -u RIG_WINEPREFIX -u RIG_CLIENT_KIND \
    DISPLAY=:163 RIG_DISPLAY=:163 bash "$RIG_RUN_CLASS_LIB" classify
}

selftest_wine_prefix_is_rig_wine() {
  assert_run_class "$RIG_RUN_CLASS_WINE" wine-prefix-is-rig-wine \
    env -u RIG_RUN -u RIG_DISPLAY -u WAYLAND_DISPLAY -u RIG_CLIENT_KIND \
    DISPLAY=:181 WINEPREFIX=/tmp/rig-prefix bash "$RIG_RUN_CLASS_LIB" classify
}

selftest_owner_prefix_stays_owner_session() {
  # The owner's EXE+Wine path carries a prefix but no rig marker.
  assert_run_class "$RIG_RUN_CLASS_OWNER" owner-prefix-stays-owner-session \
    env -u RIG_RUN -u RIG_DISPLAY -u RIG_CLIENT_KIND \
    DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 WINEPREFIX=/tmp/owner-prefix \
    bash "$RIG_RUN_CLASS_LIB" classify
}

selftest_rig_flag_on_owner_display_is_rig_x11() {
  # Regression class: an over-broad ":0 means owner" exemption must not hide
  # a rig script that accidentally landed on the owner display.
  assert_run_class "$RIG_RUN_CLASS_X11" rig-flag-on-owner-display-is-rig-x11 \
    env -u RIG_DISPLAY -u WINEPREFIX -u RIG_WINEPREFIX -u RIG_CLIENT_KIND \
    DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 RIG_RUN=1 bash "$RIG_RUN_CLASS_LIB" classify
}

selftest_native_peer_with_prefix_is_rig_x11() {
  # #719: an inherited Wine prefix on a native peer must not classify as Wine.
  assert_run_class "$RIG_RUN_CLASS_X11" native-peer-with-prefix-is-rig-x11 \
    env -u RIG_RUN -u RIG_DISPLAY -u WAYLAND_DISPLAY \
    DISPLAY=:182 RIG_WINEPREFIX=/tmp/wine-leg bash "$RIG_RUN_CLASS_LIB" \
    classify /usr/bin/recomp2jam
}

cmd_classify() {
  [ "$#" -le 1 ] || { echo "rig-run-class: classify takes at most one client" >&2; return 2; }
  rig_run_class "${1:-}"
}

main() {
  case "${1:-}" in
    classify) shift; cmd_classify "$@" ;;
    selftest)
      selftest_owner_wayland_zero_display_is_owner_session
      selftest_rig_display_is_rig_x11
      selftest_wine_prefix_is_rig_wine
      selftest_owner_prefix_stays_owner_session
      selftest_rig_flag_on_owner_display_is_rig_x11
      selftest_native_peer_with_prefix_is_rig_x11
      echo "rig-run-class: selftest OK" ;;
    *) echo "usage: rig-run-class.sh {classify [CLIENT]|selftest}" >&2; return 2 ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
