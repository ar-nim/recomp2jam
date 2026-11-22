#!/usr/bin/env bash
# rig-audio-verdict.sh — one verdict vocabulary for the audio sidecar (#744).
#
# Source this from scripts/rig-audio.sh, or query one sidecar directly:
#   scripts/lib/rig-audio-verdict.sh <audio/manifest>
#
# Prints one tab-separated decision: `WORD<tab>reason`, where WORD is PASS,
# FAIL or OPEN. The query always exits 0; a caller judges the word, never the
# query status. OPEN means "no bound stream was attributed" and is not a
# contained verdict; it keeps legacy `unseen` packs on their old level-only
# path (docs/rig-runbook.md).
#
# Live chain: scripts/rig-run.sh run -> rig_audio_capture_start;
# scripts/run-manifest.sh verify -> scripts/rig-audio.sh verify ->
# rig_audio_verdict -> this lib. The hermetic scenario legs live in
# scripts/tests/test-rig-audio-verdict.sh.
set -euo pipefail

rig_audio_verdict_sidecar_field() { # <sidecar> <key> -> value (empty when absent)
  local sidecar="$1" key="$2"
  [ -r "$sidecar" ] || return 1
  sed -n "s/^$key=//p" "$sidecar" | head -1
}

# 0.0 dBFS is the full-scale ceiling recorded by the two probe-only waves.
# A tap peak at or below it, including digital silence, is a measured level;
# an empty or unparseable field is not evidence of containment.
rig_audio_verdict_peak_is_measured() { # <peak_dbfs>
  local peak="$1"
  [ -n "$peak" ] || return 1
  awk -v peak="$peak" 'BEGIN {
    if (peak == "-inf" || peak == "-infinity") exit 0
    if (peak !~ /^-?[0-9]+([.][0-9]+)?$/) exit 1
    exit (peak > 0.0) ? 1 : 0
  }'
}

# An explicit breach outranks the route word, including a stale `unseen`
# sidecar. Missing fields are handled by the route cases below, so legacy
# packs without these fields keep their old level-only path.
rig_audio_verdict_explicit_breach() { # <sidecar> -> reason on failure
  local sidecar="$1" preboot on_default
  preboot="$(rig_audio_verdict_sidecar_field "$sidecar" preboot_breach || true)"
  [ -z "$preboot" ] || [ "$preboot" = 0 ] || {
    printf 'preboot_breach=%s' "$preboot"
    return 1
  }
  on_default="$(rig_audio_verdict_sidecar_field "$sidecar" on_default_sink || true)"
  [ -z "$on_default" ] || [ "$on_default" = 0 ] || {
    printf 'on_default_sink=%s' "$on_default"
    return 1
  }
  return 0
}

# The exact containment metric set from the rig runbook: pre-boot breach,
# attributed streams on the owner's default sink, and the measured tap peak.
rig_audio_verdict_clean_containment() { # <sidecar> -> reason on failure
  local sidecar="$1" preboot on_default peak
  preboot="$(rig_audio_verdict_sidecar_field "$sidecar" preboot_breach || true)"
  [ "$preboot" = 0 ] || {
    printf 'preboot_breach=%s (want 0)' "${preboot:-missing}"
    return 1
  }
  on_default="$(rig_audio_verdict_sidecar_field "$sidecar" on_default_sink || true)"
  [ "$on_default" = 0 ] || {
    printf 'on_default_sink=%s (want 0)' "${on_default:-missing}"
    return 1
  }
  peak="$(rig_audio_verdict_sidecar_field "$sidecar" peak_dbfs || true)"
  rig_audio_verdict_peak_is_measured "$peak" || {
    printf 'peak_dbfs=%s (want a measured level at or below 0.0 dBFS)' "${peak:-missing}"
    return 1
  }
  return 0
}

# route=sink needs a bound playback stream; route=probe-only needs its corked
# format query and no bound stream. A bound stream on the run sink must never
# be re-labeled as probe-only, even when containment fields are clean.
rig_audio_verdict_attributed_streams() { # <route> <sidecar> -> reason on failure
  local route="$1" sidecar="$2" probes clients run_streams
  clients="$(rig_audio_verdict_sidecar_field "$sidecar" client_streams || true)"
  if [ "$route" = sink ]; then
    [[ "$clients" =~ ^[0-9]+$ ]] && [ "$clients" -ge 1 ] || {
      printf 'client_streams=%s (route=sink needs a bound stream)' "${clients:-missing}"
      return 1
    }
    return 0
  fi
  probes="$(rig_audio_verdict_sidecar_field "$sidecar" probe_streams || true)"
  [[ "$probes" =~ ^[0-9]+$ ]] && [ "$probes" -ge 1 ] || {
    printf 'probe_streams=%s (want a corked format probe)' "${probes:-missing}"
    return 1
  }
  [ "$clients" = 0 ] || {
    printf 'client_streams=%s (want 0: only the probe is attributed)' "${clients:-missing}"
    return 1
  }
  run_streams="$(rig_audio_verdict_sidecar_field "$sidecar" run_sink_streams || true)"
  [ -z "$run_streams" ] || {
    printf 'run_sink_streams=%s (want empty for probe-only)' "$run_streams"
    return 1
  }
  return 0
}

rig_audio_verdict() { # <sidecar> -> "PASS|FAIL|OPEN<tab>reason"
  local sidecar="$1" route reason
  [ -r "$sidecar" ] || {
    printf 'FAIL\tno readable audio sidecar at %s\n' "$sidecar"
    return 0
  }
  if ! reason="$(rig_audio_verdict_explicit_breach "$sidecar")"; then
    printf 'FAIL\t%s (containment breach outranks the route word)\n' "$reason"
    return 0
  fi
  route="$(rig_audio_verdict_sidecar_field "$sidecar" route || true)"
  case "$route" in
    sink)
      # The house route predates the breach fields; cited legacy packs keep
      # their level-only verdict. An explicit breach was refused above.
      if ! reason="$(rig_audio_verdict_attributed_streams sink "$sidecar")"; then
        printf 'FAIL\t%s\n' "$reason"
      else
        printf 'PASS\thouse sink route accepted\n'
      fi
      ;;
    probe-only)
      if ! reason="$(rig_audio_verdict_clean_containment "$sidecar")"; then
        printf 'FAIL\t%s\n' "$reason"
      elif ! reason="$(rig_audio_verdict_attributed_streams probe-only "$sidecar")"; then
        printf 'FAIL\t%s\n' "$reason"
      else
        printf 'PASS\tcontainment clean; route=probe-only accepted\n'
      fi
      ;;
    unseen | unchecked | '')
      printf 'OPEN\tno bound stream was attributed; level evidence decides\n'
      ;;
    default-sink-breach)
      printf 'FAIL\ta stream of this run reached the default sink\n'
      ;;
    *)
      printf 'FAIL\troute=%s is not a contained route\n' "${route:-missing}"
      ;;
  esac
  return 0
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  [ "$#" -eq 1 ] || { echo "usage: rig-audio-verdict.sh <audio/manifest>" >&2; exit 2; }
  rig_audio_verdict "$1"
fi
