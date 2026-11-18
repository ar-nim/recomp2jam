#!/usr/bin/env bash
# rig-audio.sh — the agent-path audio lane (#279): monitor tap, loudness
# probe, and the audio verifier.
#
# Usage:
#   scripts/rig-audio.sh capture-start <out-dir> <sink> <expect> <client>
#   scripts/rig-audio.sh capture-stop <out-dir>
#   scripts/rig-audio.sh assert-route <out-dir> <client>
#   scripts/rig-audio.sh probe <wav> [floor-dbfs]
#   scripts/rig-audio.sh verify <out-dir>
#   scripts/rig-audio.sh stub-tools <bin-dir> <state-dir>
#   scripts/rig-audio.sh selftest
#
# capture-start: taps `<sink>.monitor` with `parec` into
#     `<out-dir>/audio/audio.wav`, then proves the recorder really reads
#     that monitor (a wrong device name does NOT fail on its own: PipeWire
#     silently substitutes the default source, which would record the
#     owner's microphone or speakers — docs/rig-runbook.md, negative #6).
# capture-stop: signals the recorder by process group, writes the loudness
#     measurement and duration into the sidecar `<out-dir>/audio/manifest`.
# assert-route: fails loud (exit 12) when a stream of the run's client is
#     linked to the system default sink: the agent path must never reach
#     the owner's speakers. Wine streams carry the exe name, so the exe
#     stream name is matched in addition to the launcher basename (#311).
#     route=unseen means the guard saw no stream, not that the tap was
#     silent; the level sidecar judges silence.
# probe: prints `peak_dbfs=` and `mean_dbfs=`; exit 0 when the peak is
#     above the silence floor, 1 when the file is silent, 2 when unreadable.
# verify: the audio verifier for a finished run — fails a run whose audio
#     is declared (`expect=sound`) but missing or silent, and a run that
#     never captured the lane at all. Wire it into `run-manifest.sh verify`
#     (the orchestrator owns that call site; see docs/rig-runbook.md).
# expect=silence marks a run that passed `--no-sound` (containment proof,
#     not an audio-parity artifact); expect=disabled marks an explicit
#     opt-out via RIG_AUDIO_CAPTURE=0.
#
# Knobs: RIG_AUDIO_CAPTURE (1 = capture, default), RIG_AUDIO_RATE (48000),
#     RIG_AUDIO_CHANNELS (2), RIG_AUDIO_FORMAT (float32le),
#     RIG_AUDIO_FLOOR_DBFS (-60), RIG_AUDIO_IDENTITY_TRIES (40).
# Selftest-only seams (never set them for a real run):
#     RIG_AUDIO_TEST_STREAM_PIDS, RIG_AUDIO_SKIP_STREAM_BASELINE.
set -euo pipefail

RIG_AUDIO_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/rig-preflight.sh"
# shellcheck source=lib/rig-preflight.sh
. "$RIG_AUDIO_LIB"
# This module's own path: the selftest re-sources the lane in a subshell.
RIG_AUDIO_SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"

die() { echo "rig-audio: $*" >&2; exit 2; }

# Wine's PulseAudio stream is named after the exe, not the launcher (#311).
RIG_AUDIO_EXE_STREAM_NAME="OTwo.exe"

# --- lane layout ------------------------------------------------------------
rig_audio_dir() { printf '%s/audio' "$1"; }
rig_audio_sidecar() { printf '%s/audio/manifest' "$1"; }
rig_audio_artifact() { printf '%s/audio/audio.wav' "$1"; }
rig_audio_recorder_pidfile() { printf '%s/audio/recorder.pid' "$1"; }
rig_audio_route_snapshot_path() { printf '%s/audio/route-snapshot.txt' "$1"; }
rig_audio_route_baseline_path() { printf '%s/audio/route-baseline.txt' "$1"; }

rig_audio_rate() { printf '%s' "${RIG_AUDIO_RATE:-48000}"; }
rig_audio_channels() { printf '%s' "${RIG_AUDIO_CHANNELS:-2}"; }
rig_audio_format() { printf '%s' "${RIG_AUDIO_FORMAT:-float32le}"; }
rig_audio_floor_dbfs() { printf '%s' "${RIG_AUDIO_FLOOR_DBFS:--60}"; }

rig_audio_capturing() { [ "${RIG_AUDIO_CAPTURE:-1}" = "1" ]; }

# --- sidecar fields ---------------------------------------------------------
# The sidecar is one `key=value` line per fact, written once at capture
# start and updated in place by capture-stop and assert-route.
rig_audio_field() {
  local out_dir="$1" key="$2" sidecar
  sidecar="$(rig_audio_sidecar "$out_dir")"
  [ -r "$sidecar" ] || return 1
  sed -n "s/^$key=//p" "$sidecar" | head -1
}

rig_audio_set_field() {
  local out_dir="$1" key="$2" value="$3" sidecar tmp
  sidecar="$(rig_audio_sidecar "$out_dir")"
  [ -f "$sidecar" ] || return 1
  tmp="$sidecar.tmp"
  if grep -q "^$key=" "$sidecar"; then
    KEY="$key" VALUE="$value" awk 'BEGIN { k = ENVIRON["KEY"] "=" } index($0, k) == 1 { print ENVIRON["KEY"] "=" ENVIRON["VALUE"]; next } { print }' "$sidecar" > "$tmp"
  else
    cp "$sidecar" "$tmp"
    printf '%s=%s\n' "$key" "$value" >> "$tmp"
  fi
  mv -f "$tmp" "$sidecar"
}

# The sink's own sample specification, so the manifest records whether the
# tap resampled anything (AC: rate, layout, and resampling are evidence).
rig_audio_sink_spec() {
  local sink="$1"
  [ -n "$sink" ] || return 0
  pactl list sinks short 2>/dev/null | awk -v sink="$sink" '$2 == sink { print $4, $5, $6 }'
}

rig_audio_tap_spec() {
  printf '%s %sch %sHz' "$(rig_audio_format)" "$(rig_audio_channels)" "$(rig_audio_rate)"
}

rig_audio_resampling() {
  local sink="$1" spec
  spec="$(rig_audio_sink_spec "$sink")"
  if [ -z "$spec" ]; then
    echo "unknown (sink specification unreadable)"
  elif [ "$spec" = "$(rig_audio_tap_spec)" ]; then
    echo "none (tap matches the sink)"
  else
    echo "parec (sink '$spec' -> tap '$(rig_audio_tap_spec)')"
  fi
}

rig_audio_write_declaration() {
  local out_dir="$1" sink="$2" expect="$3" client="$4" sidecar
  sidecar="$(rig_audio_sidecar "$out_dir")"
  {
    echo "lane=audio"
    echo "expect=$expect"
    echo "state=starting"
    echo "sink=$sink"
    echo "source=$sink.monitor"
    echo "client=$client"
    echo "recorder=parec"
    echo "rate=$(rig_audio_rate)"
    echo "channels=$(rig_audio_channels)"
    echo "format=$(rig_audio_format)"
    if [ -n "$sink" ]; then
      echo "sink_spec=$(rig_audio_sink_spec "$sink")"
      echo "resampling=$(rig_audio_resampling "$sink")"
    fi
    echo "artifact=audio/audio.wav"
    echo "source_verified=no"
    echo "route=unchecked"
    echo "started_at=$(date -Is)"
    echo "duration_s="
    echo "peak_dbfs="
    echo "mean_dbfs="
  } > "$sidecar"
}

rig_audio_write_opt_out() {
  local out_dir="$1" expect="$2" client="$3"
  mkdir -p "$(rig_audio_dir "$out_dir")"
  rig_audio_write_declaration "$out_dir" "" "disabled" "$client"
  rig_audio_set_field "$out_dir" expect "disabled"
  rig_audio_set_field "$out_dir" state "disabled"
  rig_audio_set_field "$out_dir" declared_expect "$expect"
  rig_note "audio capture disabled (RIG_AUDIO_CAPTURE=0); lane declared opt-out"
}

