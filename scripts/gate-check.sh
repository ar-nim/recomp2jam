#!/usr/bin/env bash
#
# Single merge gate for recomp2jam.
#
# Full mode (default) runs, in order, with one exit code and no pipes to
# `tail`:
#   1. cargo fmt --check
#   2. cargo clippy --workspace --all-targets -- -D warnings
#   3. cargo test --workspace  (install-root present; cheap sample suite)
#   4. full-pack sweep when the stamp is absent or older than seven days
#   5. cargo test --workspace  (install-root hidden)
#   6. git push origin main
#   7. git rev-parse equality (main == origin/main)
#
# Tiering (owner 2026-11-20): the full suite is the MERGE gate. Per-ticket
# legs use --quick (steps 1-2 plus only the affected crates' tests - seconds
# instead of minutes); the hidden arm is full-mode only, plus on demand via
# --hidden (e.g. diffs touching data-dependent guards) and nightly. Tiers
# live in docs/agents/triage-labels.md close discipline.
#
# worktree-setup.sh links install-root into this worktree's own pool slot.
# Step 4's mv renames that symlink inside this checkout only; the slot and
# other worktrees are untouched. The EXIT trap restores the link. Both
# arms export O2JAM_INSTALL_ROOT (scripts/lib/data-root.sh) so stale test
# binaries cannot borrow another worktree's archives; the hidden arm
# probes that its named root and fallback are unreachable before testing.
# The 515-chart sweeps stay in the repo but are skipped in the per-merge
# suite (issue #737); the staleness arm below runs them once a week. The
# real-window X11 proof is a rig-lane run (issue #738):
# scripts/rig-x11-click-proof.sh, not a merge-gate verdict.
#
# Usage: scripts/gate-check.sh [--no-push|--dry-run] [--quick [--] [crate...]] [--hidden]
#   --no-push / --dry-run : skip the push + verify step (for local proof runs).
#   --quick [crate...]    : fmt + clippy + `cargo test -p <crates>` only.
#                           Without crates, derives them from the branch diff.
#   --hidden              : full mode, but also run the hidden install-root arm
#                           (default full mode runs it; --quick never does).
#   GATE_CHECK_NO_PUSH=1  : same as --no-push, via environment.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
source "$REPO_ROOT/scripts/lib/data-root.sh"
source "$REPO_ROOT/scripts/lib/sweep-stamp.sh"
source "$REPO_ROOT/scripts/lib/sweep-sample.sh"

PRESENT_ROOT="$REPO_ROOT/install-root"
HIDDEN_ROOT="$REPO_ROOT/.install-root.gate-hidden"
UNREACHABLE_ROOT="$REPO_ROOT/.install-root.gate-unreachable"
SWEEP_STAMP="$REPO_ROOT/.gate-full-sweep-stamp"
SWEEP_SAMPLE="$REPO_ROOT/fixtures/full-pack-sample.sha256"
SUITE_SKIPS=(--skip real_ojn_covers_extract_and_decode
             --skip shipped_charts_set_bit26_only_on_tutorial_bgm_records
             --skip x11_arm_routes_click_through_real_game)

no_push=0
quick=0
hidden_arm=1
extra_crates=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        --no-push|--dry-run) no_push=1 ;;
        --quick) quick=1 ;;
        --hidden) hidden_arm=1 ;;
        --) shift; break ;;
        *) echo "gate-check: unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done
for arg in "$@"; do extra_crates+=("$arg"); done
if [ "${GATE_CHECK_NO_PUSH:-0}" = "1" ]; then no_push=1; fi
if [ "$quick" -eq 1 ]; then hidden_arm=0; fi

HIDDEN_BACKUP=""

restore_hidden_root() {
    if [ -n "$HIDDEN_BACKUP" ] && [ -e "$HIDDEN_BACKUP" ]; then
        mv "$HIDDEN_BACKUP" "$PRESENT_ROOT"
    fi
}

