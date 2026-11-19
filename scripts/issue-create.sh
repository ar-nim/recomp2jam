#!/usr/bin/env bash
# issue-create.sh — create ONE GitHub issue with verification and dedup.
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
# Dedup (retro #487): a resumed agent can re-run its creation step. This
# script normalizes doubled `Bug:`/`Task:` prefixes, searches OPEN issues with
# the normalized title, and refuses to create when a duplicate exists.
#
# GitHub cannot delete issues via CLI/API (web UI only). Mistakes get the
# `to-remove` label for the owner's bulk deletion:
#   gh issue edit <n> --add-label to-remove
set -euo pipefail

SEARCH_WORD_LIMIT=6
OPEN_ISSUE_SEARCH_LIMIT=100
DUPLICATE_EXIT=3

collapse_whitespace() {
  local text="${1//$'\t'/ }"
  text="${text//$'\r'/ }"
  text="${text//$'\n'/ }"
  while [[ "$text" == *"  "* ]]; do
    text="${text//  / }"
  done
  text="${text#"${text%%[! ]*}"}"
  text="${text%"${text##*[! ]}"}"
  printf '%s' "$text"
}

# Repeated type prefixes are authoring noise, not distinct titles (#377/#378).
normalize_issue_title() {
  local title lower prefix=""
  title=$(collapse_whitespace "$1")
  lower="${title,,}"
  while [[ "$lower" =~ ^(bug|task|feature):\ (.*)$ ]]; do
    if [ -z "$prefix" ]; then
      case "${BASH_REMATCH[1]}" in
        bug) prefix="Bug" ;;
        task) prefix="Task" ;;
        feature) prefix="Feature" ;;
      esac
    fi
    title="${title#*: }"
    lower="${lower#*: }"
  done
  if [ -n "$prefix" ]; then
    printf '%s: %s' "$prefix" "$title"
  else
    printf '%s' "$title"
  fi
}

# The dedup key drops the type prefix, case, and whitespace differences.
issue_title_dedup_key() {
  local title
  title=$(normalize_issue_title "$1")
  title="${title,,}"
  if [[ "$title" =~ ^(bug|task|feature):\ (.*)$ ]]; then
    title="${BASH_REMATCH[2]}"
  fi
  printf '%s' "$title"
}

build_duplicate_search_query() {
  local key="${1//\"/ }" query_phrase
  local -a words=()
  read -r -a words <<< "$key" || true
  query_phrase="${words[*]:0:SEARCH_WORD_LIMIT}"
  if [ -z "$query_phrase" ]; then
    query_phrase="$key"
  fi
  printf '"%s" in:title' "$query_phrase"
}

# Prints the number of the first open issue with the same key (or nothing).
find_duplicate_open_issue() {
  local title_key="$1" search_query search_results number title
  search_query=$(build_duplicate_search_query "$title_key")
  search_results=$(gh issue list --state open --limit "$OPEN_ISSUE_SEARCH_LIMIT" \
    --search "$search_query" --json number,title \
    --jq '.[] | "\(.number)\t\(.title)"')
  while IFS=$'\t' read -r number title; do
    [ -n "$number" ] || continue
    if [ "$(issue_title_dedup_key "$title")" = "$title_key" ]; then
      printf '%s' "$number"
      return 0
    fi
  done <<< "$search_results"
  return 0
}

TITLE=""
BODY_FILE=""
TYPE=""
LABELS=()
CREATE_ARGS=()

while [ $# -gt 0 ]; do
  case "$1" in
    --title) TITLE="$2"; shift 2 ;;
    --body-file) BODY_FILE="$2"; shift 2 ;;
    --type) TYPE="$2"; shift 2 ;;
    --label) LABELS+=(--label "$2"); shift 2 ;;
    --parent) CREATE_ARGS+=(--parent "$2"); shift 2 ;;
    --blocked-by) CREATE_ARGS+=(--blocked-by "$2"); shift 2 ;;
    *) echo "issue-create: unknown arg $1" >&2; exit 2 ;;
  esac
done

[ -n "$TITLE" ] || { echo "issue-create: --title required" >&2; exit 2; }
[ -n "$BODY_FILE" ] && [ -f "$BODY_FILE" ] || { echo "issue-create: --body-file missing/unreadable" >&2; exit 2; }
case "$TYPE" in
  Bug|Task|Feature) ;;
  *) echo "issue-create: --type Bug|Task|Feature required (got '$TYPE')" >&2; exit 2 ;;
esac

TITLE=$(normalize_issue_title "$TITLE")
[ "${#TITLE}" -ge 12 ] || { echo "issue-create: title suspiciously short ($TITLE)" >&2; exit 2; }
case "$TITLE" in
  "|"*|\#\#*|-*\|*) echo "issue-create: title looks like a markdown fragment ($TITLE)" >&2; exit 2 ;;
esac

duplicate_number=$(find_duplicate_open_issue "$(issue_title_dedup_key "$TITLE")")
if [ -n "$duplicate_number" ]; then
  echo "issue-create: duplicate open issue #$duplicate_number has the same title; refusing to create" >&2
  exit "$DUPLICATE_EXIT"
fi

URL=$(gh issue create --title "$TITLE" --body-file "$BODY_FILE" --type "$TYPE" \
  "${LABELS[@]}" "${CREATE_ARGS[@]}")
NUM=${URL##*/}

# Read-back: the caller must see number + title + type + labels and diff vs intent.
gh issue view "$NUM" --json number,title,issueType,labels \
  --jq '"created: #\(.number) [\(.issueType.name)] [\([.labels[].name] | join(","))] \(.title)"'
echo "$URL"