# --- measurement ------------------------------------------------------------
# Prints `peak_dbfs=` and `mean_dbfs=` (dBFS; a digital-silence file reads
# `-inf`). Exit 2 when the file is unreadable or holds no audio stream.
rig_audio_measure() {
  local wav="$1" measured
  [ -r "$wav" ] || return 2
  measured="$(ffmpeg -hide_banner -nostats -i "$wav" -af volumedetect -f null - 2>&1 |
    sed -n -e 's/.*mean_volume: \(.*\) dB$/mean_dbfs=\1/p' -e 's/.*max_volume: \(.*\) dB$/peak_dbfs=\1/p')" || true
  printf '%s\n' "$measured" | grep -q '^peak_dbfs=' || return 2
  printf '%s\n' "$measured"
}

rig_audio_duration_s() {
  local wav="$1"
  ffprobe -v error -show_entries format=duration -of default=nw=1:nk=1 "$wav" 2>/dev/null || true
}

# 0 when `peak` is louder than the silence floor (so the lane carried audio).
rig_audio_peak_above_floor() {
  local peak="$1" floor="${2:-$(rig_audio_floor_dbfs)}"
  case "$peak" in
    ''|-inf|-infinity) return 1 ;;
  esac
  awk -v peak="$peak" -v floor="$floor" 'BEGIN { exit (peak > floor) ? 0 : 1 }'
}

rig_audio_require_tools() {
  local tool
  for tool in pactl parec ffmpeg ffprobe; do
    if ! command -v "$tool" >/dev/null 2>&1; then
      rig_fail "$RIG_E_AUDIO" audio-capture "'$tool' not found: the audio lane cannot capture (install pulseaudio-utils + ffmpeg)"
      return "$RIG_E_AUDIO"
    fi
  done
  return 0
}

# --- recorder lifecycle -----------------------------------------------------
rig_audio_launch_recorder() {
  local out_dir="$1" sink="$2" artifact log pid
  artifact="$(rig_audio_artifact "$out_dir")"
  log="$(rig_audio_dir "$out_dir")/recorder.log"
  local -a reader=(parec -d "$sink.monitor"
    --format="$(rig_audio_format)" --rate="$(rig_audio_rate)"
    --channels="$(rig_audio_channels)" --file-format=wav "$artifact")
  # setsid gives the recorder its own process group, so capture-stop can
  # signal exactly this process tree and nothing else.
  if command -v setsid >/dev/null 2>&1; then
    setsid "${reader[@]}" >"$log" 2>&1 &
  else
    "${reader[@]}" >"$log" 2>&1 &
  fi
  pid=$!
  echo "$pid" > "$(rig_audio_recorder_pidfile "$out_dir")"
  rig_note "audio capture: parec on '$sink.monitor' (pid $pid) -> $artifact"
}

rig_audio_pids_in_group() {
  local pgid="$1" entry
  for entry in /proc/[0-9]*; do
    [ -r "$entry/stat" ] || continue
    awk -v group="$pgid" '$5 == group { found = 1 } END { exit !found }' "$entry/stat" 2>/dev/null &&
      printf '%s\n' "${entry##*/}"
  done
  return 0
}

rig_audio_group_alive() { [ -n "$(rig_audio_pids_in_group "$1")" ]; }

rig_audio_source_index_of_pid() {
  local wanted="$1"
  pactl list source-outputs 2>/dev/null | awk -v wanted="$wanted" '
    function flush() { if (pid == wanted && src != "") { print src; found = 1 } }
    /^Source Output #/ { flush(); src = ""; pid = ""; next }
    /^\tSource: / { src = $2 }
    /application\.process\.id = / { pid = $3; gsub(/"/, "", pid) }
    END { if (!found) flush() }'
}

rig_audio_source_name() {
  local wanted="$1"
  pactl list sources short 2>/dev/null | awk -v wanted="$wanted" '$1 == wanted { print $2 }'
}

# A wrong `-d` name is not an error in PipeWire: the recorder silently
# captures the default source instead. Prove the tap before trusting it.
rig_audio_await_source_identity() {
  local out_dir="$1" wanted="$2" pgid tries i pid index name
  pgid="$(cat "$(rig_audio_recorder_pidfile "$out_dir")" 2>/dev/null || true)"
  if [ -z "$pgid" ]; then
    rig_fail "$RIG_E_AUDIO" audio-capture "no recorder pid recorded; cannot verify the tap"
    return "$RIG_E_AUDIO"
  fi
  tries="${RIG_AUDIO_IDENTITY_TRIES:-40}"
  for i in $(seq 1 "$tries"); do
    for pid in $(rig_audio_pids_in_group "$pgid" || true); do
      index="$(rig_audio_source_index_of_pid "$pid" || true)"
      [ -n "$index" ] || continue
      name="$(rig_audio_source_name "$index" || true)"
      if [ "$name" = "$wanted" ]; then
        rig_note "audio capture: recorder source verified as '$name'"
        return 0
      fi
      rig_audio_set_field "$out_dir" source_actual "$name"
      rig_fail "$RIG_E_AUDIO" audio-capture "recorder captured from '$name', not '$wanted' (silent-fallback guard)"
      return "$RIG_E_AUDIO"
    done
    rig_audio_group_alive "$pgid" || break
    sleep 0.1
  done
  rig_fail "$RIG_E_AUDIO" audio-capture "recorder source never appeared in pactl source-outputs; cannot prove the '$wanted' tap"
  return "$RIG_E_AUDIO"
}

rig_audio_stop_recorder() {
  local out_dir="$1" pidfile pgid i
  pidfile="$(rig_audio_recorder_pidfile "$out_dir")"
  [ -f "$pidfile" ] || return 0
  pgid="$(cat "$pidfile" 2>/dev/null || true)"
  if [ -n "$pgid" ] && rig_audio_group_alive "$pgid"; then
    kill -- "-$pgid" 2>/dev/null || kill "$pgid" 2>/dev/null || true
    for i in $(seq 1 50); do
      rig_audio_group_alive "$pgid" || break
      sleep 0.1
    done
    if rig_audio_group_alive "$pgid"; then
      kill -9 -- "-$pgid" 2>/dev/null || true
      rig_note "audio capture: recorder (pgid $pgid) needed SIGKILL; the wav header may be short"
    else
      rig_note "audio capture: recorder (pgid $pgid) stopped"
    fi
  fi
  rm -f "$pidfile"
  return 0
}

# --- capture lane (called by rig-run.sh) ------------------------------------
rig_audio_capture_start() {
  local out_dir="$1" sink="$2" expect="$3" client="$4"
  case "$expect" in
    sound | silence) ;;
    *) die "capture-start: expect must be 'sound' or 'silence' (got '$expect')" ;;
  esac
  if ! rig_audio_capturing; then
    rig_audio_write_opt_out "$out_dir" "$expect" "$client"
    return 0
  fi
  rig_audio_require_tools || return "$?"
  if ! rig_sink_present "$sink"; then
    rig_fail "$RIG_E_AUDIO" audio-capture "sink '$sink' is absent; the monitor tap needs a live sink"
    return "$RIG_E_AUDIO"
  fi
  mkdir -p "$(rig_audio_dir "$out_dir")"
  rm -f "$(rig_audio_artifact "$out_dir")"
  rig_audio_write_declaration "$out_dir" "$sink" "$expect" "$client"
  if ! rig_audio_launch_recorder "$out_dir" "$sink"; then
    rig_fail "$RIG_E_AUDIO" audio-capture "cannot start the monitor-tap recorder for '$sink.monitor'"
    rig_audio_set_field "$out_dir" state "failed"
    return "$RIG_E_AUDIO"
  fi
  if ! rig_audio_await_source_identity "$out_dir" "$sink.monitor"; then
    rig_audio_stop_recorder "$out_dir"
    rig_audio_set_field "$out_dir" state "failed"
    return "$RIG_E_AUDIO"
  fi
  rig_audio_set_field "$out_dir" source_verified "yes"
  rig_audio_set_field "$out_dir" state "capturing"
  return 0
}

