#!/usr/bin/env bash
# extract-installer.sh — extract the O2Jam 7.55 InstallShield packages.
#
# Covers both shipped package shapes:
#   * the game installer (`O2Jam_Classic.exe`): payload is the client tree;
#   * the music pack (`Music_Full.exe`): payload is `Music\o2ma*.ojn/ojm`.
# Both are InstallShield 11.5 single-exe packages whose `Disk1\data*.cab`
# payloads sit inside the setup's own "ISc(" compressed stream. No cab tool
# reads that stream and no silent switch works under Wine, so the branch
# that runs is: `setup.exe /a` in a throwaway Wine prefix on a private Xvfb
# display, with the process working directory on a Wine drive mapped to the
# staging directory. The pack resolves its destination as "<cwd drive
# root>\Music\", so that mapping is what keeps it off the real `Z:\` and
# out of the user's `C:\`.
#
# Branches, cheapest first, each probed on the actual file:
#   A. unshield  — the exe is (or starts with) a readable InstallShield cab.
#   B. 7z        — the exe exposes archive streams that 7z can list.
#   C. wine /a   — everything else: the ISc( wrapped 7.55 packages.
#
# Nothing is ever overwritten in the destination: a name that already exists
# is kept as it is, and a content difference is reported.
#
# Usage:
#   scripts/extract-installer.sh <installer.exe> <dest-dir> [options]
#
# Options:
#   --staging DIR      Stage here instead of a temp dir. An existing staging
#                      dir is reused, so an interrupted run can be resumed
#                      with --staging-already-extracted.
#   --staging-already-extracted
#                      Skip extraction and merge what --staging holds (this
#                      is how a finished staging dir is re-merged).
#   --strip LEVELS     Drop this many leading path levels when merging
#                      (default 0). Music packs stage as `Music/o2ma*`, so
#                      `--strip 1` merges the songs into the music folder.
#   --dry-run          Probe and print the plan; change nothing.
#   --timeout SECONDS  Bound the wine branch (default 3600).
#   --keep-prefix      Leave the throwaway Wine prefix in place.
#
# Exit status: 0 on success, 1 on a blocker (reason printed), 2 on usage.
#
# Evidence trail: docs/reversing.md §0 (extract with Wine, dedicated prefix)
# and docs/install-layout.md (installed tree layout). The media-table probe
# below records why branch C is the branch that runs for these packages.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DRIVE_HELPER="$REPO_ROOT/scripts/lib/extract-installer-drive.py"

# Next/Finish sits this far from the wizard dialog's bottom-right corner on
# every O2Jam page (layout is [< Back][Next>][Cancel], so Cancel is further
# right); measured on the 581x382 and 480x721 dialogs of the music and game
# wizards.
WIZARD_BUTTON_OFFSET="158x22"

die() {
    printf 'extract-installer: %s\n' "$1" >&2
    exit 1
}

usage() {
    awk 'NR > 1 && /^#!/ { exit } NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' \
        "${BASH_SOURCE[0]}"
    exit 2
}

# ---------------------------------------------------------------- arguments

INSTALLER=""
DEST=""
STAGING=""
STRIP_LEVELS=0
DRY_RUN=0
TIMEOUT_SECONDS=3600
KEEP_PREFIX=0
STAGING_GIVEN=0
STAGING_ALREADY_EXTRACTED=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --staging) STAGING="$2"; STAGING_GIVEN=1; shift 2 ;;
        --staging-already-extracted) STAGING_ALREADY_EXTRACTED=1; shift ;;
        --strip) STRIP_LEVELS="$2"; shift 2 ;;
        --dry-run) DRY_RUN=1; shift ;;
        --timeout) TIMEOUT_SECONDS="$2"; shift 2 ;;
        --keep-prefix) KEEP_PREFIX=1; shift ;;
        -h|--help) usage ;;
        --*) die "unknown option: $1" ;;
        *)
            if [ -z "$INSTALLER" ]; then INSTALLER="$1"
            elif [ -z "$DEST" ]; then DEST="$1"
            else die "unexpected argument: $1"
            fi
            shift
            ;;
    esac
