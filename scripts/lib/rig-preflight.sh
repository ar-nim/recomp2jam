#!/usr/bin/env bash
# rig-preflight.sh — shared preflight + resource ownership for rig runs (#206).
#
# Sourced by scripts/rig-run.sh and scripts/play-local.sh. The batch-harness
# skill demands this checklist be ENFORCED, not documented:
#   1. displays: no conflicting Wine/Xvfb claim (locks; rig-run.sh owns this);
#   2. ports: relay ports free, declared listen ports free + claimed;
#   3. audio: virtual/null sink present and selected — NEVER the speakers;
#   4. server: Memoryer up and answering (TCP connect probe);
#   5. data: install-root readable.
#
# Exit codes are distinct per rejection so concurrent-run collisions and
# malformed runs are distinguishable in batch summaries:
#   2  usage / malformed invocation (historical `die` code, kept)
#   10 display conflict (foreign/caller/:98 display, or already claimed)
#   11 port collision (relay port in use, listen port taken or claimed)
#   12 audio sink missing or misrouted (would reach the speakers)
#   13 Memoryer down or not answering
#   14 install-root unreadable
#   15 Wine prefix conflict (malformed path or already claimed)
#   16 cleanup failure (something owned survived `stop`; evidence is kept)
#
# Knobs (all env, all optional):
#   RIG_STATE_DIR       lock root (default $HOME/scratch/rig-state, else /tmp)
#   RIG_INSTALL_ROOT    game-data root (default <repo>/install-root/O2Jam)
#   RIG_MEMORYER_HOST   default 127.0.0.1
#   RIG_MEMORYER_PORT   default 15010
#   RIG_RELAY_PORTS     ports that must be FREE (default "39311 39312":
#                       local Memoryer runs with relay disabled)
#   RIG_LISTEN_PORTS    ports this run will bind; verified free, then claimed
#   RIG_EXPECT_MEMORYER 1 = require Memoryer up (default); 0 = skip
#   RIG_WINEPREFIX / WINEPREFIX  claimed when set
#   RIG_SINK_NAME       null-sink name (default wine_null); explicit names are
#                       per-run owned (unloaded on stop), the default is
#                       shared (left loaded, never unloaded under others)
#   RIG_CREATE_SINK     1 = create a missing null sink via pactl (default);
#                       0 = reject when the sink is absent
#
# Doubt log (flagged for the senior, not silently resolved):
# - Memoryer "answering" is a TCP connect probe only; no protocol ping exists
#   that would not disturb the server. A squatter accepting TCP would pass.
# - Without pactl/pipewire the sink check degrades to name-only (recorded).
# - The system default sink is only warned about, never changed: Wine honors
#   PULSE_SINK, and flipping the owner's default sink is out of scope.

# --- exit codes ---------------------------------------------------------------
RIG_E_USAGE=2
RIG_E_DISPLAY=10
RIG_E_PORT=11
RIG_E_AUDIO=12
RIG_E_SERVER=13
RIG_E_DATA=14
RIG_E_PREFIX=15
RIG_E_CLEANUP=16

rig_fail() {
  local code="$1" check="$2" msg="$3"
  echo "rig-preflight: FAIL $check: $msg (exit $code)" >&2
  return "$code"
}

rig_note() { echo "rig-preflight: $1" >&2; }

# --- lock ownership -------------------------------------------------------------
# Each claim is a lockdir holding "pid=<pid> out=<out-dir>". mkdir is atomic:
# two concurrent runs cannot claim the same display/port/prefix. A lock whose
# owner is dead is stale and reclaimed with a note; a live one rejects.

rig_state_dir() {
  local dir="${RIG_STATE_DIR:-}"
  if [ -z "$dir" ]; then
    if [ -n "${HOME:-}" ]; then dir="$HOME/scratch/rig-state"; else dir="/tmp/recomp2jam-rig"; fi
  fi
  mkdir -p "$dir/locks" || return 1
  printf '%s' "$dir"
}

rig_lock_path() {
  local state="$1" kind="$2" name="$3"
  local safe
  safe="$(printf '%s' "$kind-$name" | tr -c 'A-Za-z0-9_.-' '_')"
  printf '%s/locks/%s' "$state" "$safe"
}

rig_claim() {
  local kind="$1" name="$2" out_dir="$3" ecode="$4"
  local state lock owner_pid owner_out
  state="$(rig_state_dir)" || return "$RIG_E_USAGE"
  lock="$(rig_lock_path "$state" "$kind" "$name")"
  if mkdir "$lock" 2>/dev/null; then
    echo "pid=$$ out=$out_dir" > "$lock/owner"
    echo "$kind $name" >> "$out_dir/claims"
    return 0
  fi
  if rig_owner_live "$lock"; then
    rig_fail "$ecode" "$kind-claim" "$kind '$name' already claimed by $(cat "$lock/owner" 2>/dev/null)"
    return "$ecode"
  fi
  rig_note "reclaiming stale $kind lock '$name' (owner dead or processes gone)"
  rm -rf "$lock"
  mkdir "$lock" 2>/dev/null || { rig_fail "$ecode" "$kind-claim" "cannot claim $kind '$name'"; return "$ecode"; }
  echo "pid=$$ out=$out_dir" > "$lock/owner"
  echo "$kind $name" >> "$out_dir/claims"
  return 0
}

