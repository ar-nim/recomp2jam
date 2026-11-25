#!/usr/bin/env bash
# Seam tests for the guard-evidence reviewer check (issue #917).
#
# Fixture Rust files live in a temporary scan root; no cargo run and no
# repository mutation. Run directly: bash scripts/tests/test_check_guard_evidence.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check-guard-evidence.sh"
FIXTURE_ROOT="$(mktemp -d)"
OUTPUT="$FIXTURE_ROOT/output.log"
STATUS=0
trap 'rm -rf -- "$FIXTURE_ROOT"' EXIT

run_check() {
    status=0
    CHECK_GUARD_EVIDENCE_ROOT="$FIXTURE_ROOT/crates" bash "$CHECKER" "$@" >"$OUTPUT" 2>&1 || status=$?
}

write_guard() {
    local name="$1" evidence="$2"
    mkdir -p "$FIXTURE_ROOT/crates/fixture/src"
    {
        printf '#[test]\n'
        if [ -n "$evidence" ]; then printf '%s\n' "$evidence"; fi
        printf 'fn %s() {\n    assert!(true);\n}\n' "$name"
    } >"$FIXTURE_ROOT/crates/fixture/src/guards.rs"
}

fail_with_output() {
    echo "FAIL: $1" >&2
    cat "$OUTPUT" >&2
    exit 1
}

expect_status() {
    [ "$status" -eq "$1" ] || fail_with_output "expected exit $1, got $status"
}

expect_output_contains() {
    grep -qF -- "$1" "$OUTPUT" || fail_with_output "expected output to contain: $1"
}

expect_output_not_contains() {
    if grep -qF -- "$1" "$OUTPUT"; then
        fail_with_output "expected output not to contain: $1"
    fi
}

scenario_unpinned_inert_guard_fails() {
    write_guard shop_unwired_browse_chrome_lands_nowhere ""
    run_check
    expect_status 1
    expect_output_contains 'FAIL '
    expect_output_contains 'shop_unwired_browse_chrome_lands_nowhere'
    expect_output_contains '1 without evidence'
}

scenario_pinned_inert_guard_passes() {
    write_guard shop_unwired_browse_chrome_lands_nowhere \
        '    // Exe pin: rea evidence ev_abcd1234 and 7.55 file offset 0x401000.'
    run_check
    expect_status 0
    expect_output_contains '1 inert guard(s) scanned, 0 without evidence'
}

scenario_next_step_note_exempts_a_guard() {
    write_guard item_shop_go_myroom_leaves_without_an_exit_send \
        '    // next evidence step: statics pin + live-input guard; re-pin per #916 and #178.'
    run_check
    expect_status 0
    expect_output_contains '0 without evidence'
}

scenario_negation_free_guard_is_ignored() {
    write_guard computes_expected_score ""
    run_check
    expect_status 0
    expect_output_contains '0 inert guard(s) scanned'
    expect_output_not_contains 'FAIL '
}

scenario_certifying_guard_without_inert_word_fails() {
    write_guard settled_solo_result_back_click_posts_4021_and_takes_the_4022_answer ""
    run_check
    expect_status 1
    expect_output_contains 'settled_solo_result_back_click_posts_4021_and_takes_the_4022_answer'
}

scenario_unresolvable_diff_base_is_a_usage_error() {
    run_check --diff no-such-base-for-guard-evidence
    expect_status 2
    expect_output_contains 'cannot resolve merge-base'
}

scenario_unpinned_inert_guard_fails
scenario_pinned_inert_guard_passes
scenario_next_step_note_exempts_a_guard
scenario_negation_free_guard_is_ignored
scenario_certifying_guard_without_inert_word_fails
scenario_unresolvable_diff_base_is_a_usage_error
echo 'test_check_guard_evidence: 6 scenarios passed'