done

[ -n "$INSTALLER" ] && [ -n "$DEST" ] || usage
[ -f "$INSTALLER" ] || die "no such installer: $INSTALLER"
if [ "$STAGING_ALREADY_EXTRACTED" -eq 1 ] && [ "$STAGING_GIVEN" -eq 0 ]; then
    die "--staging-already-extracted needs --staging DIR"
fi

# ------------------------------------------------------------ setup stream

# Classify the package by the InstallShield media table appended after the PE
# image: one `Disk1\<name>` record per stored file (name, target, version,
# payload size), then the payload itself. A payload starts with `ISc(`
# (compressed setup stream, unreadable by cab tools) or with a raw cab.
print_setup_stream() {
    python3 - "$INSTALLER" <<'PYTHON'
import re
import sys

SCAN_WINDOW = 1 << 24
RECORD_WINDOW = 512
RECORD = re.compile(rb'Disk1\\[A-Za-z0-9_.\-]{1,32}\x00')

def records(path):
    rows = []
    with open(path, 'rb') as handle:
        carry = b''
        base = 0
        while True:
            block = handle.read(SCAN_WINDOW)
            if not block:
                break
            blob = carry + block
            origin = base - len(carry)
            for match in RECORD.finditer(blob):
                start = match.start()
                parts = blob[start:start + RECORD_WINDOW].split(b'\x00')
                if len(parts) < 3:
                    continue
                name = parts[0].decode('latin1').split('\\')[-1]
                try:
                    size = int(parts[2])
                except ValueError:
                    continue
                version = parts[1].decode('latin1')
                tail_at = start + len(parts[0]) + len(parts[1]) + len(parts[2]) + 3
                tail = blob[tail_at:tail_at + 4]
                storage = 'isc' if tail == b'ISc(' else 'raw' if tail == b'MSCF' else 'other'
                rows.append((origin + start, name, version, size, storage))
            carry = blob[-RECORD_WINDOW:]
            base += len(block)
    return rows

for offset, name, version, size, storage in records(sys.argv[1]):
    print(f'{offset}\t{name}\t{version}\t{size}\t{storage}')
PYTHON
}

carries_compressed_cab() {
    awk -F'\t' '$2 ~ /^data[0-9]*\.cab$/ && $5 == "isc" { found = 1 } END { exit !found }'
}

# -------------------------------------------------------------- inventory

describe_tree() { # <dir>; return 3 when the tree holds no file
    [ -d "$1" ] || { printf '  (missing: %s)\n' "$1"; return 3; }
    python3 - "$1" <<'PYTHON'
import os
import sys

root = sys.argv[1]
counts, total, ids = {}, 0, []
for dirpath, _dirnames, filenames in os.walk(root):
    for filename in filenames:
        path = os.path.join(dirpath, filename)
        try:
            total += os.path.getsize(path)
        except OSError:
            continue
        extension = os.path.splitext(filename)[1].lstrip('.') or '(none)'
        counts[extension] = counts.get(extension, 0) + 1
        if filename.startswith('o2ma') and extension in ('ojn', 'ojm'):
            stem = filename[4:].rsplit('.', 1)[0]
            if stem.isdigit():
                ids.append(int(stem))

print(f'  files: {sum(counts.values())}, bytes: {total:,}')
for extension in sorted(counts):
    print(f'    .{extension:<10} {counts[extension]}')
if ids:
    print(f'  o2ma ids: {len(set(ids))} distinct, {min(ids)}..{max(ids)}')
sys.exit(0 if counts else 3)
PYTHON
}

# ------------------------------------------------------------------ merge

relative_path_after_strip() { # <path> <root>
    local relative="${1#"$2"/}"
    if [ "$STRIP_LEVELS" -gt 0 ]; then
        relative="$(printf '%s\n' "$relative" | cut -d/ -f$((STRIP_LEVELS + 1))-)"
    fi
    printf '%s' "$relative"
}

