#!/usr/bin/env bash
#
# Single check script for recomp2jam: CI and the merge gate call the same
# arms (issue #741). All commands live here; no caller repeats a cargo
# command list. Arms run with one exit code and no pipes to `tail`:
#
#   --arm quick : fmt, clippy, the cheap workspace suite, build (CI)
#   --arm hidden: the hidden install-root suite with a data probe (CI)
#   --arm full  : quick checks, sample, stale sweep, hidden suite, build,
#                 push + verify (senior merge gate)
#
# Per-ticket legs use `--arm quick [-- <crate...>]` (or the legacy
# `--quick [-- <crate...>]`): fmt, clippy and only the named crates'
# tests, without the build. Tiers live in docs/agents/triage-labels.md.
#
# worktree-setup.sh links install-root into this worktree's own pool slot.
# The hidden arm renames that symlink inside this checkout only; the slot
# and other worktrees are untouched. Both data arms export
# O2JAM_INSTALL_ROOT (scripts/lib/data-root.sh) so stale test binaries
# cannot borrow another worktree's archives. The 515-chart sweeps are
# skipped in the suite (issue #737) and run on a seven-day stamp; the
# real-window X11 proof is a rig-lane run (issue #738).
#
# Usage: scripts/gate-check.sh [--arm quick|hidden|full]
#                              [--no-push|--dry-run]
#                              [--quick [--] [crate...]] [--hidden]
#   GATE_CHECK_NO_PUSH=1 : same as --no-push, via environment.
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

arm=full
per_ticket=0
no_push=0
extra_crates=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        --arm) arm="$2"; shift 2; continue ;;
        --no-push|--dry-run) no_push=1 ;;
        --quick) arm=quick; per_ticket=1 ;;
        --hidden) arm=hidden ;;
        --) shift; break ;;
        *) echo "gate-check: unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done
for arg in "$@"; do extra_crates+=("$arg"); done
case "$arm" in
    quick|hidden|full) ;;
    *) echo "gate-check: unknown arm: $arm" >&2; exit 2 ;;
esac
if [ "${#extra_crates[@]}" -gt 0 ]; then
    [ "$arm" = quick ] || {
        echo "gate-check: crate names belong to the quick arm" >&2
        exit 2
    }
    per_ticket=1
fi
if [ "${GATE_CHECK_NO_PUSH:-0}" = "1" ]; then no_push=1; fi

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

present_root_existed=0
if [ -e "$PRESENT_ROOT" ]; then present_root_existed=1; fi
hide_dangling_data_links "$REPO_ROOT"

step() { printf '\n===== %s =====\n' "$1"; }

run_fmt_clippy() {
    step "cargo fmt --check"
    cargo fmt --check

    step "cargo clippy --workspace --all-targets -- -D warnings"
    cargo clippy --workspace --all-targets -- -D warnings
}

run_workspace_suite() {
    step "cargo test --workspace (install-root present)"
    export O2JAM_INSTALL_ROOT="$PRESENT_ROOT"
    cargo test --workspace -- "${SUITE_SKIPS[@]}"
}

run_hidden_suite() {
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
    restore_hidden_root
    HIDDEN_BACKUP=""
}

run_build() {
    step "cargo build --workspace"
    cargo build --workspace
}

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

run_per_ticket_tests() {
    local crates=("${extra_crates[@]}")
    if [ "${#crates[@]}" -eq 0 ]; then
        mapfile -t crates < <(git diff --name-only origin/main...HEAD 2>/dev/null | sed -n 's|^crates/\([^/]*\)/.*|\1|p' | sort -u)
    fi
    if [ "${#crates[@]}" -eq 0 ]; then
        echo "gate-check --quick: no crate changes on this branch; steps done"
        exit 0
    fi
    step "cargo test -p ${crates[*]} (--quick)"
    # cargo test needs one -p per crate; bare positionals become name
    # filters (found by the arch-adapter lane, 2026-11-21).
    p_args=()
    for crate in "${crates[@]}"; do p_args+=( -p "$crate" ); done
    cargo test "${p_args[@]}" -- "${SUITE_SKIPS[@]}"
}

run_push_verify() {
    if [ "$no_push" -eq 1 ]; then
        step "push + verify (SKIPPED: no-push)"
        echo "gate-check: push step skipped"
        return 0
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
}

case "$arm" in
    quick)
        run_fmt_clippy
        if [ "$per_ticket" -eq 1 ]; then
            run_per_ticket_tests
            exit 0
        fi
        run_workspace_suite
        run_build
        ;;
    hidden)
        run_hidden_suite
        ;;
    full)
        run_fmt_clippy
        run_workspace_suite
        step "hash-pinned sample of the shipped chart pack"
        verify_sweep_sample "$PRESENT_ROOT" "$SWEEP_SAMPLE"
        run_full_pack_sweep_if_stale
        run_hidden_suite
        run_build
        run_push_verify
        ;;
esac
