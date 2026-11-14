#!/usr/bin/env bash
# no-push.sh — remote helper invoked by agent worktrees' remote.origin.pushurl.
#
# Prints a clear refusal and exits non-zero so `git push` from an agent
# worktree fails locally, before any network or remote is touched.
# See docs/agents/issue-tracker.md ("Worktree setup").
echo "push disabled: agent worktrees may not push to origin (see docs/agents/issue-tracker.md)" >&2
exit 1
