"""Guard the shots oracle against data-less fallback pairs.

A pair whose room art is flat must be refused, even when both sides are
byte-identical. The positive leg pins a varied, byte-identical pair so the
guard cannot pass by refusing every pair. No game data is used: the PNGs are
synthetic and generated per test.

Set VERIFY_SHOTS_SCRIPT to run the same guards against a different copy of
verify-shots.sh (used to prove the guards can fail).
"""
from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from PIL import Image

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
VERIFY_SHOTS = Path(os.environ.get(
    "VERIFY_SHOTS_SCRIPT", str(SCRIPTS_DIR / "verify-shots.sh")))
FLAT_REFUSAL = "flat fallback, game data was invisible"
IMAGE_SIZE = 64


def write_pair(root: Path, *, flat_room: bool) -> None:
    """Write two identical capture directories with one room art variant."""
    room = Image.new("RGB", (IMAGE_SIZE, IMAGE_SIZE), (7, 11, 19))
    if not flat_room:
        pixels = bytes(
            component
            for row in range(IMAGE_SIZE)
            for column in range(IMAGE_SIZE)
            for component in (column * 4 % 256, row * 4 % 256,
                              (column + row) * 2 % 256)
        )
        room = Image.frombytes("RGB", (IMAGE_SIZE, IMAGE_SIZE), pixels)
    for side in ("before", "after"):
        directory = root / side
        directory.mkdir()
        room.save(directory / "room.png")
        Image.new("RGB", (IMAGE_SIZE, IMAGE_SIZE), (23, 31, 47)).save(
            directory / "lanking.png")
        Image.new("RGB", (IMAGE_SIZE, IMAGE_SIZE), (11, 13, 17)).save(
            directory / "planet.png")


def verify_pair(root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", str(VERIFY_SHOTS), str(root / "before"), str(root / "after")],
        capture_output=True, text=True, check=False,
    )


class ShotsFallbackGuard(unittest.TestCase):
    def test_identical_flat_pair_is_refused_as_a_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_pair(root, flat_room=True)
            result = verify_pair(root)
        self.assertNotEqual(0, result.returncode)
        self.assertIn(FLAT_REFUSAL, result.stderr)

    def test_identical_varied_pair_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_pair(root, flat_room=False)
            result = verify_pair(root)
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("verify-shots: all captures byte-identical", result.stdout)


if __name__ == "__main__":
    unittest.main()
