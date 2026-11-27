#!/usr/bin/env bash
# Re-mint t240's launch argv from a read-only Memoryer DB and verify its sha256.
#
# Usage:
#   scripts/rig-verify-remint.sh --db PATH --work-dir DIR [--manifest FILE]
#       [--pin-file FILE] [--mint-script FILE]
#   scripts/rig-verify-remint.sh --help
#
# Writes go only to the resolved --work-dir and the new --manifest file.
# Both targets are resolved and checked against their roots before any write.
# No Wine, no exe, no network.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PIN_FILE="$REPO_ROOT/docs/captures/t240/wine-argv-fields.txt"
MINT_SCRIPT="$REPO_ROOT/scripts/rig-mint-launch-argv.sh"
DB=""
WORK_DIR_ARG=""
MANIFEST_ARG=""

die() { echo "rig-verify-remint: $*" >&2; exit 2; }

usage() {
  cat <<'USAGE'
usage: rig-verify-remint.sh --db PATH --work-dir DIR [options]
  --db PATH           read-only Memoryer DB used by the mint script
  --work-dir DIR      resolved scratch dir; parent must exist
  --manifest FILE     new provenance file; scratch or docs/captures/t240
  --pin-file FILE     field record to read (default: t240 wine-argv-fields.txt)
  --mint-script FILE  mint script to invoke (default: scripts/rig-mint-launch-argv.sh)
  --help              print this help
exit 0 on a hash match, 1 on a mismatch with both hashes printed.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --db) DB="$2"; shift 2 ;;
    --work-dir) WORK_DIR_ARG="$2"; shift 2 ;;
    --manifest) MANIFEST_ARG="$2"; shift 2 ;;
    --pin-file) PIN_FILE="$2"; shift 2 ;;
    --mint-script) MINT_SCRIPT="$2"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
done

[ -n "$DB" ] || die "--db PATH is required"
[ -n "$WORK_DIR_ARG" ] || die "--work-dir DIR is required"
[ -r "$DB" ] || die "--db PATH is not readable: $DB"
[ -r "$PIN_FILE" ] || die "--pin-file FILE is not readable: $PIN_FILE"
[ -r "$MINT_SCRIPT" ] || die "--mint-script FILE is not readable: $MINT_SCRIPT"

is_within() {
  local target="$1" root="$2"
  [ "$target" = "$root" ] || [[ "$target" == "$root"/* ]]
}

resolve_write_path() {
  local label="$1" path="$2" parent resolved
  case "$path" in
    /*) ;;
    *) die "$label must be absolute: $path" ;;
  esac
  [ ! -L "$path" ] || die "$label must not be a symlink: $path"
  if [ -e "$path" ]; then
    resolved="$(realpath -e -- "$path")" || die "$label cannot be resolved: $path"
  else
    parent="$(dirname -- "$path")"
    [ -d "$parent" ] || die "$label parent directory does not exist: $parent"
    resolved="$(realpath -e -- "$parent")" || die "$label parent cannot be resolved: $parent"
    resolved="$resolved/$(basename -- "$path")"
  fi
  printf '%s\n' "$resolved"
}

require_within_roots() {
  local label="$1" path="$2" resolved root
  shift 2
  resolved="$(resolve_write_path "$label" "$path")" || exit 2
  for root in "$@"; do
    if is_within "$resolved" "$root"; then
      printf '%s\n' "$resolved"
      return 0
    fi
  done
  die "$label resolves outside the allowed roots: $path -> $resolved"
}

SCRATCH_ROOT="$(realpath -e -- "$HOME/scratch")" || die "cannot resolve \$HOME/scratch"
RECORD_ROOT="$(realpath -e -- "$REPO_ROOT/docs/captures/t240")" || die "cannot resolve docs/captures/t240"
WORK_DIR="$(require_within_roots "--work-dir DIR" "$WORK_DIR_ARG" "$SCRATCH_ROOT")"
mkdir -p "$WORK_DIR"
ARGV_FILE="$WORK_DIR/rebuilt-argv.txt"
for output in "$ARGV_FILE" "$ARGV_FILE.sha256" "$ARGV_FILE.fields"; do
  checked="$(require_within_roots "--work-dir DIR output" "$output" "$SCRATCH_ROOT")"
  [ "$checked" = "$output" ] || die "--work-dir DIR output changed during validation: $output -> $checked"
done
MANIFEST=""
if [ -n "$MANIFEST_ARG" ]; then
  MANIFEST="$(require_within_roots "--manifest FILE" "$MANIFEST_ARG" "$SCRATCH_ROOT" "$RECORD_ROOT")"
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

MINT_SCRIPT_SHA256="$(sha256sum "$MINT_SCRIPT" | awk '{print $1}')"
DB_SHA256="$(sha256sum "$DB" | awk '{print $1}')"
DB_SIZE_BYTES="$(wc -c < "$DB" | tr -d '[:space:]')"

echo "input: db=$DB (sha256 $DB_SHA256, $DB_SIZE_BYTES bytes)"
echo "input: pin_file=$PIN_FILE"
echo "input: mint_script=$MINT_SCRIPT (sha256 $MINT_SCRIPT_SHA256)"
echo "input: work_dir=$WORK_DIR_ARG (resolved $WORK_DIR)"
[ -z "$MANIFEST_ARG" ] || echo "input: manifest=$MANIFEST_ARG (resolved $MANIFEST)"
echo "input: user_index_id=$USER_INDEX_ID (from pin file)"
echo "input: gateway_address=$GATEWAY_ADDRESS (from pin file)"
echo "input: gateway_port=$GATEWAY_PORT (from pin file)"

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

declare -A MANIFEST_INPUTS=(
  [created_at]="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  [user_index_id]="$USER_INDEX_ID"
  [memoryer_db_sha256]="$DB_SHA256"
  [memoryer_db_size_bytes]="$DB_SIZE_BYTES"
  [gateway_address]="$GATEWAY_ADDRESS"
  [gateway_port]="$GATEWAY_PORT"
  [pin_file]="${PIN_FILE#"$REPO_ROOT"/}"
  [mint_script]="${MINT_SCRIPT#"$REPO_ROOT"/}"
  [mint_script_sha256]="$MINT_SCRIPT_SHA256"
  [argv_sha256]="$ACTUAL_HASH"
  [pinned_argv_sha256]="$EXPECTED_HASH"
)

write_manifest() {
  local -a arguments=("-" "$MANIFEST")
  local key
  for key in "${!MANIFEST_INPUTS[@]}"; do
    arguments+=("$key" "${MANIFEST_INPUTS[$key]}")
  done
  python3 "${arguments[@]}" <<'PY'
import hashlib
import json
import sys

path = sys.argv[1]
values = dict(zip(sys.argv[2::2], sys.argv[3::2]))
input_keys = ("user_index_id", "memoryer_db_sha256", "memoryer_db_size_bytes",
              "gateway_address", "gateway_port", "pin_file", "mint_script",
              "mint_script_sha256")
inputs = {key: values[key] for key in input_keys}
inputs["memoryer_db_size_bytes"] = int(inputs["memoryer_db_size_bytes"])
document = {
    "schema": "rig-provenance-v1", "kind": "remint-verify",
    "created_at": values["created_at"], "issue": "#889",
    "citation": "wine-citation.txt", "inputs": inputs,
    "result": {"argv_sha256": values["argv_sha256"],
                "pinned_argv_sha256": values["pinned_argv_sha256"],
                "match": True},
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
  write_manifest
  echo "manifest=$MANIFEST"
fi
