#!/usr/bin/env bash
# rig_binary_stale.sh — the one staleness guard for play-local.sh (#748).
#
# Source this file from play-local.sh; run it directly for hermetic checks:
#   scripts/lib/rig_binary_stale.sh selftest
#
# Live chain: scripts/play-local.sh (launcher entry) -> rig_binary_stale_guard
# -> rig_binary_stale_head_commit_timestamp / rig_binary_stale_binary_timestamp
# -> rig_binary_stale. The guard runs after preflight and before token lookup,
# build, or client boot. The selftests build synthetic git repos and binaries;
# no cargo, rig display, Memoryer, or game data is touched.
#
# A failed git/stat probe is an error, never a zero timestamp: play-local must
# not boot a binary it cannot date. Test-only commits (fc8cdfe) are excluded
# from the HEAD query because cargo does not rebuild for tests/.
set -euo pipefail

RIG_BINARY_STALE_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/rig_binary_stale.sh"
RIG_BINARY_STALE_SCRIPTS_DIR="$(cd "$(dirname "$RIG_BINARY_STALE_LIB")/.." && pwd)"
RIG_BINARY_STALE_PATHS=(crates/ Cargo.toml Cargo.lock ':(exclude,glob)**/tests/**')

rig_binary_stale_head_commit_timestamp() {
  local repo_root="$1" output
  if ! output="$(git -C "$repo_root" log -1 --format=%ct \
    -- "${RIG_BINARY_STALE_PATHS[@]}" 2>&1)"; then
    printf 'rig-binary-stale: cannot read binary-relevant HEAD timestamp in %s: %s\n' \
      "$repo_root" "$output" >&2
    return 1
  fi
  printf '%s\n' "${output:-0}"
}

rig_binary_stale_binary_timestamp() {
  local binary="$1" output
  if ! output="$(stat -c %Y "$binary" 2>&1)"; then
    printf 'rig-binary-stale: cannot stat binary %s: %s\n' "$binary" "$output" >&2
    return 1
  fi
  printf '%s\n' "${output:-0}"
}

rig_binary_stale() {
  local head_timestamp="$1" binary_timestamp="$2" value
  for value in "$head_timestamp" "$binary_timestamp"; do
    if [[ ! "$value" =~ ^[0-9]+$ ]]; then
      printf 'rig-binary-stale: invalid timestamp %q\n' "$value" >&2
      return 2
    fi
  done
  [ "$head_timestamp" -gt 0 ] && [ "$binary_timestamp" -lt "$head_timestamp" ]
}

rig_binary_stale_refuse() {
  local profile="$1" head_timestamp="$2" repo_root="$3"
  printf 'play-local: %s binary is stale (built before checkout HEAD %s)\n' \
    "$profile" "$head_timestamp" >&2
  printf 'play-local: rebuild first: cargo build --profile %s --manifest-path "%s/Cargo.toml"\n' \
    "$profile" "$repo_root" >&2
}

rig_binary_stale_guard() {
  local repo_root="$1" profile="$2" binary="$3" head_timestamp binary_timestamp
  [ -x "$binary" ] || return 0
  head_timestamp="$(rig_binary_stale_head_commit_timestamp "$repo_root")" || return 1
  binary_timestamp="$(rig_binary_stale_binary_timestamp "$binary")" || return 1
  if rig_binary_stale "$head_timestamp" "$binary_timestamp"; then
    rig_binary_stale_refuse "$profile" "$head_timestamp" "$repo_root"
    return 1
  fi
}

# --- hermetic scenario checks ------------------------------------------------
RIG_BINARY_STALE_BASE_TS=1700000000
RIG_BINARY_STALE_TESTS_TS=1700000200
RIG_BINARY_STALE_CRATE_TS=1700000400
RIG_BINARY_STALE_SELFTEST_DIRS=()

stale_selftest_cleanup() {
  local repo_root
  for repo_root in "${RIG_BINARY_STALE_SELFTEST_DIRS[@]}"; do
    if [ -d "$repo_root" ]; then
      rm -rf -- "$repo_root"
    fi
  done
}

stale_selftest_commit() {
  local repo_root="$1" message="$2" timestamp="$3"
  git -C "$repo_root" add -A
  GIT_AUTHOR_NAME='Arif Rahman Ibrahim' \
  GIT_AUTHOR_EMAIL='1311330+ar-nim@users.noreply.github.com' \
  GIT_COMMITTER_NAME='Arif Rahman Ibrahim' \
  GIT_COMMITTER_EMAIL='1311330+ar-nim@users.noreply.github.com' \
  GIT_AUTHOR_DATE="@$timestamp +0000" GIT_COMMITTER_DATE="@$timestamp +0000" \
    git -C "$repo_root" commit -q -m "$message"
}

