#!/usr/bin/env bash
# play-local-seams.sh — internal decisions of play-local.sh behind named seams.
#
# Source this file from play-local.sh; run it directly for hermetic checks:
#   scripts/lib/play-local-seams.sh selftest
#
# These seams do not change play-local's argv or preflight order. They give
# every decision one name: run-class pin/flock policy, binary staleness, and
# the Memoryer session-token lookup. The live launcher still supplies the
# real repo root, binary, and database; the tests supply synthetic inputs.
set -euo pipefail

PLAY_LOCAL_SEAMS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=rig-run-class.sh
. "$PLAY_LOCAL_SEAMS_DIR/rig-run-class.sh"
# shellcheck source=rig-client-display.sh
. "$PLAY_LOCAL_SEAMS_DIR/rig-client-display.sh"

play_local_needs_display_pin() {
  [ "$1" != "$RIG_RUN_CLASS_OWNER" ]
}

play_local_needs_live_lock() {
  [ "$1" != "$RIG_RUN_CLASS_OWNER" ]
}

play_local_head_commit_timestamp() {
  local repo_root="$1" timestamp
  local -a binary_paths=(crates/ Cargo.toml Cargo.lock ':(exclude,glob)**/tests/**')
  if ! timestamp="$(git -C "$repo_root" log -1 --format=%ct \
    "${binary_paths[@]}" 2>/dev/null)"; then
    printf '0\n'
    return 0
  fi
  printf '%s\n' "${timestamp:-0}"
}

play_local_binary_timestamp() {
  local binary="$1" timestamp
  if ! timestamp="$(stat -c %Y "$binary" 2>/dev/null)"; then
    printf '0\n'
    return 0
  fi
  printf '%s\n' "${timestamp:-0}"
}

play_local_binary_is_stale() {
  local head_timestamp="$1" binary_timestamp="$2"
  [ "$head_timestamp" -gt 0 ] && [ "$binary_timestamp" -lt "$head_timestamp" ]
}

play_local_read_session_token() {
  local database="$1" uid="$2"
  python3 - "$database" "$uid" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
uid = sys.argv[2]
row = db.execute(
    "select TUSER_ID from t_o2jam_login where USER_INDEX_ID=? "
    "order by LOGIN_TIME desc limit 1", (uid,)).fetchone()
if not row:
    sys.exit(f"no login row for USER_INDEX_ID={uid} in {sys.argv[1]}")
print(row[0])
PY
}

# --- hermetic scenario checks ------------------------------------------------
# These legs name the three owner regressions from F1. The red proofs call
# the same display validator the removed unconditional pin called, so a
# future "just skip :0" change cannot pass by weakening the guards.
owner_session_test_class() {
  env -u RIG_RUN -u RIG_DISPLAY -u WINEPREFIX -u RIG_WINEPREFIX \
    -u RIG_CLIENT_KIND DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 \
    bash "$PLAY_LOCAL_SEAMS_DIR/rig-run-class.sh" classify
}

rig_test_class_on_owner_display() {
  env -u RIG_DISPLAY -u WINEPREFIX -u RIG_WINEPREFIX -u RIG_CLIENT_KIND \
    DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 RIG_RUN=1 \
    bash "$PLAY_LOCAL_SEAMS_DIR/rig-run-class.sh" classify
}

assert_true() {
  local scenario="$1"; shift
  "$@" || { echo "play-local-seams: FAIL $scenario" >&2; return 1; }
  echo "play-local-seams: PASS $scenario"
}

assert_false() {
  local scenario="$1"; shift
  if "$@"; then echo "play-local-seams: FAIL $scenario" >&2; return 1; fi
  echo "play-local-seams: PASS $scenario"
}

assert_refusal() {
  local scenario="$1" expected_status="$2" output status=0
  shift 2
  output="$("$@" 2>&1)" || status=$?
  [ "$status" -eq "$expected_status" ] || {
    echo "$output" >&2
    echo "play-local-seams: FAIL $scenario (exit $status, expected $expected_status)" >&2
    return 1
  }
  grep -qF 'REFUSED: no rig display' <<<"$output" || {
    echo "$output" >&2
    echo "play-local-seams: FAIL $scenario (no display refusal)" >&2
    return 1
  }
  echo "play-local-seams: PASS $scenario"
}

selftest_owner_zero_display_skips_pin_red_proof() {
  local run_class
  run_class="$(owner_session_test_class)"
  [ "$run_class" = "$RIG_RUN_CLASS_OWNER" ] || {
    echo "play-local-seams: FAIL owner-zero-display-class ($run_class)" >&2; return 1;
  }
  assert_false owner-zero-display-skips-display-pin play_local_needs_display_pin "$run_class"
  # Red proof: commit 10cd5ef called the pin unconditionally; that same pin
  # refuses the owner's :0 before any build or token lookup.
  assert_refusal owner-zero-display-old-unconditional-pin-refuses "$RIG_E_DISPLAY" \
    env -u RIG_RUN -u RIG_DISPLAY -u WINEPREFIX -u RIG_WINEPREFIX -u RIG_CLIENT_KIND \
    DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 RIG_CLIENT_DISPLAY_SKIP_X_PROBE=1 \
    bash "$PLAY_LOCAL_SEAMS_DIR/rig-client-display.sh" probe
}

