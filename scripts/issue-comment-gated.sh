#!/usr/bin/env bash
# Post a close or progress comment to a GitHub issue, but only after the
# comment passes the citation and guard gates.
#
# Usage:
#   scripts/issue-comment-gated.sh <issue> <comment-file> [--close]
#
# Gates, in order; any failure stops before anything is posted:
#   1. scripts/check-cited-hashes.sh   (exit 1 on unresolved OR stale)
#   2. scripts/check-guard-evidence.sh --close-gate  (only with --close)
#
# A progress comment on an open story must not use --close: the close-gate
# is for closing comments, and a tier-0-only progress note is legitimate.
#
# Why this exists: on 2026-11-26 two comments with a wrong or abbreviated
# evidence id were posted because the check and the post sat in one shell
# line and a failing check inside a loop body did not stop the post.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

usage() {
    echo "usage: $0 <issue> <comment-file> [--close]" >&2
    exit 2
}

[ $# -ge 2 ] && [ $# -le 3 ] || usage
issue="$1"
comment_file="$2"
mode="${3:-}"
case "$mode" in
    ''|--close) ;;
    *) usage ;;
esac
[ -f "$comment_file" ] || { echo "no such comment file: $comment_file" >&2; exit 2; }
[[ "$issue" =~ ^[0-9]+$ ]] || { echo "issue must be a number: $issue" >&2; exit 2; }

run_gate() {
    local label="$1"
    shift
    echo "== $label"
    if ! "$@"; then
        echo "issue-comment-gated: GATE FAILED ($label); nothing posted to #$issue" >&2
        exit 1
    fi
}

run_gate "cited hashes" scripts/check-cited-hashes.sh "$comment_file"
if [ "$mode" = "--close" ]; then
    run_gate "guard evidence (close-gate)" \
        scripts/check-guard-evidence.sh --close-gate "$comment_file"
fi

gh issue comment "$issue" --body-file "$comment_file" >/dev/null
echo "issue-comment-gated: posted to #$issue"
