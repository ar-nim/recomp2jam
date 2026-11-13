#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CACHE="${1:-/tmp/guard-liveness-list.txt}"
[ -s "$CACHE" ] || (cd "$REPO_ROOT" && cargo test --workspace -- --list > "$CACHE" 2>&1)

extract_guards() {
  awk '
    /^##+ *Guards/ { in_section = 1; next }
    /^##+ / { in_section = 0 }
    in_section { print }
  ' | grep -oE '`[A-Za-z_][A-Za-z0-9_]*`|\<[a-z][a-z0-9_]*_[a-z0-9_]*\>' | tr -d '`' | sort -u
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