rig_owner_live() {
  local lock="$1" owner pid out tag p frag
  owner="$(cat "$lock/owner" 2>/dev/null || true)"
  pid="$(printf '%s' "$owner" | sed 's/.*pid=\([0-9]*\).*/\1/')"
  out="$(printf '%s' "$owner" | sed 's/.*out=\([^ ]*\).*/\1/')"
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    if tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null | grep -q "rig-run"; then
      return 0
    fi
  fi
  if [ -n "$out" ] && [ -f "$out/pids" ]; then
    while read -r tag p frag; do
      [ -n "${p:-}" ] || continue
      if [ -r "/proc/$p/cmdline" ] && tr '\0' ' ' < "/proc/$p/cmdline" | grep -qF "$frag"; then
        return 0
      fi
    done < "$out/pids"
  fi
  return 1
}

rig_claimed_by() {
  local kind="$1" name="$2" state lock
  state="$(rig_state_dir)" || return 1
  lock="$(rig_lock_path "$state" "$kind" "$name")"
  [ -d "$lock" ] || return 1
  if rig_owner_live "$lock"; then
    cat "$lock/owner" 2>/dev/null || echo "pid=? out=unknown"
    return 0
  fi
  return 1
}

rig_release_claims() {
  local out_dir="$1" state kind name lock owner_out
  [ -f "$out_dir/claims" ] || return 0
  state="$(rig_state_dir)" || return 1
  while read -r kind name; do
    [ -n "${kind:-}" ] || continue
    case "$kind" in sinkmod) continue ;; esac
    lock="$(rig_lock_path "$state" "$kind" "$name")"
    owner_out="$(sed 's/.*out=\([^ ]*\).*/\1/' "$lock/owner" 2>/dev/null || true)"
    if [ "$owner_out" = "$out_dir" ]; then rm -rf "$lock"; fi
  done < "$out_dir/claims"
  return 0
}

# --- install-root -----------------------------------------------------------------
rig_check_install_root() {
  local root="${1:-${RIG_INSTALL_ROOT:-}}"
  if [ -z "$root" ]; then
    rig_fail "$RIG_E_DATA" "install-root" "no install-root path (set RIG_INSTALL_ROOT)"
    return "$RIG_E_DATA"
  fi
  if [ ! -d "$root" ] || [ ! -r "$root" ] || [ ! -x "$root" ]; then
    rig_fail "$RIG_E_DATA" "install-root" "$root unreadable — link the game data first (ln -s /path/to/install-root <repo>/install-root)"
    return "$RIG_E_DATA"
  fi
  return 0
}

# --- Memoryer liveness --------------------------------------------------------------
rig_check_memoryer() {
  local host="${1:-${RIG_MEMORYER_HOST:-127.0.0.1}}"
  local port="${2:-${RIG_MEMORYER_PORT:-15010}}"
  if [ "${RIG_EXPECT_MEMORYER:-1}" != "1" ]; then
    rig_note "Memoryer check skipped (RIG_EXPECT_MEMORYER=0)"
    return 0
  fi
  if ! python3 - "$host" "$port" <<'PYEOF' 2>/dev/null; then
import socket, sys
host, port = sys.argv[1], int(sys.argv[2])
sock = socket.create_connection((host, port), timeout=3)
sock.close()
PYEOF
    rig_fail "$RIG_E_SERVER" "memoryer" "Memoryer not answering at $host:$port (refused or timed out)"
    return "$RIG_E_SERVER"
  fi
  return 0
}

# --- ports --------------------------------------------------------------------------
rig_port_in_use() {
  local port="$1"
  python3 - "$port" <<'PYEOF' 2>/dev/null
import socket, sys
port = int(sys.argv[1])
try:
    probe = socket.create_connection(("127.0.0.1", port), timeout=1)
    probe.close()
    sys.exit(0)
except OSError:
    pass
for family, socktype in ((socket.AF_INET, socket.SOCK_STREAM),
                         (socket.AF_INET, socket.SOCK_DGRAM)):
    try:
        sock = socket.socket(family, socktype)
        sock.bind(("127.0.0.1", port))
        sock.close()
    except OSError:
        sys.exit(0)
sys.exit(1)
PYEOF
}

rig_check_relay_ports_free() {
  local ports="${RIG_RELAY_PORTS:-39311 39312}" port
  for port in $ports; do
    if rig_port_in_use "$port"; then
      # One retry rides out a sub-second drive-by occupant (observed live on
      # this box); a persistent squatter still fails the second probe.
      sleep 0.5
      if rig_port_in_use "$port"; then
        rig_fail "$RIG_E_PORT" "relay-ports" "relay port $port already in use (local Memoryer runs with relay disabled; a squatter or second run owns it)"
        return "$RIG_E_PORT"
      fi
      rig_note "relay port $port transiently busy; second probe free"
    fi
  done
  return 0
}

