#!/usr/bin/env bash
# Hash-pinned sample of the shipped chart pack (issue #737).
#
# The full 515-chart sweep runs on a seven-day stamp; every merge still
# checks this dozen-chart subset. Hashes are names only — no copyrighted
# bytes live in the repo. The Rust sample decoder fixture reads the same
# manifest (fixtures/full-pack-sample.sha256).
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/data-root.sh"

verify_sweep_sample() {
    local root="$1" manifest="$2" name digest path actual
    if ! data_root_has_archives "$root"; then
        printf 'gate-check: sweep sample skipped (no archives)\n'
        return 0
    fi
    while read -r name digest; do
        case "$name" in
            ''|\#*) continue ;;
        esac
        path="$root/O2Jam/Music/$name"
        if [ ! -f "$path" ]; then
            printf 'gate-check: pinned sample chart missing: %s\n' "$name" >&2
            return 1
        fi
        actual="$(sha256sum "$path")"
        actual="${actual%% *}"
        if [ "$actual" != "$digest" ]; then
            printf 'gate-check: pinned sample chart changed: %s\n' "$name" >&2
            return 1
        fi
    done < "$manifest"
}
