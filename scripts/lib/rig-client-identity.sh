#!/usr/bin/env bash
# rig-client-identity.sh — per-uid 1000 client id for multi-client rig boots (#726).
#
# Source this file from a launcher, or run it directly for hermetic checks:
#   scripts/lib/rig-client-identity.sh probe [UID]
#   scripts/lib/rig-client-identity.sh selftest
#
# The exe's 7.55 Authorize builder carries a fixed client id; the launch
# blob varies user id, token, and gender only. Memoryer deduplicates on the
# token (AuthController.cs:35-39), not on the client id, so a second client
# must use a different uid regardless of this setting. The override exists
# only to give rig boots one distinct *logged* identity per uid.
#
# Caller chain: play-local.sh --user-id N -> rig_export_client_identity N ->
# O2JAM_CLIENT_ID -> recomp2jam online -> setup_online -> connect_and_login ->
# authorize -> AuthorizeRequest::encode_body (1000 frame).
set -euo pipefail

RIG_CLIENT_ID_BASE="${RIG_CLIENT_ID_BASE:-726PNA-B1YW-28ZG9M-I5SC-Q0OPM6}"
RIG_CLIENT_ID_ENV=O2JAM_CLIENT_ID

# Derive a distinct wire identity for one Memoryer uid. Reject anything
# that is not a decimal uid so a typo cannot silently mint a shared id.
rig_client_id_for_uid() {
  local uid="${1:-}"
  case "$uid" in
    '' | *[!0-9]*) echo "rig-client-identity: invalid uid '$uid'" >&2; return 2 ;;
  esac
  printf '%s-UID%s\n' "$RIG_CLIENT_ID_BASE" "$uid"
}

# Export the 1000 client id. Owner boots keep the exe literal by default;
# an explicit caller value wins; the multi-client mode derives from --user-id.
rig_export_client_identity() {
  local uid="${1:-}" derived
  if [ -n "${O2JAM_CLIENT_ID:-}" ]; then
    export O2JAM_CLIENT_ID
    echo "rig-client-identity: explicit $RIG_CLIENT_ID_ENV=$O2JAM_CLIENT_ID"
    return 0
  fi
  if [ "${RIG_DISTINCT_CLIENT_IDS:-0}" = "1" ]; then
    derived="$(rig_client_id_for_uid "$uid")" || return "$?"
    export O2JAM_CLIENT_ID="$derived"
    echo "rig-client-identity: uid $uid -> $RIG_CLIENT_ID_ENV=$O2JAM_CLIENT_ID"
    return 0
  fi
  unset O2JAM_CLIENT_ID
  echo "rig-client-identity: default 7.55 client id ($RIG_CLIENT_ID_ENV unset)"
}

# --- hermetic scenario checks ------------------------------------------------
# These pin the exported function, not a boot: no display, server, or game
# data is touched. The live two-socket probe and the two-client boot pin the
# env-to-wire leg end to end.
selftest_owner_run_keeps_exe_client_id() {
  local output
  output="$(env -u O2JAM_CLIENT_ID RIG_DISTINCT_CLIENT_IDS=0 bash "$0" probe 2 2>&1)"
  grep -qF 'O2JAM_CLIENT_ID=<unset>' <<<"$output" || { echo "$output"; return 1; }
  echo "rig-client-identity: PASS owner-run-keeps-exe-client-id"
}

selftest_two_uids_get_distinct_client_ids() {
  local first second
  first="$(env -u O2JAM_CLIENT_ID RIG_DISTINCT_CLIENT_IDS=1 bash "$0" probe 1 2>&1)"
  second="$(env -u O2JAM_CLIENT_ID RIG_DISTINCT_CLIENT_IDS=1 bash "$0" probe 2 2>&1)"
  grep -qF 'O2JAM_CLIENT_ID=726PNA-B1YW-28ZG9M-I5SC-Q0OPM6-UID1' <<<"$first" || {
    echo "$first"; return 1;
  }
  grep -qF 'O2JAM_CLIENT_ID=726PNA-B1YW-28ZG9M-I5SC-Q0OPM6-UID2' <<<"$second" || {
    echo "$second"; return 1;
  }
  [ "$first" != "$second" ] || { echo "$first"; return 1; }
  echo "rig-client-identity: PASS two-uids-get-distinct-client-ids"
}

selftest_explicit_client_id_wins_over_derivation() {
  local output
  output="$(O2JAM_CLIENT_ID=rig-explicit RIG_DISTINCT_CLIENT_IDS=1 \
    bash "$0" probe 2 2>&1)"
  grep -qF 'O2JAM_CLIENT_ID=rig-explicit' <<<"$output" || { echo "$output"; return 1; }
  echo "rig-client-identity: PASS explicit-client-id-wins"
}

selftest_rejects_non_numeric_uid() {
  local output status
  set +e
  output="$(env -u O2JAM_CLIENT_ID RIG_DISTINCT_CLIENT_IDS=1 bash "$0" probe tester 2>&1)"
  status=$?
  set -e
  [ "$status" -eq 2 ] || { echo "$output"; return 1; }
  grep -qF "invalid uid 'tester'" <<<"$output" || { echo "$output"; return 1; }
  echo "rig-client-identity: PASS non-numeric-uid-refused"
}

probe() {
  rig_export_client_identity "${1:-1}"
  printf 'O2JAM_CLIENT_ID=%s\n' "${O2JAM_CLIENT_ID:-<unset>}"
}

main() {
  case "${1:-}" in
    probe) shift; probe "$@" ;;
    selftest)
      selftest_owner_run_keeps_exe_client_id
      selftest_two_uids_get_distinct_client_ids
      selftest_explicit_client_id_wins_over_derivation
      selftest_rejects_non_numeric_uid
      echo "rig-client-identity: selftest OK" ;;
    *) echo "usage: rig-client-identity.sh {probe [UID]|selftest}" >&2; return 2 ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
