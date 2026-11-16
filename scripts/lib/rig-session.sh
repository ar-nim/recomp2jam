#!/usr/bin/env bash
# rig-session.sh — one home for the native rig session launch shape.
#
# Sourced by scripts/rig-afk.sh (it boots the session) and by
# scripts/planet-to-waiting.sh (standalone mode boots the same session), so
# the client argv never forks between the two callers.
#
# Usage:
#   mapfile -t argv < <(rig_session_argv OUT_DIR --sound --music-id 757)
#   "$binary" "${argv[@]}"
#
# The argv is the native online walk the rig has proven (docs/smoke/
# u1-windowed-online.md): login rides CLI args (ours-by-design — the exe
# takes a launcher cmdline), the lobby walk rides real window input, the
# frame dump and the input log land on the paths the caller chose.
#
# Knobs:
#   RIG_SESSION_BINARY   client binary (default <repo>/target/debug/recomp2jam)
#   RIG_SESSION_TOKEN    gateway session guid (default: the pinned local
#                        Memoryer token; a dead token fails the boot loudly)
#   RIG_SESSION_HOST / RIG_SESSION_PORT / RIG_SESSION_USER_ID
#   RIG_SESSION_ROOM     room title created at boot (default DriveRoom)

RIG_SESSION_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# The pinned local-Memoryer session token (docs/smoke/u1-windowed-online.md:
# the 2026-11-06 token expired server-side, read the live one from
# `t_o2jam_login` when Memoryer moves).
rig_session_pinned_token() {
  printf '%s' 'eebaccee-fde9-4c50-b941-fda6c7004376'
}

rig_session_token() {
  printf '%s' "${RIG_SESSION_TOKEN:-$(rig_session_pinned_token)}"
}

# The capture paths a session writes. They are client outputs and stay where
# the client wrote them (docs/rig-runbook.md, file-separation rule).
rig_session_log() { printf '%s/input.log' "$1"; }
rig_session_frames() { printf '%s/frames' "$1"; }

rig_session_binary() {
  printf '%s' "${RIG_SESSION_BINARY:-$RIG_SESSION_REPO/target/debug/recomp2jam}"
}

# Print the session argv, one argument per line (for `mapfile -t`).
# Usage: rig_session_argv OUT_DIR [--sound|--no-sound] [--music-id N]
rig_session_argv() {
  local out_dir="$1"; shift
  local sound=1 music_id=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --sound) sound=1; shift ;;
      --no-sound) sound=0; shift ;;
      --music-id) music_id="$2"; shift 2 ;;
      *) die "session: unknown option: $1" ;;
    esac
  done
  # Every argument on its own line; callers read with `mapfile -t`.
  printf '%s\n' online
  printf '%s\n' --host "${RIG_SESSION_HOST:-127.0.0.1}"
  printf '%s\n' --port "${RIG_SESSION_PORT:-15010}"
  printf '%s\n' --user-id "${RIG_SESSION_USER_ID:-1}"
  printf '%s\n' --token "$(rig_session_token)"
  printf '%s\n' --gateway-id 0 --channel-id 0
  printf '%s\n' --create-room "${RIG_SESSION_ROOM:-DriveRoom}"
  if [ -n "$music_id" ]; then
    printf '%s\n' --music-id "$music_id"
  fi
  if [ "$sound" = "0" ]; then
    printf '%s\n' --no-sound
  fi
  printf '%s\n' --frame-dump "$(rig_session_frames "$out_dir")"
  printf '%s\n' --input-log "$(rig_session_log "$out_dir")"
  return 0
}

# The whole launch command line, as plain words, for provenance and reports.
rig_session_command_line() {
  printf '%s' "$(rig_session_binary)"
  local argument
  while IFS= read -r argument; do
    printf ' %s' "$argument"
  done < <(rig_session_argv "$@")
  printf '\n'
}
