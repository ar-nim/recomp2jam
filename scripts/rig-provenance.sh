#!/usr/bin/env bash
# rig-provenance.sh - capture provenance manifests (docs/reversing.md section 6).
#
# Usage:
#   scripts/rig-provenance.sh emit OUT_DIR --producer BINARY [options]
#   scripts/rig-provenance.sh check OUT_DIR
#   scripts/rig-provenance.sh cite OUT_DIR [--claim none|exe]
#
# emit: probes the tool versions (python3, Xvfb, wine, capture tool,
#     injector), reads the session straight out of the run dir that
#     scripts/rig-run.sh leaves behind (pids -> full command, env.sh ->
#     DISPLAY / PULSE_SINK / WAYLAND_DISPLAY, claims -> display), hashes
#     every capture under OUT_DIR, and writes OUT_DIR/provenance.json with
#     its own manifest hash last. No field is hand-edited.
# check: `scripts/run-manifest.sh verify` runs this; a capture without a
#     provenance entry fails the run.
# cite: prints the one-paragraph citation block for a manifest-complete run.
#     --claim exe also demands that the exe itself produced the pixels
#     (producer kind exe + wine version), which is the #278 check.
#
# Options for emit:
#   --producer BINARY            the binary that produced the captures
#   --producer-kind exe|client-build   (default: inferred from the name)
#   --producer-version VER       required for exe (e.g. 7.55)
#   --expect-producer-sha256 HEX refuse to emit when the binary differs
#   --cargo-profile PROFILE      (default: inferred from the build path)
#   --git-commit SHA             (default: HEAD of the current repo)
#   --pair NAME                  (default: from the run's manifest.json)
#   --build SHA                  (default: from the run's manifest.json or HEAD)
#   --seed SEED                  (default: from the run's manifest.json)
#   --display D                  (default: the run's env.sh / claims)
#   --command "CMD"              repeatable; default: the run's pids line
#   --capture-tool NAME          ffmpeg/xwd/import/magick/scrot, or
#                                client-frame-dump (default for a client build)
#   --injector PATH              input injector (recorded with its hash)
#   --artifact PATH              extra file to hash (captures are automatic)
#   --tool-version ROLE VERSION  override a probed version
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="$SCRIPT_DIR/lib/rig-provenance.py"

die() { echo "rig-provenance: $*" >&2; exit 2; }

require_python() {
  command -v python3 >/dev/null 2>&1 || die "python3 is required"
}

# --- probing ---------------------------------------------------------------

first_line() {
  printf '%s' "${1%%$'\n'*}"
}

probe_binary_version() {  # binary [version-flag]
  local binary="$1" flag="${2:--version}" output version
  command -v "$binary" >/dev/null 2>&1 || return 0
  output="$("$binary" "$flag" 2>&1 || true)"
  version="$(first_line "$output")"
  version="${version#"$binary version "}"
  version="${version#"$binary "}"
  printf '%s' "${version%% Copyright*}"
}

probe_xvfb_version() {
  local output=""
  if command -v pacman >/dev/null 2>&1; then
    output="$(pacman -Q xorg-server-xvfb 2>/dev/null | awk '{ print $2; exit }' || true)"
  elif command -v dpkg-query >/dev/null 2>&1; then
    output="$(dpkg-query -W -f='${Version}' xvfb 2>/dev/null || true)"
  fi
  first_line "$output"
}

probe_wine_version() {
  local version
  version="$(probe_binary_version wine --version)"
  printf '%s' "${version#wine-}"
}

probe_capture_version() {  # tool name
  case "$1" in
    client-frame-dump) echo "in-process frame dump" ;;
    ffmpeg) probe_binary_version ffmpeg -version ;;
    magick) probe_binary_version magick -version ;;
    import) probe_binary_version import -version ;;
    scrot) probe_binary_version scrot --version ;;
    xwd) probe_binary_version xwd -version ;;
    *) probe_binary_version "$1" ;;
  esac
}

file_sha256_short() {
  python3 - "$1" <<'PY'
import hashlib, sys
print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest()[:12])
PY
}