cleanup_gate() {
    restore_hidden_root
    restore_dangling_data_links
}
trap cleanup_gate EXIT

# The full sweep is a data arm: no archives means nothing to sweep and no
# stamp to write, so the next data-bearing gate still pays the evidence cost.
run_full_pack_sweep_if_stale() {
    if ! data_root_has_archives "$PRESENT_ROOT"; then
        printf 'gate-check: full-pack sweep skipped (no archives)\n'
        return 0
    fi
    if ! sweep_stamp_is_stale "$SWEEP_STAMP"; then
        printf 'gate-check: full-pack sweep skipped (stamp is fresh)\n'
        return 0
    fi
    step "full-pack sweep (stamp absent or older than seven days)"
    cargo test -p o2jam-file real_ojn_covers_extract_and_decode -- --exact
    cargo test -p o2jam-file --test ojn_bit26_sweep -- \
        --exact shipped_charts_set_bit26_only_on_tutorial_bgm_records
    date +%s > "$SWEEP_STAMP"
}

present_root_existed=0
if [ -e "$PRESENT_ROOT" ]; then present_root_existed=1; fi
hide_dangling_data_links "$REPO_ROOT"

step() { printf '\n===== %s =====\n' "$1"; }

step "cargo fmt --check"
cargo fmt --check

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

if [ "$quick" -eq 1 ]; then
    crates=("${extra_crates[@]}")
    if [ "${#crates[@]}" -eq 0 ]; then
        mapfile -t crates < <(git diff --name-only origin/main...HEAD 2>/dev/null | sed -n 's|^crates/\([^/]*\)/.*|\1|p' | sort -u)
    fi
    if [ "${#crates[@]}" -eq 0 ]; then
        echo "gate-check --quick: no crate changes on this branch; steps done"
        exit 0
    fi
    step "cargo test -p ${crates[*]} (--quick)"
    cargo test -p "${crates[@]}" -- "${SUITE_SKIPS[@]}"
    exit 0
fi

step "cargo test --workspace (install-root present)"
export O2JAM_INSTALL_ROOT="$PRESENT_ROOT"
cargo test --workspace -- "${SUITE_SKIPS[@]}"

step "hash-pinned sample of the shipped chart pack"
verify_sweep_sample "$PRESENT_ROOT" "$SWEEP_SAMPLE"

run_full_pack_sweep_if_stale

if [ "$hidden_arm" -eq 1 ]; then
    step "hidden-arm data probe (archives must be unreachable)"
    if [ "$present_root_existed" -eq 1 ]; then
        # Slot symlink: this rename stays inside $REPO_ROOT.
        HIDDEN_BACKUP="$HIDDEN_ROOT"
        mv "$PRESENT_ROOT" "$HIDDEN_BACKUP"
    fi
    export O2JAM_INSTALL_ROOT="$UNREACHABLE_ROOT"
    assert_hidden_data_unreachable "$UNREACHABLE_ROOT" "$PRESENT_ROOT"

    step "cargo test --workspace (install-root hidden)"
    cargo test --workspace -- "${SUITE_SKIPS[@]}"
    if [ -n "$HIDDEN_BACKUP" ] && [ -e "$HIDDEN_BACKUP" ]; then
        mv "$HIDDEN_BACKUP" "$PRESENT_ROOT"
    fi
    HIDDEN_BACKUP=""
fi

if [ "$no_push" -eq 1 ]; then
    step "push + verify (SKIPPED: no-push)"
    echo "gate-check: push step skipped"
    exit 0
fi

step "git push origin main"
git push origin main

step "git rev-parse equality"
git fetch origin main
local_sha="$(git rev-parse main)"
remote_sha="$(git rev-parse origin/main)"
echo "main        = $local_sha"
echo "origin/main = $remote_sha"
[ "$local_sha" = "$remote_sha" ] || { echo "gate-check: SHA mismatch" >&2; exit 1; }
echo "gate-check: all gates green"
