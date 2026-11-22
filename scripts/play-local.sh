#!/usr/bin/env bash
# play-local.sh — build (if needed) and run the client against the local
# Memoryer, reading the session token from its database automatically.
#
# Usage: scripts/play-local.sh [--user-id N] [--check-only] [extra recomp2jam online args...]
#   --user-id 1 = tester, 2 = tester2 (default 2; add accounts in Memoryer)
#   --check-only = run the shared rig preflight and exit (no build, no client)
# Examples:
#   scripts/play-local.sh                       # plain run, sound on
#   scripts/play-local.sh --create-room 1       # host room 1 directly
#   scripts/play-local.sh --input-log /tmp/in.log --frame-dump /tmp/frames
#
# Conformance (#206): the same scripts/lib/rig-preflight.sh checks rig-run.sh
# enforces run here — install-root, Memoryer liveness (TCP connect probe,
# stronger than the old ss grep), relay ports free, and audio containment.
# Display pin (#719): this script boots OUR Rust client only. Every boot needs
# a rig :NNN display (RIG_DISPLAY or DISPLAY); the owner's Wayland session and
# any inherited Wine prefix are cleared before the build or the client spawn.
# Audio policy is "attended" when sound is on (a human at a TTY acknowledges
# the speakers) and "null" with --no-sound; scripted runs (no TTY) with
# sound on are rejected so automated audio never reaches the speakers.
set -euo pipefail

# shellcheck source=lib/rig-preflight.sh
. "$(cd "$(dirname "$0")/lib" && pwd)/rig-preflight.sh"
# shellcheck source=lib/rig-client-display.sh
. "$(cd "$(dirname "$0")/lib" && pwd)/rig-client-display.sh"
# shellcheck source=lib/rig-client-identity.sh
. "$(cd "$(dirname "$0")/lib" && pwd)/rig-client-identity.sh"
# shellcheck source=lib/play-local-seams.sh
. "$(cd "$(dirname "$0")/lib" && pwd)/play-local-seams.sh"
# shellcheck source=lib/rig_binary_stale.sh
. "$(cd "$(dirname "$0")/lib" && pwd)/rig_binary_stale.sh"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MEMORYER_DIR="${MEMORYER_DIR:-$HOME/memoryer-local}"
USER_ID=2
CHECK_ONLY=0
RIG_RUN=0
EXTRA=()

while [ $# -gt 0 ]; do
  case "$1" in
    --user-id) USER_ID="$2"; shift 2 ;;
    --check-only) CHECK_ONLY=1; shift ;;
    --rig) export RIG_RUN=1; shift ;;
    *) EXTRA+=("$1"); shift ;;
  esac
done

RIG_INSTALL_ROOT="${RIG_INSTALL_ROOT:-$REPO_ROOT/install-root/O2Jam}"
export RIG_INSTALL_ROOT

# Client identity (#726): owner boots keep the exe's fixed client id.
# Multi-client rig boots set RIG_DISTINCT_CLIENT_IDS=1 so --user-id N
# reaches the 1000 frame as a distinct O2JAM_CLIENT_ID.
rig_export_client_identity "$USER_ID"

# 0. Display pin (#719, #745): one class query, reused by the flock below.
# Owner-session interactive runs (Wayland + :0) skip the pin; the pin
# refusing :0 was a regression (owner report 2026-11-20). The hostile
# case the pin exists for - a rig script accidentally on the owner
# display - always passes --rig, so the refusal still fires there.
RUN_CLASS="$(rig_run_class)"
if play_local_needs_display_pin "$RUN_CLASS"; then
  rig_pin_rust_client_display || exit "$?"
else
  echo "play-local: owner-session run (class $RUN_CLASS, DISPLAY=${DISPLAY:-unset}); display pin skipped - rig boots pass --rig or preset RIG_DISPLAY" >&2
fi

# 1. Game data: the client stages art from install-root (git-ignored).
if ! rig_check_install_root; then
  echo "play-local: claim a pooled data slot first:" >&2
  echo "  scripts/worktree-setup.sh" >&2
  echo "  (symlinks install-root/ and extracted/ into ~/scratch/install-data/slotN)" >&2
  exit "$RIG_E_DATA"
fi