rig_audio_capture_stop() {
  local out_dir="$1" state artifact measured peak mean duration
  if [ ! -f "$(rig_audio_sidecar "$out_dir")" ]; then
    return 0
  fi
  state="$(rig_audio_field "$out_dir" state || true)"
  case "$state" in
    disabled) return 0 ;;
    failed) return "$RIG_E_AUDIO" ;;
  esac
  rig_audio_stop_recorder "$out_dir"
  artifact="$(rig_audio_artifact "$out_dir")"
  rig_audio_set_field "$out_dir" stopped_at "$(date -Is)"
  if [ ! -r "$artifact" ]; then
    rig_audio_set_field "$out_dir" state "failed"
    rig_note "audio capture FAILED: no artifact at $artifact"
    return "$RIG_E_AUDIO"
  fi
  measured="$(rig_audio_measure "$artifact" || true)"
  peak="$(printf '%s\n' "$measured" | sed -n 's/^peak_dbfs=//p')"
  mean="$(printf '%s\n' "$measured" | sed -n 's/^mean_dbfs=//p')"
  duration="$(rig_audio_duration_s "$artifact")"
  if [ -z "$peak" ]; then
    rig_audio_set_field "$out_dir" state "failed"
    rig_note "audio capture FAILED: '$artifact' has no readable audio stream"
    return "$RIG_E_AUDIO"
  fi
  rig_audio_set_field "$out_dir" peak_dbfs "$peak"
  rig_audio_set_field "$out_dir" mean_dbfs "$mean"
  rig_audio_set_field "$out_dir" duration_s "$duration"
  rig_audio_set_field "$out_dir" state "captured"
  rig_note "audio capture: $artifact (${duration}s, peak ${peak} dBFS, mean ${mean} dBFS)"
  return 0
}

# --- containment assertion --------------------------------------------------
# The route guard preserves the raw PipeWire graph at two moments: before
# boot (the baseline of streams owned by somebody else) and at the verdict.
# Both files are text provenance artifacts, never audio bytes. The verdict
# names the owning process of every stream it attributes to this run.
rig_audio_write_graph_snapshot() {
  local path="$1"
  {
    echo "# pactl get-default-sink"
    pactl get-default-sink 2>/dev/null || :
    echo "# pactl list sinks short"
    pactl list sinks short 2>/dev/null || :
    echo "# pactl list sink-inputs"
    pactl list sink-inputs 2>/dev/null || :
  } > "$path"
}

rig_audio_record_route_baseline() {
  local out_dir="$1" path
  path="$(rig_audio_route_baseline_path "$out_dir")"
  # Hermetic stub seam: a stub stream cannot predate a stub boot.
  if [ "${RIG_AUDIO_SKIP_STREAM_BASELINE:-0}" = "1" ]; then
    : > "$path"
  else
    rig_audio_write_graph_snapshot "$path"
  fi
  rig_audio_set_field "$out_dir" route_baseline "audio/route-baseline.txt"
  rig_note "audio route: recorded the pre-boot stream baseline at $path"
  return 0
}

rig_audio_client_pid() {
  awk '$1 == "client" { print $2; exit }' "$1/pids" 2>/dev/null || true
}

# A Wine prefix must not be launched into a graph that already holds one of
# its exe streams on the owner's default sink. The baseline snapshot is
# taken before boot, so this check can refuse the launch before the exe
# opens an audio device. A stale stream from another Wine prefix is not
# this run's stream and is left alone.
rig_audio_check_preboot_streams() {
  local out_dir="$1" baseline prefix display test_pids verdict breach
  baseline="$(rig_audio_route_baseline_path "$out_dir")"
  [ -f "$baseline" ] || return 0
  prefix="$(rig_audio_field "$out_dir" client_prefix || true)"
  display="$(rig_audio_field "$out_dir" client_display || true)"
  test_pids="${RIG_AUDIO_TEST_STREAM_PIDS:-}"
  verdict="$(python3 - "$baseline" "$prefix" "$display" "$test_pids" \
    "$RIG_AUDIO_EXE_STREAM_NAME" <<'PYEOF' || true
import os
import sys

baseline_file, prefix, display, test_pids, exe_stream = sys.argv[1:6]
HEADER = "# pactl "


def read_graph(path):
    sections, current = {}, None
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh.read().splitlines():
            if line.startswith(HEADER):
                current = line[len(HEADER):]
                sections[current] = []
            elif current is not None:
                sections[current].append(line)
    return sections


def unquote(value):
    return value.strip().strip('"')


baseline = read_graph(baseline_file)
sink_names = {}
for line in baseline.get("list sinks short", []):
    fields = line.split("\t")
    if len(fields) >= 2:
        sink_names[fields[0]] = fields[1]
default_lines = [line.strip() for line in baseline.get("get-default-sink", []) if line.strip()]
default_sink = default_lines[0] if default_lines else ""

streams, current = [], None
for line in baseline.get("list sink-inputs", []):
    if line.startswith("Sink Input #"):
        current = {"index": line.split("#", 1)[1].strip(),
                   "sink_index": "", "names": [], "pid": ""}
        streams.append(current)
    elif current is None:
        continue
    elif line.startswith("\tSink: "):
        current["sink_index"] = line.split(":", 1)[1].strip()
    elif line.startswith("\t\tapplication.process.id = "):
        current["pid"] = unquote(line.split("=", 1)[1])
    elif line.startswith("\t\tnode.name = ") or line.startswith("\t\tapplication.name = "):
        current["names"].append(unquote(line.split("=", 1)[1]).lower())

allowed = {pid for pid in test_pids.replace(",", " ").split() if pid.isdigit()}


def belongs_to_this_prefix(pid):
    if not pid.isdigit():
        return False
    if pid in allowed:
        return True
    if not prefix:
        return False
    try:
        with open("/proc/%s/environ" % pid, "rb") as fh:
            raw = fh.read()
    except OSError:
        return False
    values = dict(item.decode("utf-8", "replace").split("=", 1)
                  for item in raw.split(b"\0") if b"=" in item)
    if values.get("WINEPREFIX") != prefix:
        return False
    return not display or values.get("DISPLAY") == display


matched = [s for s in streams
           if any(exe_stream.lower() in name for name in s["names"])]
breaches = [s for s in matched
            if s["sink_index"] != "4294967295"
            and sink_names.get(s["sink_index"]) == default_sink
            and belongs_to_this_prefix(s["pid"])]
print("preboot_streams=%d" % len(matched))
print("preboot_breach=%d" % len(breaches))
print("preboot_sink=" + (default_sink or "unknown"))
print("preboot_stream_index=" + (breaches[0]["index"] if breaches else ""))
print("preboot_pid=" + (breaches[0]["pid"] if breaches else ""))
PYEOF
)"
  [ -n "$verdict" ] || return 0
  local key
  for key in preboot_streams preboot_breach preboot_sink preboot_stream_index preboot_pid; do
    rig_audio_set_field "$out_dir" "$key" "$(rig_audio_verdict_field "$verdict" "$key")"
  done
  breach="$(rig_audio_verdict_field "$verdict" preboot_breach)"
  if [ "${breach:-0}" != "0" ]; then
    rig_fail "$RIG_E_AUDIO" pre-boot-stream \
      "exe stream $(rig_audio_verdict_field "$verdict" preboot_stream_index) of process $(rig_audio_verdict_field "$verdict" preboot_pid) already holds the default sink '$(rig_audio_verdict_field "$verdict" preboot_sink)' before boot; agent audio must never reach the owner's speakers"
    return "$RIG_E_AUDIO"
  fi
  return 0
}

