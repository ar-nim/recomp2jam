#!/usr/bin/env bash
# rig-channel-digits.sh — drive the shots walk and assert the painted
# Channel fill digits against the exe-pinned `channel_id + 1` rule.
#
# Usage: scripts/rig-channel-digits.sh [OUT_DIR [IDS]]
#   OUT_DIR defaults to a fresh temp dir; IDS defaults to 0,1,2,3 (the
#   four open rows the shots walk stages at the native canvas).
#
# No game bytes enter the repo. Without install-root the probe skips.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-$(mktemp -d)}"
IDS="${2:-0,1,2,3}"

if [ ! -f "$ROOT/install-root/O2Jam/Image/Interface1.opi" ]; then
    echo "rig-channel-digits: SKIP (no install-root Interface1.opi)"
    exit 0
fi

mkdir -p "$OUT"
cargo run --quiet --manifest-path "$ROOT/Cargo.toml" \
    -p recomp2jam --bin recomp2jam -- shots --out-dir "$OUT" >/dev/null
exec python3 "$ROOT/scripts/rig-channel-digits-probe.py" \
    --frame "$OUT/channel-digits-native.png" \
    --ids "$IDS" \
    --install-root "$ROOT/install-root"
