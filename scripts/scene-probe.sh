#!/usr/bin/env bash
# scene-probe.sh - report the current scene as text from a capture input log.
set -euo pipefail

die() { echo "scene-probe: $*" >&2; exit 2; }

current_scene() {
  local log_file="$1"
  awk '
    $1 == "frame" && /screen=/ {
      for (i = 1; i <= NF; i++) {
        if ($i ~ /^screen=/) {
          scene = substr($i, 8)
        }
      }
    }
    END { if (scene == "") exit 1; print scene }
  ' "$log_file"
}

current_frame() {
  local log_file="$1"
  awk '
    $1 == "frame" && /screen=/ { frame = $2 }
    END { if (frame == "") exit 1; print frame }
  ' "$log_file"
}

current_wall_ms() {
  local log_file="$1"
  awk '
    $1 == "frame" && /screen=/ {
      for (i = 1; i <= NF; i++) {
        if ($i ~ /^wall_ms=/) wall = substr($i, 9)
      }
    }
    END { if (wall == "") exit 1; print wall }
  ' "$log_file"
}

scene_sequence() {
  local log_file="$1"
  awk '
    $1 == "frame" && /screen=/ {
      for (i = 1; i <= NF; i++) {
        if ($i ~ /^screen=/) {
          scene = substr($i, 8)
          if (scene != last) { print scene; last = scene }
        }
      }
    }
  ' "$log_file"
}

print_json() {
  local log_file="$1"
  local scene frame wall
  scene="$(current_scene "$log_file")"
  frame="$(current_frame "$log_file")"
  wall="$(current_wall_ms "$log_file")"
  printf '{"scene":"%s","frame":%s,"wall_ms":%s}\n' "$scene" "$frame" "$wall"
}

cmd_once() {
  local log_file="$1" json="$2"
  [ -r "$log_file" ] || die "cannot read input log: $log_file"
  if [ "$json" = "1" ]; then
    print_json "$log_file"
  else
    current_scene "$log_file"
  fi
}

cmd_watch() {
  local log_file="$1" interval="$2" count="$3" changes_only="$4" json="$5"
  [ -r "$log_file" ] || die "cannot read input log: $log_file"
  local seen="" current="" printed=0
  while :; do
    if current="$(current_scene "$log_file" 2>/dev/null)"; then
      if [ "$changes_only" = "1" ]; then
        if [ "$current" != "$seen" ]; then
          if [ "$json" = "1" ]; then print_json "$log_file"; else echo "$current"; fi
          seen="$current"
        fi
      else
        if [ "$json" = "1" ]; then print_json "$log_file"; else echo "$current"; fi
      fi
    fi
    printed=$((printed + 1))
    if [ "$count" -gt 0 ] && [ "$printed" -ge "$count" ]; then
      break
    fi
    sleep "$interval"
  done
}

main() {
  local log_file="${1:-}"
  [ -n "$log_file" ] || die "usage: scene-probe.sh INPUT_LOG [--watch ...]"
  shift || true

  local watch=0 interval="0.5" count=1 count_set=0 changes_only=0 json=0 sequence=0
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --watch|--poll) watch=1; shift ;;
      --interval) interval="$2"; shift 2 ;;
      --count) count="$2"; count_set=1; shift 2 ;;
      --changes-only) changes_only=1; shift ;;
      --json) json=1; shift ;;
      --sequence|--scenes) sequence=1; shift ;;
      *) die "unknown option: $1" ;;
    esac
  done

  # A watch without an explicit count polls forever; cap it so no
  # invocation (human or subagent) hangs unbounded. 240 polls at the
  # default interval is two minutes; pass --count for more.
  if [ "$watch" = "1" ] && [ "$count_set" = "0" ]; then
    count=240
  fi

  if [ "$sequence" = "1" ]; then
    [ -r "$log_file" ] || die "cannot read input log: $log_file"
    scene_sequence "$log_file"
    return 0
  fi

  if [ "$watch" = "1" ]; then
    cmd_watch "$log_file" "$interval" "$count" "$changes_only" "$json"
  else
    cmd_once "$log_file" "$json"
  fi
}

main "$@"
