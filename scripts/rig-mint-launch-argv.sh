#!/usr/bin/env bash
# rig-mint-launch-argv.sh — write the OTwo.exe argv for a Memoryer account.
#
# Usage:
#   scripts/rig-mint-launch-argv.sh --user-index-id N --out FILE
#       [--db PATH] [--gateway-address ADDR] [--gateway-port PORT] [--cargo-locked]
#
# The launch identity comes from the running Memoryer database, read-only
# (`mode=ro`; no session is created, no row is written): the session token
# from `t_o2jam_login`, the level/gender/nickname from
# `t_o2jam_charinfo`, the rank from `t_o2jam_user_ranking`. The blob itself
# is minted by the committed #265 builder
# (`crates/o2jam-net/examples/mint_launch_argv.rs`, which wraps
# `o2jam_net::auth_parameters::assemble_launch_arguments`), never by hand:
# the re-brief on #240 replaced hand-built argv with that builder.
#
# Output: FILE holds the argv words one per line — the blob first, then the
# five `|test|??|addr|port` mirrors — which is what
# `scripts/rig-wine.sh launch FILE` execs. FILE also carries the account's
# session token (inside the blob), so it stays in the run's scratch dir and
# is never committed; `FILE.sha256` and `FILE.fields` are the shareable
# record.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

die() { echo "rig-mint-launch-argv: $*" >&2; exit 2; }

DB="${MEMORYER_DB:-$HOME/memoryer-local/O2JAM.db}"
USER_INDEX_ID=""
OUT=""
GATEWAY_ADDRESS="127.0.0.1"
GATEWAY_PORT="15010"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --user-index-id) USER_INDEX_ID="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --db) DB="$2"; shift 2 ;;
    --gateway-address) GATEWAY_ADDRESS="$2"; shift 2 ;;
    --gateway-port) GATEWAY_PORT="$2"; shift 2 ;;
    *) die "unknown option: $1" ;;
  esac
done

[ -n "$USER_INDEX_ID" ] || die "--user-index-id is required"
[ -n "$OUT" ] || die "--out FILE is required"
[ -r "$DB" ] || die "Memoryer database not readable: $DB"

identity="$(python3 - "$DB" "$USER_INDEX_ID" <<'PY'
import sqlite3, sys
path, user_index_id = sys.argv[1], int(sys.argv[2])
db = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
login = db.execute(
    "select USER_ID, TUSER_ID from t_o2jam_login where USER_INDEX_ID = ?",
    (user_index_id,),
).fetchone()
char = db.execute(
    "select USER_NICKNAME, Sex, Level from t_o2jam_charinfo where USER_INDEX_ID = ?",
    (user_index_id,),
).fetchone()
rank_row = db.execute(
    "select Ranking from t_o2jam_user_ranking where User_Index_ID = ?",
    (user_index_id,),
).fetchone()
if login is None or char is None:
    sys.exit(f"no login/charinfo row for user_index_id={user_index_id}")
user_id, token = login
nickname, gender, level = char
rank = rank_row[0] if rank_row else 0
print(user_id, token, level, gender, nickname, rank, sep="\t")
PY
)" || die "cannot read the identity for user_index_id=$USER_INDEX_ID from $DB"

IFS=$'\t' read -r user_id token level gender nickname rank <<<"$identity"

mkdir -p "$(dirname "$OUT")"
( cd "$REPO_ROOT" && cargo run -q -p o2jam-net --example mint_launch_argv -- \
    --user-index-id "$USER_INDEX_ID" \
    --username "$token" \
    --level "$level" \
    --gender "$gender" \
    --nickname "$nickname" \
    --rank "$rank" \
    --gateway-address "$GATEWAY_ADDRESS" \
    --gateway-port "$GATEWAY_PORT" ) > "$OUT" || die "the #265 builder failed"

words="$(wc -l < "$OUT")"
[ "$words" -eq 6 ] || die "argv file holds $words words; the pinned shape is a blob plus 5 mirrors"

sha256sum "$OUT" | awk '{ print $1 }' > "$OUT.sha256"
{
  echo "user_index_id=$USER_INDEX_ID"
  echo "user_id=$user_id"
  echo "username_sha256=$(printf '%s' "$token" | sha256sum | cut -d' ' -f1)"
  echo "level=$level"
  echo "gender=$gender"
  echo "nickname=$nickname"
  echo "rank=$rank"
  echo "gateway_address=$GATEWAY_ADDRESS"
  echo "gateway_port=$GATEWAY_PORT"
  echo "argv_words=$words"
  echo "argv_sha256=$(cat "$OUT.sha256")"
  echo "blob_chars=$(head -1 "$OUT" | wc -c)"
} > "$OUT.fields"

echo "rig-mint-launch-argv: wrote $OUT ($words words, sha256 $(cut -c1-12 < "$OUT.sha256")), field record $OUT.fields"
