#!/usr/bin/env bash
# check-cited-hashes.sh — resolve cited commit hashes at read time (issue #887).
#
# Usage:
#   scripts/check-cited-hashes.sh FILE [FILE...]
#   gh issue view 139 --json body,comments \
#     --jq '[.body] + [.comments[].body] | join("\n")' | scripts/check-cited-hashes.sh -
#   CHECK_CITED_HASHES_GIT_DIR=/path/to/.git scripts/check-cited-hashes.sh FILE
# Environment (fixture seam): CHECK_CITED_HASHES_GIT_DIR selects the object
# store; without it the check uses this checkout.
#
# Every 7-40 character hex token in the input must be a commit in this
# checkout. RESOLVED lines carry the full 40-character object name, so a
# close table can replace a short citation without a second lookup. STALE
# lines name tokens that are not commits; they are never evidence.
#
# Exit 0: all citations resolve to commits. Exit 1: at least one stale
# citation. Exit 2: usage or unreadable input.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STDIN_COPIES=()
PREPARED_PATH=""
PREPARED_LABEL=""
resolved_count=0
stale_count=0

die() { echo "check-cited-hashes: $*" >&2; exit 2; }

git_in_checked_repo() {
    if [ -n "${CHECK_CITED_HASHES_GIT_DIR:-}" ]; then
        git --git-dir="$CHECK_CITED_HASHES_GIT_DIR" "$@"
    else
        git -C "$REPO_ROOT" "$@"
    fi
}

cleanup_stdin_copies() {
    local copy
    for copy in "${STDIN_COPIES[@]}"; do
        rm -f -- "$copy"
    done
}
trap cleanup_stdin_copies EXIT

prepare_input() {
    local input="$1" stdin_copy
    if [ "$input" != "-" ]; then
        [ -f "$input" ] || die "unreadable input: $input"
        PREPARED_PATH="$input"
        PREPARED_LABEL="$input"
        return
    fi
    stdin_copy="$(mktemp -t cited-hashes-stdin.XXXXXX)"
    STDIN_COPIES+=("$stdin_copy")
    cat >"$stdin_copy"
    PREPARED_PATH="$stdin_copy"
    PREPARED_LABEL="stdin"
}

# A token must be its own word: this skips hash-shaped parts of longer
# identifiers and SHA-256-like stamps written as one unbroken string.
extract_citations() {
    grep -nPo '(?<![0-9A-Za-z])[0-9a-fA-F]{7,40}(?![0-9A-Za-z])' "$1" || true
}

full_commit_name() {
    local token="$1" full
    full="$(git_in_checked_repo rev-parse --verify --quiet "${token}^{commit}" 2>/dev/null || true)"
    if [[ "$full" =~ ^[0-9a-f]{40}$ ]]; then
        printf '%s\n' "$full"
    fi
}

check_source() {
    local path="$1" label="$2" line token object_type full
    while IFS=: read -r line token; do
        [ -n "$token" ] || continue
        object_type="$(git_in_checked_repo cat-file -t "$token" 2>/dev/null || true)"
        full="$(full_commit_name "$token")"
        if [ "$object_type" = "commit" ] && [ -n "$full" ]; then
            printf 'RESOLVED %s -> %s %s:%s\n' "$token" "$full" "$label" "$line"
            resolved_count=$((resolved_count + 1))
        else
            printf 'STALE %s %s:%s\n' "$token" "$label" "$line"
            stale_count=$((stale_count + 1))
        fi
    done < <(extract_citations "$path")
}

[ "$#" -gt 0 ] || die "usage: $0 FILE [FILE...] (use '-' for an issue body on stdin)"

sources=()
for input in "$@"; do
    prepare_input "$input"
    sources+=("$PREPARED_PATH"$'\t'"$PREPARED_LABEL")
done

for source in "${sources[@]}"; do
    IFS=$'\t' read -r path label <<<"$source"
    check_source "$path" "$label"
done

printf 'check-cited-hashes: %d resolved, %d stale\n' "$resolved_count" "$stale_count"
[ "$stale_count" -eq 0 ]
