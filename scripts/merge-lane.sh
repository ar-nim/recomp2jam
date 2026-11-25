#!/usr/bin/env bash
# merge-lane.sh — the one senior command that lands an agent lane (issue #918).
#
# Usage:
#   scripts/merge-lane.sh [--no-push] [--keep-worktree] <branch>
#
# Runs from the primary checkout on main with a clean tree. The merge is
# --no-ff; gate-check owns every check and never pushes in this flow
# (GATE_CHECK_NO_PUSH=1). A red merge or gate resets local main to the
# pre-merge SHA and pushes nothing. Green means push, verify origin/main,
# print the merge commit, then release the lane worktree and delete its
# branch. --no-push leaves the merge local; --keep-worktree keeps the lane
# attached to its branch. A scratch clone is a primary checkout too, so the
# merge proof never touches the real origin.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATE_CHECK="$REPO_ROOT/scripts/gate-check.sh"
WORKTREE_RELEASE="$REPO_ROOT/scripts/worktree-release.sh"

lane_branch=""
no_push=0
keep_worktree=0
pre_merge_sha=""
merge_sha=""
lane_path=""
push_attempted=0
FAILED_STEPS=()

die() {
    printf 'merge-lane: %s\n' "$1" >&2
    exit 2
}

warn() {
    printf 'merge-lane: %s\n' "$1" >&2
}

parse_arguments() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --no-push) no_push=1 ;;
            --keep-worktree) keep_worktree=1 ;;
            -*) die "unknown flag: $1" ;;
            *)
                [ -z "$lane_branch" ] || die "expected one branch, got: $1"
                lane_branch="$1"
                ;;
        esac
        shift
    done
    [ -n "$lane_branch" ] || die "usage: scripts/merge-lane.sh [--no-push] [--keep-worktree] <branch>"
    git check-ref-format --branch "$lane_branch" >/dev/null || die "invalid branch name: $lane_branch"
}

refuse_linked_worktree() {
    local git_dir
    git_dir="$(git -C "$REPO_ROOT" rev-parse --absolute-git-dir)"
    case "$git_dir" in
        */worktrees/*) die "$REPO_ROOT is a linked worktree; run in the main checkout" ;;
    esac
}

refuse_wrong_directory() {
    local caller_root
    caller_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
    [ "$caller_root" = "$REPO_ROOT" ] || die "run from the main checkout root $REPO_ROOT"
}

refuse_wrong_branch() {
    local current_branch
    current_branch="$(git symbolic-ref --quiet --short HEAD || true)"
    [ "$current_branch" = main ] || die "main checkout is on '${current_branch:-detached HEAD}', expected main"
}

refuse_missing_branch() {
    git show-ref --verify --quiet "refs/heads/$lane_branch" || die "branch not found: $lane_branch"
}

refuse_merged_branch() {
    if git merge-base --is-ancestor "$lane_branch" HEAD; then
        die "branch already merged into main: $lane_branch"
    fi
}

refuse_dirty_tree() {
    [ -z "$(git status --porcelain --untracked-files=all)" ] || die "main checkout has uncommitted changes"
}

refuse_bad_preconditions() {
    refuse_linked_worktree
    refuse_wrong_directory
    refuse_wrong_branch
    refuse_missing_branch
    refuse_merged_branch
    refuse_dirty_tree
}

lane_worktree_path() {
    # No early exit: awk must drain the pipe. An `exit` here SIGPIPEs
    # `git worktree list` on a repo with many worktrees, and pipefail
    # then aborts the whole script (found by the first dogfood run).
    git worktree list --porcelain | awk -v target="branch refs/heads/$lane_branch" '
        /^worktree / { path = substr($0, 10) }
        $0 == target && !found { print path; found = 1 }
    '
}

run_step() {
    local step_name="$1"
    shift
    printf '\n===== merge-lane: %s =====\n' "$step_name"
    if "$@"; then
        return 0
    fi
    FAILED_STEPS+=("$step_name")
    return 1
}

merge_branch() {
    pre_merge_sha="$(git rev-parse HEAD)"
    git merge --no-ff -m "Merge branch '$lane_branch' into main" "$lane_branch"
    merge_sha="$(git rev-parse HEAD)"
}

run_gate() {
    GATE_CHECK_NO_PUSH=1 bash "$GATE_CHECK" --arm full
}

verify_remote_main() {
    git fetch origin main
    local local_sha remote_sha
    local_sha="$(git rev-parse main)"
    remote_sha="$(git rev-parse origin/main)"
    printf 'main        = %s\norigin/main = %s\n' "$local_sha" "$remote_sha"
    [ "$local_sha" = "$remote_sha" ] || {
        warn "main and origin/main differ"
        return 1
    }
}

push_and_verify() {
    if [ "$no_push" -eq 1 ]; then
        printf 'merge-lane: --no-push: merge %s stays local; nothing pushed\n' "$merge_sha"
        return 0
    fi
    if ! run_step "push origin main" git push origin main; then
        return 1
    fi
    push_attempted=1
    run_step "verify origin/main" verify_remote_main
}

rollback_after_failure() {
    printf 'merge-lane: failing step(s): %s\n' "${FAILED_STEPS[*]}" >&2
    if [ "$push_attempted" -eq 1 ]; then
        warn "push succeeded; local reset skipped (origin may hold the merge)"
        exit 1
    fi
    git reset --hard "$pre_merge_sha"
    printf 'merge-lane: main reset to %s (nothing pushed)\n' "$pre_merge_sha"
    exit 1
}

run_cleanup_step() {
    local step_name="$1"
    shift
    if "$@"; then
        return 0
    fi
    warn "cleanup step failed ($step_name); finish it by hand"
}

release_lane_data() {
    # Use the lane's own copy: worktree-release.sh resolves its checkout
    # from its own path, so main's copy always refuses (Guard 1) and the
    # slot claim orphans when the worktree is removed anyway.
    local release_script="$lane_path/scripts/worktree-release.sh"
    [ -f "$release_script" ] || release_script="$WORKTREE_RELEASE"
    (cd "$lane_path" && bash "$release_script")
}

cleanup_lane() {
    if [ "$keep_worktree" -eq 1 ]; then
        printf 'merge-lane: --keep-worktree: lane kept at %s\n' "${lane_path:-<none>}"
        return 0
    fi
    if [ -n "$lane_path" ] && [ -d "$lane_path" ]; then
        run_cleanup_step "release data slot" release_lane_data
        run_cleanup_step "remove worktree" git worktree remove "$lane_path"
    fi
    run_cleanup_step "delete merged branch" git branch -d "$lane_branch"
    printf 'merge-lane: lane cleaned up\n'
}

main() {
    parse_arguments "$@"
    lane_path="$(lane_worktree_path)"
    refuse_bad_preconditions

    if ! run_step "merge --no-ff $lane_branch" merge_branch; then
        rollback_after_failure
    fi
    if ! run_step "gate-check --arm full" run_gate; then
        rollback_after_failure
    fi
    if ! push_and_verify; then
        rollback_after_failure
    fi

    printf 'merge-lane: merge commit %s\n' "$merge_sha"
    cleanup_lane
}

main "$@"
