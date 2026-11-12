#!/usr/bin/env bash
# play-local.sh — build (if needed) and run the client against the local
# Memoryer, reading the session token from its database automatically.
#
# Usage: scripts/play-local.sh [--user-id N] [extra recomp2jam online args...]
#   --user-id 1 = tester, 2 = tester2 (default 2; add accounts in Memoryer)
# Examples:
#   scripts/play-local.sh                       # plain run, sound on
#   scripts/play-local.sh --create-room 1       # host room 1 directly
#   scripts/play-local.sh --input-log /tmp/in.log --frame-dump /tmp/frames
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MEMORYER_DIR="${MEMORYER_DIR:-$HOME/memoryer-local}"
USER_ID=2
EXTRA=()

while [ $# -gt 0 ]; do
  case "$1" in
    --user-id) USER_ID="$2"; shift 2 ;;
    *) EXTRA+=("$1"); shift ;;
  esac
done

# 1. Game data: the client stages art from install-root (git-ignored).
if [ ! -d "$REPO_ROOT/install-root/O2Jam" ]; then
  echo "play-local: install-root/O2Jam missing — link the game data first:" >&2
  echo "  ln -s /path/to/your/install-root \"$REPO_ROOT/install-root\"" >&2
  exit 1
fi

# 2. Memoryer must be listening.
if ! ss -ltn 2>/dev/null | grep -q ':15010 '; then
  echo "play-local: Memoryer is not running. Start it with:" >&2
  echo "  cd ~/memoryer-local && setsid nohup env PATH=\$HOME/.dotnet:\$PATH dotnet bin/Memoryer.Encore.dll > server.log 2>&1 &" >&2
  exit 1
fi

# 3. Fresh token for the account (t_o2jam_login holds the session guids;
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

# 4. Build once, then run. Sound on by default (#120); pass
# --no-sound in EXTRA for a silent rig.
BIN="$REPO_ROOT/target/release/recomp2jam"
[ -x "$BIN" ] || cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
cd "$REPO_ROOT"
echo "play-local: user-id $USER_ID, token ${TOKEN:0:8}…, memoryer 127.0.0.1:15010"
exec env WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}" "$BIN" online \
  --host 127.0.0.1 --port 15010 \
  --user-id "$USER_ID" --token "$TOKEN" \
  "${EXTRA[@]}"
