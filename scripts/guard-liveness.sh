#!/usr/bin/env bash
# guard-liveness.sh — compare guard-section tokens with `cargo test --list`.
#
# Usage: scripts/guard-liveness.sh [TEST_LIST_CACHE] [ISSUE_NUMBER]
# The DEAD list is advisory. Close-discipline rule 7 asks closing comments
# to put live caller chains under `## Guards`; the shared extractor returns
# every snake_case token there, including chain functions such as
# `run_play`, `paint_room`, and `load_worn_figure`. Those names are expected
# to miss the test list and are not rot. `scripts/tracker-audit.sh` is the
# authoritative guard-name gate. Exit semantics are unchanged: exit 1 while
# any extracted token is absent from the test list (issue #186 requires no
# behavior change to the shared extractor).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CACHE="${1:-/tmp/guard-liveness-list.txt}"
[ -s "$CACHE" ] || (cd "$REPO_ROOT" && cargo test --workspace -- --list > "$CACHE" 2>&1)

extract_guards() {
  awk -f "$REPO_ROOT/scripts/lib/extract-guards.awk" | grep -oE '`[A-Za-z_][A-Za-z0-9_]*`|\<[a-z][a-z0-9_]*_[a-z0-9_]*\>' | tr -d '`' | sort -u
}

command -v gh >/dev/null 2>&1 || { echo "guard-liveness: warning: gh missing; skip" >&2; exit 0; }
gh auth status >/dev/null 2>&1 || { echo "guard-liveness: warning: gh unauth; skip" >&2; exit 0; }

if [ "$#" -gt 1 ]; then
  nums="${*:2}"
else
  nums="$(gh issue list --state closed --limit 1000 --json number --jq '.[].number')"
fi

scanned=0; total=0; dead=()
while read -r num; do
  [ -n "$num" ] || continue
  scanned=$((scanned+1))
  text="$(gh issue view "$num" --json body,comments --jq '[.body] + [.comments[].body] | join("\n")' 2>/dev/null || true)"
  [ -n "$text" ] || continue
  guards="$(echo "$text" | extract_guards || true)"
  [ -z "$guards" ] && continue
  while read -r name; do
    [ -n "$name" ] || continue
    total=$((total+1))
    if grep -qF "$name" "$CACHE"; then
      echo "live #$num $name"
    else
      echo "DEAD #$num $name"
      dead+=("$name")
    fi
  done <<< "$guards"
done <<< "$nums"

if [ -n "${TRACKER_AUDIT_EXTRA_GUARD:-}" ]; then
  total=$((total+1))
  if grep -qF "$TRACKER_AUDIT_EXTRA_GUARD" "$CACHE"; then
    echo "live \$EXTRA $TRACKER_AUDIT_EXTRA_GUARD"
  else
    echo "DEAD \$EXTRA $TRACKER_AUDIT_EXTRA_GUARD"
    dead+=("$TRACKER_AUDIT_EXTRA_GUARD")
  fi
fi

echo "guard-liveness: scanned $scanned issues, $total guards checked, ${#dead[@]} dead"
if [ "${#dead[@]}" -gt 0 ]; then
  printf 'dead: %s\n' "${dead[@]}"
  exit 1
fi
exit 0
