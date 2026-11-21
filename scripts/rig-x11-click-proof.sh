#!/usr/bin/env bash
# rig-x11-click-proof.sh — the real-window click proof lives in the rig lane (#738).
#
# The unit suite excludes `x11_arm_routes_click_through_real_game`: that
# test spawns Xvfb, boots the real game, and logs into the shared Memoryer.
# This script is its only gate-bearing caller. It checks the shared rig
# preflight, then holds the live-lane flock for the whole proof so an
# owner boot cannot race it. All output stays on the virtual display with
# the probe's --no-sound fence; no speaker audio is produced.
#
# Usage: scripts/rig-x11-click-proof.sh [--timeout SECONDS] [--dry-run]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT_DIR="$REPO_ROOT/scripts"
# shellcheck source=lib/rig-preflight.sh
. "$SCRIPT_DIR/lib/rig-preflight.sh"
# shellcheck source=lib/rig-live-lock.sh
. "$SCRIPT_DIR/lib/rig-live-lock.sh"

TIMEOUT="${RIG_LIVE_LOCK_TIMEOUT:-120}"
DRY_RUN=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --timeout) TIMEOUT="$2"; shift 2 ;;
        --dry-run) DRY_RUN=1; shift ;;
        *) echo "usage: rig-x11-click-proof.sh [--timeout SECONDS] [--dry-run]" >&2; exit 2 ;;
    esac
done

PROOF=(cargo test -p input-drive x11_arm_routes_click_through_real_game -- --exact)

if [ "$DRY_RUN" -eq 1 ]; then
    printf 'preflight: install-root + Memoryer (rig-preflight.sh)\n'
    printf 'flock: owner=x11-click-proof timeout=%s\n' "$TIMEOUT"
    printf 'proof: %s\n' "${PROOF[*]}"
    exit 0
fi

cd "$REPO_ROOT"
rig_check_install_root || exit "$RIG_E_DATA"
# The test logs into the one shared Memoryer; refuse to start a second
# instance even though memoryer-start.sh --check-only is the usual wrapper.
rig_check_memoryer || exit "$RIG_E_SERVER"

rig_live_lock_run "x11-click-proof" "$TIMEOUT" -- "${PROOF[@]}"
