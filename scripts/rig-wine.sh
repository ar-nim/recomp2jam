#!/usr/bin/env bash
# rig-wine.sh — the Wine leg's procedure home (issue #240).
#
# Usage:
#   scripts/rig-wine.sh build-injector [OUT_DIR]
#   scripts/rig-wine.sh launch <argv-file>
#   scripts/rig-wine.sh send <move|click|dclick|press|down|up|key|type> [args...]
#   scripts/rig-wine.sh reset
#
# Everything the Wine pair needs to start the 7.55 exe and to put input
# into its DirectDraw canvas lives here, because the canvas reads Win32
# input that XTEST never reaches (docs/wine-boot.md, Attempts 6-9):
#
#   build-injector  winegcc-build scripts/sendinput-helper.c into
#                   <OUT_DIR>/sendinput-helper.exe.so (default OUT_DIR is
#                   $RIG_WINE_INJECTOR_DIR or ~/scratch/240-fix/injector).
#                   The build needs winegcc only; the source is committed.
#   launch          exec the client with the pinned argv shape
#                   `OTwo.exe <blob> (|test|??|<addr>|<port>) x5` read from
#                   argv-file, one argument per line, exactly as
#                   `scripts/rig-mint-launch-argv.sh` writes it. The
#                   launch is what scripts/rig-run.sh records in its
#                   pidfile, so the recorded command names this script
#                   (a bare `wine` fragment would let a PID-file teardown
#                   match processes it did not start).
#   send            drive one SendInput command through the built helper
#                   (the helper joins the `o2jam` virtual desktop itself).
#   reset           `wineserver -k` for THIS run's WINEPREFIX only: a
#                   relaunch against a stale X window id dies in
#                   X_CreateWindow (BadWindow), docs/wine-boot.md
#                   Attempt 11 tooling notes.
#
# Pins (override by environment, never by editing this file):
#   RIG_WINE_EXE       default ~/scratch/run-full/OTwo.exe (sha256 checked
#                      by scripts/rig-provenance.sh's --expect-producer-sha256)
#   RIG_WINE_DESKTOP   default o2jam (the name the helper joins)
#   RIG_WINE_SIZE      default 800x600 (the exe's native canvas)
#   WINEPREFIX         required for launch/send/reset
#   RIG_WINE_LOCALE    default ko_KR.UTF-8 (the Korean prefix's ANSI
#                      codepage is CP949; the font fallback lives in the
#                      prefix registry, docs/wine-boot.md Attempt 5)
#
# The exe's working directory must be its own game dir: a repo-root cwd
# produces init-time ACCESS_VIOLATIONs (docs/wine-boot.md, Attempt 12).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RIG_WINE_EXE="${RIG_WINE_EXE:-$HOME/scratch/run-full/OTwo.exe}"
RIG_WINE_DESKTOP="${RIG_WINE_DESKTOP:-o2jam}"
RIG_WINE_SIZE="${RIG_WINE_SIZE:-800x600}"
RIG_WINE_LOCALE="${RIG_WINE_LOCALE:-ko_KR.UTF-8}"
RIG_WINE_INJECTOR_DIR="${RIG_WINE_INJECTOR_DIR:-$HOME/scratch/240-fix/injector}"

die() { echo "rig-wine: $*" >&2; exit 2; }

injector_path() { printf '%s/sendinput-helper.exe.so' "$RIG_WINE_INJECTOR_DIR"; }

require_prefix() {
  [ -n "${WINEPREFIX:-}" ] || die "WINEPREFIX is not set; the Wine leg never uses the default prefix"
  [ -d "$WINEPREFIX" ] || die "WINEPREFIX '$WINEPREFIX' is not a directory"
}

require_exe() {
  [ -f "$RIG_WINE_EXE" ] || die "client exe not found: $RIG_WINE_EXE (set RIG_WINE_EXE)"
}

cmd_build_injector() {
  local out_dir="${1:-$RIG_WINE_INJECTOR_DIR}"
  command -v winegcc >/dev/null || die "winegcc not found (install wine)"
  mkdir -p "$out_dir"
  # winegcc writes '<out>' (launcher) and '<out>.so' (the PE); naming the
  # output '*.exe.so' makes Wine resolve the PE directly, which is how the
  # proven helpers were invoked (docs/wine-boot.md, Attempt 11).
  ( cd "$out_dir" && winegcc -O2 -o sendinput-helper "$SCRIPT_DIR/sendinput-helper.c" -luser32 ) \
    >/dev/null 2>&1 || die "winegcc failed for sendinput-helper.c"
  local built="$out_dir/sendinput-helper.exe.so"
  [ -f "$built" ] || die "winegcc produced no $built"
  # The sibling 'sendinput-helper.exe' is winegcc's shell launcher; Wine
  # runs the PE, and two entry points invite the wrong one being recorded
  # in provenance. Keep one artifact.
  rm -f "$out_dir/sendinput-helper.exe"
  echo "rig-wine: built $built ($(sha256sum "$built" | cut -c1-12))"
}

cmd_launch() {
  local argv_file="${1:-}"
  [ -n "$argv_file" ] || die "launch: argv file required (one argument per line)"
  [ -r "$argv_file" ] || die "launch: cannot read $argv_file"
  require_prefix
  require_exe
  local -a argv=()
  mapfile -t argv < "$argv_file"
  [ "${#argv[@]}" -ge 6 ] || die "launch: $argv_file holds ${#argv[@]} arguments; the pinned shape needs a blob plus 5 mirrors"
  # A relaunch against a wineserver that still holds the previous run's X
  # window id dies in X_CreateWindow (BadWindow 0xa00007) and leaves a black
  # screen with no client at all (docs/wine-boot.md, Attempt 11 tooling
  # notes). The prefix is this run's own claim, so reset it before exec.
  wineserver -k 2>/dev/null || true
  sleep 1
  cd "$(dirname "$RIG_WINE_EXE")"
  export LANG="$RIG_WINE_LOCALE"
  if [ -d "$HOME/.local/share/wine-locales" ]; then
    export LOCPATH="$HOME/.local/share/wine-locales"
  fi
  exec wine explorer "/desktop=$RIG_WINE_DESKTOP,$RIG_WINE_SIZE" "$RIG_WINE_EXE" "${argv[@]}"
}

cmd_send() {
  [ "$#" -ge 1 ] || die "send: a helper command is required (move|click|dclick|press|down|up|key|type)"
  require_prefix
  [ -f "$(injector_path)" ] || die "send: no injector at $(injector_path); run build-injector first"
  LANG="$RIG_WINE_LOCALE" wine "$(injector_path)" "$@"
}

cmd_reset() {
  require_prefix
  wineserver -k 2>/dev/null || true
  echo "rig-wine: wineserver reset for $WINEPREFIX"
}

main() {
  local command="${1:-}"; shift || true
  case "$command" in
    build-injector) cmd_build_injector "$@" ;;
    launch) cmd_launch "$@" ;;
    send) cmd_send "$@" ;;
    reset) cmd_reset ;;
    *) die "usage: rig-wine.sh {build-injector|launch|send|reset} ..." ;;
  esac
}

main "$@"
