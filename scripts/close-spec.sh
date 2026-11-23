#!/usr/bin/env bash
# close-spec.sh — graph-shaped spec close guard (retro #869, 2026-11-22).
#
# A spec may be closed only when BOTH sources of truth are clean:
#   (a) no OPEN sub-issues (the sub_issues API returns lowercase
#       "state" — the filter below matches it explicitly; the
#       2026-11-22 retro found an uppercase filter silently matched
#       nothing and a spec closed twice with 8 open tickets);
#   (b) no unchecked story rows (`- [ ]`) in the spec body.
#
# Usage: scripts/close-spec.sh <issue-number>
# Exit 0 = closable; exit 1 = blocked (the blocking items are listed).
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "usage: scripts/close-spec.sh <issue-number>" >&2
    exit 2
fi

SPEC="$1"
BLOCKED=0

# (a) open sub-issues — lowercase state, paginated (retro: the default
# 30-item page hid 4 open sub-issues under #290 and 2 under #291 in
# the first version of this script; the unfiltered count below is the
# known-positive control, retro #871).
SUBS_JSON=$(gh api --paginate "repos/ar-nim/recomp2jam/issues/${SPEC}/sub_issues" --jq '.[]')
OPEN_SUBS=$(printf '%s\n' "$SUBS_JSON" | jq -s 'map(select(.state == "open") | {number, title})')
OPEN_COUNT=$(printf '%s' "$OPEN_SUBS" | jq 'length')
TOTAL_COUNT=$(printf '%s' "$SUBS_JSON" | jq -s 'length')
echo "sub-issue control: ${TOTAL_COUNT} total, ${OPEN_COUNT} open"
if [ "$OPEN_COUNT" -ne 0 ]; then
    echo "BLOCKED: ${OPEN_COUNT} open sub-issue(s) under #${SPEC}:"
    printf '%s\n' "$OPEN_SUBS" | jq -r '.[] | "  #\(.number): \(.title)"'
    BLOCKED=1
fi

# (b) unchecked story rows in the body.
BODY=$(gh issue view "$SPEC" --repo ar-nim/recomp2jam --json body --jq '.body')
UNCHECKED=$(printf '%s\n' "$BODY" | grep -c '^- \[ \]' || true)
if [ "$UNCHECKED" -ne 0 ]; then
    echo "BLOCKED: ${UNCHECKED} unchecked story row(s) in the #${SPEC} body:"
    printf '%s\n' "$BODY" | grep '^- \[ \]' | cut -c1-100 | sed 's/^/  /'
    BLOCKED=1
fi

if [ "$BLOCKED" -ne 0 ]; then
    exit 1
fi
echo "#${SPEC} is closable: 0 open sub-issues, 0 unchecked body rows."