merge_staging_into_dest() { # <staging>
    local staging="$1"
    local added=0 identical=0 kept_different=0 path relative target
    mkdir -p "$DEST"
    while IFS= read -r -d '' path; do
        relative="$(relative_path_after_strip "$path" "$staging")"
        target="$DEST/$relative"
        if [ ! -e "$target" ]; then
            mkdir -p "$(dirname "$target")"
            cp -p "$path" "$target"
            added=$((added + 1))
        elif cmp -s "$path" "$target"; then
            identical=$((identical + 1))
        else
            printf 'extract-installer: kept existing, content differs: %s\n' "$relative" >&2
            kept_different=$((kept_different + 1))
        fi
    done < <(find "$staging" -type f -print0)
    printf 'extract-installer: merge added %d, identical %d, kept-differing %d\n' \
        "$added" "$identical" "$kept_different"
}

# ----------------------------------------------------------- wine branch C

free_display_number() {
    local number
    for number in $(seq 90 260); do
        [ -e "/tmp/.X11-unix/X$number" ] || { printf ':%s\n' "$number"; return 0; }
    done
    return 1
}

free_drive_letter() { # <wine-prefix>
    local letter
    for letter in m n o p q r s t u v w x y z; do
        [ -e "$1/dosdevices/${letter}:" ] || { printf '%s\n' "$letter"; return 0; }
    done
    return 1
}

run_wine_admin_extract() { # <staging> <work-dir>
    local staging="$1" work="$2"
    command -v wine >/dev/null || die "wine not found (branch C needs it)"
    command -v Xvfb >/dev/null || die "Xvfb not found (branch C runs headless)"
    command -v wineboot >/dev/null || die "wineboot not found (branch C needs it)"
    [ -f "$DRIVE_HELPER" ] || die "missing wizard driver: $DRIVE_HELPER"

    local wine_prefix desktop driver_pid xvfb_pid wine_pid status
    wine_prefix="$work/wine-prefix"
    trap 'kill ${xvfb_pid:-0} ${driver_pid:-0} 2>/dev/null || true; \
          [ "$KEEP_PREFIX" -eq 1 ] || rm -rf "$wine_prefix"; \
          rm -rf "$WORK_DIR"' EXIT

    desktop="$(free_display_number)" || die "no free X display in :90..:260"
    export DISPLAY="$desktop"
    export WINEPREFIX="$wine_prefix"
    export WINEDEBUG="-all"
    export WINEDLLOVERRIDES="mscoree,mshtml="

    Xvfb "$desktop" -screen 0 1024x768x24 -nolisten tcp >"$work/xvfb.log" 2>&1 &
    xvfb_pid=$!
    sleep 2

    printf 'extract-installer: throwaway Wine prefix %s, display %s\n' "$wine_prefix" "$desktop"
    timeout 600 wineboot -i >"$work/wineboot.log" 2>&1 \
        || die "wineboot failed (see $work/wineboot.log)"
    # A modal Wine crash dialog (the InstallShield engine helper ISBEW64.exe
    # faults under Wine) covers the wizard and swallows the driver's clicks.
    wine reg add 'HKCU\Software\Wine\WineDbg' /v ShowCrashDialog /t REG_DWORD /d 0 /f \
        >"$work/reg.log" 2>&1 || true

    local letter
    letter="$(free_drive_letter "$wine_prefix")" || die "no free Wine drive letter"
    ln -sfn "$staging" "$wine_prefix/dosdevices/$letter:"
    printf 'extract-installer: drive %s: -> %s\n' "${letter^^}" "$staging"

    local windows_path
    windows_path="Z:$(realpath "$INSTALLER" | tr '/' '\\')"

    ( cd "$staging" && timeout "$TIMEOUT_SECONDS" wine "$windows_path" /a ) \
        >"$work/wine.log" 2>&1 &
    wine_pid=$!

    python3 "$DRIVE_HELPER" --display "$desktop" --watch "$staging" \
        --button-offset "$WIZARD_BUTTON_OFFSET" \
        --fill-destination "${letter^}:\\" \
        --max-seconds "$TIMEOUT_SECONDS" >"$work/wizard-drive.log" 2>&1 &
    driver_pid=$!

    status=0
    wait "$wine_pid" || status=$?
    kill "$driver_pid" 2>/dev/null || true
    wait "$driver_pid" 2>/dev/null || true
    sed -n '1,20p' "$work/wizard-drive.log" >&2 || true

    case "$status" in
        0) ;;
        124) die "installer exceeded ${TIMEOUT_SECONDS}s; staging kept at $staging" ;;
        *) die "installer exited with status $status (log: $work/wine.log)" ;;
    esac
    if [ -z "$(find "$staging" -mindepth 1 -print -quit)" ]; then
        die "installer finished but staged nothing: a wizard page gated it (the game installer asks for survey/e-mail details before Next enables); staging kept at $staging"
    fi
}

