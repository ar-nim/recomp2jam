#!/usr/bin/env bash
# One runtime data-root contract for gate arms (issue #736).
#
# Test binaries used to bake the archive path in at compile time, so a
# binary built in another worktree could read that worktree's archives
# during the "hidden" arm. The gate now names the root at RUNTIME with
# O2JAM_INSTALL_ROOT (already used by o2jam-dump) and falls back to the
# checkout's git-ignored install-root. Every data-dependent arm goes
# through these helpers, so a stale binary cannot pick another root.
set -euo pipefail

DATA_ROOT_ENV_NAME=O2JAM_INSTALL_ROOT

# gate_data_root REPO_ROOT -> the root the suite must see at runtime.
gate_data_root() {
    local repo_root="$1"
    if [ -n "${O2JAM_INSTALL_ROOT:-}" ]; then
        printf '%s\n' "$O2JAM_INSTALL_ROOT"
    else
        printf '%s\n' "$repo_root/install-root"
    fi
}

# Archive presence matches the skip probes in data-dependent guards.
data_root_has_archives() {
    local root="$1"
    [ -d "$root/O2Jam" ] || [ -d "$root/o2jam" ]
}

# A dangling data symlink is the normal no-data state: it must not stop
# the gate or be mistaken for an archive-bearing root.
data_root_is_dangling_link() {
    local root="$1"
    [ -L "$root" ] && [ ! -e "$root" ]
}

DANGLING_LINK_BACKUPS=()

# hide_dangling_data_links REPO_ROOT
# A fresh clone may carry dead install-root/extracted symlinks. Move them
# aside for the run: they are the no-data state, but the suite's canary
# treats a visible dangling link as a broken real-data arm. The caller
# restores them on exit, so the owner still sees the broken link.
hide_dangling_data_links() {
    local repo_root="$1" name link backup
    for name in install-root extracted; do
        link="$repo_root/$name"
        if data_root_is_dangling_link "$link"; then
            backup="$repo_root/.$name.gate-dangling"
            if [ -e "$backup" ] || [ -L "$backup" ]; then
                printf 'gate-check: stale dangling-link backup: %s\n' "$backup" >&2
                return 1
            fi
            mv "$link" "$backup"
            DANGLING_LINK_BACKUPS+=("$backup|$link")
        fi
    done
}

restore_dangling_data_links() {
    local entry backup target
    for entry in "${DANGLING_LINK_BACKUPS[@]}"; do
        backup="${entry%%|*}"
        target="${entry#*|}"
        if [ -L "$backup" ] || [ -e "$backup" ]; then
            mv "$backup" "$target"
        fi
    done
}

# assert_hidden_data_unreachable ROOT FALLBACK
# Fails the arm when either route can still see archives.
assert_hidden_data_unreachable() {
    local hidden_root="$1" fallback_root="$2"
    if [ -e "$hidden_root" ] || [ -L "$hidden_root" ]; then
        printf 'gate-check: hidden data root still exists: %s\n' "$hidden_root" >&2
        return 1
    fi
    if data_root_has_archives "$hidden_root"; then
        printf 'gate-check: hidden arm can still reach archives at %s\n' "$hidden_root" >&2
        return 1
    fi
    if [ -e "$fallback_root" ] && data_root_has_archives "$fallback_root"; then
        printf 'gate-check: hidden arm can still reach fallback archives at %s\n' \
            "$fallback_root" >&2
        return 1
    fi
}
