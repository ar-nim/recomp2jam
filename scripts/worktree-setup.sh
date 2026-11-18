#!/usr/bin/env bash
# worktree-setup.sh — prepare a linked worktree: no push, pooled game data.
#
# Run this once, from inside an agent worktree, right after `git worktree add`.
# It points origin's push URL at scripts/no-push.sh (an ext:: remote helper
# that refuses with a clear message) using per-worktree config, so `git push`
# fails locally and never reaches origin.
#
# It also claims one of four persistent data slots under
# ~/scratch/install-data/slot{1..4}, refreshes that slot from the read-only
# main checkout, and symlinks this worktree's install-root/extracted at the
# slot. Slots are never shared: a hidden-arm run cannot flake another
# worktree. A claim marker is the lock; scripts/worktree-release.sh drops it
# without deleting the slot data. Refreshing costs ~0.1s here on btrfs CoW
# (cp -a, since rsync is absent), up to ~2.3G where reflinks are unavailable.
# Refuses to run in the main checkout. Idempotent for an already-linked tree.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$(dirname "${BASH_SOURCE[0]}")/lib/install-data-pool.sh"
MAIN_CHECKOUT="/home/arnim/Projects/recomp2jam"

# Guard 1: never run in the main checkout.
TOPLEVEL="$(git -C "$REPO_ROOT" rev-parse --show-toplevel)"
if [ "$TOPLEVEL" = "$MAIN_CHECKOUT" ]; then
    echo "worktree-setup: refusing to run in the main checkout (push path must stay intact)" >&2
    exit 2
fi

# Guard 2: must actually be a linked worktree (git-dir lives under worktrees/).
GIT_DIR="$(git -C "$REPO_ROOT" rev-parse --git-dir)"
case "$GIT_DIR" in
    */worktrees/*) ;;
    *)
        echo "worktree-setup: refusing: not a linked worktree (git-dir=$GIT_DIR)" >&2
        exit 2
        ;;
esac

verify_slot_symlinks() {
    local slot_name="$1"
    local data_name target expected
    for data_name in "${POOL_DATA_DIRS[@]}"; do
        target="$REPO_ROOT/$data_name"
        expected="$POOL_ROOT/$slot_name/$data_name"
        if [ ! -L "$target" ] || [ "$(readlink "$target")" != "$expected" ]; then
            return 1
        fi
    done
}

refuse_stale_data_entries() {
    local data_name target
    for data_name in "${POOL_DATA_DIRS[@]}"; do
        target="$REPO_ROOT/$data_name"
        if [ -e "$target" ] || [ -L "$target" ]; then
            echo "worktree-setup: refusing existing $target; remove the stale entry first" >&2
            exit 1
        fi
    done
}

report_busy_slots() {
    local slot_number owner_file slot_dir
    echo "worktree-setup: all $POOL_SLOT_COUNT data slots are claimed (max 4 concurrent worktrees)" >&2
    for ((slot_number = 1; slot_number <= POOL_SLOT_COUNT; slot_number++)); do
        slot_dir="$POOL_ROOT/slot$slot_number"
        owner_file="$slot_dir/$POOL_CLAIM_DIR/$POOL_OWNER_FILE"
        if [ -f "$owner_file" ]; then
            printf '  slot%d: %s\n' "$slot_number" "$(<"$owner_file")" >&2
        fi
    done
}

claimed_slot=""
setup_complete=0
cleanup_failed_claim() {
    if [ "$setup_complete" -eq 0 ] && [ -n "$claimed_slot" ]; then
        install_data_pool_release_slot "$claimed_slot" || true
    fi
}
trap cleanup_failed_claim EXIT

existing_slot="$(install_data_pool_marker_slot "$REPO_ROOT")" || existing_slot=""
if [ -n "$existing_slot" ]; then
    if ! verify_slot_symlinks "$existing_slot"; then
        echo "worktree-setup: marker names $existing_slot but the data links disagree" >&2
        echo "worktree-setup: run scripts/worktree-release.sh, then retry setup" >&2
        exit 1
    fi
    echo "worktree-setup: already linked to $existing_slot; skipping claim and refresh"
    setup_complete=1
    exit 0
fi

NO_PUSH="$REPO_ROOT/scripts/no-push.sh"
[ -x "$NO_PUSH" ] || { echo "worktree-setup: missing $NO_PUSH" >&2; exit 1; }

# Per-worktree config requires the extension; enable it if missing (shared
# feature flag only, never pushurl/transport settings).
if [ "$(git -C "$REPO_ROOT" config --get extensions.worktreeConfig 2>/dev/null || true)" != "true" ]; then
    git -C "$REPO_ROOT" config extensions.worktreeConfig true
fi

git -C "$REPO_ROOT" config --worktree protocol.ext.allow always
git -C "$REPO_ROOT" config --worktree remote.origin.pushurl "ext::$NO_PUSH"

echo "worktree-setup: pushurl set (worktree config) to ext::$NO_PUSH"
echo "worktree-setup: verifying push is blocked..."
if git -C "$REPO_ROOT" push origin HEAD 2>&1; then
    echo "worktree-setup: ERROR: push unexpectedly succeeded" >&2
    exit 1
else
    echo "worktree-setup: OK — push refused locally"
fi

refuse_stale_data_entries
claimed_slot="$(install_data_pool_claim_slot "$REPO_ROOT")" || {
    report_busy_slots
    exit 1
}
echo "worktree-setup: claimed $claimed_slot; refreshing from $POOL_SOURCE_ROOT"
install_data_pool_refresh_slot "$claimed_slot"
install_data_pool_link_worktree "$REPO_ROOT" "$claimed_slot"
echo "worktree-setup: linked install-root/ and extracted/ to $POOL_ROOT/$claimed_slot"
setup_complete=1
