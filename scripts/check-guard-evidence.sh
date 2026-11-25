#!/usr/bin/env bash
# check-guard-evidence.sh — reject inert-behavior guards with no exe pin (#917).
#
# Usage:
#   scripts/check-guard-evidence.sh
#   scripts/check-guard-evidence.sh --diff <base>
#   CHECK_GUARD_EVIDENCE_ROOT=/fixture/crates scripts/check-guard-evidence.sh
# Environment (fixture seam): CHECK_GUARD_EVIDENCE_ROOT replaces the scan
# root; without it the check scans this checkout's crates/.
#
# A #[test] guard whose name signals inert behavior needs either an exe
# evidence pin or a "next evidence step:" note naming #916 and its ticket.
# CERTIFYING_GUARDS also covers guards whose names hid a certified bug.
# --diff <base> audits only guards added or annotated since the merge-base.
#
# Exit 0: every flagged guard is pinned. Exit 1: a guard has neither a pin
# nor a next-step note. Exit 2: usage or unresolvable diff base.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Keep the audit signals in this one place.
# Underscore boundaries keep tokens such as `mono_to` out of `no_<noun>`.
readonly INERT_NAME_REGEX='(^|_)nowhere($|_)|(^|_)leaves($|_)|(^|_)nothing($|_)|(^|_)stays_|(^|_)no_[A-Za-z0-9]+($|_)'
readonly CERTIFYING_GUARDS='shop_unwired_browse_chrome_lands_nowhere
myroom_my_bag_tab_press_leaves_the_gift_panel_open
item_shop_go_myroom_leaves_without_an_exit_send
settled_solo_result_back_click_posts_4021_and_takes_the_4022_answer'

# An awk program reads each Rust file once: comment lines above a guard
# are its evidence block, and code lines end that block.
readonly AUDIT_PROGRAM='
function has_exe_pin(line) {
    return (line ~ /ev_[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]*/ ||
            (line ~ /7\.55/ && line ~ /0x[0-9A-Fa-f]+/) ||
            line ~ /docs\/[A-Za-z0-9_.\/-]+\.md:[0-9]+/)
}

function has_next_step(line) {
    return (index(line, "next evidence step:") > 0 &&
            line ~ /#[0-9]+/ &&
            index(line, "#916") > 0)
}

function note_evidence(line) {
    if (has_exe_pin(line) || has_next_step(line)) evidence = 1
}

function is_comment_or_attribute(line) {
    return (line ~ /^[[:space:]]*(\/\/|\/\*|\*|\*\/)/ ||
            line ~ /^[[:space:]]*#\[/ ||
            line ~ /^[[:space:]]*$/)
}

function marks_test(line) {
    return (line ~ /^[[:space:]]*#\[.*test/)
}

function guard_in_scope(path, fn_line,    i, key) {
    if (!diff_mode) return 1
    key = path ":" fn_line
    if (key in changed_lines) return 1
    for (i = 1; i <= comment_count; i++) {
        key = path ":" comment_lines[i]
        if (key in changed_lines) return 1
    }
    return 0
}

BEGIN {
    n = split(certifying, certifying_list, "\n")
    for (i = 1; i <= n; i++) {
        if (certifying_list[i] != "") certifying_names[certifying_list[i]] = 1
    }
    flagged = 0
    unevidenced = 0
}

NR == FNR {
    if ($0 != "__guard_evidence_scope__") changed_lines[$0] = 1
    next
}

{
    line = $0
    if (is_comment_or_attribute(line)) {
        note_evidence(line)
        if (marks_test(line)) is_test = 1
        if (line ~ /^[[:space:]]*(\/\/|\/\*|\*|\*\/)/) {
            comment_lines[++comment_count] = FNR
        }
        next
    }
    if (match(line, /fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\(/)) {
        guard_name = substr(line, RSTART, RLENGTH)
        sub(/^fn[[:space:]]+/, "", guard_name)
        sub(/[[:space:]]*\($/, "", guard_name)
        if (is_test && ((guard_name ~ inert_regex) ||
                        (guard_name in certifying_names))) {
            note_evidence(line)
            if (guard_in_scope(FILENAME, FNR)) {
                flagged++
                if (!evidence) {
                    printf "FAIL %s:%d: %s\n", FILENAME, FNR, guard_name
                    unevidenced++
                }
            }
        }
    }
    evidence = 0
    is_test = 0
    comment_count = 0
}

END {
    printf "check-guard-evidence: %d inert guard(s) scanned, %d without evidence\n", flagged, unevidenced
    exit (unevidenced > 0)
}
'

die() { echo "check-guard-evidence: $*" >&2; exit 2; }

usage() {
    echo "usage: $0 [--diff <base>]" >&2
    exit 2
}

diff_mode=0
diff_base=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --diff)
            [ "$#" -ge 2 ] || usage
            diff_mode=1
            diff_base="$2"
            shift 2
            ;;
        *) die "unknown argument: $1" ;;
    esac
done

scan_root="${CHECK_GUARD_EVIDENCE_ROOT:-crates}"
[ -d "$scan_root" ] || die "scan root does not exist: $scan_root"

mapfile -t source_files < <(find "$scan_root" -type f -name '*.rs' | sort)
[ "${#source_files[@]}" -gt 0 ] || die "no Rust files under $scan_root"

# Every added line is a scope hit. A guard whose annotation comment was
# added also hits, because its comment lines are in the same scope set.
diff_scope_lines() {
    printf '__guard_evidence_scope__\n'
    git diff --unified=0 "$1" -- "$scan_root" | awk '
        /^\+\+\+ b\// { path = substr($0, 7); next }
        /^@@ / {
            hunk = $0
            sub(/^@@ [^ ]+ /, "", hunk)
            sub(/ @@.*/, "", hunk)
            split(hunk, bounds, ",")
            added_line = bounds[1] + 0
            next
        }
        /^\+/ { print path ":" added_line; added_line++ }
    '
    while IFS= read -r path; do
        awk -v path="$path" '{ print path ":" FNR }' "$path"
    done < <(git ls-files --others --exclude-standard -- "$scan_root")
}

audit_sources() {
    awk -v inert_regex="$INERT_NAME_REGEX" \
        -v certifying="$CERTIFYING_GUARDS" \
        -v diff_mode="$diff_mode" \
        -f <(printf '%s\n' "$AUDIT_PROGRAM") \
        "$1" "${source_files[@]}"
}

if [ "$diff_mode" -eq 1 ]; then
    merge_base="$(git merge-base "$diff_base" HEAD 2>/dev/null)" ||
        die "cannot resolve merge-base with: $diff_base"
    audit_sources <(diff_scope_lines "$merge_base")
else
    audit_sources <(printf '__guard_evidence_scope__\n')
fi
