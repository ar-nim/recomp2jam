#!/usr/bin/env bash
# worktree-setup.sh — prepare a linked worktree: no push, private game data.
#
# Run this once, from inside an agent worktree, right after `git worktree add`.
# It points origin's push URL at scripts/no-push.sh (an ext:: remote helper
# that refuses with a clear message) using per-worktree config, so `git push`
# fails locally and never reaches origin. It also makes real recursive copies
# of the git-ignored game data from the main checkout: ~2.3G install-root and
# ~3.9M extracted. cp -a gives every worktree its own inodes (btrfs CoW
# reflinks are write-independent): ~0.1s locally, up to ~2.3G disk per copy
# where reflinks are unavailable. Never link or hardlink (cp -al): runs write
# into the data, and shared inodes or paths make one worktree's hidden-arm
# gate flake every other worktree.
#
# The main checkout is untouched: it stays the read-only copy source.
# Refuses to run in the main checkout. Idempotent.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MAIN_CHECKOUT="/home/arnim/Projects/recomp2jam"
GAME_DATA_DIRS=(install-root extracted)

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

require_real_source_dir() {
    local source_dir="$1"
    if [ ! -d "$source_dir" ] || [ -L "$source_dir" ]; then
        echo "worktree-setup: missing real data source $source_dir" >&2
        echo "worktree-setup: the main checkout must hold the game data" >&2
        exit 1
    fi
}

copy_game_data_dir() {
    local data_name="$1"
    local source_dir="$MAIN_CHECKOUT/$data_name"
    local target_dir="$REPO_ROOT/$data_name"
    local staging_dir

    require_real_source_dir "$source_dir"

    if [ -L "$target_dir" ]; then
        echo "worktree-setup: refusing: $target_dir is a symlink; remove it before setup" >&2
        exit 1
    fi
    if [ -e "$target_dir" ]; then
        [ -d "$target_dir" ] || { echo "worktree-setup: $target_dir is not a directory" >&2; exit 1; }
        echo "worktree-setup: $data_name already copied; skipping"
        return 0
    fi

    staging_dir="$(mktemp -d "$REPO_ROOT/.worktree-data.XXXXXX")"
    chmod --reference="$source_dir" "$staging_dir"
    if ! cp -a --reflink=auto -- "$source_dir/." "$staging_dir/"; then
        rm -rf -- "$staging_dir"
        echo "worktree-setup: copy failed: $source_dir -> $target_dir" >&2
        exit 1
    fi
    mv -T -- "$staging_dir" "$target_dir"
    echo "worktree-setup: copied $data_name ($source_dir -> $target_dir)"
}

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

for data_name in "${GAME_DATA_DIRS[@]}"; do
    copy_game_data_dir "$data_name"
done
