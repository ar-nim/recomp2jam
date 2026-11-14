#!/usr/bin/env bash
# worktree-setup.sh — make the current git worktree push-proof.
#
# Run this once, from inside an agent worktree, right after `git worktree add`.
# It points origin's push URL at scripts/no-push.sh (an ext:: remote helper
# that refuses with a clear message) using per-worktree config, so `git push`
# fails locally and never reaches origin. The main checkout is untouched.
#
# Refuses to run in the main checkout. Idempotent.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
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