# Every bound stream that looks like this run's client must sit on the run's
# sink. A name alias alone is not ownership: another Wine session can own
# an OTwo.exe stream. A stream belongs to this run when its process is a
# descendant of the launched client, or its environment carries this run's
# display, sink, and Wine prefix. Pre-boot streams are never ours. Wine also
# opens a corked, unbound "format test stream" before playback; it is a
# probe, so it cannot satisfy the route guard (observed in the live
# hover319 pack). Prints `key=value` facts, including process ids.
rig_audio_route_verdict() {
  local out_dir="$1" client="$2" sink="$3" exe_stream="$4"
  local snapshot baseline client_pid display prefix test_pids
  snapshot="$(rig_audio_route_snapshot_path "$out_dir")"
  baseline="$(rig_audio_route_baseline_path "$out_dir")"
  rig_audio_write_graph_snapshot "$snapshot"
  [ -f "$baseline" ] || : > "$baseline"
  client_pid="$(rig_audio_client_pid "$out_dir")"
  display="$(rig_audio_field "$out_dir" client_display || true)"
  prefix="$(rig_audio_field "$out_dir" client_prefix || true)"
  test_pids="${RIG_AUDIO_TEST_STREAM_PIDS:-}"
  python3 - "$snapshot" "$baseline" "$client" "$sink" "$exe_stream" \
    "$client_pid" "$display" "$prefix" "$test_pids" <<'PYEOF' || true
import sys

(snapshot_file, baseline_file, client, sink, exe_stream,
 client_pid, display, prefix, test_pids) = sys.argv[1:10]
HEADER = "# pactl "


def read_graph(path):
    sections, current = {}, None
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh.read().splitlines():
            if line.startswith(HEADER):
                current = line[len(HEADER):]
                sections[current] = []
            elif current is not None:
                sections[current].append(line)
    return sections


def unquote(value):
    return value.strip().strip('"')


def parse_streams(lines, sink_names):
    streams, current = [], None
    for line in lines:
        if line.startswith("Sink Input #"):
            current = {
                "index": line.split("#", 1)[1].strip(),
                "sink_index": "", "names": [], "node_names": [],
                "muted": False, "corked": False, "media_name": "",
                "target": "", "pid": "", "binary": "",
            }
            streams.append(current)
        elif current is None:
            continue
        elif line.startswith("\tSink: "):
            current["sink_index"] = line.split(":", 1)[1].strip()
        elif line.startswith("\tMute: "):
            current["muted"] = line.split(":", 1)[1].strip() == "yes"
        elif line.startswith("\tCorked: "):
            current["corked"] = line.split(":", 1)[1].strip() == "yes"
        elif line.startswith("\t\tmedia.name = "):
            current["media_name"] = unquote(line.split("=", 1)[1])
        elif line.startswith("\t\ttarget.object = "):
            current["target"] = unquote(line.split("=", 1)[1])
        elif line.startswith("\t\tapplication.process.id = "):
            current["pid"] = unquote(line.split("=", 1)[1])
        elif line.startswith("\t\tapplication.process.binary = "):
            current["binary"] = unquote(line.split("=", 1)[1])
        elif line.startswith("\t\tnode.name = "):
            value = unquote(line.split("=", 1)[1])
            current["node_names"].append(value)
            current["names"].append(value)
        elif line.startswith("\t\tapplication.name = "):
            current["names"].append(unquote(line.split("=", 1)[1]))
    for stream in streams:
        stream["sink"] = sink_names.get(stream["sink_index"], "unknown")
    return streams


def stream_identity(stream):
    return (stream["pid"], ",".join(stream["node_names"]))


def is_format_probe(stream):
    """A corked, unbound PulseAudio format query is not a playback route."""
    return (stream["sink_index"] == "4294967295" or
            (stream["corked"] and "format test" in stream["media_name"].lower()))


def descendants(root):
    seen, pending = set(), [root] if root.isdigit() else []
    while pending:
        pid = pending.pop()
        if pid in seen:
            continue
        seen.add(pid)
        try:
            path = "/proc/%s/task/%s/children" % (pid, pid)
            with open(path, encoding="ascii") as fh:
                pending.extend(fh.read().split())
        except OSError:
            pass
    return seen


def environment_of(pid):
    try:
        with open("/proc/%s/environ" % pid, "rb") as fh:
            raw = fh.read()
    except OSError:
        return {}
    values = {}
    for item in raw.split(b"\0"):
        if b"=" in item:
            key, value = item.split(b"=", 1)
            values[key.decode("utf-8", "replace")] = value.decode("utf-8", "replace")
    return values


snapshot = read_graph(snapshot_file)
baseline = read_graph(baseline_file)
sink_names = {}
for line in snapshot.get("list sinks short", []):
    fields = line.split("\t")
    if len(fields) >= 2:
        sink_names[fields[0]] = fields[1]
default_lines = [line.strip() for line in snapshot.get("get-default-sink", []) if line.strip()]
default_sink = default_lines[0] if default_lines else ""

streams = parse_streams(snapshot.get("list sink-inputs", []), sink_names)
baseline_streams = parse_streams(baseline.get("list sink-inputs", []), {})
baseline_indices = {s["index"] for s in baseline_streams}
baseline_identities = {stream_identity(s) for s in baseline_streams if s["pid"]}

aliases = [alias.lower() for alias in (client, exe_stream) if alias]
matched = [s for s in streams
           if any(alias in name.lower() for name in s["names"] for alias in aliases)]
leftovers = [s for s in matched
             if s["index"] in baseline_indices
             or (s["pid"] and stream_identity(s) in baseline_identities)]
candidates = [s for s in matched if s not in leftovers]

allowed_pids = {p for p in test_pids.replace(",", " ").split() if p.isdigit()}
allowed_pids.update(descendants(client_pid))


def belongs_to_run(pid, stream=None):
    if not pid.isdigit():
        # PipeWire's ALSA plugin exposes no process id for its sink input.
        # A post-boot stream that names the client and targets THIS run's
        # sink is still contained evidence; an unnamed default-sink stream
        # stays unattributed (and is a breach) because the sink differs.
        return bool(stream and sink and stream["sink"] == sink
                    and stream["target"] == sink)
    if pid in allowed_pids:
        return True
    values = environment_of(pid)
    if not values or not display or values.get("DISPLAY") != display:
        return False
    if sink and values.get("PULSE_SINK") != sink:
        return False
    if prefix and values.get("WINEPREFIX") != prefix:
        return False
    return True


probes = [s for s in candidates if is_format_probe(s)]
playback = [s for s in candidates if s not in probes]
owned_probes = [s for s in probes if belongs_to_run(s["pid"], s)]
owned = [s for s in playback if belongs_to_run(s["pid"], s)]
foreign = [s for s in playback if s not in owned]
unattributed_default = [s for s in foreign
                        if not s["pid"] and default_sink and s["sink"] == default_sink]
on_default = [s for s in owned if default_sink and s["sink"] == default_sink]
on_default += unattributed_default
on_run_sink = [s for s in owned if sink and s["sink"] == sink]

if on_default:
    route = "default-sink-breach"
elif on_run_sink:
    route = "sink"
elif owned:
    route = "foreign-sink"
elif owned_probes:
    route = "probe-only"
else:
    route = "unseen"

print("route=" + route)
print("client_streams=%d" % len(owned))
print("on_default_sink=%d" % len(on_default))
print("default_sink=" + (default_sink or "unknown"))
print("run_sink_streams=" + ",".join(s["index"] for s in on_run_sink))
print("muted_run_sink_streams=%d" % len([s for s in on_run_sink if s["muted"]]))
print("client_stream_pid=" + ",".join(s["pid"] for s in owned))
print("client_stream_binary=" + ",".join(s["binary"] for s in owned))
print("probe_streams=%d" % len(owned_probes))
print("probe_target=" + (owned_probes[0]["target"] if owned_probes else ""))
print("foreign_process_streams=%d" % len(foreign))
print("baseline_streams=%d" % len(baseline_streams))
print("baseline_streams_excluded=%d" % len(leftovers))
print("unattributed_default_streams=%d" % len(unattributed_default))
PYEOF
}

