"""Synthetic frames for the NORMAL-mode popup detector (no game art)."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS / "lib"))

import planet_popup  # noqa: E402  (path set above for the in-repo helper)


def write_frame(directory: Path, colour: tuple[int, int, int]) -> Path:
    path = directory / "planet.png"
    Image.new("RGB", (800, 600), colour).save(path)
    return path


class PlanetPopupTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)

    def test_blue_message_box_is_detected(self) -> None:
        frame = write_frame(self.directory, (6, 113, 167))
        self.assertTrue(planet_popup.popup_is_up(frame))

    def test_plain_planet_frame_is_not_a_popup(self) -> None:
        frame = write_frame(self.directory, (33, 26, 52))
        self.assertFalse(planet_popup.popup_is_up(frame))


if __name__ == "__main__":
    unittest.main()
