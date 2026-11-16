#!/usr/bin/env bash
# rig-drive.sh — input driver for rig sessions.
#
# Backends:
#   ydotool (default) — kernel uinput via ydotoold (live seat / Wine).
#   xtest             — XTEST via python-xlib (bare Xvfb rigs).
#
# Usage:
#   scripts/rig-drive.sh [--backend ydotool|xtest] [--display :N] [--dry-run] \
#       {wait-device|move|click|key|focus|dry-run} <out-dir> [args...]
#   scripts/rig-drive.sh {ydotool|xtest} {wait-device|...} <out-dir> [args...]
#
# The backend may also be selected with RIG_DRIVER_BACKEND. Dry-run mode
# (command `dry-run`, or --dry-run) prints the planned commands and touches
# no input device.
set -euo pipefail

die() { echo "rig-drive: $*" >&2; exit 2; }

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
YDOTOOL="${YDOTOOL:-ydotool}"
YDOTOOLD="${YDOTOOLD:-ydotoold}"

# Default socket lives in the out-dir so a daemon we start is owned by
# that run and recorded in its pidfile (rig-run.sh PID discipline).
yd_socket() { printf '%s/.ydotool.sock' "$1"; }
XTEST_PY="$SCRIPT_DIR/rig-xtest.py"

backend="${RIG_DRIVER_BACKEND:-ydotool}"
dry_run="${RIG_DRIVER_DRY_RUN:-0}"
display_override="${RIG_DISPLAY:-}"

# --- option parsing (global flags may appear anywhere) --------------------
remaining=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --backend) backend="${2:-}"; shift 2 ;;
    --backend=*) backend="${1#*=}"; shift ;;
    --display) display_override="${2:-}"; shift 2 ;;
    --display=*) display_override="${1#*=}"; shift ;;
    --dry-run) dry_run=1; shift ;;
    --) shift; break ;;
    *) remaining+=("$1"); shift ;;
  esac
done
set -- "${remaining[@]}"

# backend may be given as the first positional token
if [[ "${1:-}" == "ydotool" || "${1:-}" == "xtest" ]]; then
  backend="$1"; shift
fi

case "$backend" in
  ydotool|xtest) ;;
  *) die "unknown backend: $backend (expected ydotool or xtest)" ;;
esac

cmd="${1:-}"; shift || true

load_env() {
  local out_dir="$1"
  if [[ -f "$out_dir/env.sh" ]]; then
    # shellcheck source=/dev/null
    . "$out_dir/env.sh"
  fi
  if [[ -n "$display_override" ]]; then
    export DISPLAY="$display_override"
  fi
}

# --- ydotool backend --------------------------------------------------------
# Ensure ydotoold is running (starting it user-level when needed) and poll
# for udev device recognition instead of sleeping once. Poll, never a
# fixed sleep, so the udev recognition delay is tolerated deterministically.
yd_ensure() {
  local out_dir="$1" i
  [[ -d "$out_dir" ]] || mkdir -p "$out_dir"
  export YDOTOOL_SOCKET="$(yd_socket "$out_dir")"
  if "$YDOTOOL" debug >/dev/null 2>&1; then
    return 0
  fi
  # Dry-run must not start a daemon or touch uinput.
  if [[ "$dry_run" == "1" ]]; then
    return 0
  fi
  "$YDOTOOLD" --socket-path "$YDOTOOL_SOCKET" >"$out_dir/ydotoold.log" 2>&1 &
  echo "ydotoold $! ydotoold" >> "$out_dir/pids"
  for i in $(seq 1 50); do
    if "$YDOTOOL" debug >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done
  die "ydotoold did not become ready within 5s"
}

yd_wait() {
  local out_dir="$1" i
  yd_ensure "$out_dir"
  # Already ready after ensure; keep a final poll for late udev recognition.
  for i in $(seq 1 50); do
    if "$YDOTOOL" debug >/dev/null 2>&1; then
      echo "rig-drive: ydotool device ready"
      return 0
    fi
    sleep 0.1
  done
  die "ydotoold did not become ready within 5s"
}

# --- xtest backend ---------------------------------------------------------
xt() {
  # xt <out_dir> <xtest-subcommand> [args...]
  local out_dir="$1"; shift
  load_env "$out_dir"
  if [[ "$dry_run" == "1" ]]; then
    echo "rig-drive: DRY RUN (xtest) — DISPLAY=${DISPLAY:-<unset>} python3 $XTEST_PY --display ${DISPLAY:-<unset>} $*"
    return 0
  fi
  [[ -n "${DISPLAY:-}" ]] || die "xtest: DISPLAY is not set (source $out_dir/env.sh or pass --display)"
  python3 "$XTEST_PY" --display "$DISPLAY" "$@"
}

