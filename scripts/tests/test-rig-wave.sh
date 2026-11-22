#!/usr/bin/env bash
# One command for the rig-wave selftests. Hermetic: no Wine, display,
# Memoryer, game data, or network is touched.
#
#   bash scripts/tests/test-rig-wave.sh
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "$0")/.." && pwd)"

python3 -m unittest discover -s "$SCRIPTS_DIR/tests" -p 'test_wire_frames.py' -v
python3 -m unittest discover -s "$SCRIPTS_DIR/tests" -p 'test_wave_recipe.py' -v
python3 -m unittest discover -s "$SCRIPTS_DIR/tests" -p 'test_rig_wave.py' -v
python3 "$SCRIPTS_DIR/lib/popup_state.py" lint-aims "$SCRIPTS_DIR"

echo "test-rig-wave: wire framing, recipe, and driver selftests OK"
