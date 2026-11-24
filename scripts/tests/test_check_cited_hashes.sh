#!/usr/bin/env bash
# Seam tests for the cited-hash reviewer check (issue #887).
#
# Runs against this checkout's object store: the fixture's good token must
# resolve to a commit and its stale token must not. No gh calls, no network.
# Run directly: bash scripts/tests/test_check_cited_hashes.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check-cited-hashes.sh"
FIXTURE="$REPO_ROOT/scripts/tests/fixtures/cited-hashes/close-table.md"
GOOD_SHORT=b41dde6
GOOD_FULL=b41dde6759e43bd5695a135d324489d0ee1f2e93
STALE=9692d41
GIT_DIR="$(git -C "$REPO_ROOT" rev-parse --absolute-git-dir)"
OUTPUT="$(mktemp)"
TEMP_FILE="$(mktemp)"
trap 'rm -f -- "$OUTPUT" "$TEMP_FILE"' EXIT

run_check() {
    CHECK_CITED_HASHES_GIT_DIR="$GIT_DIR" bash "$CHECKER" "$@" >"$OUTPUT" 2>&1
}

fail_with_output() {
    echo "FAIL: $1" >&2
    cat "$OUTPUT" >&2
    exit 1
}

expect_output_contains() {
    grep -qF -- "$1" "$OUTPUT" || fail_with_output "expected output to contain: $1"
}

expect_output_not_contains() {
    if grep -qF -- "$1" "$OUTPUT"; then
        fail_with_output "expected output not to contain: $1"
    fi
}

scenario_stale_citation_fails_and_resolves_good_citation() {
    local status=0
    run_check "$FIXTURE" || status=$?
    [ "$status" -eq 1 ] || fail_with_output "stale citation must exit 1, got $status"
    expect_output_contains "RESOLVED $GOOD_SHORT -> $GOOD_FULL"
    expect_output_contains "STALE $STALE"
    expect_output_contains "1 stale"
}

scenario_only_resolving_citations_pass() {
    printf 'Resolved citation: `%s`.\n' "$GOOD_SHORT" >"$TEMP_FILE"
    run_check "$TEMP_FILE" || fail_with_output "resolving citation must pass"
    expect_output_contains "RESOLVED $GOOD_SHORT -> $GOOD_FULL"
}

scenario_full_length_citation_resolves_without_shortening() {
    printf 'Close-table citation: `%s`.\n' "$GOOD_FULL" >"$TEMP_FILE"
    run_check "$TEMP_FILE" || fail_with_output "full-length citation must pass"
    expect_output_contains "RESOLVED $GOOD_FULL -> $GOOD_FULL"
}

scenario_sha256_stamp_is_not_a_git_citation() {
    printf 'stamp: %s\n' "$(printf 'a%.0s' {1..64})" >"$TEMP_FILE"
    run_check "$TEMP_FILE" || fail_with_output "sha256 stamp must not be a git citation"
    expect_output_not_contains 'aaaaaaaa'
}

scenario_issue_body_on_stdin_is_checked() {
    local status=0
    CHECK_CITED_HASHES_GIT_DIR="$GIT_DIR" bash "$CHECKER" - <"$FIXTURE" >"$OUTPUT" 2>&1 || status=$?
    [ "$status" -eq 1 ] || fail_with_output "stdin citation check must exit 1, got $status"
    expect_output_contains "STALE $STALE"
}

scenario_missing_input_is_a_usage_error() {
    local status=0
    run_check "$REPO_ROOT/scripts/tests/fixtures/cited-hashes/missing.md" || status=$?
    [ "$status" -eq 2 ] || fail_with_output "missing input must exit 2, got $status"
}

scenario_stale_citation_fails_and_resolves_good_citation
scenario_only_resolving_citations_pass
scenario_full_length_citation_resolves_without_shortening
scenario_sha256_stamp_is_not_a_git_citation
scenario_issue_body_on_stdin_is_checked
scenario_missing_input_is_a_usage_error
echo 'test_check_cited_hashes: 6 scenarios passed'
