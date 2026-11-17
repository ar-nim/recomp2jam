#!/usr/bin/env bash
# rig-net-sample.sh — read-only socket census for a Wine rig run (#323).
#
# Usage: scripts/rig-net-sample.sh <out-file> <display> <wine-prefix> <seconds>
#
# Samples once per second while the named Wine process is alive. A sample is
# attributed to this run only when /proc/<pid>/environ carries both the run's
# DISPLAY and WINEPREFIX. It never sends input and never signals a game pid.
set -euo pipefail

out_file="${1:-}"
display="${2:-}"
prefix="${3:-}"
duration="${4:-}"
if [ -z "$out_file" ] || [ -z "$display" ] || [ -z "$prefix" ] || [ -z "$duration" ]; then
  echo "usage: rig-net-sample.sh <out-file> <display> <wine-prefix> <seconds>" >&2
  exit 2
fi

case "$display" in
  :[0-9]*) ;;
  *) echo "rig-net-sample: display must look like :245" >&2; exit 2 ;;
esac

is_run_game() {
  local pid="$1" env_text
  [ -r "/proc/$pid/comm" ] || return 1
  [ "$(cat "/proc/$pid/comm" 2>/dev/null)" = "OTwo.exe" ] || return 1
  env_text="$(tr '\0' '\n' < "/proc/$pid/environ" 2>/dev/null)" || return 1
  grep -Fxq "DISPLAY=$display" <<<"$env_text" || return 1
  grep -Fxq "WINEPREFIX=$prefix" <<<"$env_text" || return 1
  return 0
}

sample_pids() {
  local pid
  for pid in /proc/[0-9]*; do
    pid="${pid##*/}"
    if is_run_game "$pid"; then
      printf '%s\n' "$pid"
    fi
  done
}

write_sample() {
  local pid="$1" stamp
  stamp="$(date +%Y-%m-%dT%H:%M:%S.%N)"
  printf 'sample pid=%s at=%s\n' "$pid" "$stamp"
  printf 'fds: '
  ls -l "/proc/$pid/fd" 2>/dev/null | grep 'socket:' |
    sed 's/.*socket:/socket:/' | sort | uniq -c | tr '\n' ' '
  printf '\n'
  printf 'ss: '
  ss -tanup 2>/dev/null | grep "pid=$pid" | tr '\n' ';'
  printf '\n'
}

deadline=$((SECONDS + duration))
while [ "$SECONDS" -lt "$deadline" ]; do
  for game_pid in $(sample_pids); do
    write_sample "$game_pid"
  done
  sleep 1
done
