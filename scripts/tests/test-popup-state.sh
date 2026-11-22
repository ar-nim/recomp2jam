#!/usr/bin/env bash
# Hermetic selftest for the one popup-state query (#747).
#
# Live caller chains pinned here:
#   scripts/planet-to-waiting.sh -> lib/planet_popup.py -> lib/popup_state.py
#   scripts/wine-scene.py Frames.popup_fraction/popup_is_up -> lib/popup_state.py
#   scripts/wine-scene.py Input.send_click("fallback-ok") -> AIMS -> pop-up aim
# The winedbg leg pins the X-plate NCC core from ~/scratch/planet-verify/common.py
# and docs/captures/t207/channel.md with a synthetic stopped-state byte reader.
# No Xvfb, Wine, Memoryer, game data, or capture from the live rig is used.
#
#   bash scripts/tests/test-popup-state.sh
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1

SCRIPTS_DIR="$(cd "$(dirname "$0")/.." && pwd)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf -- "$WORK_DIR"' EXIT

fail() {
  printf 'test-popup-state: FAIL %s\n' "$1" >&2
  exit 1
}

pass() {
  printf 'test-popup-state: PASS %s\n' "$1"
}

python3 - "$WORK_DIR" <<'PY'
"""Write synthetic 800x600 frames; no game pixels are copied."""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

work_dir = Path(sys.argv[1])
Image.new("RGB", (800, 600), (8, 110, 175)).save(work_dir / "popup.png")
Image.new("RGB", (800, 600), (33, 26, 52)).save(work_dir / "bare.png")

template = np.fromfunction(
    lambda row, col: (row * 3 + col * 5) % 200 + 20,
    (50, 67),
    dtype=np.int16,
).astype(np.uint8)
plate = np.full((600, 800), 97, dtype=np.uint8)
plate[148:198, 548:615] = template
np.save(work_dir / "template.npy", template)
np.save(work_dir / "plate.npy", plate)
PY

# Window-driver PNG adapter: the existing planet_popup.py path.
[ "$(python3 "$SCRIPTS_DIR/lib/planet_popup.py" check "$WORK_DIR/popup.png")" = "up" ] ||
  fail "png-adapter-blue-popup-up"
[ "$(python3 "$SCRIPTS_DIR/lib/planet_popup.py" check "$WORK_DIR/bare.png")" = "down" ] ||
  fail "png-adapter-no-popup"
pass "png-adapter-blue-popup-up-and-no-popup"

# The same query over a winedbg stopped-state memory read and over the
# wine-scene ndarray frame. The X-plate leg uses the pinned NCC core.
python3 - "$SCRIPTS_DIR" "$WORK_DIR" <<'PY'
import importlib.util
import sys
from pathlib import Path

import numpy as np
from PIL import Image

scripts_dir = Path(sys.argv[1])
work_dir = Path(sys.argv[2])
sys.path.insert(0, str(scripts_dir / "lib"))
import popup_state

spec = importlib.util.spec_from_file_location("wine_scene", scripts_dir / "wine-scene.py")
wine_scene = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wine_scene)

with Image.open(work_dir / "popup.png") as image:
    blue_pixels = image.convert("RGB").tobytes()
with Image.open(work_dir / "bare.png") as image:
    bare_pixels = image.convert("RGB").tobytes()

def read_rgb(source, box):
    left, top, right, bottom = box
    return b"".join(source[(y * 800 + left) * 3:(y * 800 + right) * 3]
                     for y in range(top, bottom))

def read_popup_bytes(box):
    return read_rgb(blue_pixels, box)

def read_bare_bytes(box):
    return read_rgb(bare_pixels, box)

up_adapter = popup_state.WinedbgMemoryAdapter(read_popup_bytes)
down_adapter = popup_state.WinedbgMemoryAdapter(read_bare_bytes)
assert popup_state.query_popup(popup_state.NORMAL_MODE_NOTICE, up_adapter).is_up
assert not popup_state.query_popup(popup_state.NORMAL_MODE_NOTICE, down_adapter).is_up

