#!/usr/bin/env bash
# Per-worktree pre-push hook installed by scripts/worktree-setup.sh (issue #488).
# Guard only: `git push --no-verify` bypasses hooks. The no-push rule is a
# hard rule; the reviewer enforces it regardless of this hook.
echo "push refused: agent worktrees never push to origin (see docs/agents/delegation.md)" >&2
exit 1
