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
# Audio policy is "attended" when sound is on (a human at a TTY acknowledges
# the speakers) and "null" with --no-sound; scripted runs (no TTY) with
# sound on are rejected so automated audio never reaches the speakers.
set -euo pipefail

# shellcheck source=lib/rig-preflight.sh
. "$(cd "$(dirname "$0")/lib" && pwd)/rig-preflight.sh"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MEMORYER_DIR="${MEMORYER_DIR:-$HOME/memoryer-local}"
USER_ID=2
CHECK_ONLY=0
EXTRA=()

while [ $# -gt 0 ]; do
  case "$1" in
    --user-id) USER_ID="$2"; shift 2 ;;
    --check-only) CHECK_ONLY=1; shift ;;
    *) EXTRA+=("$1"); shift ;;
  esac
done

RIG_INSTALL_ROOT="${RIG_INSTALL_ROOT:-$REPO_ROOT/install-root/O2Jam}"
export RIG_INSTALL_ROOT

# 1. Game data: the client stages art from install-root (git-ignored).
if ! rig_check_install_root; then
  echo "play-local: link the game data first:" >&2
  echo "  ln -s /path/to/your/install-root \"$REPO_ROOT/install-root\"" >&2
  exit "$RIG_E_DATA"
fi

# 2. Memoryer must be up and answering (connect probe, not just listening).
if ! rig_check_memoryer; then
  echo "play-local: Memoryer is not running. Start it with:" >&2
  echo "  cd ~/memoryer-local && setsid nohup env PATH=\$HOME/.dotnet:\$PATH dotnet bin/Memoryer.Encore.dll > server.log 2>&1 &" >&2
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
BIN="$REPO_ROOT/target/release/recomp2jam"
HEAD_TS="$(git -C "$REPO_ROOT" log -1 --format=%ct -- crates/ Cargo.toml Cargo.lock 2>/dev/null || echo 0)"
if [ -x "$BIN" ] && [ "${HEAD_TS:-0}" -gt 0 ]; then
  BIN_TS="$(stat -c %Y "$BIN" 2>/dev/null || echo 0)"
  if [ "${BIN_TS:-0}" -lt "$HEAD_TS" ]; then
    echo "play-local: release binary is stale (built before checkout HEAD ${HEAD_TS})" >&2
    echo "play-local: rebuild first: cargo build --release --manifest-path \"$REPO_ROOT/Cargo.toml\"" >&2
    exit 1
  fi
fi

[ "$CHECK_ONLY" = 1 ] && { echo "play-local: preflight OK"; exit 0; }

# 5. Fresh token for the account (t_o2jam_login holds the session guids;
#    tokens expire server-side, so always read the current one).
TOKEN=$(python3 - "$MEMORYER_DIR/O2JAM.db" "$USER_ID" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
uid = sys.argv[2]
row = db.execute(
    "select TUSER_ID from t_o2jam_login where USER_INDEX_ID=? "
    "order by LOGIN_TIME desc limit 1", (uid,)).fetchone()
if not row:
    sys.exit(f"no login row for USER_INDEX_ID={uid} in {sys.argv[1]}")
print(row[0])
PY
) || { echo "play-local: token lookup failed ($TOKEN)" >&2; exit 1; }

# 6. Build once, then run. Sound on by default (#120); pass
# --no-sound in EXTRA for a silent rig.
[ -x "$BIN" ] || cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
cd "$REPO_ROOT"
echo "play-local: user-id $USER_ID, token ${TOKEN:0:8}…, memoryer 127.0.0.1:15010"
exec env WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}" "$BIN" online \
  --host 127.0.0.1 --port 15010 \
  --user-id "$USER_ID" --token "$TOKEN" \
  "${EXTRA[@]}"