# One fixture repo per leg; its binary predates both follow-up commits.
stale_selftest_repo() {
  local kind="$1" repo_root binary
  repo_root="$(mktemp -d)"
  RIG_BINARY_STALE_SELFTEST_DIRS+=("$repo_root")
  git init -q "$repo_root" >/dev/null 2>&1
  mkdir -p "$repo_root/crates/demo/src" "$repo_root/crates/demo/tests" \
    "$repo_root/target/release"
  printf '[package]\nname = "demo"\nversion = "0.1.0"\n' > "$repo_root/Cargo.toml"
  printf 'pub fn seeded() {}\n' > "$repo_root/crates/demo/src/lib.rs"
  printf '#[test]\nfn seeded() {}\n' > "$repo_root/crates/demo/tests/only.rs"
  stale_selftest_commit "$repo_root" 'seeded binary-relevant files' \
    "$RIG_BINARY_STALE_BASE_TS"
  binary="$repo_root/target/release/recomp2jam"
  printf '#!/bin/sh\nexit 0\n' > "$binary"
  chmod +x "$binary"
  touch -d "@$((RIG_BINARY_STALE_BASE_TS + 60))" "$binary"
  case "$kind" in
    tests-only)
      printf '#[test]\nfn from_fc8cdfe() {}\n' > "$repo_root/crates/demo/tests/only.rs"
      stale_selftest_commit "$repo_root" 'tests only (fc8cdfe)' \
        "$RIG_BINARY_STALE_TESTS_TS" ;;
    crate)
      printf 'pub fn changed() {}\n' >> "$repo_root/crates/demo/src/lib.rs"
      stale_selftest_commit "$repo_root" 'crate change' "$RIG_BINARY_STALE_CRATE_TS" ;;
  esac
  printf '%s\n' "$repo_root"
}

stale_selftest_play_local_output() {
  local repo_root="$1" status=0 output
  output="$(env -u RIG_RUN -u RIG_DISPLAY -u WINEPREFIX -u RIG_WINEPREFIX \
    -u RIG_CLIENT_KIND -u O2JAM_CLIENT_ID -u PLAY_LOCAL_PROFILE \
    DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 \
    bash "$repo_root/scripts/play-local.sh" --check-only 2>&1)" || status=$?
  printf '%s\n%s\n' "$status" "$output"
}

selftest_test_only_commit_keeps_binary_current() {
  local repo_root head_timestamp binary_timestamp
  repo_root="$(stale_selftest_repo tests-only)"
  binary_timestamp="$(rig_binary_stale_binary_timestamp "$repo_root/target/release/recomp2jam")"
  head_timestamp="$(rig_binary_stale_head_commit_timestamp "$repo_root")"
  [ "$head_timestamp" = "$RIG_BINARY_STALE_BASE_TS" ] || {
    echo "rig-binary-stale: FAIL tests-only-head ($head_timestamp)" >&2; return 1;
  }
  if rig_binary_stale "$head_timestamp" "$binary_timestamp"; then
    echo "rig-binary-stale: FAIL tests-only-marked-binary-stale" >&2; return 1;
  fi
  rig_binary_stale_guard "$repo_root" release "$repo_root/target/release/recomp2jam"
  echo "rig-binary-stale: PASS tests-only-commit-keeps-binary-current"
}

selftest_crate_commit_marks_binary_stale() {
  local repo_root head_timestamp binary_timestamp output status=0
  repo_root="$(stale_selftest_repo crate)"
  binary_timestamp="$(rig_binary_stale_binary_timestamp "$repo_root/target/release/recomp2jam")"
  head_timestamp="$(rig_binary_stale_head_commit_timestamp "$repo_root")"
  [ "$head_timestamp" = "$RIG_BINARY_STALE_CRATE_TS" ] || {
    echo "rig-binary-stale: FAIL crate-head ($head_timestamp)" >&2; return 1;
  }
  rig_binary_stale "$head_timestamp" "$binary_timestamp" || {
    echo "rig-binary-stale: FAIL crate-commit-not-stale" >&2; return 1;
  }
  output="$(rig_binary_stale_guard "$repo_root" release \
    "$repo_root/target/release/recomp2jam" 2>&1)" || status=$?
  [ "$status" -eq 1 ] || { echo "$output"; echo "rig-binary-stale: FAIL crate-guard-status ($status)" >&2; return 1; }
  grep -qF 'binary is stale' <<<"$output" || { echo "$output"; return 1; }
  grep -qF 'rebuild first' <<<"$output" || { echo "$output"; return 1; }
  echo "rig-binary-stale: PASS crate-commit-marks-binary-stale"
}

selftest_failed_git_probe_fails_loudly() {
  local output status=0
  output="$(rig_binary_stale_head_commit_timestamp /nonexistent-root 2>&1)" || status=$?
  [ "$status" -ne 0 ] || { echo "rig-binary-stale: FAIL git-fallback-silent" >&2; return 1; }
  grep -qF 'cannot read binary-relevant HEAD timestamp' <<<"$output" || {
    echo "$output"; return 1;
  }
  echo "rig-binary-stale: PASS failed-git-probe-fails-loudly"
}

