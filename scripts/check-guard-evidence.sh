#!/usr/bin/env bash
# check-guard-evidence.sh — reject inert-behavior guards with no exe pin (#917)
# and screenshot-only close claims (#921).
#
# Usage:
#   scripts/check-guard-evidence.sh
#   scripts/check-guard-evidence.sh --diff <base>
#   scripts/check-guard-evidence.sh --close-gate <close-comment-file> [--diff <base>]
#   CHECK_GUARD_EVIDENCE_ROOT=/fixture/crates scripts/check-guard-evidence.sh ...
# Environment (fixture seam): CHECK_GUARD_EVIDENCE_ROOT replaces the scan
# root; without it the check scans this checkout's crates/.
#
# Guard-evidence audit (#917): a #[test] guard whose name signals inert
# behavior needs either an exe evidence pin or a "next evidence step:" note
# naming #916 and its ticket. CERTIFYING_GUARDS also covers guards whose
# names hid a certified bug. --diff <base> audits only guards added or
# annotated since the merge-base.
#
# Close-gate check (#921): --close-gate reads a closing comment (use '-'
# for stdin) and checks each `## Guards` entry. An entry declares a tier
# and either a live caller chain or `seam: <reason>`:
#
#   `guard_name` — tier 3 live chain: `main.rs::run_play` -> ... -> symbol
#   `guard_name` — tier 0 seam: pure parser math
#
# The gate fails a state whose guards are all Tier 0, a `seam:` reason that
# conflicts with a live caller named in the guard's source, and any guard
# whose tier cannot be determined. With --diff, a NEW/changed guard must
# declare its tier. Current tree guards are grandfathered: without a close
# comment the check never revisits them.
#
# Limits: this is a fast textual spot check, not a call graph or a runtime.
# It reads tier words, live-entry tokens, and the comment block above each
# test fn; it cannot prove a chain, and it reports an ambiguity instead of
# guessing when the declaration is absent. It does not replace the live
# confirmation leg in docs/live-path-reachability.md. OPEN debt stays in
# the census (docs/interaction-census.md).
#
# Exit 0: every flagged guard is pinned / the close passes. Exit 1: a guard
# has neither a pin nor a next-step note, or the close fails. Exit 2: usage
# or unresolvable diff base.
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

# Close-gate tokens mirror the live-entry names in
# docs/live-path-reachability.md. Tier words are declared, not guessed.
readonly LIVE_CALLER_TOKENS='main.rs::run_play;run_online;session_walk::drive_walk;live_boot_gate;live_input_legs;connect::connect_and_login_with_transport;GameLoop::ingest_snapshot;stage_live_scenes;OnlineSession'
readonly LIVE_CALLER_REGEX='main\.rs::run_play|run_online|session_walk::drive_walk|live_boot_gate|live_input_legs|connect::connect_and_login_with_transport|GameLoop::ingest_snapshot|stage_live_scenes|OnlineSession'
readonly LIVE_BOOT_REGEX='live_boot_gate|session_walk::drive_walk|live_input_legs'
readonly GATEWAY_TOKENS='gateway;session.rs'
readonly GATEWAY_REGEX='gateway|session\.rs'
readonly TIMING_TOKENS='timing;pacing;wall_trace;latency;capture;ssim;frame clock;fps'
readonly TIMING_REGEX='timing|pacing|wall_trace|latency|capture|ssim|frame clock|fps'

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

# The close comment is one line per guard. State headings let one file
# carry more than one touched state; every entry carries an annotation.
readonly CLOSE_PROGRAM='
function emit(state, guard, annotation, tier) {
    printf "%s\t%s\t%s\t%s\n", state, guard, annotation, tier
}

/^##[[:space:]]/ {
    heading = $0
    sub(/^##[[:space:]]+/, "", heading)
    lower = tolower(heading)
    if (lower ~ /^guards/) {
        in_guards = 1
        next
    }
    if (lower ~ /^state/) {
        sub(/^[^[:space:]:]+[[:space:]]*[:=-]?[[:space:]]*/, "", heading)
        state = heading
    }
    in_guards = 0
    next
}

