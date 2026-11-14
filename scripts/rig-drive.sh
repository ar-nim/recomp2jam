#!/usr/bin/env bash
# rig-drive.sh — ydotool (kernel uinput) input driver for rig sessions.
#
# Usage:
#   scripts/rig-drive.sh wait-device <out-dir>     # poll until ydotoold answers
#   scripts/rig-drive.sh move <out-dir> <x> <y>    # absolute move
#   scripts/rig-drive.sh click <out-dir> [button]   # default left (0xC0)
#   scripts/rig-drive.sh key <out-dir> <code[:state]>...
#   scripts/rig-drive.sh dry-run <out-dir>          # print planned commands, move nothing
#
# Honors the same out-dir/PID discipline as scripts/rig-run.sh: any daemon
# started is recorded in <out-dir>/pids and stopped via rig-run.sh stop.
set -euo pipefail

die() { echo "rig-drive: $*" >&2; exit 2; }

YDOTOOL="${YDOTOOL:-ydotool}"
YDOTOOLD="${YDOTOOLD:-ydotoold}"

wait_device() {
  local out_dir="$1" i
  [ -d "$out_dir" ] || mkdir -p "$out_dir"
  for i in $(seq 1 50); do
    if "$YDOTOOL" debug >/dev/null 2>&1; then
      echo "rig-drive: ydotool device ready"
      return 0
    fi
    sleep 0.1
  done
  die "ydotoold did not become ready within 5s"
}

cmd_wait() { wait_device "$1"; }
cmd_move() {
  local out_dir="$1" x="$2" y="$3"
  wait_device "$out_dir"
  "$YDOTOOL" mousemove --absolute -x "$x" -y "$y"
  echo "rig-drive: moved to $x,$y"
}
cmd_click() {
  local out_dir="$1" btn="${2:-0xC0}"
  wait_device "$out_dir"
  "$YDOTOOL" click "$btn"
  echo "rig-drive: clicked $btn"
}
cmd_key() {
  local out_dir="$1"; shift
  [ "$#" -ge 1 ] || die "key: need at least one code[:state]"
  wait_device "$out_dir"
  "$YDOTOOL" key "$@"
  echo "rig-drive: key $*"
}
cmd_dry() {
  local out_dir="$1"
  echo "rig-drive: DRY RUN — no pointer or keyboard action taken"
  echo "rig-drive: planned: wait-device -> move -> click -> key"
  echo "rig-drive: out-dir=$out_dir"
}

main() {
  local cmd="${1:-}"; shift || true
  case "$cmd" in
    wait-device) [ "$#" -ge 1 ] || die "usage: wait-device <out-dir>"; cmd_wait "$1" ;;
    move) [ "$#" -ge 3 ] || die "usage: move <out-dir> <x> <y>"; cmd_move "$@" ;;
    click) [ "$#" -ge 1 ] || die "usage: click <out-dir> [button]"; cmd_click "$@" ;;
    key) [ "$#" -ge 2 ] || die "usage: key <out-dir> <code[:state]>..."; cmd_key "$@" ;;
    dry-run) [ "$#" -ge 1 ] || die "usage: dry-run <out-dir>"; cmd_dry "$1" ;;
    *) die "usage: rig-drive.sh {wait-device|move|click|key|dry-run} ..." ;;
  esac
}
main "$@"
