#!/usr/bin/env bash
# Seven-day staleness rule for the full-pack sweep (issue #737).
#
# A git-ignored stamp records the last successful full sweep. A fresh
# stamp keeps per-merge gates fast; an absent or old stamp makes the full
# gate run the pack sweep itself. No scheduler or always-on PC required.
set -euo pipefail

FULL_SWEEP_STALENESS_SECONDS=$((7 * 24 * 60 * 60))

sweep_stamp_is_stale() {
    local stamp="$1" modified now age
    [ -f "$stamp" ] || return 0
    modified="$(stat -c %Y "$stamp")" || return 0
    now="$(date +%s)"
    age=$((now - modified))
    [ "$age" -gt "$FULL_SWEEP_STALENESS_SECONDS" ]
}