/^[[:space:]]*State[[:space:]]*:/ {
    state = $0
    sub(/^[[:space:]]*State[[:space:]]*:[[:space:]]*/, "", state)
    next
}

in_guards && /\(none[[:space:]]+--[[:space:]]*(docs|presentational):/ {
    emit(state, "EXEMPT", $0, "")
    next
}

in_guards && /`[^`]+`/ {
    line = $0
    match(line, /`[^`]+`/)
    guard = substr(line, RSTART + 1, RLENGTH - 2)
    annotation = substr(line, RSTART + RLENGTH + 1)
    sub(/^[[:space:]]*[-—–]*[[:space:]]*/, "", annotation)
    tier = ""
    lowered = tolower(line)
    if (match(lowered, /tier[ -]?[0-4]/)) {
        tier = substr(lowered, RSTART, RLENGTH)
        gsub(/[^0-9]/, "", tier)
    }
    emit(state, guard, annotation, tier)
}
'

# One pass over the wanted guards collects their tier words and live-caller
# tokens. Marks from the comment block and the test body share a bucket:
# either one is enough to make a `seam:` claim ambiguous.
readonly GUARD_INDEX_PROGRAM='
function note_tier(line) {
    if (source_tier == "" && match(tolower(line), /tier[ -]?[0-4]/)) {
        source_tier = substr(line, RSTART, RLENGTH)
        gsub(/[^0-9]/, "", source_tier)
    }
}

function note_marks(line,    i) {
    if (live_mark == "") {
        for (i = 1; i <= live_count; i++) {
            if (index(line, live_tokens_list[i]) > 0) live_mark = live_tokens_list[i]
        }
    }
    if (gateway_mark == "") {
        for (i = 1; i <= gateway_count; i++) {
            if (index(line, gateway_tokens_list[i]) > 0) gateway_mark = gateway_tokens_list[i]
        }
    }
    if (timing_mark == "") {
        for (i = 1; i <= timing_count; i++) {
            if (index(line, timing_tokens_list[i]) > 0) timing_mark = timing_tokens_list[i]
        }
    }
}

function is_comment_or_attribute(line) {
    return (line ~ /^[[:space:]]*(\/\/|\/\*|\*|\*\/)/ ||
            line ~ /^[[:space:]]*#\[/ ||
            line ~ /^[[:space:]]*$/)
}

function reset_guard_state(    key) {
    source_tier = ""
    live_mark = ""
    gateway_mark = ""
    timing_mark = ""
    for (key in comment_lines) delete comment_lines[key]
    comment_count = 0
}

function emit_guard(    i, changed, key) {
    changed = scope_all
    if (!changed) {
        if ((guard_path ":" fn_line) in changed_lines) changed = 1
        for (i = 1; i <= comment_count; i++) {
            key = guard_path ":" comment_lines[i]
            if (key in changed_lines) changed = 1
        }
    }
    printf "%s|%s|%d|%s|%s|%s|%s|%d\n", fn_name, guard_path, fn_line, \
        source_tier, live_mark, gateway_mark, timing_mark, changed
}

BEGIN {
    wanted_count = split(wanted, wanted_list, "\n")
    for (i = 1; i <= wanted_count; i++) {
        if (wanted_list[i] != "") wanted_names[wanted_list[i]] = 1
    }
    live_count = split(live_tokens, live_tokens_list, ";")
    gateway_count = split(gateway_tokens, gateway_tokens_list, ";")
    timing_count = split(timing_tokens, timing_tokens_list, ";")
}

NR == FNR {
    if ($0 == "__guard_evidence_scope__") scope_all = 1
    else changed_lines[$0] = 1
    next
}

