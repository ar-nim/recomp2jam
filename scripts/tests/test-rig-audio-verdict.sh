#!/usr/bin/env bash
# Scenario-named guards for the #744 audio sidecar verdict.
#
#   bash scripts/tests/test-rig-audio-verdict.sh
#
# Live caller chain: scripts/rig-run.sh run -> rig_audio_capture_start;
# scripts/rig-run.sh stop -> rig_audio_capture_stop;
# scripts/run-manifest.sh verify -> scripts/rig-audio.sh verify ->
# rig_audio_verdict -> scripts/lib/rig-audio-verdict.sh.
# The last two legs drive that verify chain with synthetic sidecars: no
# sound server, client, Memoryer, game data, or copyrighted bytes are used.
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "$0")/.." && pwd)"
VERDICT_LIB="$SCRIPTS_DIR/lib/rig-audio-verdict.sh"
VERDICT_SCRIPT="$SCRIPTS_DIR/rig-audio.sh"
TMP_RUN_DIR=""

fail() {
  printf 'test-rig-audio-verdict: FAIL %s: %s\n' "$1" "$2" >&2
  exit 1
}

pass() { printf 'test-rig-audio-verdict: PASS %s\n' "$1"; }

# One synthetic sidecar. Every field the verdict reads is explicit, so a
# leg cannot pass by inheriting a default from another scenario.
write_sidecar() { # <path> <route> <preboot> <on_default> <peak> <probes> <clients> <run_sinks>
  local path="$1" route="$2" preboot="$3" on_default="$4" peak="$5"
  local probes="$6" clients="$7" run_sinks="$8"
  mkdir -p "${path%/*}"
  {
    printf 'lane=audio\n'
    printf 'expect=silence\n'
    printf 'state=captured\n'
    printf 'sink=rig_null_test\n'
    printf 'route=%s\n' "$route"
    printf 'preboot_breach=%s\n' "$preboot"
    printf 'on_default_sink=%s\n' "$on_default"
    printf 'peak_dbfs=%s\n' "$peak"
    printf 'probe_streams=%s\n' "$probes"
    printf 'client_streams=%s\n' "$clients"
    printf 'run_sink_streams=%s\n' "$run_sinks"
  } > "$path"
}

assert_verdict() { # <scenario> <expected word> <sidecar path>
  local scenario="$1" expected="$2" sidecar="$3" decision actual
  decision="$(bash "$VERDICT_LIB" "$sidecar")" || fail "$scenario" "verdict query failed"
  actual="${decision%%$'\t'*}"
  [ "$actual" = "$expected" ] || fail "$scenario" "expected '$expected', got '$decision'"
  pass "$scenario"
}

assert_verify_result() { # <scenario> <expected status> <run dir>
  local scenario="$1" expected="$2" run_dir="$3" status=0 output
  output="$(bash "$VERDICT_SCRIPT" verify "$run_dir" 2>&1)" || status=$?
  [ "$status" -eq "$expected" ] || {
    printf '%s\n' "$output"
    fail "$scenario" "verify exited $status, expected $expected"
  }
  printf '%s\n' "$output"
  pass "$scenario"
}

main() {
  TMP_RUN_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_RUN_DIR"' EXIT
  local tmp="$TMP_RUN_DIR"

  # The two capture waves (RV582, RH-HANDOFF-01): only a corked format
  # probe was attributed, the tap measured 0.0 dBFS, containment held.
  write_sidecar "$tmp/probe-only" probe-only 0 0 0.0 1 0 ""
  assert_verdict 'corked-probe-only-with-clean-containment-is-pass' PASS "$tmp/probe-only"
  write_sidecar "$tmp/house-sink" sink 0 0 0.0 0 1 900
  assert_verdict 'house-sink-route-stays-pass' PASS "$tmp/house-sink"
  # Sidecars cited before the breach fields existed keep the old level-only
  # house verdict; the explicit-breach check above still applies to them.
  cat > "$tmp/legacy-sink" <<'SIDECAR'
route=sink
expect=sound
state=captured
client_streams=1
peak_dbfs=-3.6
SIDECAR
  assert_verdict 'legacy-sink-sidecar-without-breach-fields-stays-pass' PASS "$tmp/legacy-sink"

  # A real playback stream on the owner's default sink is still a breach,
  # even when the sidecar's route word has been rewritten to probe-only.
  write_sidecar "$tmp/default-sink" default-sink-breach 0 1 0.0 0 1 ""
  assert_verdict 'default-sink-stream-refuses-the-verdict' FAIL "$tmp/default-sink"
  write_sidecar "$tmp/forged-probe" probe-only 0 1 0.0 1 0 ""
  assert_verdict 'probe-only-with-a-default-sink-count-refuses' FAIL "$tmp/forged-probe"

  # probe-only is an equivalence only when the probe is the sole attributed
  # stream and every containment fact is readable and clean.
  write_sidecar "$tmp/bound-probe" probe-only 0 0 0.0 1 1 900
  assert_verdict 'probe-only-with-bound-playback-refuses' FAIL "$tmp/bound-probe"
  write_sidecar "$tmp/no-peak" probe-only 0 0 "" 1 0 ""
  assert_verdict 'probe-only-without-a-measured-peak-refuses' FAIL "$tmp/no-peak"
  write_sidecar "$tmp/preboot" probe-only 1 0 0.0 1 0 ""
  assert_verdict 'probe-only-with-preboot-breach-refuses' FAIL "$tmp/preboot"
  write_sidecar "$tmp/unseen" unseen 0 0 0.0 0 0 ""
  assert_verdict 'unseen-route-stays-provisional' OPEN "$tmp/unseen"
  write_sidecar "$tmp/unseen-breach" unseen 0 1 0.0 0 0 ""
  assert_verdict 'unseen-route-with-a-default-sink-count-refuses' FAIL "$tmp/unseen-breach"

  # The live verifier applies the same rule to a finished sidecar.
  mkdir -p "$tmp/probe-run/audio" "$tmp/breach-run/audio"
  cp "$tmp/probe-only" "$tmp/probe-run/audio/manifest"
  cp "$tmp/default-sink" "$tmp/breach-run/audio/manifest"
  : > "$tmp/probe-run/audio/audio.wav"
  : > "$tmp/breach-run/audio/audio.wav"
  assert_verify_result 'verify-accepts-clean-probe-only-sidecar' 0 "$tmp/probe-run"
  assert_verify_result 'verify-refuses-default-sink-sidecar' 1 "$tmp/breach-run"
  local output
  output="$(bash "$VERDICT_SCRIPT" verify "$tmp/breach-run" 2>&1 || true)"
  grep -q 'verdict FAIL' <<<"$output" || fail 'verify-refuses-default-sink-sidecar' \
    'no loud verdict FAIL in verify output'

  echo 'test-rig-audio-verdict: shell suite OK'
}

main "$@"