# Writes every verdict fact, including the breach facts, before the abort:
# a failed run must leave an auditable sidecar and a raw graph snapshot.
rig_audio_persist_verdict() {
  local out_dir="$1" verdict="$2" key
  local keys=(route client_streams on_default_sink default_sink run_sink_streams
    muted_run_sink_streams client_stream_pid client_stream_binary
    probe_streams probe_target foreign_process_streams baseline_streams
    baseline_streams_excluded unattributed_default_streams)
  for key in "${keys[@]}"; do
    rig_audio_set_field "$out_dir" "$key" "$(rig_audio_verdict_field "$verdict" "$key")"
  done
  rig_audio_set_field "$out_dir" route_snapshot "audio/route-snapshot.txt"
  rig_audio_set_field "$out_dir" route_baseline "audio/route-baseline.txt"
}

rig_audio_assert_route() {
  local out_dir="$1" client="$2" verdict breaches run_streams muted owners
  [ -f "$(rig_audio_sidecar "$out_dir")" ] || return 0
  [ "$(rig_audio_field "$out_dir" state || true)" = "capturing" ] || return 0
  verdict="$(rig_audio_route_verdict "$out_dir" "$client" \
    "$(rig_audio_field "$out_dir" sink || true)" "$RIG_AUDIO_EXE_STREAM_NAME")"
  if [ -z "$verdict" ]; then
    rig_note "audio route: not verified (pactl could not be parsed)"
    return 0
  fi
  rig_audio_persist_verdict "$out_dir" "$verdict"
  breaches="$(rig_audio_verdict_field "$verdict" on_default_sink)"
  if [ "${breaches:-0}" != "0" ]; then
    owners="$(rig_audio_verdict_field "$verdict" client_stream_pid)"
    rig_fail "$RIG_E_AUDIO" audio-route \
      "$(rig_audio_verdict_field "$verdict" client_streams) stream(s) of client '$client' (process ${owners:-unknown}) on the default sink '$(rig_audio_verdict_field "$verdict" default_sink)'; agent audio must never reach the owner's speakers"
    return "$RIG_E_AUDIO"
  fi
  run_streams="$(rig_audio_verdict_field "$verdict" run_sink_streams)"
  muted="$(rig_audio_verdict_field "$verdict" muted_run_sink_streams)"
  rig_audio_normalize_streams "$out_dir" "$run_streams" "$muted"
  return 0
}

rig_audio_verdict_field() { sed -n "s/^$2=//p" <<<"$1"; }

# A remembered mute or a zero volume leaves the tap digital silence while
# the stream looks healthy (#279: the first native acceptance run captured
# 39 s of -91 dBFS with the client stream muted by stream-restore). The run
# owns these streams, so the lane normalizes them; a stream on any other
# sink is never touched.
rig_audio_normalize_streams() {
  local out_dir="$1" run_streams="$2" muted="$3" index count=0
  [ -n "${run_streams:-}" ] || return 0
  for index in ${run_streams//,/ }; do
    pactl set-sink-input-mute "$index" 0 >/dev/null 2>&1 || true
    pactl set-sink-input-volume "$index" 100% >/dev/null 2>&1 || true
    count=$((count + 1))
    if [ "$index" = "${run_streams%%,*}" ]; then
      rig_note "audio route: normalized client stream $index on the run sink ($(printf '%s' "${muted:-0}") muted of $count)"
    fi
  done
  rig_audio_set_field "$out_dir" streams_normalized "$count"
  rig_audio_set_field "$out_dir" muted_streams "${muted:-0}"
}

# --- probe / verify ---------------------------------------------------------
rig_audio_probe() {
  local wav="$1" floor="${2:-}" measured peak
  measured="$(rig_audio_measure "$wav")" || return 2
  printf '%s\n' "$measured"
  peak="$(printf '%s\n' "$measured" | sed -n 's/^peak_dbfs=//p')"
  if [ -n "$floor" ]; then
    rig_audio_peak_above_floor "$peak" "$floor"
  else
    rig_audio_peak_above_floor "$peak"
  fi
}

rig_audio_verify() {
  local out_dir="$1" sidecar expect state artifact measured peak
  sidecar="$(rig_audio_sidecar "$out_dir")"
  if [ ! -f "$sidecar" ]; then
    echo "rig-audio: FAIL no audio lane captured in $out_dir (run through scripts/rig-run.sh)" >&2
    return 1
  fi
  expect="$(rig_audio_field "$out_dir" expect || true)"
  state="$(rig_audio_field "$out_dir" state || true)"
  artifact="$(rig_audio_artifact "$out_dir")"
  if [ "$state" = "failed" ]; then
    echo "rig-audio: FAIL audio lane failed (expect=$expect, see $sidecar)" >&2
    return 1
  fi
  if [ ! -r "$artifact" ]; then
    echo "rig-audio: FAIL audio artifact missing: $artifact" >&2
    return 1
  fi
  case "$expect" in
    disabled)
      echo "rig-audio: audio lane disabled by the run (opt-out recorded in $sidecar)"
      return 0
      ;;
    silence)
      echo "rig-audio: audio lane declared silent (--no-sound); artifact present: $artifact"
      return 0
      ;;
    sound) ;;
    *)
      echo "rig-audio: FAIL unknown audio declaration '$expect' in $sidecar" >&2
      return 1
      ;;
  esac
  measured="$(rig_audio_measure "$artifact")" || {
    echo "rig-audio: FAIL audio artifact unreadable: $artifact" >&2
    return 1
  }
  peak="$(printf '%s\n' "$measured" | sed -n 's/^peak_dbfs=//p')"
  if ! rig_audio_peak_above_floor "$peak"; then
    echo "rig-audio: FAIL run declared sound but the artifact is silent (peak ${peak} dBFS <= floor $(rig_audio_floor_dbfs) dBFS): $artifact" >&2
    return 1
  fi
  echo "rig-audio: audio OK (peak ${peak} dBFS > floor $(rig_audio_floor_dbfs) dBFS): $artifact"
  return 0
}

# --- hermetic stub tools (selftest support) ---------------------------------
# Writes a `pactl` and a `parec` stub into <bin-dir> that model the parts of
# PipeWire this lane depends on, so both selftests prove the lane without a
# sound server, a game, or the owner's speakers.
rig_audio_stub_tools() {
  local bin="$1" state="$2"
  mkdir -p "$bin" "$state"
  : > "$state/sinks"
  : > "$state/sinkinputs"
  : > "$state/recorders"
  printf '%s' "${STUB_DEFAULT_SINK:-alsa_output.default}" > "$state/default"
  rig_audio_write_stub_pactl "$bin"
  rig_audio_write_stub_parec "$bin"
}