# --- commands ---------------------------------------------------------------
cmd_wait() {
  local out_dir="$1"
  if [[ "$backend" == "xtest" ]]; then
    xt "$out_dir" wait-device
  else
    if [[ "$dry_run" == "1" ]]; then
      echo "rig-drive: DRY RUN (ydotool) — $YDOTOOL debug"
      return 0
    fi
    yd_wait "$out_dir"
  fi
}

cmd_move() {
  local out_dir="$1" x="$2" y="$3"
  if [[ "$backend" == "xtest" ]]; then
    xt "$out_dir" move "$x" "$y"
  else
    if [[ "$dry_run" == "1" ]]; then
      echo "rig-drive: DRY RUN (ydotool) — $YDOTOOL mousemove --absolute -x $x -y $y"
      return 0
    fi
    yd_wait "$out_dir"
    "$YDOTOOL" mousemove --absolute -x "$x" -y "$y"
    echo "rig-drive: moved to $x,$y"
  fi
}

cmd_click() {
  local out_dir="$1" btn="${2:-1}"
  if [[ "$backend" == "xtest" ]]; then
    xt "$out_dir" click "$btn"
  else
    if [[ "$dry_run" == "1" ]]; then
      echo "rig-drive: DRY RUN (ydotool) — $YDOTOOL click $btn"
      return 0
    fi
    yd_wait "$out_dir"
    "$YDOTOOL" click "$btn"
    echo "rig-drive: clicked $btn"
  fi
}

cmd_key() {
  local out_dir="$1"; shift
  [[ "$#" -ge 1 ]] || die "key: need at least one code[:state]"
  if [[ "$backend" == "xtest" ]]; then
    xt "$out_dir" key "$@"
  else
    if [[ "$dry_run" == "1" ]]; then
      echo "rig-drive: DRY RUN (ydotool) — $YDOTOOL key $*"
      return 0
    fi
    yd_wait "$out_dir"
    "$YDOTOOL" key "$@"
    echo "rig-drive: key $*"
  fi
}

cmd_focus() {
  local out_dir="$1" needle="${2:-recomp2jam}"
  if [[ "$backend" == "xtest" ]]; then
    xt "$out_dir" focus "$needle"
  else
    if [[ "$dry_run" == "1" ]]; then
      echo "rig-drive: DRY RUN (ydotool) — focus $needle (no-op on uinput backend)"
      return 0
    fi
    echo "rig-drive: focus requested for $needle (ydotool backend drives the seat; no focus call)"
  fi
}

cmd_dry() {
  local out_dir="$1"
  load_env "$out_dir"
  echo "rig-drive: DRY RUN ($backend) — no pointer or keyboard action taken"
  echo "rig-drive: planned sequence: wait-device -> focus -> move -> click -> key"
  echo "rig-drive: backend=$backend out-dir=$out_dir display=${display_override:-${DISPLAY:-<from env.sh>}}"
  echo "rig-drive: example: $0 --backend $backend move $out_dir <x> <y>"
  echo "rig-drive: example: $0 --backend $backend click $out_dir [button]"
  echo "rig-drive: example: $0 --backend $backend key $out_dir <code[:state]>..."
  echo "rig-drive: example: $0 --backend $backend focus $out_dir [window]"
}

main() {
  case "$cmd" in
    wait-device) [[ $# -ge 1 ]] || die "usage: wait-device <out-dir>"; cmd_wait "$1" ;;
    move) [[ $# -ge 3 ]] || die "usage: move <out-dir> <x> <y>"; cmd_move "$@" ;;
    click) [[ $# -ge 1 ]] || die "usage: click <out-dir> [button]"; cmd_click "$@" ;;
    key) [[ $# -ge 2 ]] || die "usage: key <out-dir> <code[:state]>..."; cmd_key "$@" ;;
    focus) [[ $# -ge 1 ]] || die "usage: focus <out-dir> [window]"; cmd_focus "$@" ;;
    dry-run) [[ $# -ge 1 ]] || die "usage: dry-run <out-dir>"; cmd_dry "$1" ;;
    "") die "usage: rig-drive.sh {wait-device|move|click|key|focus|dry-run} ..." ;;
    *) die "unknown command: $cmd" ;;
  esac
}

main "$@"