template = np.load(work_dir / "template.npy").tobytes()
plate = np.load(work_dir / "plate.npy").tobytes()

def read_plate(box):
    left, top, right, bottom = box
    return b"".join(plate[y * 800 + left:y * 800 + right]
                    for y in range(top, bottom))

channel_adapter = popup_state.WinedbgMemoryAdapter(read_plate, template=template)
assert popup_state.query_popup(popup_state.CHANNEL_DIALOG, channel_adapter).is_up
flat_adapter = popup_state.WinedbgMemoryAdapter(
    lambda box: bytes((box[2] - box[0]) * (box[3] - box[1])), template=template
)
assert not popup_state.query_popup(popup_state.CHANNEL_DIALOG, flat_adapter).is_up

blue_frame = np.asarray(Image.open(work_dir / "popup.png").convert("RGB"))
bare_frame = np.asarray(Image.open(work_dir / "bare.png").convert("RGB"))
frames = wine_scene.Frames(":99", work_dir / "frames")
frames.last = blue_frame
assert frames.popup_is_up()
frames.last = bare_frame
assert not frames.popup_is_up()

# The live shell and the Wine drive read their aim from the definition.
aim_from_lib = popup_state.aim("normal-mode-ok")
assert aim_from_lib.as_pair() == (400, 365)
assert popup_state.aim("crash-report-cancel").as_pair() == (449, 329)
assert wine_scene.AIMS["fallback-ok"][:2] == aim_from_lib.as_pair()
assert popup_state.NORMAL_MODE_NOTICE.aim == aim_from_lib
PY
pass "winedbg-memory-and-array-adapters-blue-popup-up-and-no-popup"

# Aims live beside the popup definitions; new hardcoded aims fail the lint.
[ "$(python3 "$SCRIPTS_DIR/lib/popup_state.py" aim normal-mode-ok)" = "400 365" ] ||
  fail "aim-normal-mode-ok"
[ "$(python3 "$SCRIPTS_DIR/lib/planet_popup.py" aim crash-report-cancel)" = "449 329" ] ||
  fail "aim-crash-report-cancel"
python3 "$SCRIPTS_DIR/lib/popup_state.py" lint-aims "$SCRIPTS_DIR" ||
  fail "lint-real-scripts"
mkdir -p "$WORK_DIR/scripts"
printf 'click_at 400 365\n' > "$WORK_DIR/scripts/hardcoded.sh"
if python3 "$SCRIPTS_DIR/lib/popup_state.py" lint-aims "$WORK_DIR/scripts" >"$WORK_DIR/lint.out" 2>&1; then
  fail "lint-accepted-hardcoded-aim"
fi
grep -q 'hardcoded.sh' "$WORK_DIR/lint.out" || fail "lint-names-offending-file"
printf 'click_at 449 329\n' > "$WORK_DIR/scripts/hardcoded.sh"
if python3 "$SCRIPTS_DIR/lib/popup_state.py" lint-aims "$WORK_DIR/scripts" >"$WORK_DIR/crash-lint.out" 2>&1; then
  fail "lint-accepted-crash-report-aim"
fi
grep -q 'crash-report-cancel' "$WORK_DIR/crash-lint.out" ||
  fail "lint-names-crash-report-aim"
printf 'AIM = (\n  400,\n  365,\n)\n' > "$WORK_DIR/scripts/hardcoded.py"
if python3 "$SCRIPTS_DIR/lib/popup_state.py" lint-aims "$WORK_DIR/scripts" >"$WORK_DIR/multiline-lint.out" 2>&1; then
  fail "lint-accepted-multiline-aim"
fi
grep -q 'hardcoded.py:2' "$WORK_DIR/multiline-lint.out" ||
  fail "lint-names-multiline-aim-line"
pass "aims-beside-definitions-and-lint-flags-hardcoded-aim"

echo "test-popup-state: shell suite OK"