# A stub sound server that models exactly what the lane reads: sinks,
# monitors, source-outputs (recorder attribution), sink-inputs (route and
# mute), and the set-sink-input-* calls the lane makes.
rig_audio_write_stub_pactl() {
  cat > "$1/pactl" <<'STUBEOF'
#!/usr/bin/env python3
"""Stub `pactl` for the rig audio selftest (no real sound server)."""
import os
import sys

state = os.environ["STUBSTATE"]
missing = os.environ.get("STUB_SINK_MISSING") == "1"


def lines(name):
    path = os.path.join(state, name)
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as fh:
        return [line.rstrip("\n") for line in fh if line.strip()]


DEFAULT_SINK_INDEX = "70"


def sinks():
    """[(index, name)]: the system default first, then the stub's rig sinks."""
    known = [(DEFAULT_SINK_INDEX, "alsa_output.default")]
    if not missing:
        known.append(("7", "wine_null"))
    for position, entry in enumerate(lines("sinks")):
        fields = entry.split("\t")
        if len(fields) >= 2:
            known.append((str(99 + position), fields[1]))
    return known


def monitor_index(sink_index):
    return str(int(sink_index) + 1000)


def source_index_for(device):
    """A monitor source for the requested sink; the default monitor if the
    device name is unknown (this is the silent fallback PipeWire performs)."""
    for index, name in sinks():
        if device == name + ".monitor":
            return monitor_index(index)
    return monitor_index(DEFAULT_SINK_INDEX)


def sink_index_for(device):
    for index, name in sinks():
        if device == name:
            return index
    return DEFAULT_SINK_INDEX


def emit_list_sinks_short():
    for index, name in sinks():
        print("%s\t%s\tPipeWire\tfloat32le 2ch 48000Hz\tIDLE" % (index, name))


def emit_list_sources_short():
    for index, name in sinks():
        print("%s\t%s.monitor\tPipeWire\tfloat32le 2ch 48000Hz\tIDLE"
              % (monitor_index(index), name))


def emit_list_source_outputs():
    for entry in lines("recorders"):
        fields = entry.split("\t")
        pid, device = fields[0], fields[1] if len(fields) > 1 else ""
        print("Source Output #%s" % pid)
        print("\tSource: %s" % source_index_for(device))
        print('\t\tapplication.name = "parec"')
        print('\t\tapplication.process.id = "%s"' % pid)
        print('\t\tapplication.process.binary = "pacat"')


def probe_phase():
    """First stub reads are the corked format probe; later reads play."""
    path = os.path.join(state, "probe-calls")
    try:
        with open(path, encoding="ascii") as fh:
            count = int(fh.read().strip() or 0)
    except (OSError, ValueError):
        count = 0
    count += 1
    with open(path, "w", encoding="ascii") as fh:
        fh.write(str(count))
    return count <= 2


def emit_list_sink_inputs():
    seeded = os.environ.get("STUB_SINKINPUT_NAME", "")
    seeded_sink = os.environ.get("STUB_SINKINPUT_SINK", "alsa_output.default")
    seeded_muted = os.environ.get("STUB_SINKINPUT_MUTE") == "1"
    entries = list(lines("sinkinputs"))
    if seeded:
        entries.append("%s\t%s\t%s" % (seeded, seeded_sink, "yes" if seeded_muted else "no"))
    stream_pid = os.environ.get("STUB_SINKINPUT_PID", "4242")
    unbound = os.environ.get("STUB_SINKINPUT_UNBOUND") == "1"
    if os.environ.get("STUB_SINKINPUT_PROBE_TRANSITION") == "1":
        unbound = probe_phase()
    for number, entry in enumerate(entries):
        fields = entry.split("\t")
        node, sink = fields[0], fields[1] if len(fields) > 1 else ""
        muted = fields[2] if len(fields) > 2 else "no"
        print("Sink Input #%d" % (900 + number))
        print("\tSink: %s" % ("4294967295" if unbound else sink_index_for(sink)))
        print("\tCorked: %s" % ("yes" if unbound else "no"))
        print("\tMute: %s" % muted)
        print("\tVolume: front-left: 65536 / 100% / 0.00 dB")
        print("\tProperties:")
        print('\t\tapplication.name = "PipeWire ALSA [%s]"' % node)
        print('\t\tnode.name = "alsa_playback.%s"' % node)
        print('\t\tmedia.name = "%s"' %
              ("format test stream" if unbound else "Playback"))
        print('\t\ttarget.object = "%s"' %
              os.environ.get("STUB_SINKINPUT_TARGET", sink))
        if unbound:
            print('\t\tpulse.corked = "true"')
        if stream_pid:
            print('\t\tapplication.process.id = "%s"' % stream_pid)
            print('\t\tapplication.process.binary = "wine-preloader"')


def load_module(arguments):
    name = ""
    for argument in arguments:
        if argument.startswith("sink_name="):
            name = argument.split("=", 1)[1]
    with open(os.path.join(state, "sinks"), "a", encoding="utf-8") as fh:
        fh.write("99\t%s\n" % name)
    print("99")


def unload_module(module):
    if os.environ.get("STUB_UNLOAD_FAIL") == "1":
        sys.exit(1)
    open(os.path.join(state, "unloaded-%s" % module), "w").close()


def main():
    args = sys.argv[1:]
    if not args:
        return 1
    if args[:2] == ["list", "sinks"]:
        emit_list_sinks_short()
    elif args[:2] == ["list", "sources"]:
        emit_list_sources_short()
    elif args[:2] == ["list", "source-outputs"]:
        emit_list_source_outputs()
    elif args[:2] == ["list", "sink-inputs"]:
        emit_list_sink_inputs()
    elif args[0] == "load-module":
        load_module(args[2:])
    elif args[0] == "unload-module":
        unload_module(args[1])
    elif args[0] in ("set-sink-input-mute", "set-sink-input-volume"):
        with open(os.path.join(state, "stream-ops"), "a", encoding="utf-8") as fh:
            fh.write("%s %s %s\n" % (args[0], args[1], args[2]))
    elif args[0] == "get-default-sink":
        with open(os.path.join(state, "default"), encoding="utf-8") as fh:
            print(fh.read().strip())
    return 0


sys.exit(main())
STUBEOF
  chmod +x "$1/pactl"
}

# A stub recorder: registers itself for the requested device, writes a wav
# (tone unless STUB_AUDIO_SILENT=1), then stays briefly so the lane can
# verify and stop it. The bound keeps an interrupted selftest from leaving
# a `sleep` orphan; a real run's recorder is stopped by process group.
rig_audio_write_stub_parec() {
  cat > "$1/parec" <<'STUBEOF'
#!/bin/sh
# Stub `parec`: registers itself as a recorder of the requested device and
# writes a wav (tone unless STUB_AUDIO_SILENT=1), then stays alive so the
# lane sees a live recorder until capture-stop signals it.
out=""
device=""
previous=""
for argument in "$@"; do
  case "$previous" in
    -d) device="$argument" ;;
  esac
  case "$argument" in
    --*) ;;
    *) out="$argument" ;;
  esac
  previous="$argument"
done
if [ "${STUB_RECORDER_FALLBACK:-0}" = "1" ]; then
  device="alsa_output.default"
fi
printf '%s\t%s\n' "$$" "$device" >> "$STUBSTATE/recorders"
if [ -n "$out" ]; then
  if [ "${STUB_AUDIO_SILENT:-0}" = "1" ]; then
    source="anullsrc=r=48000:cl=stereo"
  else
    source="sine=frequency=440:sample_rate=48000"
  fi
  ffmpeg -hide_banner -loglevel error -f lavfi -i "$source" -t 3 -ac 2 "$out" || :
fi
exec sleep 5
STUBEOF
  chmod +x "$1/parec"
}

# --- selftest ---------------------------------------------------------------
selftest_fail() {
  echo "rig-audio: selftest leg $1 FAIL: $2" >&2
  return 1
}

selftest_wavs() {
  local dir="$1"
  mkdir -p "$dir"
  ffmpeg -hide_banner -loglevel error -f lavfi -i "sine=frequency=440:sample_rate=48000" -t 2 -ac 2 -y "$dir/tone.wav"
  ffmpeg -hide_banner -loglevel error -f lavfi -i "anullsrc=r=48000:cl=stereo" -t 2 -y "$dir/silent.wav"
}

# A silent file must never pass as captured audio (the ticket's core rule:
# a successful recording process alone proves nothing).
selftest_probe_legs() {
  local wavs="$1" out
  if rig_audio_probe "$wavs/silent.wav" >/dev/null 2>&1; then
    selftest_fail "probe-silence" "a silent wav passed the loudness probe"
    return 1
  fi
  echo "rig-audio: selftest leg probe-silence PASS (silent wav rejected)"
  if ! out="$(rig_audio_probe "$wavs/tone.wav" 2>&1)"; then
    selftest_fail "probe-tone" "a tone wav failed the loudness probe: $out"
    return 1
  fi
  grep -q "^peak_dbfs=-" <<<"$out" || {
    selftest_fail "probe-tone" "probe printed no peak: $out"
    return 1
  }
  echo "rig-audio: selftest leg probe-tone PASS (tone accepted, $(sed -n 's/^peak_dbfs=//p' <<<"$out") dBFS)"
}