# -------------------------------------------------------------------- main

printf 'extract-installer: installer %s\n' "$INSTALLER"
printf 'extract-installer: destination %s\n' "$DEST"

mapfile -t STREAM_ROWS < <(print_setup_stream)
branch=""
if [ "${#STREAM_ROWS[@]}" -gt 0 ]; then
    printf 'extract-installer: InstallShield media table\n'
    printf '%s\n' "${STREAM_ROWS[@]}" \
        | awk -F'\t' '{ printf "    %-14s %-12s %14s  %s\n", $2, $3, $4, $5 }'
    if printf '%s\n' "${STREAM_ROWS[@]}" | carries_compressed_cab; then
        branch="C"
        printf 'extract-installer: data cab is an ISc( stream; cab tools cannot read it\n'
    fi
fi

if [ -z "$branch" ] && timeout 120 unshield l "$INSTALLER" >/dev/null 2>&1; then
    branch="A"
fi
if [ -z "$branch" ] && timeout 300 7z l "$INSTALLER" >/dev/null 2>&1; then
    branch="B"
fi
[ -n "$branch" ] || die "unrecognised package: no media table, not an InstallShield cab, not a 7z archive"

case "$branch" in
    A) printf 'extract-installer: branch A (unshield reads the exe)\n' ;;
    B) printf 'extract-installer: branch B (7z reads the exe)\n' ;;
    C) printf 'extract-installer: branch C (wine /a, throwaway prefix, private Xvfb)\n' ;;
esac

if [ "$DRY_RUN" -eq 1 ]; then
    printf 'extract-installer: dry run, nothing written\n'
    exit 0
fi

if [ "$STAGING_GIVEN" -eq 0 ]; then
    STAGING="$(mktemp -d -t recomp2jam-extract-stage-XXXXXX)"
fi
mkdir -p "$STAGING"
printf 'extract-installer: staging %s\n' "$STAGING"
df -h "$STAGING" | tail -1

WORK_DIR="$(mktemp -d -t recomp2jam-extract-work-XXXXXX)"
trap 'rm -rf "$WORK_DIR"' EXIT

if [ "$STAGING_ALREADY_EXTRACTED" -eq 0 ]; then
    case "$branch" in
        A) ( cd "$STAGING" && timeout "$TIMEOUT_SECONDS" unshield -L x "$INSTALLER" ) \
               || die "unshield failed (output above)" ;;
        B) ( cd "$STAGING" && timeout "$TIMEOUT_SECONDS" 7z x -y "$INSTALLER" ) \
               || die "7z extraction failed (output above)" ;;
        C) run_wine_admin_extract "$STAGING" "$WORK_DIR" ;;
    esac
fi

printf 'extract-installer: staged tree\n'
describe_tree "$STAGING" || die "staging holds no files after extraction"

merge_staging_into_dest "$STAGING"

printf 'extract-installer: destination inventory\n'
describe_tree "$DEST" || die "destination holds no files after merge"
printf 'extract-installer: done\n'