selftest_failed_stat_probe_fails_loudly() {
  local output status=0
  output="$(rig_binary_stale_binary_timestamp /nonexistent-binary 2>&1)" || status=$?
  [ "$status" -ne 0 ] || { echo "rig-binary-stale: FAIL stat-fallback-silent" >&2; return 1; }
  grep -qF 'cannot stat binary' <<<"$output" || { echo "$output"; return 1; }
  echo "rig-binary-stale: PASS failed-stat-probe-fails-loudly"
}

# Copy the live launcher into the fixture and stub only its preflight checks.
# The guard itself and its caller in play-local.sh are the real ones.
stale_selftest_fixture_launcher() {
  local repo_root="$1"
  mkdir -p "$repo_root/scripts/lib"
  local lib_file
  cp "$RIG_BINARY_STALE_SCRIPTS_DIR/play-local.sh" "$repo_root/scripts/play-local.sh"
  for lib_file in "$RIG_BINARY_STALE_SCRIPTS_DIR"/lib/*; do
    if [ -f "$lib_file" ]; then
      cp "$lib_file" "$repo_root/scripts/lib/"
    fi
  done
  cat >> "$repo_root/scripts/lib/play-local-seams.sh" <<'STUBS'

rig_check_install_root() { return 0; }
rig_check_memoryer() { return 0; }
rig_check_relay_ports_free() { return 0; }
rig_check_audio_sink() { return 0; }
rig_run_class() { printf '%s\n' "$RIG_RUN_CLASS_OWNER"; }
STUBS
}

selftest_play_local_refuses_stale_binary_from_launcher() {
  local repo_root result status output
  repo_root="$(stale_selftest_repo crate)"
  stale_selftest_fixture_launcher "$repo_root"
  result="$(stale_selftest_play_local_output "$repo_root")"
  status="${result%%$'\n'*}"
  output="${result#*$'\n'}"
  [ "$status" -eq 1 ] || { echo "$output"; echo "rig-binary-stale: FAIL live-stale-status ($status)" >&2; return 1; }
  grep -qF 'binary is stale' <<<"$output" || { echo "$output"; return 1; }
  echo "rig-binary-stale: PASS play-local-refuses-stale-binary-from-launcher"
}

selftest_fixture_copy_ignores_lib_subdirectories() {
  local source_scripts fixture_root saved_scripts_dir
  source_scripts="$(mktemp -d)"
  RIG_BINARY_STALE_SELFTEST_DIRS+=("$source_scripts")
  fixture_root="$(mktemp -d)"
  RIG_BINARY_STALE_SELFTEST_DIRS+=("$fixture_root")
  mkdir -p "$source_scripts/lib/__pycache__"
  printf '#!/bin/sh\nexit 0\n' > "$source_scripts/play-local.sh"
  printf '# stub\n' > "$source_scripts/lib/play-local-seams.sh"

  saved_scripts_dir="$RIG_BINARY_STALE_SCRIPTS_DIR"
  RIG_BINARY_STALE_SCRIPTS_DIR="$source_scripts"
  if ! stale_selftest_fixture_launcher "$fixture_root"; then
    RIG_BINARY_STALE_SCRIPTS_DIR="$saved_scripts_dir"
    echo 'rig-binary-stale: FAIL fixture-copy-with-lib-subdirectory' >&2
    return 1
  fi
  RIG_BINARY_STALE_SCRIPTS_DIR="$saved_scripts_dir"
  [ -f "$fixture_root/scripts/lib/play-local-seams.sh" ] || {
    echo 'rig-binary-stale: FAIL fixture-copy-missing-lib' >&2
    return 1
  }
  echo 'rig-binary-stale: PASS fixture-copy-ignores-lib-subdirectories'
}

selftest_play_local_accepts_tests_only_binary_from_launcher() {
  local repo_root result status output
  repo_root="$(stale_selftest_repo tests-only)"
  stale_selftest_fixture_launcher "$repo_root"
  result="$(stale_selftest_play_local_output "$repo_root")"
  status="${result%%$'\n'*}"
  output="${result#*$'\n'}"
  [ "$status" -eq 0 ] || { echo "$output"; echo "rig-binary-stale: FAIL live-tests-only-status ($status)" >&2; return 1; }
  grep -qF 'preflight OK' <<<"$output" || { echo "$output"; return 1; }
  echo "rig-binary-stale: PASS play-local-accepts-tests-only-binary-from-launcher"
}

main() {
  case "${1:-}" in
    selftest)
      trap stale_selftest_cleanup EXIT
      selftest_test_only_commit_keeps_binary_current
      selftest_crate_commit_marks_binary_stale
      selftest_failed_git_probe_fails_loudly
      selftest_failed_stat_probe_fails_loudly
      selftest_fixture_copy_ignores_lib_subdirectories
      selftest_play_local_refuses_stale_binary_from_launcher
      selftest_play_local_accepts_tests_only_binary_from_launcher
      echo "rig-binary-stale: selftest OK" ;;
    *) echo "usage: rig_binary_stale.sh selftest" >&2; return 2 ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