selftest_verify_legs() {
  local wavs="$1" run="$2" leg
  # sound declared + tone captured -> pass
  rig_audio_stub_sidecar "$run/audio" sound captured "$wavs/tone.wav"
  rig_audio_verify "$run" >/dev/null || {
    selftest_fail "verify-sound-tone" "a run with a tone artifact was rejected"
    return 1
  }
  echo "rig-audio: selftest leg verify-sound-tone PASS"
  # sound declared + silent artifact -> fail
  rig_audio_stub_sidecar "$run/audio" sound captured "$wavs/silent.wav"
  if rig_audio_verify "$run" >/dev/null 2>&1; then
    selftest_fail "verify-sound-silent" "a silent artifact passed verify although the run declared sound"
    return 1
  fi
  echo "rig-audio: selftest leg verify-sound-silent PASS (silent artifact rejected)"
  # silence declared (--no-sound run) -> artifact required, level irrelevant
  rig_audio_stub_sidecar "$run/audio" silence captured "$wavs/silent.wav"
  rig_audio_verify "$run" >/dev/null || {
    selftest_fail "verify-declared-silence" "a --no-sound run with a silent artifact was rejected"
    return 1
  }
  echo "rig-audio: selftest leg verify-declared-silence PASS"
  # missing artifact -> fail
  rig_audio_stub_sidecar "$run/audio" sound captured "$wavs/silent.wav"
  : > "$run/audio/audio.wav"
  sed -i '/^state=/s/.*/state=failed/' "$run/audio/manifest"
  if rig_audio_verify "$run" >/dev/null 2>&1; then
    selftest_fail "verify-failed-lane" "a failed lane passed verify"
    return 1
  fi
  echo "rig-audio: selftest leg verify-failed-lane PASS (failed lane rejected)"
  # no lane at all -> fail
  leg="$(mktemp -d)"
  if rig_audio_verify "$leg" >/dev/null 2>&1; then
    selftest_fail "verify-missing-lane" "a run with no audio lane passed verify"
    rm -rf "$leg"
    return 1
  fi
  rm -rf "$leg"
  echo "rig-audio: selftest leg verify-missing-lane PASS (uncaptured run rejected)"
}

rig_audio_stub_sidecar() {
  local dir="$1" expect="$2" state="$3" source_wav="$4"
  mkdir -p "$dir"
  cp -f "$source_wav" "$dir/audio.wav"
  {
    echo "lane=audio"
    echo "expect=$expect"
    echo "state=$state"
    echo "sink=stub_sink"
    echo "source=stub_sink.monitor"
    echo "client=stub-client"
    echo "artifact=audio/audio.wav"
  } > "$dir/manifest"
}

selftest_capture_legs() {
  local bin="$1" state="$2" run="$3" red="$4"
  env PATH="$bin:$PATH" STUBSTATE="$state" \
    bash "$0" capture-start "$run" stub_sink sound stub-client >/dev/null || {
    selftest_fail "capture-start" "capture-start refused a present sink"
    return 1
  }
  [ "$(rig_audio_field "$run" source_verified)" = "yes" ] || {
    selftest_fail "capture-start" "recorder source was not verified"
    return 1
  }
  rig_audio_capture_stop "$run" || {
    selftest_fail "capture-stop" "capture-stop failed on a recorded artifact"
    return 1
  }
  [ "$(rig_audio_field "$run" state)" = "captured" ] || {
    selftest_fail "capture-stop" "lane state is not captured"
    return 1
  }
  rig_audio_verify "$run" >/dev/null || {
    selftest_fail "capture-verify" "verify rejected a captured tone"
    return 1
  }
  echo "rig-audio: selftest leg capture-roundtrip PASS (source verified, artifact captured, verify OK)"
  # the silent-fallback guard: PipeWire swaps in the default source
  if env PATH="$bin:$PATH" STUBSTATE="$state" STUB_RECORDER_FALLBACK=1 \
    bash "$0" capture-start "$red" stub_sink sound stub-client >/dev/null 2>&1; then
    selftest_fail "capture-source-mismatch" "capture-start accepted a recorder on the wrong source"
    return 1
  fi
  [ "$(rig_audio_field "$red" state)" = "failed" ] || {
    selftest_fail "capture-source-mismatch" "lane state is not failed after a source mismatch"
    return 1
  }
  if [ -f "$(rig_audio_recorder_pidfile "$red")" ]; then
    selftest_fail "capture-source-mismatch" "a recorder survived a rejected capture"
    return 1
  fi
  echo "rig-audio: selftest leg capture-source-mismatch PASS (wrong source rejected, recorder reaped)"
}

# Asserts the exit code, route verdict and stream bookkeeping of one stub graph.
route_leg() {  # <leg> <state> <run> <want-route> <want-muted> <want-code>
  local leg="$1" state="$2" run="$3" want_route="$4" want_muted="$5" want_code="$6"; shift 6
  rig_audio_set_field "$run" state "capturing"
  local field
  for field in client_stream_pid client_stream_binary probe_streams probe_target foreign_process_streams baseline_streams_excluded; do
    rig_audio_set_field "$run" "$field" ""
  done
  rm -f "$state/stream-ops"
  local code=0
  env "$@" bash -c ". '$RIG_AUDIO_SCRIPT'; rig_audio_assert_route '$run' stub-client" >/dev/null 2>&1 || code=$?
  if [ "$code" -ne "$want_code" ]; then
    selftest_fail "$leg" "assert-route exited $code, want $want_code"
    return 1
  fi
  local route
  route="$(rig_audio_field "$run" route)"
  if [ "$route" != "$want_route" ]; then
    selftest_fail "$leg" "route is '$route', want '$want_route'"
    return 1
  fi
  if [ -n "$want_muted" ] && [ "$(rig_audio_field "$run" muted_streams)" != "$want_muted" ]; then
    selftest_fail "$leg" "muted_streams is '$(rig_audio_field "$run" muted_streams)', want '$want_muted'"
    return 1
  fi
  return 0
}

