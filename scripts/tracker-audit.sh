#!/usr/bin/env bash
# tracker-audit.sh — verify closed-issue guard coverage (issue #173).
#
# Usage:
#   scripts/tracker-audit.sh
#   TRACKER_AUDIT_EXTRA_GUARD=bogus_guard_xyz scripts/tracker-audit.sh  # self-test: must exit 1
#
# Walks CLOSED issues via gh, extracts the test names listed under a
# '## Guards' heading in each issue's body/comments (the close-discipline
# rule in docs/agents/triage-labels.md), and asserts each name exists in
# crates/. Historical closures without the heading are EXEMPT — a missing
# heading is never a failure; only a named guard absent from the tree fails.
#
# Verdicts (one line per issue, stdout):
#   ok #<n> (<k> guards)          — every named guard found in crates/
#   exempt #<n> (no ## Guards)    — no heading; historical closure, not a failure
#   MISSING #<n>: <name> ...      — named guard(s) absent from the tree
#
# Exit 0: no missing guards (reports the exempt count).
# Exit 1: at least one named guard is missing from crates/ (lists the names).
# Exit 2: usage/environment error (gh missing or unauthenticated degrades to
#   a warning with exit 0 instead, so offline runs never fail the build —
#   network IO stays out of the build path by owner decision on #173).
set -euo pipefail

die() { echo "tracker-audit: $*" >&2; exit 2; }

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$REPO_ROOT/crates"

command -v gh >/dev/null 2>&1 || { echo "tracker-audit: warning: gh not installed; skipping tracker audit" >&2; exit 0; }
gh auth status >/dev/null 2>&1 || { echo "tracker-audit: warning: gh not authenticated (GH_TOKEN unset?); skipping tracker audit" >&2; exit 0; }

[ -d "$CRATE_DIR" ] || die "crates/ not found at $CRATE_DIR"

# --- collect closed issue numbers (--limit pages internally; 1000 covers >200) ---
NUMBERS="$(gh issue list --state closed --limit 1000 --json number --jq '.[].number')" || die "gh issue list failed"
[ -n "$NUMBERS" ] || die "no closed issues returned"

# --- extract guard names from the '## Guards' section of issue text --------
# Section = lines after a '## Guards' heading up to the next '##' heading.
# Accepts backticked names and bare snake_case tokens (which must contain
# an underscore, so prose words never become guard candidates).
extract_guards() {
  awk -f "$REPO_ROOT/scripts/lib/extract-guards.awk" | grep -oE '`[A-Za-z_][A-Za-z0-9_]*`|\<[a-z][a-z0-9_]*_[a-z0-9_]*\>' | tr -d '`' | sort -u
}

guard_in_tree() {
  grep -rq --fixed-strings --include='*.rs' -e "$1" "$CRATE_DIR"
}

total=0
exempt=0
guard_issues=0
missing=()

while read -r num; do
  [ -n "$num" ] || continue
  total=$((total + 1))
  text="$(gh issue view "$num" --json body,comments --jq '[.body] + [.comments[].body] | join("\n")' 2>/dev/null || true)"
  if [ -z "$text" ]; then
    echo "warn #$num (unreadable; skipped)"
    continue
  fi
  guards="$(echo "$text" | extract_guards || true)"
  if [ -z "$guards" ]; then
    exempt=$((exempt + 1))
    echo "exempt #$num (no ## Guards)"
    continue
  fi
  guard_issues=$((guard_issues + 1))
  count="$(echo "$guards" | wc -l | tr -d ' ')"
  bad=""
  while read -r name; do
    [ -n "$name" ] || continue
    if ! guard_in_tree "$name"; then
      bad="$bad $name"
      missing+=("#$num:$name")
    fi
  done <<< "$guards"
  if [ -n "$bad" ]; then
    echo "MISSING #$num:$bad"
  else
    echo "ok #$num ($count guards)"
  fi
done <<< "$NUMBERS"

# --- self-test hook: an extra guard name that must resolve (exit 1 if not) --
if [ -n "${TRACKER_AUDIT_EXTRA_GUARD:-}" ]; then
  total=$((total + 1))
  if guard_in_tree "$TRACKER_AUDIT_EXTRA_GUARD"; then
    echo "ok \$TRACKER_AUDIT_EXTRA_GUARD ($TRACKER_AUDIT_EXTRA_GUARD found)"
  else
    echo "MISSING \$TRACKER_AUDIT_EXTRA_GUARD: $TRACKER_AUDIT_EXTRA_GUARD"
    missing+=("\$TRACKER_AUDIT_EXTRA_GUARD:$TRACKER_AUDIT_EXTRA_GUARD")
  fi
fi

echo "tracker-audit: $total closed issues, $guard_issues with ## Guards, $exempt exempt, ${#missing[@]} missing guards"
if [ "${#missing[@]}" -gt 0 ]; then
  printf 'missing: %s\n' "${missing[@]}"
  exit 1
fi
exit 0