tool_available() {
  case "$1" in
    client-frame-dump) return 0 ;;
    *) command -v "$1" >/dev/null 2>&1 ;;
  esac
}

# --- run-dir discovery -----------------------------------------------------

value_from_env_sh() {  # run-dir key
  local file="$1/env.sh" value
  [ -r "$file" ] || return 0
  value="$(sed -n "s/^export $2='\([^']*\)'\$/\1/p" "$file" | head -1)"
  printf '%s' "$value"
}

value_from_claims() {  # run-dir key
  local file="$1/claims"
  [ -r "$file" ] || return 0
  awk -v key="$2" '$1 == key { print $2; exit }' "$file"
}

discovered_display() {  # run-dir
  local display
  display="$(value_from_env_sh "$1" DISPLAY)"
  [ -n "$display" ] || display="$(value_from_claims "$1" display)"
  printf '%s' "$display"
}

discovered_command() {  # run-dir
  local pids="$1/pids"
  [ -r "$pids" ] || return 0
  awk '$1 == "client" { sub(/^client +[0-9]+ +/, ""); command = $0 }
       END { print command }' "$pids"
}

# --- emit ------------------------------------------------------------------

infer_producer_kind() {
  case "$1" in
    *.exe|*.EXE) echo "exe" ;;
    *) echo "client-build" ;;
  esac
}

