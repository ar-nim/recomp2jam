#!/usr/bin/env bash
# run-manifest.sh - standard run manifest: write, verify, rerun.
set -euo pipefail

die() { echo "run-manifest: $*" >&2; exit 2; }

require_python() {
  command -v python3 >/dev/null 2>&1 || die "python3 is required"
}

manifest_path_from_arg() {
  local arg="$1"
  if [ -d "$arg" ]; then
    echo "$arg/manifest.json"
  else
    echo "$arg"
  fi
}

derive_scene_sequence() {
  local log_file="$1"
  python3 - "$log_file" <<'PY'
import re, sys
path = sys.argv[1]
scenes = []
last = None
try:
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if not line.startswith("frame "):
                continue
            match = re.search(r"screen=(\S+)", line)
            if not match:
                continue
            scene = match.group(1)
            if scene != last:
                scenes.append(scene)
                last = scene
except FileNotFoundError:
    pass
print("\n".join(scenes))
PY
}

cmd_write() {
  local out_dir="$1"; shift
  [ -n "$out_dir" ] || die "write: output directory required"
  require_python

  local pair="" build="" params="" script="" seed="" input_log=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --pair) pair="$2"; shift 2 ;;
      --build) build="$2"; shift 2 ;;
      --params) params="$2"; shift 2 ;;
      --script) script="$2"; shift 2 ;;
      --seed) seed="$2"; shift 2 ;;
      --input-log) input_log="$2"; shift 2 ;;
      *) die "write: unknown option: $1" ;;
    esac
  done

  mkdir -p "$out_dir"
  local artifacts_dir="$out_dir/artifacts"
  mkdir -p "$artifacts_dir/screenshots" "$artifacts_dir/stats"

  if [ -z "$pair" ]; then pair="native-xvfb-xtest"; fi
  if [ -z "$build" ]; then
    build="$(git -C "$out_dir" rev-parse HEAD 2>/dev/null || git rev-parse HEAD 2>/dev/null || echo "unknown")"
  fi
  if [ -z "$seed" ]; then seed="$RANDOM$RANDOM"; fi

  local copied_input_log=""
  local scene_sequence_file="$artifacts_dir/scene-sequence.txt"
  local scene_sequence_json="[]"
  if [ -n "$input_log" ]; then
    if [ ! -r "$input_log" ]; then die "write: cannot read input log: $input_log"; fi
    cp -f "$input_log" "$artifacts_dir/input.log"
    copied_input_log="artifacts/input.log"
    derive_scene_sequence "$input_log" > "$scene_sequence_file"
    scene_sequence_json="$(python3 - "$scene_sequence_file" <<'PY'
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    scenes = [line.rstrip("\n") for line in fh if line.strip()]
print(json.dumps(scenes))
PY
)"
  else
    : > "$scene_sequence_file"
  fi

  local manifest="$out_dir/manifest.json"
  python3 - "$manifest" "$pair" "$build" "$params" "$script" "$seed" \
    "$copied_input_log" "$scene_sequence_json" "$artifacts_dir" <<'PY'
import json, os, sys
from datetime import datetime, timezone

manifest_path, pair, build, params, script, seed, input_log_rel, scenes_json, artifacts_dir = sys.argv[1:10]
scenes = json.loads(scenes_json)

required = ["manifest.json", "artifacts/screenshots", "artifacts/stats", "artifacts/scene-sequence.txt"]
if input_log_rel:
    required.insert(1, input_log_rel)

manifest = {
    "pair": pair,
    "build": build,
    "params": params,
    "script": script,
    "seed": seed,
    "created_at": datetime.now(timezone.utc).isoformat(),
    "input_log": input_log_rel,
    "scene_sequence": scenes,
    "artifacts": {
        "root": ".",
        "input_log": input_log_rel,
        "screenshots_dir": "artifacts/screenshots",
        "stats_dir": "artifacts/stats",
        "scene_sequence_file": "artifacts/scene-sequence.txt",
    },
    "required_artifacts": required,
}
with open(manifest_path, "w", encoding="utf-8") as fh:
    json.dump(manifest, fh, indent=2)
    fh.write("\n")
PY

  echo "run-manifest: wrote $manifest"
}

