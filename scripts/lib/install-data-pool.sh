#!/usr/bin/env bash
# Shared helpers for the persistent install-data slot pool.
#
# Four slots live under ~/scratch/install-data/slot{1..4}. Each slot owns its
# own real install-root/extracted copies. A worktree claims one slot and
# symlinks its data paths there; a claim marker is the lock. Two worktrees
# never resolve to the same physical data dir, so a hidden-arm run cannot
# flake another worktree.

POOL_ROOT="${INSTALL_DATA_POOL:-$HOME/scratch/install-data}"
POOL_SOURCE_ROOT="${INSTALL_DATA_SOURCE:-/home/arnim/Projects/recomp2jam}"
POOL_SLOT_COUNT=4
POOL_DATA_DIRS=(install-root extracted)
POOL_CLAIM_DIR=".claimed"
POOL_OWNER_FILE="owner"
POOL_WORKTREE_MARKER=".worktree-data-slot"

install_data_pool_require_source() {
    local source_dir="$1"
    if [ ! -d "$source_dir" ] || [ -L "$source_dir" ]; then
        printf 'install-data-pool: missing real source dir %s\n' "$source_dir" >&2
        return 1
    fi
}

install_data_pool_refresh_dir() {
    local source_dir="$1"
    local slot_data_dir="$2"
    local staging_dir stale_dir=""

    install_data_pool_require_source "$source_dir" || return 1
    if [ -L "$slot_data_dir" ] || { [ -e "$slot_data_dir" ] && [ ! -d "$slot_data_dir" ]; }; then
        printf 'install-data-pool: refusing non-directory %s\n' "$slot_data_dir" >&2
        return 1
    fi

    staging_dir="$(mktemp -d "$POOL_ROOT/.refresh.XXXXXX")" || return 1
    chmod --reference="$source_dir" "$staging_dir"
    if ! cp -a --reflink=auto -- "$source_dir/." "$staging_dir/"; then
        rm -rf -- "$staging_dir"
        printf 'install-data-pool: copy failed: %s\n' "$source_dir" >&2
        return 1
    fi

    if [ -d "$slot_data_dir" ]; then
        stale_dir="$(mktemp -d "$POOL_ROOT/.stale.XXXXXX")" || { rm -rf -- "$staging_dir"; return 1; }
        rmdir -- "$stale_dir"
        if ! mv -T -- "$slot_data_dir" "$stale_dir"; then
            rm -rf -- "$staging_dir"
            [ -d "$stale_dir" ] && rmdir -- "$stale_dir"
            printf 'install-data-pool: could not retire %s\n' "$slot_data_dir" >&2
            return 1
        fi
    fi

    if ! mv -T -- "$staging_dir" "$slot_data_dir"; then
        [ -n "$stale_dir" ] && mv -T -- "$stale_dir" "$slot_data_dir"
        printf 'install-data-pool: could not install %s\n' "$slot_data_dir" >&2
        return 1
    fi
    if [ -n "$stale_dir" ]; then
        rm -rf -- "$stale_dir"
    fi
    return 0
}

install_data_pool_refresh_slot() {
    local slot_name="$1"
    local data_name
    for data_name in "${POOL_DATA_DIRS[@]}"; do
        install_data_pool_refresh_dir \
            "$POOL_SOURCE_ROOT/$data_name" \
            "$POOL_ROOT/$slot_name/$data_name" || return 1
    done
}

install_data_pool_claim_slot() {
    local worktree_root="$1"
    local slot_number slot_dir claim_dir
    mkdir -p -- "$POOL_ROOT" || return 1
    for ((slot_number = 1; slot_number <= POOL_SLOT_COUNT; slot_number++)); do
        slot_dir="$POOL_ROOT/slot$slot_number"
        mkdir -p -- "$slot_dir" || return 1
        claim_dir="$slot_dir/$POOL_CLAIM_DIR"
        if mkdir -- "$claim_dir" 2>/dev/null; then
            printf '%s\n' "$worktree_root" > "$claim_dir/$POOL_OWNER_FILE"
            printf 'slot%d\n' "$slot_number"
            return 0
        fi
    done
    return 1
}

install_data_pool_release_slot() {
    local slot_name="$1"
    local claim_dir="$POOL_ROOT/$slot_name/$POOL_CLAIM_DIR"
    [ -d "$claim_dir" ] || return 1
    rm -f -- "$claim_dir/$POOL_OWNER_FILE"
    rmdir -- "$claim_dir"
}

install_data_pool_link_worktree() {
    local worktree_root="$1"
    local slot_name="$2"
    local data_name slot_data_dir target
    for data_name in "${POOL_DATA_DIRS[@]}"; do
        slot_data_dir="$POOL_ROOT/$slot_name/$data_name"
        target="$worktree_root/$data_name"
        [ -d "$slot_data_dir" ] && [ ! -L "$slot_data_dir" ] || return 1
        if [ -e "$target" ] || [ -L "$target" ]; then
            printf 'install-data-pool: refusing existing %s\n' "$target" >&2
            return 1
        fi
        ln -s -- "$slot_data_dir" "$target" || return 1
    done
    printf '%s\n' "$slot_name" > "$worktree_root/$POOL_WORKTREE_MARKER"
}

install_data_pool_marker_slot() {
    local worktree_root="$1"
    local marker="$worktree_root/$POOL_WORKTREE_MARKER"
    local slot_name
    [ -f "$marker" ] || return 1
    IFS= read -r slot_name < "$marker" || return 1
    case "$slot_name" in
        slot[1-4]) printf '%s\n' "$slot_name" ;;
        *) return 1 ;;
    esac
}

install_data_pool_unlink_worktree() {
    local worktree_root="$1"
    local data_name target
    for data_name in "${POOL_DATA_DIRS[@]}"; do
        target="$worktree_root/$data_name"
        if [ -L "$target" ]; then
            rm -- "$target" || return 1
        elif [ -e "$target" ]; then
            printf 'install-data-pool: refusing to remove non-symlink %s\n' "$target" >&2
            return 1
        fi
    done
    rm -f -- "$worktree_root/$POOL_WORKTREE_MARKER"
}
