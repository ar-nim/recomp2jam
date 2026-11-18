#!/usr/bin/env bash
# memoryer-start.sh — start the one local Memoryer with managed crash dumps (#345).
#
# Usage: scripts/memoryer-start.sh [--check-only]
#
# Refuses to start when RIG_MEMORYER_PORT (default 15010) is already taken:
# the 14:05:36 SIGABRT was a second instance failing to bind that port. The
# wrapper never signals or restarts a server; it only starts one when the
# port is free. DOTNET_Dbg* make the next unhandled exception land a managed
# heap dump that dotnet-dump can read, unlike a kernel core from systemd.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/rig-preflight.sh
. "$SCRIPT_DIR/lib/rig-preflight.sh"

CHECK_ONLY=0
case "${1:-}" in
  "") ;;
  --check-only) CHECK_ONLY=1 ;;
  *) echo "usage: memoryer-start.sh [--check-only]" >&2; exit 2 ;;
esac
[ "$#" -le 1 ] || { echo "usage: memoryer-start.sh [--check-only]" >&2; exit 2; }

PORT="${RIG_MEMORYER_PORT:-15010}"
MEMORYER_DIR="${MEMORYER_DIR:-$HOME/memoryer-local}"
DUMP_DIR="${MEMORYER_DUMP_DIR:-$HOME/scratch/memoryer-dumps}"
DOTNET_HOST="${MEMORYER_DOTNET:-$HOME/.dotnet/dotnet}"

if rig_port_in_use "$PORT"; then
  identity="$(rig_server_identity "$PORT" 2>/dev/null || true)"
  echo "memoryer-start: port $PORT is taken; refusing a second instance (${identity:-identity unknown})" >&2
  exit "$RIG_E_SERVER"
fi

if [ "$CHECK_ONLY" -eq 1 ]; then
  echo "memoryer-start: port $PORT free; managed dumps would go to $DUMP_DIR"
  exit 0
fi

[ -d "$MEMORYER_DIR/bin" ] || {
  echo "memoryer-start: no $MEMORYER_DIR/bin (set MEMORYER_DIR)" >&2
  exit "$RIG_E_DATA"
}
[ -x "$DOTNET_HOST" ] || {
  echo "memoryer-start: dotnet host '$DOTNET_HOST' is not executable" >&2
  exit 2
}
mkdir -p "$DUMP_DIR"

cd "$MEMORYER_DIR"
setsid nohup env PATH="$(dirname "$DOTNET_HOST"):$PATH" \
  DOTNET_DbgEnableMiniDump=1 DOTNET_DbgMiniDumpType=2 \
  DOTNET_DbgMiniDumpName="$DUMP_DIR/coredump.%p.%t.dmp" \
  "$DOTNET_HOST" bin/Memoryer.Encore.dll >> server.log 2>&1 < /dev/null &
echo "memoryer-start: started Memoryer (pid $!) on :$PORT; managed dump path $DUMP_DIR/coredump.<pid>.<epoch>.dmp"
