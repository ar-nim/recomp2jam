#!/usr/bin/env bash
# Argument-validation seam tests for merge-lane.sh (issue #918).
#
# Each case runs in a scratch clone. No cargo command, no push, no tracker
# write: these cases only prove that a lane cannot start from a bad place.
# Run directly: bash scripts/tests/test_merge_lane_arguments.sh
set -euo pipefail

SOURCE_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRATCH_ROOT="${MERGE_LANE_TEST_SCRATCH:-$HOME/scratch}"
TEST_ROOT="$(mktemp -d "$SCRATCH_ROOT/merge-lane-args.XXXXXX")"
CLONE="$TEST_ROOT/repo"
LANE_WORKTREE="$TEST_ROOT/lane"
trap 'rm -rf -- "$TEST_ROOT"' EXIT

fail() {
    echo "FAIL: $1" >&2
    exit 1
}

expect_refusal() {
    local expected_message="$1" status=0 output
    shift
    output="$("$@" 2>&1)" || status=$?
    [ "$status" -ne 0 ] || fail "expected a nonzero exit for: $expected_message"
    grep -qF -- "$expected_message" <<<"$output" || fail "expected '$expected_message' in: $output"
}

run_merge_lane_from_clone() {
    bash -c "cd '$CLONE' && bash '$CLONE/scripts/merge-lane.sh' '$1'"
}

create_unmerged_lane() {
    git -C "$CLONE" checkout -q -b "$1"
    git -C "$CLONE" commit -q --allow-empty -m "test: $1 tip"
    git -C "$CLONE" checkout -q main
}

scenario_refuses_invocation_from_linked_worktree() {
    create_unmerged_lane lane-worktree
    git -C "$CLONE" worktree add -q "$LANE_WORKTREE" lane-worktree
    expect_refusal "run from the main checkout root" \
        bash -c "cd '$LANE_WORKTREE' && bash '$CLONE/scripts/merge-lane.sh' lane-worktree"
    git -C "$CLONE" worktree remove "$LANE_WORKTREE"
    git -C "$CLONE" branch -D lane-worktree >/dev/null
}

scenario_refuses_dirty_tree() {
    create_unmerged_lane lane-dirty
    printf '\n' >>"$CLONE/README.md"
    expect_refusal "main checkout has uncommitted changes" \
        run_merge_lane_from_clone lane-dirty
    git -C "$CLONE" checkout -q -- README.md
    git -C "$CLONE" branch -D lane-dirty >/dev/null
}

scenario_survives_many_worktrees_before_a_refusal() {
    # Regression: lane discovery piped `git worktree list` into an awk that
    # exited early; on a repo with enough worktrees the pipe filled, git
    # took SIGPIPE, and pipefail killed the script with exit 141 before any
    # output (found on the first real dogfood run of #917). With a dirty
    # tree the first real step is a refusal, so reaching that refusal
    # proves discovery survived the pipe.
    local i
    create_unmerged_lane lane-many-worktrees
    for i in $(seq 1 150); do
        git -C "$CLONE" worktree add -q --detach "$TEST_ROOT/wt-$i" main
    done
    printf '\n' >>"$CLONE/README.md"
    expect_refusal "main checkout has uncommitted changes" \
        run_merge_lane_from_clone lane-many-worktrees
    git -C "$CLONE" checkout -q -- README.md
    git -C "$CLONE" worktree list --porcelain \
        | awk '/^worktree / { print substr($0, 10) }' \
        | grep "$TEST_ROOT/wt-" \
        | xargs -r -n1 git -C "$CLONE" worktree remove
    git -C "$CLONE" branch -D lane-many-worktrees >/dev/null
}

scenario_refuses_missing_branch() {
    expect_refusal "branch not found: lane-missing" \
        run_merge_lane_from_clone lane-missing
}

scenario_refuses_already_merged_branch() {
    git -C "$CLONE" branch lane-merged HEAD
    expect_refusal "branch already merged into main: lane-merged" \
        run_merge_lane_from_clone lane-merged
}

git clone -q -b main "$SOURCE_REPO" "$CLONE"
# The script is still uncommitted in this lane; install the exact file under
# test into the clone so its own repo root is the scratch checkout.
cp "$SOURCE_REPO/scripts/merge-lane.sh" "$CLONE/scripts/merge-lane.sh"
git -C "$CLONE" add scripts/merge-lane.sh
git -C "$CLONE" commit -q -m 'test: install merge-lane under test'
scenario_refuses_invocation_from_linked_worktree
scenario_refuses_dirty_tree
scenario_refuses_missing_branch
scenario_refuses_already_merged_branch
scenario_survives_many_worktrees_before_a_refusal
echo 'test_merge_lane_arguments: 5 scenarios passed'
