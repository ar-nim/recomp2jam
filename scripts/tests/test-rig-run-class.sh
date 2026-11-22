#!/usr/bin/env bash
# One command for the run-class shell guards. All legs are hermetic: no Xvfb,
# audio sink, Memoryer, game data, or live client is touched.
#
#   bash scripts/tests/test-rig-run-class.sh
#
# The live caller chains are named inside each lib's selftest. rig-run.sh's
# heavier stub-world selftest is separate (it drives real Xvfb audio stubs).
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "$0")/.." && pwd)"

bash "$SCRIPTS_DIR/lib/rig-run-class.sh" selftest
bash "$SCRIPTS_DIR/lib/play-local-seams.sh" selftest
bash "$SCRIPTS_DIR/lib/rig_binary_stale.sh" selftest
bash "$SCRIPTS_DIR/lib/rig-client-display.sh" selftest
bash "$SCRIPTS_DIR/lib/rig-client-identity.sh" selftest
bash "$SCRIPTS_DIR/tests/test-rig-audio-verdict.sh"

echo "test-rig-run-class: shell suite OK"
