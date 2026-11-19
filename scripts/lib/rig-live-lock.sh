#!/usr/bin/env bash
# rig-live-lock.sh — one live client lane machine-wide (#489).
#
# Usage (CLI):
#   scripts/lib/rig-live-lock.sh run --owner OWNER [--timeout SECONDS] -- COMMAND [ARGS...]
#
# Source this file from shell callers:
#   rig_live_lock_acquire OWNER [TIMEOUT]   open + flock the lane file
#   rig_live_lock_install_release_trap      release the caller's descriptor on exit
#   rig_live_lock_run OWNER TIMEOUT -- CMD  acquire, run CMD, release on exit
#
# The lock is an advisory flock on `$RIG_LIVE_LOCK_DIR/lane.lock`. The owner
# text lives in that file, written only by the process that holds the lock, so
# a waiter can name the live lane. A client started inside a locked shell
# inherits the descriptor: the lane stays held until the client exits, while
# the launching shell can still return to its caller. RIG_STATE_DIR gives
# hermetic selftests a private lock; otherwise all callers share one lane.
set -euo pipefail

RIG_LIVE_LOCK_DIR="${RIG_LIVE_LOCK_DIR:-${RIG_STATE_DIR:+$RIG_STATE_DIR/live-lock}}"
RIG_LIVE_LOCK_DIR="${RIG_LIVE_LOCK_DIR:-${XDG_RUNTIME_DIR:-/tmp}/recomp2jam-live-lane}"
RIG_LIVE_LOCK_FILE="$RIG_LIVE_LOCK_DIR/lane.lock"
RIG_LIVE_LOCK_DEFAULT_TIMEOUT="${RIG_LIVE_LOCK_TIMEOUT:-30}"
RIG_LIVE_LOCK_RETRY_SECONDS=1
RIG_LIVE_LOCK_DENIED=17
RIG_LIVE_LOCK_DEFAULT_OWNER="pid $$"
RIG_LIVE_LOCK_FD="${RIG_LIVE_LOCK_FD:-}"
RIG_LIVE_LOCK_HELD=0
RIG_LIVE_LOCK_CHILD=""

rig_live_lock_holder() {
  local holder
  holder="$(head -n 1 "$RIG_LIVE_LOCK_FILE" 2>/dev/null || true)"
  printf '%s' "${holder:-unknown}"
}

rig_live_lock_inherited() {
  local fd="${RIG_LIVE_LOCK_FD:-}" target
  [ -n "$fd" ] || return 1
  target="$(readlink "/proc/$$/fd/$fd" 2>/dev/null || true)"
  [ "$target" = "$RIG_LIVE_LOCK_FILE" ]
}

rig_live_lock_acquire() {
  local owner="${1:-$RIG_LIVE_LOCK_DEFAULT_OWNER}" timeout="${2:-$RIG_LIVE_LOCK_DEFAULT_TIMEOUT}"
  local deadline=$((SECONDS + timeout))
  mkdir -p "$RIG_LIVE_LOCK_DIR"
  if rig_live_lock_inherited; then
    # An outer boot already owns this lock; never re-lock our own lane.
    printf 'rig-live-lock: live lane already held by %s (inherited)\n' \
      "$(rig_live_lock_holder)" >&2
    return 0
  fi
  # <> keeps the previous owner readable while a contender waits.
  exec {RIG_LIVE_LOCK_FD}<>"$RIG_LIVE_LOCK_FILE"
  export RIG_LIVE_LOCK_FD
  until flock -w "$RIG_LIVE_LOCK_RETRY_SECONDS" "$RIG_LIVE_LOCK_FD"; do
    printf 'live lane held by %s, retrying\n' "$(rig_live_lock_holder)" >&2
    if [ "$SECONDS" -ge "$deadline" ]; then
      exec {RIG_LIVE_LOCK_FD}>&-
      RIG_LIVE_LOCK_FD=""
      export RIG_LIVE_LOCK_FD=""
      return "$RIG_LIVE_LOCK_DENIED"
    fi
  done
  : > "$RIG_LIVE_LOCK_FILE"
  printf '%s\n' "$owner" > "$RIG_LIVE_LOCK_FILE"
  RIG_LIVE_LOCK_HELD=1
}

rig_live_lock_release() {
  [ "${RIG_LIVE_LOCK_HELD:-0}" -eq 1 ] || return 0
  RIG_LIVE_LOCK_HELD=0
  exec {RIG_LIVE_LOCK_FD}>&-
  RIG_LIVE_LOCK_FD=""
  export RIG_LIVE_LOCK_FD=""
}

rig_live_lock_install_release_trap() { trap 'rig_live_lock_release' EXIT; }

rig_live_lock_forward_signal() {
  local signal="$1" child="${RIG_LIVE_LOCK_CHILD:-}" status=143
  case "$signal" in
    INT) status=130 ;;
    HUP) status=129 ;;
  esac
  trap - TERM INT HUP
  if [ -n "$child" ] && kill -0 "$child" 2>/dev/null; then
    kill -"$signal" "$child" 2>/dev/null || true
    wait "$child" 2>/dev/null || true
  fi
  exit "$status"
}

rig_live_lock_run() {
  local owner="$1" timeout="$2" status
  shift 2
  [ "$#" -gt 0 ] || { echo "rig-live-lock: no command given" >&2; return 2; }
  rig_live_lock_acquire "$owner" "$timeout" || return $?
  "$@" &
  RIG_LIVE_LOCK_CHILD=$!
  trap 'rig_live_lock_forward_signal TERM' TERM
  trap 'rig_live_lock_forward_signal INT' INT
  trap 'rig_live_lock_forward_signal HUP' HUP
  trap 'rig_live_lock_release' EXIT
  status=0
  wait "$RIG_LIVE_LOCK_CHILD" || status=$?
  RIG_LIVE_LOCK_CHILD=""
  rig_live_lock_release
  trap - EXIT TERM INT HUP
  return "$status"
}

cmd_run() {
  local owner="$RIG_LIVE_LOCK_DEFAULT_OWNER" timeout="$RIG_LIVE_LOCK_DEFAULT_TIMEOUT"
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --owner) owner="$2"; shift 2 ;;
      --timeout) timeout="$2"; shift 2 ;;
      --) shift; break ;;
      *) echo "rig-live-lock: unknown option $1" >&2; return 2 ;;
    esac
  done
  [ "$#" -gt 0 ] || { echo "rig-live-lock: no command given after --" >&2; return 2; }
  rig_live_lock_run "$owner" "$timeout" "$@"
}

main() {
  local command="${1:-}"
  shift || true
  case "$command" in
    run) cmd_run "$@" ;;
    *) echo "usage: rig-live-lock.sh run --owner OWNER [--timeout S] -- COMMAND..." >&2; return 2 ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