infer_cargo_profile() {
  case "$1" in
    */release/*) echo "release" ;;
    */debug/*) echo "debug" ;;
    *) echo "" ;;
  esac
}

git_value() {  # args...
  git "$@" 2>/dev/null || true
}

current_commit() {
  first_line "$(git_value rev-parse HEAD)"
}

working_tree_is_dirty() {
  [ -n "$(git_value status --porcelain --untracked-files=no)" ] && echo "1" || echo "0"
}

append_command_words() {  # user- or run-dir-supplied command line
  local word
  for word in $1; do
    # `--command-arg=VALUE` keeps leading dashes (`--` separators) intact.
    command_args+=("--command-arg=$word")
  done
}

available_flag() {
  if tool_available "$1"; then echo "1"; else echo "0"; fi
}

collect_tools() {  # capture-tool injector tool-version-overrides...
  local capture_tool="$1" injector="$2"
  shift 2
  local python_version xvfb_version wine_version capture_version
  python_version="$(probe_binary_version python3 --version)"
  xvfb_version="$(probe_xvfb_version)"
  wine_version="$(probe_wine_version)"
  capture_version="$(probe_capture_version "$capture_tool")"
  tool_args+=(
    --tool python3 python3 "$python_version" "$(available_flag python3)"
    --tool xvfb Xvfb "$xvfb_version" "$(available_flag Xvfb)"
    --tool wine wine "$wine_version" "$(available_flag wine)"
    --tool capture "$capture_tool" "$capture_version" "$(available_flag "$capture_tool")"
  )
  if [ -n "$injector" ]; then
    tool_args+=(--tool injector "$(basename "$injector")" \
      "$(file_sha256_short "$injector")" "$(available_flag "$injector")")
  fi
  apply_tool_overrides "$*"
}

apply_tool_overrides() {
  local role version index
  for override in $*; do
    role="${override%%=*}"
    version="${override#*=}"
    for ((index = 0; index < ${#tool_args[@]}; index += 1)); do
      if [ "${tool_args[index]}" = "--tool" ] && [ "${tool_args[index + 1]}" = "$role" ]; then
        tool_args[index + 3]="$version"
      fi
    done
  done
}

cmd_emit() {
  local out_dir="${1:-}"
  [ -n "$out_dir" ] || die "emit: output directory required"
  shift
  [ -d "$out_dir" ] || die "emit: output directory not found: $out_dir"
  require_python

  local producer="" kind="" producer_version="" cargo_profile="" git_commit=""
  local expect_sha="" pair="" build="" seed="" display="" capture_tool=""
  local injector="" tool_versions=""
  tool_args=()
  command_args=()
  artifact_args=()
  local user_command=""

  while [ "$#" -gt 0 ]; do
    case "$1" in
      --producer) producer="$2"; shift 2 ;;
      --producer-kind) kind="$2"; shift 2 ;;
      --producer-version) producer_version="$2"; shift 2 ;;
      --expect-producer-sha256) expect_sha="$2"; shift 2 ;;
      --cargo-profile) cargo_profile="$2"; shift 2 ;;
      --git-commit) git_commit="$2"; shift 2 ;;
      --pair) pair="$2"; shift 2 ;;
      --build) build="$2"; shift 2 ;;
      --seed) seed="$2"; shift 2 ;;
      --display) display="$2"; shift 2 ;;
      --command) user_command="${user_command}${user_command:+ }$2"; shift 2 ;;
      --capture-tool) capture_tool="$2"; shift 2 ;;
      --injector) injector="$2"; shift 2 ;;
      --artifact) artifact_args+=(--artifact "$2"); shift 2 ;;
      --tool-version) tool_versions="${tool_versions}${2}=${3} "; shift 3 ;;
      *) die "emit: unknown option: $1" ;;
    esac
  done

  [ -n "$producer" ] || die "emit: --producer BINARY is required"
  [ -f "$producer" ] || die "emit: producer binary not found: $producer"
  [ -z "$kind" ] && kind="$(infer_producer_kind "$producer")"
  [ -z "$cargo_profile" ] && cargo_profile="$(infer_cargo_profile "$producer")"
  [ -z "$git_commit" ] && git_commit="$(current_commit)"
  if [ "$kind" = "client-build" ] && [ -z "$git_commit" ]; then
    die "emit: a client build needs --git-commit (run inside the repo, or pass the commit)"
  fi
  if [ -z "$capture_tool" ]; then
    [ "$kind" = "exe" ] && die "emit: an exe run needs --capture-tool (ffmpeg/xwd/import/...) — the exe cannot dump its own Wine pixels"
    capture_tool="client-frame-dump"
  fi
  [ -n "$display" ] || display="$(discovered_display "$out_dir")"
  local dirty_args=()
  if [ "$kind" = "client-build" ] && [ "$(working_tree_is_dirty)" = "1" ]; then
    dirty_args+=(--git-dirty)
  fi

  if [ -n "$user_command" ]; then
    append_command_words "$user_command"
  else
    append_command_words "$(discovered_command "$out_dir")"
  fi

  collect_tools "$capture_tool" "$injector" "$tool_versions"

  python3 "$LIB" emit "$out_dir" \
    --producer "$producer" \
    --producer-kind "$kind" \
    --producer-version "$producer_version" \
    --cargo-profile "$cargo_profile" \
    --git-commit "$git_commit" \
    "${dirty_args[@]}" \
    --expect-producer-sha256 "$expect_sha" \
    --pair "$pair" --build "$build" --seed "$seed" \
    --display "$display" \
    --wayland-display "$(value_from_env_sh "$out_dir" WAYLAND_DISPLAY)" \
    --xdg-session-type "${XDG_SESSION_TYPE:-}" \
    --pulse-sink "$(value_from_env_sh "$out_dir" PULSE_SINK)" \
    "${tool_args[@]}" \
    "${artifact_args[@]}" \
    "${command_args[@]}"
}

cmd_check() {
  local out_dir="${1:-}"
  [ -n "$out_dir" ] || die "check: run directory required"
  require_python
  python3 "$LIB" check "$out_dir"
}

cmd_cite() {
  local out_dir="${1:-}" claim="none"
  [ -n "$out_dir" ] || die "cite: run directory required"
  shift
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --claim) claim="$2"; shift 2 ;;
      *) die "cite: unknown option: $1" ;;
    esac
  done
  require_python
  python3 "$LIB" cite "$out_dir" --claim "$claim"
}

main() {
  local command="${1:-}"; shift || true
  case "$command" in
    emit) cmd_emit "$@" ;;
    check) cmd_check "$@" ;;
    cite) cmd_cite "$@" ;;
    *) die "usage: rig-provenance.sh {emit|check|cite} OUT_DIR [options]" ;;
  esac
}

main "$@"