selftest_rig_run_on_owner_display_is_refused() {
  local run_class
  run_class="$(rig_test_class_on_owner_display)"
  [ "$run_class" = "$RIG_RUN_CLASS_X11" ] || {
    echo "play-local-seams: FAIL rig-run-on-owner-display-class ($run_class)" >&2; return 1;
  }
  assert_true rig-run-on-owner-display-takes-display-pin \
    play_local_needs_display_pin "$run_class"
  assert_refusal rig-run-on-owner-display-still-refused "$RIG_E_DISPLAY" \
    env -u RIG_DISPLAY -u WINEPREFIX -u RIG_WINEPREFIX -u RIG_CLIENT_KIND \
    DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 RIG_RUN=1 RIG_CLIENT_DISPLAY_SKIP_X_PROBE=1 \
    bash "$PLAY_LOCAL_SEAMS_DIR/rig-client-display.sh" probe
}

selftest_two_owner_clients_take_no_live_lock() {
  local uid run_class
  for uid in 1 2; do
    run_class="$(owner_session_test_class)"
    [ "$run_class" = "$RIG_RUN_CLASS_OWNER" ] || {
      echo "play-local-seams: FAIL owner-uid-$uid-class ($run_class)" >&2; return 1;
    }
    assert_false "owner-uid-$uid-takes-no-live-lock" \
      play_local_needs_live_lock "$run_class"
  done
  # Red proof: the old unconditional wrapper would have serialized both
  # attended clients on the same live-lane flock.
  assert_true old-unconditional-flock-would-serialize-owners \
    play_local_needs_live_lock "$RIG_RUN_CLASS_X11"
}

selftest_test_only_commit_keeps_binary_current() {
  # A test-only commit leaves HEAD_TS at 100; the binary built at 100 is current.
  assert_false test-only-commit-does-not-mark-binary-stale \
    play_local_binary_is_stale 100 100
}

selftest_crate_commit_marks_binary_stale() {
  # A crate commit moves HEAD_TS to 101; the binary built at 100 is stale.
  assert_true crate-commit-marks-binary-stale \
    play_local_binary_is_stale 101 100
}

selftest_token_lookup_reads_latest_login() {
  local token_database token
  token_database="$(mktemp)"
  trap 'rm -f -- "${token_database:-}"' RETURN
  python3 - "$token_database" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("create table t_o2jam_login (USER_INDEX_ID text, TUSER_ID text, LOGIN_TIME integer)")
db.executemany("insert into t_o2jam_login values (?, ?, ?)", [
    ("2", "older-token", 1), ("1", "other-uid", 2), ("2", "newest-token", 3)])
db.commit()
PY
  token="$(play_local_read_session_token "$token_database" 2)"
  [ "$token" = "newest-token" ] || {
    echo "play-local-seams: FAIL token-lookup-latest ($token)" >&2; return 1;
  }
  echo "play-local-seams: PASS token-lookup-reads-latest-login"
}

selftest_token_lookup_refuses_unknown_uid() {
  local token_database output status=0
  token_database="$(mktemp)"
  trap 'rm -f -- "${token_database:-}"' RETURN
  python3 - "$token_database" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("create table t_o2jam_login (USER_INDEX_ID text, TUSER_ID text, LOGIN_TIME integer)")
db.commit()
PY
  output="$(play_local_read_session_token "$token_database" 7 2>&1)" || status=$?
  [ "$status" -ne 0 ] || { echo "play-local-seams: FAIL unknown-uid-accepted" >&2; return 1; }
  grep -qF 'no login row for USER_INDEX_ID=7' <<<"$output" || {
    echo "$output" >&2
    echo "play-local-seams: FAIL unknown-uid-message" >&2
    return 1
  }
  echo "play-local-seams: PASS token-lookup-refuses-unknown-uid"
}

main() {
  case "${1:-}" in
    selftest)
      selftest_owner_zero_display_skips_pin_red_proof
      selftest_rig_run_on_owner_display_is_refused
      selftest_two_owner_clients_take_no_live_lock
      selftest_test_only_commit_keeps_binary_current
      selftest_crate_commit_marks_binary_stale
      selftest_token_lookup_reads_latest_login
      selftest_token_lookup_refuses_unknown_uid
      echo "play-local-seams: selftest OK" ;;
    *) echo "usage: play-local-seams.sh selftest" >&2; return 2 ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