cmd_verify() {
  local arg="$1"
  [ -n "$arg" ] || die "verify: manifest or directory required"
  require_python
  local manifest
  manifest="$(manifest_path_from_arg "$arg")"
  [ -r "$manifest" ] || die "verify: cannot read manifest: $manifest"

  local base_dir
  base_dir="$(cd "$(dirname "$manifest")" && pwd)"

  python3 - "$manifest" "$base_dir" <<'PY'
import json, os, sys
manifest_path, base_dir = sys.argv[1], sys.argv[2]
with open(manifest_path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

missing = []
for field in ("pair", "build", "params", "script", "seed"):
    if field not in data:
        missing.append("field:" + field)

required = data.get("required_artifacts") or [
    "manifest.json", "artifacts/screenshots", "artifacts/stats",
    "artifacts/scene-sequence.txt",
]
for rel in required:
    full = os.path.join(base_dir, rel)
    if not os.path.exists(full):
        missing.append(rel)

if missing:
    print("run-manifest: INCOMPLETE artifact set", file=sys.stderr)
    for item in missing:
        print("  missing: " + item, file=sys.stderr)
    sys.exit(1)
print("run-manifest: artifact set complete")
PY
}

cmd_rerun() {
  local manifest_arg="$1"; shift
  [ -n "$manifest_arg" ] || die "rerun: manifest required"
  require_python

  local new_input_log=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --new-input-log) new_input_log="$2"; shift 2 ;;
      --execute) die "rerun: --execute is not supported; rerun compares scene sequences" ;;
      *) die "rerun: unknown option: $1" ;;
    esac
  done

  local manifest
  manifest="$(manifest_path_from_arg "$manifest_arg")"
  [ -r "$manifest" ] || die "rerun: cannot read manifest: $manifest"
  local base_dir
  base_dir="$(cd "$(dirname "$manifest")" && pwd)"

  python3 - "$manifest" "$base_dir" "$new_input_log" <<'PY'
import json, os, re, sys

manifest_path, base_dir, new_input_log = sys.argv[1], sys.argv[2], sys.argv[3]
with open(manifest_path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

recorded = data.get("scene_sequence") or []

def derive(path):
    scenes = []
    last = None
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                if not line.startswith("frame "):
                    continue
                m = re.search(r"screen=(\S+)", line)
                if not m:
                    continue
                scene = m.group(1)
                if scene != last:
                    scenes.append(scene)
                    last = scene
    except FileNotFoundError:
        return None
    return scenes

if new_input_log:
    actual = derive(new_input_log)
    source = new_input_log
else:
    rel = data.get("input_log") or ""
    if not rel:
        print("run-manifest: rerun FAILED: manifest has no input log", file=sys.stderr)
        sys.exit(1)
    actual = derive(os.path.join(base_dir, rel))
    source = rel

if actual is None:
    print("run-manifest: rerun FAILED: input log not found: " + source, file=sys.stderr)
    sys.exit(1)

if actual == recorded:
    print("run-manifest: scene sequence reproduced (" + str(len(actual)) + " transitions)")
    sys.exit(0)

print("run-manifest: rerun FAILED: scene sequence differs", file=sys.stderr)
print("  recorded: " + " -> ".join(recorded), file=sys.stderr)
print("  actual:   " + " -> ".join(actual), file=sys.stderr)
sys.exit(1)
PY
}

cmd_sequence() {
  local log_file="$1"
  [ -n "$log_file" ] || die "sequence: input log required"
  [ -r "$log_file" ] || die "sequence: cannot read input log: $log_file"
  derive_scene_sequence "$log_file"
}

main() {
  local cmd="${1:-}"; shift || true
  case "$cmd" in
    write) [ "$#" -ge 1 ] || die "usage: run-manifest.sh write OUT_DIR [options]"; cmd_write "$@" ;;
    verify) [ "$#" -ge 1 ] || die "usage: run-manifest.sh verify MANIFEST_OR_DIR"; cmd_verify "$1" ;;
    rerun) [ "$#" -ge 1 ] || die "usage: run-manifest.sh rerun MANIFEST [options]"; cmd_rerun "$@" ;;
    sequence) [ "$#" -ge 1 ] || die "usage: run-manifest.sh sequence INPUT_LOG"; cmd_sequence "$1" ;;
    *) die "usage: run-manifest.sh {write|verify|rerun|sequence} ..." ;;
  esac
}

main "$@"
