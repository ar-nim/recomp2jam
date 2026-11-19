#!/usr/bin/env bash
# Seam tests for the tracker-audit remote leg (issue #488).
#
# Uses a fixture git dir: no gh calls, no network, no change to this repo's
# config. Run directly: bash scripts/tests/test_tracker_audit_remote.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AUDIT="$REPO_ROOT/scripts/tracker-audit.sh"
OUTPUT="$(mktemp)"
FIXTURE="$(mktemp -d)"
trap 'rm -rf -- "$FIXTURE" "$OUTPUT"' EXIT

git -C "$FIXTURE" init -q
git -C "$FIXTURE" config user.email audit@example.test
git -C "$FIXTURE" config user.name 'Audit Fixture'
git -C "$FIXTURE" commit -q --allow-empty -m fixture
git -C "$FIXTURE" remote add origin https://example.test/recomp2jam.git

run_audit() {
    TRACKER_AUDIT_GIT_DIR="$FIXTURE/.git" TRACKER_AUDIT_REMOTE_ONLY=1 \
        bash "$AUDIT" >"$OUTPUT" 2>&1
}

expect_output_contains() {
    if ! grep -qF -- "$1" "$OUTPUT"; then
        echo "FAIL: expected output to contain: $1" >&2
        cat "$OUTPUT" >&2
        exit 1
    fi
}

expect_output_not_contains() {
    if grep -qF -- "$1" "$OUTPUT"; then
        echo "FAIL: expected output not to contain: $1" >&2
        cat "$OUTPUT" >&2
        exit 1
    fi
}

scenario_clean_pushurl_passes() {
    if ! run_audit; then
        echo "FAIL: clean remote must pass the audit" >&2
        cat "$OUTPUT" >&2
        exit 1
    fi
    expect_output_contains 'ok origin push URL matches fetch URL'
}

scenario_poisoned_pushurl_fails_audit() {
    git -C "$FIXTURE" config remote.origin.pushurl 'ext::/bin/false'
    if run_audit; then
        echo "FAIL: poisoned push URL must fail the audit" >&2
        cat "$OUTPUT" >&2
        exit 1
    fi
    expect_output_contains 'REMOTE MISMATCH'
    git -C "$FIXTURE" config --unset remote.origin.pushurl
}

scenario_poisoned_pushurl_fails_full_audit_before_gh() {
    git -C "$FIXTURE" config remote.origin.pushurl 'ext::/bin/false'
    status=0
    TRACKER_AUDIT_GIT_DIR="$FIXTURE/.git" bash "$AUDIT" >"$OUTPUT" 2>&1 || status=$?
    [ "$status" -eq 1 ] || {
        echo "FAIL: full audit must exit 1 on poisoned push URL, got $status" >&2
        cat "$OUTPUT" >&2
        exit 1
    }
    expect_output_contains 'REMOTE MISMATCH'
    git -C "$FIXTURE" config --unset remote.origin.pushurl
}

scenario_remote_only_branch_without_local_counterpart_is_listed() {
    git -C "$FIXTURE" update-ref refs/remotes/origin/cb2/tt-999 HEAD
    run_audit
    expect_output_contains 'stray remote branch origin/cb2/tt-999'
    git -C "$FIXTURE" branch cb2/tt-999
    run_audit
    expect_output_not_contains 'stray remote branch origin/cb2/tt-999'
}

scenario_clean_pushurl_passes
scenario_poisoned_pushurl_fails_audit
scenario_poisoned_pushurl_fails_full_audit_before_gh
scenario_remote_only_branch_without_local_counterpart_is_listed
echo 'test_tracker_audit_remote: 4 scenarios passed'
