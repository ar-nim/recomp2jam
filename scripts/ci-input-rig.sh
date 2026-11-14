#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$REPO_ROOT/.ci-rig"
rm -rf "$OUT"; mkdir -p "$OUT"
scripts/rig-run.sh run :99 "$OUT" -- "$REPO_ROOT/target/debug/recomp2jam" --input-log "$OUT/input.log" &
sleep 2
for _ in $(seq 1 50); do xdpyinfo -display :99 >/dev/null 2>&1 && break; sleep 0.2; done
export DISPLAY=:99
xdotool mousemove 120 80
xdotool click 1
sleep 2
scripts/rig-run.sh stop "$OUT" || true
if [ ! -f "$OUT/input.log" ]; then echo "FAIL: no input log" >&2; exit 1; fi
if ! grep -qE '^click [0-9]+ [0-9]+$' "$OUT/input.log"; then echo "FAIL: click not logged" >&2; exit 1; fi
if ! grep -qE '^click-up [0-9]+ [0-9]+$' "$OUT/input.log"; then echo "FAIL: click-up not logged" >&2; exit 1; fi
echo "PASS: injected move+click arrived"