selftest_route_legs() {
  local bin="$1" state="$2" run="$3"
  # The stub stream's process id is not a child of this shell. Name it as
  # this run's process through the stub seam; the /proc checks are excluded
  # from the hermetic leg on purpose.
  local env_common=(PATH="$bin:$PATH" STUBSTATE="$state"
    STUB_SINKINPUT_PID=4242 RIG_AUDIO_TEST_STREAM_PIDS=4242)
  # A client stream on the run sink: contained and normalized (an unmuted
  # stream still gets mute 0 / volume 100%, which is harmless on our sink).
  route_leg "route-run-sink" "$state" "$run" "sink" "0" 0 \
    "${env_common[@]}" STUB_SINKINPUT_NAME=stub-client STUB_SINKINPUT_SINK=stub_sink || return 1
  [ "$(rig_audio_field "$run" streams_normalized)" = "1" ] || {
    selftest_fail "route-run-sink" "streams_normalized is '$(rig_audio_field "$run" streams_normalized)', want 1"
    return 1
  }
  echo "rig-audio: selftest leg route-run-sink PASS (contained, normalized)"
  # A remembered mute leaves the tap silent: the lane must count and clear it.
  route_leg "route-muted-stream" "$state" "$run" "sink" "1" 0 \
    "${env_common[@]}" STUB_SINKINPUT_NAME=stub-client STUB_SINKINPUT_SINK=stub_sink \
    STUB_SINKINPUT_MUTE=1 || return 1
  grep -q "^set-sink-input-mute 900 0$" "$state/stream-ops" || {
    selftest_fail "route-muted-stream" "a muted run stream was left muted"
    return 1
  }
  echo "rig-audio: selftest leg route-muted-stream PASS (mute counted and cleared)"
  # A stream on a foreign sink is never touched: only this run's sink is ours.
  route_leg "route-foreign-sink" "$state" "$run" "foreign-sink" "" 0 \
    "${env_common[@]}" STUB_SINKINPUT_NAME=stub-client STUB_SINKINPUT_SINK=wine_null || return 1
  if [ -s "$state/stream-ops" ]; then
    selftest_fail "route-foreign-sink" "the lane touched a foreign stream ($(cat "$state/stream-ops"))"
    return 1
  fi
  echo "rig-audio: selftest leg route-foreign-sink PASS (foreign stream untouched)"
  # A client stream on the system default sink is a breach.
  route_leg "route-default-sink" "$state" "$run" "default-sink-breach" "" "$RIG_E_AUDIO" \
    "${env_common[@]}" STUB_SINKINPUT_NAME=stub-client \
    STUB_SINKINPUT_SINK=alsa_output.default || return 1
  echo "rig-audio: selftest leg route-default-sink PASS (breach rejected)"
  # Wine's stream is named after the exe, not the launcher basename (#311).
  route_leg "wine-exe-run-sink" "$state" "$run" "sink" "0" 0 \
    "${env_common[@]}" STUB_SINKINPUT_NAME=OTwo.exe STUB_SINKINPUT_SINK=stub_sink || return 1
  echo "rig-audio: selftest leg wine-exe-run-sink PASS (OTwo.exe on run sink recognized)"
  route_leg "wine-exe-default-sink" "$state" "$run" "default-sink-breach" "" "$RIG_E_AUDIO" \
    "${env_common[@]}" STUB_SINKINPUT_NAME=OTwo.exe \
    STUB_SINKINPUT_SINK=alsa_output.default || return 1
  echo "rig-audio: selftest leg wine-exe-default-sink PASS (OTwo.exe on default sink rejected)"
  # The live hover319 graph: a corked, unbound format query aimed at the
  # owner's sink. It must stay provisional and never count as a route.
  route_leg "wine-exe-unbound-format-probe" "$state" "$run" "probe-only" "" 0 \
    "${env_common[@]}" STUB_SINKINPUT_NAME=OTwo.exe \
    STUB_SINKINPUT_SINK=alsa_output.default STUB_SINKINPUT_UNBOUND=1 \
    STUB_SINKINPUT_TARGET=spotifast.soloist.output || return 1
  [ "$(rig_audio_field "$run" probe_streams)" = "1" ] || {
    selftest_fail "wine-exe-unbound-format-probe" "probe stream was not counted"
    return 1
  }
  [ -z "$(rig_audio_field "$run" run_sink_streams)" ] || {
    selftest_fail "wine-exe-unbound-format-probe" "probe stream was written to run_sink_streams"
    return 1
  }
  echo "rig-audio: selftest leg wine-exe-unbound-format-probe PASS (probe-only, no bound route)"
  # A leftover exe stream from ANOTHER process is not this run's client.
  route_leg "wine-exe-foreign-process-id" "$state" "$run" "unseen" "" 0 \
    "${env_common[@]}" STUB_SINKINPUT_NAME=OTwo.exe STUB_SINKINPUT_SINK=stub_sink \
    STUB_SINKINPUT_PID=7777 || return 1
  [ "$(rig_audio_field "$run" foreign_process_streams)" = "1" ] || {
    selftest_fail "wine-exe-foreign-process-id" "foreign stream was not counted separately"
    return 1
  }
  [ -z "$(rig_audio_field "$run" client_stream_pid)" ] || {
    selftest_fail "wine-exe-foreign-process-id" "foreign pid was recorded as this run's client"
    return 1
  }
  echo "rig-audio: selftest leg wine-exe-foreign-process-id PASS (leftover owner not attributed)"
  # The same leftover on the owner's sink must not abort an innocent run.
  route_leg "wine-exe-foreign-process-id-default-sink" "$state" "$run" "unseen" "" 0 \
    "${env_common[@]}" STUB_SINKINPUT_NAME=OTwo.exe \
    STUB_SINKINPUT_SINK=alsa_output.default STUB_SINKINPUT_PID=7777 || return 1
  echo "rig-audio: selftest leg wine-exe-foreign-process-id-default-sink PASS (leftover does not abort)"
  # An exe stream with no process id cannot be attributed; fail safe on the
  # owner's sink rather than pretend it belongs to another process.
  route_leg "wine-exe-unknown-pid-default-sink" "$state" "$run" "default-sink-breach" "" "$RIG_E_AUDIO" \
    "${env_common[@]}" STUB_SINKINPUT_NAME=OTwo.exe \
    STUB_SINKINPUT_SINK=alsa_output.default STUB_SINKINPUT_PID= || return 1
  echo "rig-audio: selftest leg wine-exe-unknown-pid-default-sink PASS (unattributed stream rejected)"
  # Native ALSA clients reach PipeWire without a process id on the sink
  # input; a post-boot stream aimed at the run sink is still our route.
  route_leg "native-alsa-no-pid-on-run-sink" "$state" "$run" "sink" "0" 0 \
    "${env_common[@]}" STUB_SINKINPUT_NAME=stub-client \
    STUB_SINKINPUT_SINK=stub_sink STUB_SINKINPUT_PID= || return 1
  [ "$(rig_audio_field "$run" on_default_sink)" = "0" ] || {
    selftest_fail "native-alsa-no-pid-on-run-sink" "no-pid stream reached the default sink"
    return 1
  }
  echo "rig-audio: selftest leg native-alsa-no-pid-on-run-sink PASS (contained route)"
  # A stream already present before boot is a leftover, whatever its pid.
  env "${env_common[@]}" STUB_SINKINPUT_NAME=OTwo.exe \
    STUB_SINKINPUT_SINK=alsa_output.default \
    bash "$0" record-baseline "$run" >/dev/null || {
    selftest_fail "wine-exe-baseline-stream-excluded" "record-baseline failed"
    return 1
  }
  route_leg "wine-exe-baseline-stream-excluded" "$state" "$run" "unseen" "" 0 \
    "${env_common[@]}" STUB_SINKINPUT_NAME=OTwo.exe \
    STUB_SINKINPUT_SINK=alsa_output.default || return 1
  [ "$(rig_audio_field "$run" baseline_streams_excluded)" = "1" ] || {
    selftest_fail "wine-exe-baseline-stream-excluded" "pre-boot stream was not excluded"
    return 1
  }
  echo "rig-audio: selftest leg wine-exe-baseline-stream-excluded PASS (pre-boot stream is not ours)"
}

cmd_selftest() {
  local tools
  for tools in ffmpeg ffprobe python3; do
    command -v "$tools" >/dev/null 2>&1 || die "selftest: '$tools' is required"
  done
  local tmp; tmp="$(mktemp -d)"
  trap "rm -rf '$tmp'" EXIT
  selftest_wavs "$tmp/wavs"
  rig_audio_stub_tools "$tmp/bin" "$tmp/state"
  # the stub sink exists: capture legs ask the stub pactl whether it is there
  printf '99\tstub_sink\n' > "$tmp/state/sinks"
  mkdir -p "$tmp/run" "$tmp/red"
  selftest_probe_legs "$tmp/wavs"
  selftest_verify_legs "$tmp/wavs" "$tmp/verify"
  selftest_capture_legs "$tmp/bin" "$tmp/state" "$tmp/run" "$tmp/red"
  selftest_route_legs "$tmp/bin" "$tmp/state" "$tmp/run"
  echo "rig-audio: selftest OK"
}

main() {
  local cmd="${1:-}"; shift || true
  case "$cmd" in
    capture-start) [ "$#" -ge 4 ] || die "capture-start: <out-dir> <sink> <expect> <client>"; rig_audio_capture_start "$@" ;;
    capture-stop) [ "$#" -ge 1 ] || die "capture-stop: <out-dir>"; rig_audio_capture_stop "$1" ;;
    assert-route) [ "$#" -ge 2 ] || die "assert-route: <out-dir> <client>"; rig_audio_assert_route "$@" ;;
    record-baseline) [ "$#" -ge 1 ] || die "record-baseline: <out-dir>"; rig_audio_record_route_baseline "$1" ;;
    probe) [ "$#" -ge 1 ] || die "probe: <wav> [floor-dbfs]"; rig_audio_probe "$@" ;;
    verify) [ "$#" -ge 1 ] || die "verify: <out-dir>"; rig_audio_verify "$1" ;;
    stub-tools) [ "$#" -ge 2 ] || die "stub-tools: <bin-dir> <state-dir>"; rig_audio_stub_tools "$@" ;;
    selftest) cmd_selftest ;;
    *) die "usage: rig-audio.sh {capture-start|capture-stop|assert-route|probe|verify|stub-tools|selftest} ..." ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main "$@"
fi
