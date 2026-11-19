#!/usr/bin/env bash
# di-interposer-run.sh — prepare a run-scoped DirectInput replay pack (#699).
#
# This script does NOT boot the exe. It compiles the pinned schedule into the
# binary feed, builds the 32-bit dinput.dll stand-in, and prints the launch
# environment. The boot leg (senior-coordinated) exports these variables and
# starts the client with them in scope.
#
# Feed path: chart groups -> di-interposer-feed -> feed.bin -> dinput.dll ->
# GetDeviceState (0x100 bytes) and GetDeviceData (16-byte records).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PINNED_SCHEDULE="${PINNED_SCHEDULE:-$HOME/scratch/evidence-playing/chart864-l1-groups.txt}"
PINNED_SHA256=554d0fbcfbb9a88fb53a466b5c4b90e58c93fba33437c161b8858cad28e67a61
OUT_DIR=""
EPOCH_MS=""
HOLD_MS=80
MODE="both"
STAGE_DLL_DIR=""

die() { echo "di-interposer-run: $*" >&2; exit 2; }

usage() {
    cat >&2 <<'EOF'
usage: scripts/di-interposer-run.sh --epoch-ms UNIX_MS --out-dir DIR [options]
  --schedule PATH      chart schedule (default: pinned 403-group file)
  --hold-ms MS         DIK hold time (default: 80)
  --mode both|state|events  feed path (default: both)
  --stage-dll DIR      also copy dinput.dll into DIR (e.g. beside OTwo.exe)
EOF
    exit 2
}

parse_arguments() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --epoch-ms) EPOCH_MS="${2:-}"; shift 2 ;;
            --out-dir) OUT_DIR="${2:-}"; shift 2 ;;
            --schedule) PINNED_SCHEDULE="${2:-}"; shift 2 ;;
            --hold-ms) HOLD_MS="${2:-}"; shift 2 ;;
            --mode) MODE="${2:-}"; shift 2 ;;
            --stage-dll) STAGE_DLL_DIR="${2:-}"; shift 2 ;;
            *) usage ;;
        esac
    done
    [ -n "$EPOCH_MS" ] || die "--epoch-ms is required (song clock zero)"
    [ -n "$OUT_DIR" ] || die "--out-dir is required"
    case "$MODE" in both|state|events) ;; *) die "invalid --mode $MODE" ;; esac
    [ -f "$PINNED_SCHEDULE" ] || die "schedule not found: $PINNED_SCHEDULE"
}

verify_schedule() {
    local actual
    actual="$(sha256sum "$PINNED_SCHEDULE" | cut -d' ' -f1)"
    [ "$actual" = "$PINNED_SHA256" ] ||
        die "schedule hash $actual != pinned $PINNED_SHA256"
}

compile_feed() {
    cargo run --quiet --manifest-path "$REPO_ROOT/Cargo.toml" -p di-interposer -- \
        --schedule "$PINNED_SCHEDULE" --epoch-ms "$EPOCH_MS" --hold-ms "$HOLD_MS" \
        --out "$OUT_DIR/feed.bin"
}

build_interposer() {
    bash "$REPO_ROOT/tools/di-interposer/build.sh" "$OUT_DIR"
}

stage_dll() {
    [ -n "$STAGE_DLL_DIR" ] || return 0
    [ -d "$STAGE_DLL_DIR" ] || die "stage dir does not exist: $STAGE_DLL_DIR"
    cp "$OUT_DIR/dinput.dll" "$STAGE_DLL_DIR/dinput.dll"
    echo "di-interposer-run: staged $STAGE_DLL_DIR/dinput.dll"
}

print_environment() {
    local feed_path="$OUT_DIR/feed.bin"
    if command -v winepath >/dev/null 2>&1; then
        feed_path="$(winepath -w "$feed_path" 2>/dev/null || printf '%s' "$feed_path")"
    fi
    cat <<EOF
export DI_INTERPOSER_SCHEDULE="$feed_path"
export DI_INTERPOSER_MODE="$MODE"
export WINEDLLPATH="$OUT_DIR\${WINEDLLPATH:+:\$WINEDLLPATH}"
# Then launch the client from its game directory with the rig's argv file.
EOF
}

main() {
    parse_arguments "$@"
    mkdir -p "$OUT_DIR"
    verify_schedule
    compile_feed
    build_interposer
    stage_dll
    print_environment
}

main "$@"
