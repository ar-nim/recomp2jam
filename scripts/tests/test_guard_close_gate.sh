#!/usr/bin/env bash
# Seam tests for the close-gate mode of the guard-evidence check (#921).
#
# Fixture Rust files and close comments live in ~/scratch; no cargo run and
# no repository mutation. Run directly: bash scripts/tests/test_guard_close_gate.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check-guard-evidence.sh"
FIXTURE_ROOT="$(mktemp -d "$HOME/scratch/guard-close-gate.XXXXXX")"
CLOSE_FILE="$FIXTURE_ROOT/close-comment.md"
OUTPUT="$FIXTURE_ROOT/output.log"
STATUS=0
trap 'rm -rf -- "$FIXTURE_ROOT"' EXIT

run_check() {
    status=0
    CHECK_GUARD_EVIDENCE_ROOT="$FIXTURE_ROOT/crates" \
        bash "$CHECKER" --close-gate "$CLOSE_FILE" "$@" >"$OUTPUT" 2>&1 || status=$?
}

write_guards() {
    mkdir -p "$FIXTURE_ROOT/crates/fixture/src"
    cat >"$FIXTURE_ROOT/crates/fixture/src/guards.rs"
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

scenario_tier0_only_close_fails() {
    write_guards <<'RUST'
/// Tier 0 seam: pure arithmetic on note speeds.
#[test]
fn parse_speed_cell_math() {
    assert_eq!(2 + 2, 4);
}

/// Tier 0 seam: pure arithmetic on price digits.
#[test]
fn decode_price_digit_shift() {
    assert_eq!(10 / 2, 5);
}
RUST
    cat >"$CLOSE_FILE" <<'CLOSE'
State: Playing
## Guards
`parse_speed_cell_math` — tier 0 seam: pure arithmetic
`decode_price_digit_shift` — tier 0 seam: pure arithmetic
CLOSE
    run_check
    expect_status 1
    expect_output_contains 'state Playing: tier-0-only close'
    expect_output_contains '2 guard(s) checked'
}

scenario_seam_reason_with_live_caller_fails() {
    write_guards <<'RUST'
/// Live chain: main.rs::run_play -> setup_online -> ScreenGame::paint_at
#[test]
fn paint_playing_from_boot() {
    assert!(true);
}
RUST
    cat >"$CLOSE_FILE" <<'CLOSE'
State: Playing
## Guards
`paint_playing_from_boot` — seam: pure math
CLOSE
    run_check
    expect_status 1
    expect_output_contains 'seam: conflicts with live caller main.rs::run_play'
    expect_output_contains 'ambiguity'
}

scenario_declared_tier3_live_close_passes() {
    write_guards <<'RUST'
/// Live chain: main.rs::run_play -> setup_online -> ScreenGame::paint_at
#[test]
fn paint_playing_from_boot() {
    assert!(true);
}
RUST
    cat >"$CLOSE_FILE" <<'CLOSE'
State: Playing
## Guards
`paint_playing_from_boot` — tier 3 live chain: `main.rs::run_play` -> ScreenGame::paint_at
CLOSE
    run_check
    expect_status 0
    expect_output_contains 'guard paint_playing_from_boot tier 3'
    expect_output_contains '1 guard(s) checked, 0 failure(s)'
}

scenario_single_line_guard_body_is_indexed() {
    write_guards <<'RUST'
/// Live chain: main.rs::run_play -> ScreenGame::paint_at
#[test]
fn paint_playing_from_boot() { assert!(true); }
RUST
    cat >"$CLOSE_FILE" <<'CLOSE'
State: Playing
## Guards
`paint_playing_from_boot` — tier 3 live chain: `main.rs::run_play` -> ScreenGame::paint_at
CLOSE
    run_check
    expect_status 0
    expect_output_contains 'guard paint_playing_from_boot tier 3'
}

scenario_missing_annotation_is_an_ambiguity() {
    write_guards <<'RUST'
#[test]
fn paint_playing_from_boot() {
    assert!(true);
}
RUST
    cat >"$CLOSE_FILE" <<'CLOSE'
State: Playing
## Guards
`paint_playing_from_boot` — the guard passes
CLOSE
    run_check
    expect_status 1
    expect_output_contains 'no live chain or seam: reason'
}

scenario_docs_exemption_passes_without_a_guard() {
    write_guards <<'RUST'
#[test]
fn unrelated_guard() {
    assert!(true);
}
RUST
    cat >"$CLOSE_FILE" <<'CLOSE'
State: Playing
## Guards
(none -- docs: caption spelling only)
CLOSE
    run_check
    expect_status 0
    expect_output_contains 'exempt'
    expect_output_contains '0 guard(s) checked, 0 failure(s)'
}

scenario_diff_mode_still_checks_the_close_comment() {
    write_guards <<'RUST'
/// Tier 0 seam: pure arithmetic.
#[test]
fn parse_speed_cell_math() {
    assert_eq!(1, 1);
}
RUST
    cat >"$CLOSE_FILE" <<'CLOSE'
State: Playing
## Guards
`parse_speed_cell_math` — tier 0 seam: pure arithmetic
CLOSE
    run_check --diff HEAD
    expect_status 1
    expect_output_contains 'state Playing: tier-0-only close'
}

scenario_unreadable_close_comment_is_a_usage_error() {
    write_guards <<'RUST'
#[test]
fn unrelated_guard() {
    assert!(true);
}
RUST
    CLOSE_FILE="$FIXTURE_ROOT/does-not-exist.md"
    run_check
    expect_status 2
    expect_output_contains 'unreadable close comment'
}

scenario_tier0_only_close_fails
scenario_seam_reason_with_live_caller_fails
scenario_declared_tier3_live_close_passes
scenario_single_line_guard_body_is_indexed
scenario_missing_annotation_is_an_ambiguity
scenario_docs_exemption_passes_without_a_guard
scenario_diff_mode_still_checks_the_close_comment
scenario_unreadable_close_comment_is_a_usage_error
echo 'test_guard_close_gate: 8 scenarios passed'
