#!/usr/bin/env bash
# issue-create.sh — create ONE GitHub issue with verification.
#
# Usage: scripts/issue-create.sh --title "..." --body-file path --type Bug|Task|Feature [gh-label-flags...]
#    or: scripts/issue-create.sh --title "..." --body-file path --type Feature --label component:shell --label ready-for-agent
#
# Why this exists (retro 2026-11-12): a batch loop that split heredoc text on
# '|' shredded markdown tables into 20 junk issues (#143-#162). The failure
# was structural — delimiter-splitting free text — so the fix is structural:
# one create per invocation, a body passed by FILE (never interpolated), and
# a read-back of the created issue's number, title, and labels so the caller
# diffs intent against reality immediately.
#
# GitHub cannot delete issues via CLI/API (web UI only). Mistakes get the
# `to-remove` label for the owner's bulk deletion:
#   gh issue edit <n> --add-label to-remove
set -euo pipefail

TITLE=""
BODY_FILE=""
TYPE=""
LABELS=()

while [ $# -gt 0 ]; do
  case "$1" in
    --title) TITLE="$2"; shift 2 ;;
    --body-file) BODY_FILE="$2"; shift 2 ;;
    --type) TYPE="$2"; shift 2 ;;
    --label) LABELS+=(--label "$2"); shift 2 ;;
    *) echo "issue-create: unknown arg $1" >&2; exit 2 ;;
  esac
done

[ -n "$TITLE" ] || { echo "issue-create: --title required" >&2; exit 2; }
[ -n "$BODY_FILE" ] && [ -f "$BODY_FILE" ] || { echo "issue-create: --body-file missing/unreadable" >&2; exit 2; }
case "$TYPE" in
  Bug|Task|Feature) ;;
  *) echo "issue-create: --type Bug|Task|Feature required (got '$TYPE')" >&2; exit 2 ;;
esac
[ "${#TITLE}" -ge 12 ] || { echo "issue-create: title suspiciously short ($TITLE)" >&2; exit 2; }
case "$TITLE" in
  "|"*|\#\#*|-*\|*) echo "issue-create: title looks like a markdown fragment ($TITLE)" >&2; exit 2 ;;
esac

URL=$(gh issue create --title "$TITLE" --body-file "$BODY_FILE" --type "$TYPE" "${LABELS[@]}")
NUM=${URL##*/}

# Read-back: the caller must see number + title + type + labels and diff vs intent.
gh issue view "$NUM" --json number,title,issueType,labels \
  --jq '"created: #\(.number) [\(.issueType.name)] [\([.labels[].name] | join(","))] \(.title)"'
echo "$URL"