{
    line = $0
    if (in_fn) {
        note_marks(line)
        if (line ~ /^[[:space:]]{0,4}\}/) {
            emit_guard()
            in_fn = 0
            reset_guard_state()
        }
        next
    }
    if (is_comment_or_attribute(line)) {
        note_tier(line)
        note_marks(line)
        if (line ~ /^[[:space:]]*(\/\/|\/\*|\*|\*\/)/) {
            comment_lines[++comment_count] = FNR
        }
        next
    }
    if (match(line, /fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\(/)) {
        fn_name = substr(line, RSTART, RLENGTH)
        sub(/^fn[[:space:]]+/, "", fn_name)
        sub(/[[:space:]]*\($/, "", fn_name)
        if (fn_name in wanted_names) {
            guard_path = FILENAME
            fn_line = FNR
            note_marks(line)
            if (line ~ /\}[[:space:]]*$/) {
                emit_guard()
                reset_guard_state()
            } else {
                in_fn = 1
            }
            next
        }
    }
    reset_guard_state()
}
'

die() { echo "check-guard-evidence: $*" >&2; exit 2; }

usage() {
    echo "usage: $0 [--diff <base> | --close-gate <file> [--diff <base>]]" >&2
    exit 2
}

close_gate_mode=0
close_comment=""
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
        --close-gate)
            [ "$#" -ge 2 ] || usage
            close_gate_mode=1
            close_comment="$2"
            shift 2
            ;;
        *) die "unknown argument: $1" ;;
    esac
done

scan_root="${CHECK_GUARD_EVIDENCE_ROOT:-crates}"
[ -d "$scan_root" ] || die "scan root does not exist: $scan_root"

mapfile -t source_files < <(find "$scan_root" -type f -name '*.rs' | sort)
[ "${#source_files[@]}" -gt 0 ] || die "no Rust files under $scan_root"

merge_base=""
if [ "$diff_mode" -eq 1 ]; then
    merge_base="$(git merge-base "$diff_base" HEAD 2>/dev/null)" ||
        die "cannot resolve merge-base with: $diff_base"
fi

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

if [ "$close_gate_mode" -eq 0 ]; then
    if [ "$diff_mode" -eq 1 ]; then
        audit_sources <(diff_scope_lines "$merge_base")
    else
        audit_sources <(printf '__guard_evidence_scope__\n')
    fi
    exit $?
fi

# Close-gate mode: parse the comment, index the named guards, then check
# annotation conflicts and tier-0-only states.
close_lines=()
if [ "$close_comment" = "-" ]; then
    mapfile -t close_lines
else
    [ -f "$close_comment" ] || die "unreadable close comment: $close_comment"
    mapfile -t close_lines < "$close_comment"
fi

parse_close_comment() {
    printf '%s\n' "${close_lines[@]}" | awk -f <(printf '%s\n' "$CLOSE_PROGRAM")
}

close_records=()
mapfile -t close_records < <(parse_close_comment)
[ "${#close_records[@]}" -gt 0 ] ||
    die "close comment has no ## Guards entries: $close_comment"

declare -A guard_meta
close_failures=0
guards_checked=0
declare -A state_total
declare -A state_tier0

close_fail() {
    printf 'FAIL check-guard-evidence close-gate: %s\n' "$1"
    close_failures=$((close_failures + 1))
}

guard_names=()
for record in "${close_records[@]}"; do
    IFS=$'\t' read -r _state guard _annotation _tier <<<"$record"
    [ "$guard" = "EXEMPT" ] || guard_names+=("$guard")
done

build_guard_index() {
    awk -v wanted="$(printf '%s\n' "${guard_names[@]}")" \
        -v live_tokens="$LIVE_CALLER_TOKENS" \
        -v gateway_tokens="$GATEWAY_TOKENS" \
        -v timing_tokens="$TIMING_TOKENS" \
        -f <(printf '%s\n' "$GUARD_INDEX_PROGRAM") \
        "$1" "${source_files[@]}"
}

if [ "${#guard_names[@]}" -gt 0 ]; then
    guard_index=()
    if [ "$diff_mode" -eq 1 ]; then
        mapfile -t guard_index < <(build_guard_index <(diff_scope_lines "$merge_base"))
    else
        mapfile -t guard_index < <(build_guard_index <(printf '__guard_evidence_scope__\n'))
    fi
    for entry in "${guard_index[@]}"; do
        IFS='|' read -r name meta <<<"$entry"
        guard_meta["$name"]="$meta"
    done
fi

