#!/usr/bin/env bash
# rig-stub-display.sh — hermetic X display stubs for rig selftests.
#
# Sourced by the selftests of scripts/rig-run.sh and scripts/rig-afk.sh: a
# stub `xdpyinfo` that answers only for displays "brought up" by the stub
# `Xvfb`. The sound-server stubs stay with the audio lane
# (scripts/rig-audio.sh, `rig_audio_stub_tools`), so PipeWire has one home.
#
# Usage: rig_display_stub_tools <bin-dir> <state-dir>
#
# The stub Xvfb touches <state>/up-<display> and then sleeps, so a test can
# ask `xdpyinfo -display <display>` about a display it started.

rig_display_stub_tools() {
  local bin="$1" state="$2"
  mkdir -p "$bin" "$state"
  cat >"$bin/xdpyinfo" <<'STUBEOF'
#!/bin/sh
if [ -f "$STUBSTATE/up-$2" ]; then exit 0; else exit 1; fi
STUBEOF
  cat >"$bin/Xvfb" <<'STUBEOF'
#!/bin/sh
touch "$STUBSTATE/up-$1"
exec sleep 120
STUBEOF
  chmod +x "$bin/xdpyinfo" "$bin/Xvfb"
}