# 2. Memoryer must be up and answering (connect probe, not just listening).
if ! rig_check_memoryer; then
  echo "play-local: Memoryer is not running. Start ONE instance:" >&2
  echo "  bash \"$REPO_ROOT/scripts/memoryer-start.sh\"" >&2
  echo "  The wrapper sets DOTNET_DbgEnableMiniDump=1 DOTNET_DbgMiniDumpType=2 and" >&2
  echo "  DOTNET_DbgMiniDumpName=\"\$HOME/scratch/memoryer-dumps/coredump.%p.%t.dmp\", and" >&2
  echo "  refuses a second instance while :15010 is listening (docs/rig-runbook.md)." >&2
  exit "$RIG_E_SERVER"
fi

# 3. Relay ports must be free (local Memoryer runs with relay disabled).
rig_check_relay_ports_free || exit "$RIG_E_PORT"

# 4. Audio containment: null sink when silenced, attended note otherwise.
AUDIO_POLICY=attended
if [ "${#EXTRA[@]}" -gt 0 ]; then
  for arg in "${EXTRA[@]}"; do
    [ "$arg" = "--no-sound" ] && AUDIO_POLICY=null
  done
fi
if [ "$AUDIO_POLICY" = "null" ]; then
  : "${PULSE_SINK:=${RIG_SINK_NAME:-wine_null}}"
  export PULSE_SINK
fi
if ! rig_check_audio_sink "$AUDIO_POLICY" "${PULSE_SINK:-}"; then
  exit "$RIG_E_AUDIO"
fi

# 4b. Staleness guard (retro #215): a release binary built before the
#     current checkout HEAD would let a playtester live-test stale code.
#     Refuse and ask for a rebuild rather than silently running it.
#     The timestamp is scoped to binary-relevant paths (#221): HEAD
#     commits that touch only docs, scripts, or CI must not mark a
#     cargo-current binary stale. Uncommitted edits are NOT detected
#     (limitation: the guard reads committed timestamps only).
# PLAY_LOCAL_PROFILE=debug selects the dev profile (e.g. the #223
# avatar-breathing check, which needs the slow debug paint path).
PROFILE="${PLAY_LOCAL_PROFILE:-release}"
case "$PROFILE" in
  release|debug) ;;
  *) echo "play-local: PLAY_LOCAL_PROFILE must be release|debug (got '$PROFILE')" >&2; exit 2 ;;
esac
BIN="$REPO_ROOT/target/$PROFILE/recomp2jam"
# The lib owns the binary-relevant path scope and refuses a failed timestamp
# probe instead of treating it as a current binary (#748).
rig_binary_stale_guard "$REPO_ROOT" "$PROFILE" "$BIN" || exit 1

[ "$CHECK_ONLY" = 1 ] && { echo "play-local: preflight OK"; exit 0; }

# 5. Fresh token for the account (t_o2jam_login holds the session guids;
#    tokens expire server-side, so always read the current one).
TOKEN="$(play_local_read_session_token "$MEMORYER_DIR/O2JAM.db" "$USER_ID")" || {
  echo "play-local: token lookup failed ($TOKEN)" >&2; exit 1;
}

# 6. Build once, then run. Sound on by default (#120); pass
# --no-sound in EXTRA for a silent rig.
[ -x "$BIN" ] || cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
cd "$REPO_ROOT"
echo "play-local: user-id $USER_ID, token ${TOKEN:0:8}…, memoryer 127.0.0.1:15010"
# The wrapper holds the flock until this client exits; exec keeps one
# process tree and traps inside the wrapper, not in the replaced shell.
# The flock is a RIG-lane guard (one boot lane machine-wide); owner-session
# multi-client testing is legitimate and takes no flock (owner report
# 2026-11-21: the second attended client waited forever on the first).
CLIENT_CMD=(
  "$BIN" online
  --host 127.0.0.1 --port 15010
  --user-id "$USER_ID" --token "$TOKEN"
  "${EXTRA[@]}"
)
if play_local_needs_live_lock "$RUN_CLASS"; then
  exec bash "$REPO_ROOT/scripts/lib/rig-live-lock.sh" \
    run --owner "play-local:uid=$USER_ID pid=$$" -- \
    "${CLIENT_CMD[@]}"
else
  exec "${CLIENT_CMD[@]}"
fi
