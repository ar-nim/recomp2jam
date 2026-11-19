#!/usr/bin/env bash
# tracker-audit.sh — verify closed-issue guard coverage (issue #173).
#
# Usage:
#   scripts/tracker-audit.sh                      # full closed-issue walk (CI default)
#   scripts/tracker-audit.sh 129 163 165         # fast path: lint only these issues
#   TRACKER_AUDIT_EXTRA_GUARD=bogus_guard_xyz scripts/tracker-audit.sh  # self-test: must exit 1
#   TRACKER_AUDIT_REMOTE_ONLY=1 scripts/tracker-audit.sh             # remote leg only
#   TRACKER_AUDIT_GIT_DIR=/path/to/.git scripts/tracker-audit.sh     # fixture seam
#
# Before any gh work, asserts origin's push URL equals its fetch URL and
# lists remote cb2/* branches with no local counterpart (issue #488).
# A shared remote config mutation fails the audit with exit 1.
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
# Exit 1: shared remote mismatch or at least one named guard missing.
# Exit 2: usage/environment error (gh missing or unauthenticated degrades to
#   a warning with exit 0 instead, so offline runs never fail the build —
#   network IO stays out of the build path by owner decision on #173).
set -euo pipefail

die() { echo "tracker-audit: $*" >&2; exit 2; }

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$REPO_ROOT/crates"
AUDIT_GIT_DIR="${TRACKER_AUDIT_GIT_DIR:-$REPO_ROOT/.git}"

git_in_audit_repo() { git --git-dir="$AUDIT_GIT_DIR" "$@"; }

assert_push_url_matches_fetch() {
    local fetch_url push_url
    fetch_url="$(git_in_audit_repo remote get-url origin)" ||
        die "cannot read origin fetch URL from $AUDIT_GIT_DIR"
    push_url="$(git_in_audit_repo remote get-url --push origin)" ||
        die "cannot read origin push URL from $AUDIT_GIT_DIR"
    if [ "$push_url" != "$fetch_url" ]; then
        echo "tracker-audit: REMOTE MISMATCH: origin push='$push_url' fetch='$fetch_url'" >&2
        echo "tracker-audit: an agent worktree mutated the shared remote config; restore the fetch URL" >&2
        return 1
    fi
    case "$push_url" in
        https://*) ;;
        *) echo "tracker-audit: REMOTE MISMATCH: origin push URL is not https: $push_url" >&2; return 1 ;;
    esac
    echo "tracker-audit: ok origin push URL matches fetch URL ($fetch_url)"
}

list_stray_origin_branches() {
    local remote_ref local_branch
    while read -r remote_ref; do
        [ -n "$remote_ref" ] || continue
        local_branch="${remote_ref#origin/}"
        if ! git_in_audit_repo show-ref --verify --quiet "refs/heads/$local_branch"; then
            echo "tracker-audit: stray remote branch $remote_ref (no local branch $local_branch)"
        fi
    done < <(git_in_audit_repo for-each-ref --format='%(refname:short)' refs/remotes/origin/cb2/)
}

assert_push_url_matches_fetch || exit 1
list_stray_origin_branches

if [ "${TRACKER_AUDIT_REMOTE_ONLY:-0}" = "1" ]; then
    exit 0
fi

command -v gh >/dev/null 2>&1 || { echo "tracker-audit: warning: gh not installed; skipping tracker audit" >&2; exit 0; }
gh auth status >/dev/null 2>&1 || { echo "tracker-audit: warning: gh not authenticated (GH_TOKEN unset?); skipping tracker audit" >&2; exit 0; }

[ -d "$CRATE_DIR" ] || die "crates/ not found at $CRATE_DIR"

# --- collect issue numbers ---------------------------------------------------
# Fast path: positional args are an explicit issue subset (one gh call per
# issue instead of a full tracker walk). Bare invocation keeps the full-walk
# CI behavior. Mirrors guard-liveness.sh's [ISSUES...] fast path.
if [ "$#" -gt 0 ]; then
  for arg in "$@"; do
    case "$arg" in
      ''|*[!0-9]*) die "issue numbers must be positive integers (got '$arg')" ;;
    esac
  done
  NUMBERS="$(printf '%s\n' "$@")"
else
  NUMBERS="$(gh issue list --state closed --limit 1000 --json number --jq '.[].number')" || die "gh issue list failed"
  [ -n "$NUMBERS" ] || die "no closed issues returned"
fi

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

# --- blocked-by leg (retro 2026-11-18) ---------------------------------------
# Wave planning blind spot: #260 was scheduled while its own body declared
# "no agent starts" on its blockedBy edges; the planning query fetched labels
# but not blockedBy. This leg lists every OPEN ready-for-agent ticket that
# still carries blockedBy edges, so a pre-wave audit read catches it. The
# edges are often stale (closed blockers) — informational, never an exit
# code: the wave planner removes stale edges, the ticket body decides the
# rest. Uses --template because `gh issue view --json blockedBy` errors on
# the array shape ("expected an object but got: array"); the template path
# is the working recipe (now also in docs/agents/issue-tracker.md).
if [ "$#" -eq 0 ] && [ "${TRACKER_AUDIT_SKIP_BLOCKED:-0}" != "1" ]; then
  open_ready="$(gh issue list --state open --label ready-for-agent --limit 500 --json number --jq '.[].number' 2>/dev/null || true)"
  if [ -n "$open_ready" ]; then
    while read -r onum; do
      [ -n "$onum" ] || continue
      blockers="$(gh issue view "$onum" --template '{{range .blockedBy.nodes}}{{.number}} {{end}}' 2>/dev/null | sed 's/[[:space:]]*$//')"
      if [ -n "$blockers" ]; then
        echo "blocked #$onum -> $blockers"
      fi
    done <<< "$open_ready"
  fi
fi
if [ "${#missing[@]}" -gt 0 ]; then
  printf 'missing: %s\n' "${missing[@]}"
  exit 1
fi
exit 0
