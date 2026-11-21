#!/usr/bin/env bash
#
# Single merge gate for recomp2jam.
#
# Full mode (default) runs, in order, with one exit code and no pipes to
# `tail`:
#   1. cargo fmt --check
#   2. cargo clippy --workspace --all-targets -- -D warnings
#   3. cargo test --workspace  (install-root present)
#   4. cargo test --workspace  (install-root hidden)
#   5. git push origin main
#   6. git rev-parse equality (main == origin/main)
#
# Tiering (owner 2026-11-20): the full suite is the MERGE gate. Per-ticket
# legs use --quick (steps 1-2 plus only the affected crates' tests - seconds
# instead of minutes); the hidden arm is full-mode only, plus on demand via
# --hidden (e.g. diffs touching data-dependent guards) and nightly. Tiers
# live in docs/agents/triage-labels.md close discipline.
#
# worktree-setup.sh links install-root into this worktree's own pool slot.
# Step 4's mv renames that symlink inside this checkout only; the slot and
# other worktrees are untouched. The EXIT trap restores the link.
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
trap 'if [ -n "${HIDDEN_BACKUP:-}" ] && [ -e "${HIDDEN_BACKUP:-}" ]; then mv "${HIDDEN_BACKUP}" "$REPO_ROOT/install-root"; fi' EXIT

present_root_existed=0
if [ -e "$REPO_ROOT/install-root" ]; then present_root_existed=1; fi

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
    # cargo test needs one -p per crate; extra positionals become name
    # filters (found by the arch-adapter lane, 2026-11-21).
    p_args=()
    for crate in "${crates[@]}"; do p_args+=( -p "$crate" ); done
    cargo test "${p_args[@]}"
    exit 0
fi

step "cargo test --workspace (install-root present)"
cargo test --workspace

if [ "$hidden_arm" -eq 1 ]; then
    step "cargo test --workspace (install-root hidden)"
    if [ "$present_root_existed" -eq 1 ]; then
        # Slot symlink: this rename stays inside $REPO_ROOT.
        HIDDEN_BACKUP="$REPO_ROOT/.install-root.gate-hidden"
        mv "$REPO_ROOT/install-root" "$HIDDEN_BACKUP"
    fi
    cargo test --workspace
    if [ -n "$HIDDEN_BACKUP" ] && [ -e "$HIDDEN_BACKUP" ]; then
        mv "$HIDDEN_BACKUP" "$REPO_ROOT/install-root"
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