check_close_guard() {
    local record="$1" state guard annotation declared_tier meta
    local guard_path guard_line source_tier live_mark gateway_mark timing_mark guard_changed
    local lower tier is_seam=0 ambiguity=""

    IFS=$'\t' read -r state guard annotation declared_tier <<<"$record"
    if [ "$guard" = "EXEMPT" ]; then
        printf 'check-guard-evidence close-gate: state %s exempt (%s)\n' \
            "$state" "documentation or presentational fix"
        return
    fi
    guards_checked=$((guards_checked + 1))
    meta="${guard_meta[$guard]:-}"
    if [ -z "$meta" ]; then
        close_fail "$guard: no test found under $scan_root (ambiguity: unknown guard name)"
        return
    fi
    IFS='|' read -r guard_path guard_line source_tier live_mark gateway_mark \
        timing_mark guard_changed <<<"$meta"

    tier="$declared_tier"
    if [ -n "$source_tier" ]; then
        if [ -n "$tier" ] && [ "$tier" != "$source_tier" ]; then
            ambiguity="close says tier $tier, $guard_path:$guard_line says tier $source_tier"
        fi
        [ -n "$tier" ] || tier="$source_tier"
    fi

    lower="${annotation,,}"
    if [[ "$lower" == *"seam:"* ]]; then
        is_seam=1
    elif [[ "$lower" != *"live chain"* && "$lower" != *"live caller chain"* ]]; then
        close_fail "$guard: no live chain or seam: reason (ambiguity: $annotation)"
        return
    fi

    if [ "$is_seam" -eq 1 ]; then
        if [ -n "$live_mark" ] || [ -n "$gateway_mark" ] ||
           [[ "$annotation" =~ $LIVE_CALLER_REGEX ]] ||
           [[ "$annotation" =~ $GATEWAY_REGEX ]]; then
            ambiguity="${ambiguity:+$ambiguity; }ambiguity: seam: conflicts with live caller ${live_mark:-${gateway_mark:-in the close comment}} at $guard_path:$guard_line"
        fi
        if [ -n "$tier" ] && [ "$tier" != 0 ]; then
            ambiguity="${ambiguity:+$ambiguity; }seam: reason declares tier $tier"
        fi
        if [ -n "$ambiguity" ]; then
            close_fail "$guard: $ambiguity"
            return
        fi
        tier=0
    fi

    if [ "$diff_mode" -eq 1 ] && [ "$guard_changed" -eq 1 ] && [ -z "$tier" ]; then
        close_fail "$guard: changed guard has no tier (ambiguity: declare tier N in its name, comment, or close entry)"
        return
    fi

    if [ -z "$tier" ]; then
        if [[ "$annotation" =~ $LIVE_BOOT_REGEX ]]; then
            tier=3
        elif [[ "$annotation" =~ $GATEWAY_REGEX ]] || [ -n "$gateway_mark" ]; then
            tier=2
        elif [[ "$annotation" =~ $TIMING_REGEX ]] || [ -n "$timing_mark" ]; then
            tier=4
        elif [ -n "$live_mark" ] || [[ "$annotation" =~ $LIVE_CALLER_REGEX ]]; then
            tier=3
        else
            close_fail "$guard: cannot infer tier (ambiguity: declare tier N in its name, comment, or close entry)"
            return
        fi
    fi

    state_total["$state"]=$(( ${state_total["$state"]:-0} + 1 ))
    if [ "$tier" -eq 0 ]; then
        state_tier0["$state"]=$(( ${state_tier0["$state"]:-0} + 1 ))
    fi
    printf 'check-guard-evidence close-gate: state %s guard %s tier %s\n' \
        "$state" "$guard" "$tier"
}

for record in "${close_records[@]}"; do
    check_close_guard "$record"
done

for state in "${!state_total[@]}"; do
    if [ "${state_tier0["$state"]:-0}" -eq "${state_total["$state"]}" ]; then
        close_fail "state $state: tier-0-only close (all ${state_total["$state"]} guard(s) are Tier 0)"
    fi
done

printf 'check-guard-evidence close-gate: %d guard(s) checked, %d failure(s)\n' \
    "$guards_checked" "$close_failures"
[ "$close_failures" -eq 0 ]
