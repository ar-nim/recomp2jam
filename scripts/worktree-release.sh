#!/usr/bin/env bash
# worktree-release.sh — give up this worktree's pooled data slot.
#
# Removes the worktree's data symlinks and the claim marker, then drops the
# slot lock. Slot data stays on disk for the next worktree to refresh. Safe
# to run from any linked worktree; refuses in the main checkout.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$(dirname "${BASH_SOURCE[0]}")/lib/install-data-pool.sh"
MAIN_CHECKOUT="/home/arnim/Projects/recomp2jam"

TOPLEVEL="$(git -C "$REPO_ROOT" rev-parse --show-toplevel)"
if [ "$TOPLEVEL" = "$MAIN_CHECKOUT" ]; then
    echo "worktree-release: refusing to run in the main checkout" >&2
    exit 2
fi

slot_name="$(install_data_pool_marker_slot "$REPO_ROOT")" || {
    echo "worktree-release: no $POOL_WORKTREE_MARKER claim marker in $REPO_ROOT" >&2
    exit 1
}

install_data_pool_unlink_worktree "$REPO_ROOT"
install_data_pool_release_slot "$slot_name"
echo "worktree-release: released $slot_name (data kept at $POOL_ROOT/$slot_name)"
