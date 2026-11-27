#!/usr/bin/env bash
# Re-mint t240's launch argv from a read-only Memoryer DB and verify its sha256.
#
# Usage:
#   scripts/rig-verify-remint.sh --db PATH --work-dir DIR [--manifest FILE]
#       [--pin-file FILE] [--mint-script FILE]
#   scripts/rig-verify-remint.sh --help
#
# --work-dir and the optional --manifest are the only write targets;
# --work-dir must live under $HOME/scratch. No Wine, no exe, no network.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PIN_FILE="$REPO_ROOT/docs/captures/t240/wine-argv-fields.txt"
MINT_SCRIPT="$REPO_ROOT/scripts/rig-mint-launch-argv.sh"
DB=""
WORK_DIR=""
MANIFEST=""

die() { echo "rig-verify-remint: $*" >&2; exit 2; }

usage() {
  cat <<'USAGE'
usage: rig-verify-remint.sh --db PATH --work-dir DIR [options]
  --db PATH           read-only Memoryer DB used by the mint script
  --work-dir DIR      scratch dir for the rebuilt argv (must be under $HOME/scratch)
  --manifest FILE     new provenance file, written only after a hash match
  --pin-file FILE     field record to read (default: t240 wine-argv-fields.txt)
  --mint-script FILE  mint script to invoke (default: scripts/rig-mint-launch-argv.sh)
  --help              print this help
exit 0 on a hash match, 1 on a mismatch with both hashes printed.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --db) DB="$2"; shift 2 ;;
    --work-dir) WORK_DIR="$2"; shift 2 ;;
    --manifest) MANIFEST="$2"; shift 2 ;;
    --pin-file) PIN_FILE="$2"; shift 2 ;;
    --mint-script) MINT_SCRIPT="$2"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
done

[ -n "$DB" ] || die "--db PATH is required"
[ -n "$WORK_DIR" ] || die "--work-dir DIR is required"
[ -r "$DB" ] || die "--db PATH is not readable: $DB"
[ -r "$PIN_FILE" ] || die "--pin-file FILE is not readable: $PIN_FILE"
[ -r "$MINT_SCRIPT" ] || die "--mint-script FILE is not readable: $MINT_SCRIPT"
case "$WORK_DIR" in
  "$HOME"/scratch/*) ;;
  *) die "--work-dir DIR must be under $HOME/scratch: $WORK_DIR" ;;
esac
if [ -n "$MANIFEST" ]; then
  case "$MANIFEST" in
    "$HOME"/scratch/*|"$REPO_ROOT"/docs/captures/t240/*) ;;
    *) die "--manifest FILE must be under $HOME/scratch or docs/captures/t240: $MANIFEST" ;;
  esac
  [ ! -e "$MANIFEST" ] || die "--manifest FILE already exists: $MANIFEST"
fi

read_pin() {
  awk -F= -v key="$1" '$1 == key { print $2; exit }' "$PIN_FILE"
}

USER_INDEX_ID="$(read_pin user_index_id)"
GATEWAY_ADDRESS="$(read_pin gateway_address)"
GATEWAY_PORT="$(read_pin gateway_port)"
EXPECTED_HASH="$(read_pin argv_sha256)"
for field in USER_INDEX_ID GATEWAY_ADDRESS GATEWAY_PORT EXPECTED_HASH; do
  [ -n "${!field}" ] || die "missing $field in --pin-file FILE: $PIN_FILE"
done

echo "input: db=$DB"
echo "input: pin_file=$PIN_FILE"
echo "input: mint_script=$MINT_SCRIPT (sha256 $(sha256sum "$MINT_SCRIPT" | awk '{print $1}'))"
echo "input: work_dir=$WORK_DIR"
echo "input: user_index_id=$USER_INDEX_ID (from pin file)"
echo "input: gateway_address=$GATEWAY_ADDRESS (from pin file)"
echo "input: gateway_port=$GATEWAY_PORT (from pin file)"

mkdir -p "$WORK_DIR"
ARGV_FILE="$WORK_DIR/rebuilt-argv.txt"
CARGO_TARGET_DIR="$WORK_DIR/cargo-target"
export CARGO_TARGET_DIR
bash "$MINT_SCRIPT" --user-index-id "$USER_INDEX_ID" --db "$DB" \
  --out "$ARGV_FILE" --gateway-address "$GATEWAY_ADDRESS" \
  --gateway-port "$GATEWAY_PORT"

ACTUAL_HASH="$(sha256sum "$ARGV_FILE" | awk '{print $1}')"
SIDECAR_HASH="$(cat "$ARGV_FILE.sha256")"
[ "$ACTUAL_HASH" = "$SIDECAR_HASH" ] || die "mint sidecar sha256 $SIDECAR_HASH differs from sha256sum $ACTUAL_HASH"
echo "expected_argv_sha256=$EXPECTED_HASH"
echo "actual_argv_sha256=$ACTUAL_HASH"

if [ "$ACTUAL_HASH" != "$EXPECTED_HASH" ]; then
  echo "REMINT MISMATCH: rebuilt argv differs from the t240 pin" >&2
  exit 1
fi
echo "REMINT MATCH: rebuilt argv equals the t240 pin"

write_manifest() {
  python3 - "$1" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    "$(sha256sum "$MINT_SCRIPT" | awk '{print $1}')" "$ACTUAL_HASH" \
    "$EXPECTED_HASH" "$DB" "$USER_INDEX_ID" "$GATEWAY_ADDRESS" \
    "$GATEWAY_PORT" "${PIN_FILE#"$REPO_ROOT"/}" \
    "${MINT_SCRIPT#"$REPO_ROOT"/}" <<'PY'
import hashlib
import json
import sys

(path, created_at, mint_sha256, argv_sha256, pinned_sha256, db,
 user_index_id, gateway_address, gateway_port, pin_file, mint_script) = sys.argv[1:]
document = {
    "schema": "rig-provenance-v1",
    "kind": "remint-verify",
    "created_at": created_at,
    "issue": "#889",
    "citation": "wine-citation.txt",
    "inputs": {
        "user_index_id": user_index_id,
        "memoryer_db": db,
        "gateway_address": gateway_address,
        "gateway_port": gateway_port,
        "pin_file": pin_file,
        "mint_script": mint_script,
        "mint_script_sha256": mint_sha256,
    },
    "result": {
        "argv_sha256": argv_sha256,
        "pinned_argv_sha256": pinned_sha256,
        "match": True,
    },
}
without_hash = {k: v for k, v in document.items() if k != "manifest_sha256"}
document["manifest_sha256"] = hashlib.sha256(
    json.dumps(without_hash, sort_keys=True, separators=(",", ":")).encode("utf-8")
).hexdigest()
with open(path, "x", encoding="utf-8") as stream:
    json.dump(document, stream, indent=2, sort_keys=True)
    stream.write("\n")
PY
}

if [ -n "$MANIFEST" ]; then
  mkdir -p "$(dirname "$MANIFEST")"
  write_manifest "$MANIFEST"
  echo "manifest=$MANIFEST"
fi