rig_claim_listen_ports() {
  local out_dir="$1" ports="${RIG_LISTEN_PORTS:-}" port
  for port in $ports; do
    if rig_port_in_use "$port"; then
      rig_fail "$RIG_E_PORT" "listen-ports" "listen port $port already in use"
      return "$RIG_E_PORT"
    fi
    rig_claim "port" "$port" "$out_dir" "$RIG_E_PORT" || return "$RIG_E_PORT"
  done
  return 0
}

# --- audio sink -----------------------------------------------------------------------
rig_sink_present() {
  local sink="$1"
  command -v pactl >/dev/null 2>&1 || return 2
  pactl list sinks short 2>/dev/null | awk '{print $2}' | grep -qxF "$sink"
}

rig_check_audio_sink() {
  local policy="$1" sink="${2:-}"
  if [ -z "$sink" ]; then
    if [ "$policy" = "null" ]; then sink="${RIG_SINK_NAME:-${PULSE_SINK:-wine_null}}";
    else sink="${PULSE_SINK:-}"; fi
  fi
  if [ -z "$sink" ]; then
    if [ "$policy" = "attended" ] && [ -t 0 ]; then
      rig_note "audio routes to the system default sink on an attended terminal; pass --no-sound for a contained rig run"
      return 0
    fi
    rig_fail "$RIG_E_AUDIO" "audio-sink" "scripted run would reach the speakers (no null sink selected)"
    return "$RIG_E_AUDIO"
  fi
  local lowered
  lowered="$(printf '%s' "$sink" | tr 'A-Z' 'a-z')"
  case "$lowered" in
    *null*|*virtual*|*dummy*) ;;
    *)
      if [ "$policy" = "attended" ] && [ -t 0 ]; then
        rig_note "attended run on '$sink' (explicit non-null sink, human present)"
        return 0
      fi
      rig_fail "$RIG_E_AUDIO" "audio-sink" "'$sink' is not a null/virtual sink; automated audio must never reach the speakers"
      return "$RIG_E_AUDIO"
      ;;
  esac
  if ! command -v pactl >/dev/null 2>&1; then
    rig_note "pactl absent: sink '$sink' verified by name only (source: no pipewire/pulse tooling)"
    return 0
  fi
  if rig_sink_present "$sink"; then
    return 0
  fi
  if [ "${RIG_CREATE_SINK:-1}" != "1" ]; then
    rig_fail "$RIG_E_AUDIO" "audio-sink" "null sink '$sink' absent (creation disabled via RIG_CREATE_SINK=0)"
    return "$RIG_E_AUDIO"
  fi
  local module
  module="$(pactl load-module module-null-sink "sink_name=$sink" 2>/dev/null)" || module=""
  if [ -z "$module" ] || ! rig_sink_present "$sink"; then
    rig_fail "$RIG_E_AUDIO" "audio-sink" "null sink '$sink' absent and creation failed"
    return "$RIG_E_AUDIO"
  fi
  if [ -n "${RIG_SINK_NAME:-}" ]; then
    echo "sinkmod $module $sink" >> "${RIG_CLAIMS_FILE:-/dev/null}"
    rig_note "created owned null sink '$sink' (module $module; unloaded on stop)"
  else
    rig_note "created shared null sink '$sink' (module $module; left loaded)"
  fi
  return 0
}

rig_release_sink() {
  local out_dir="$1" failures="$2" tag module sink
  [ -f "$out_dir/claims" ] || return 0
  while read -r tag module sink; do
    [ "$tag" = "sinkmod" ] || continue
    if pactl unload-module "$module" 2>/dev/null; then
      rig_note "unloaded owned null sink '$sink' (module $module)"
    else
      echo "owned sink '$sink' (module $module) failed to unload" >> "$failures"
    fi
  done < "$out_dir/claims"
  return 0
}

# --- Wine prefix ----------------------------------------------------------------------
rig_check_prefix() {
  local out_dir="$1" prefix="${2:-${RIG_WINEPREFIX:-${WINEPREFIX:-}}}"
  if [ -z "$prefix" ]; then
    rig_note "no Wine prefix set (WINEPREFIX/RIG_WINEPREFIX empty); nothing to claim"
    return 0
  fi
  if [ -e "$prefix" ] && [ ! -d "$prefix" ]; then
    rig_fail "$RIG_E_PREFIX" "wine-prefix" "'$prefix' exists and is not a directory"
    return "$RIG_E_PREFIX"
  fi
  mkdir -p "$prefix" 2>/dev/null || {
    rig_fail "$RIG_E_PREFIX" "wine-prefix" "cannot create prefix dir '$prefix'"
    return "$RIG_E_PREFIX"
  }
  rig_claim "prefix" "$prefix" "$out_dir" "$RIG_E_PREFIX"
}
