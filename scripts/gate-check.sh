#!/usr/bin/env bash
#
# Single merge gate for recomp2jam.
#
# Runs, in order, with one exit code and no pipes to `tail`:
#   1. cargo fmt --check
#   2. cargo clippy --workspace --all-targets -- -D warnings
#   3. cargo test --workspace  (install-root present)
#   4. cargo test --workspace  (install-root hidden)
#   5. git push origin main
#   6. git rev-parse equality (main == origin/main)
#
# Usage: scripts/gate-check.sh [--no-push|--dry-run]
#   --no-push / --dry-run : skip the push + verify step (for local proof runs).
#   GATE_CHECK_NO_PUSH=1   : same, via environment.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

no_push=0
if [ "${GATE_CHECK_NO_PUSH:-0}" = "1" ]; then no_push=1; fi
for arg in "$@"; do
    case "$arg" in
        --no-push|--dry-run) no_push=1 ;;
        *) echo "gate-check: unknown argument: $arg" >&2; exit 2 ;;
    esac
done

HIDDEN_BACKUP=""
trap 'if [ -n "${HIDDEN_BACKUP:-}" ] && [ -e "${HIDDEN_BACKUP:-}" ]; then mv "${HIDDEN_BACKUP}" "$REPO_ROOT/install-root"; fi' EXIT

present_root_existed=0
if [ -e "$REPO_ROOT/install-root" ]; then present_root_existed=1; fi

step() { printf '\n===== %s =====\n' "$1"; }

step "cargo fmt --check"
cargo fmt --check

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "cargo test --workspace (install-root present)"
cargo test --workspace

step "cargo test --workspace (install-root hidden)"
if [ "$present_root_existed" -eq 1 ]; then
    HIDDEN_BACKUP="$REPO_ROOT/.install-root.gate-hidden"
    mv "$REPO_ROOT/install-root" "$HIDDEN_BACKUP"
fi
cargo test --workspace
if [ -n "$HIDDEN_BACKUP" ] && [ -e "$HIDDEN_BACKUP" ]; then
    mv "$HIDDEN_BACKUP" "$REPO_ROOT/install-root"
fi
HIDDEN_BACKUP=""

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
