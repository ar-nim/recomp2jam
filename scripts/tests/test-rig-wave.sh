#!/usr/bin/env bash
# One command for the rig-wave selftest legs. Hermetic: no Wine, display,
# Memoryer, game data, or network is touched. Each of the four leg kinds from
# #865 has its own named leg so a failure names the vocabulary it broke.
#
#   bash scripts/tests/test-rig-wave.sh
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "$0")/.." && pwd)"
export PYTHONPATH="$SCRIPTS_DIR/tests:$SCRIPTS_DIR/lib${PYTHONPATH:+:$PYTHONPATH}"

run_leg() {
  local name="$1"
  shift
  printf '\n===== rig-wave selftest leg: %s =====\n' "$name"
  "$@"
}

status=0
run_leg "wire framing" python3 -m unittest -v test_wire_frames || status=1
run_leg "recipe core" python3 -m unittest -v \
  test_wave_recipe.WaveRecipeTests \
  test_wave_recipe.WaveRecipeLegTests.test_boot_attach_scene_sync_host_marker_and_injector_load || status=1
run_leg "driver core" python3 -m unittest -v test_rig_wave.RigWaveTests || status=1
run_leg "boot/attach leg" python3 -m unittest -v \
  test_wave_recipe.WaveRecipeLegTests.test_boot_attach_requires_a_login_target \
  test_rig_wave.BootAttachLegTests || status=1
run_leg "scene-sync leg" python3 -m unittest -v \
  test_wave_recipe.WaveRecipeLegTests.test_screen_sync_requires_a_scene_and_a_positive_timeout \
  test_rig_wave.SceneSyncLegTests || status=1
run_leg "host-marker leg" python3 -m unittest -v \
  test_wave_recipe.WaveRecipeLegTests.test_host_marker_requires_a_gate_opcode_and_an_ack \
  test_rig_wave.HostMarkerLegTests || status=1
run_leg "native-injector leg" python3 -m unittest -v \
  test_wave_recipe.WaveRecipeLegTests.test_unpublished_injector_is_refused \
  test_rig_wave.NativeInjectorLegTests || status=1
run_leg "aim lint" python3 "$SCRIPTS_DIR/lib/popup_state.py" lint-aims \
  "$SCRIPTS_DIR" || status=1

[ "$status" -eq 0 ] || {
  echo "test-rig-wave: a selftest leg failed; no green claim" >&2
  exit "$status"
}
echo "test-rig-wave: wire framing, recipe, driver, and four #865 legs OK"
