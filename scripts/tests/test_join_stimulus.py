"""Seam tests for the #240 hover-press join stimulus.

The exe leg is the live proof; these tests pin the stimulus ORDER and the
no-SetCursorPos press shape at the seam the drive owns. A green test never
substitutes for a qualified capture (rules 7-8).
"""
from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

MODULE_PATH = Path(__file__).resolve().parents[1] / "wine-scene.py"
spec = importlib.util.spec_from_file_location("wine_scene", MODULE_PATH)
scene = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = scene
spec.loader.exec_module(scene)


class FakeLog:
    def __init__(self) -> None:
        self.events: list[str] = []

    def event(self, message: str) -> None:
        self.events.append(message)


class FakeFrames:
    def __init__(self) -> None:
        import numpy as np
        self.rest = np.zeros((600, 800, 3), dtype=np.int16)
        self.hovered = self.rest.copy()
        self.hovered[85:120, 40:330] = 200
        self.saved: list[Path] = []

    def grab(self):
        return self.rest if not self.saved else self.hovered

    def capture(self, path: Path) -> float:
        self.saved.append(path)
        return 0.0

    def row_highlight_delta(self, rest) -> float:
        return 42.0


class FakeServer:
    def __init__(self, joined: bool = True) -> None:
        self.joined = joined
        self.waited: list[tuple[str, float]] = []

    def wait_for(self, opcode: str, timeout: float) -> bool:
        self.waited.append((opcode, timeout))
        return self.joined


class FakeInjector:
    def __init__(self) -> None:
        self.calls: list[tuple] = []

    def cite(self, aim: str) -> tuple[int, int]:
        self.calls.append(("cite", aim))
        return 100, 100

    def warp_pointer(self, x: int, y: int) -> None:
        self.calls.append(("warp", x, y))

    def send_press(self) -> None:
        self.calls.append(("press",))


class HoverPressStimulusTest(unittest.TestCase):
    def test_warp_then_capture_highlight_then_one_press(self) -> None:
        frames, injector, server, log = (
            FakeFrames(), FakeInjector(), FakeServer(), FakeLog())
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"):
                joined = scene.hover_press_join(
                    frames, injector, server, log, Path(tmp), 12.0)
        self.assertTrue(joined)
        self.assertEqual(
            [call[0] for call in injector.calls],
            ["cite", "warp", "press"],
        )
        self.assertEqual(("warp", 100, 100), injector.calls[1])
        self.assertEqual([("0BBA", 12.0)], server.waited)
        self.assertEqual(1, len(frames.saved))
        self.assertEqual("room-row-hover.png", frames.saved[0].name)
        self.assertTrue(any("hover delta=" in line for line in log.events))

    def test_no_join_keeps_the_single_stimulus(self) -> None:
        frames, injector, server, log = (
            FakeFrames(), FakeInjector(), FakeServer(joined=False), FakeLog())
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"):
                joined = scene.hover_press_join(
                    frames, injector, server, log, Path(tmp), 12.0)
        self.assertFalse(joined)
        self.assertEqual(1, sum(1 for c in injector.calls if c[0] == "press"))


class PressShapeTest(unittest.TestCase):
    def test_press_sends_no_cursor_move(self) -> None:
        source = (MODULE_PATH.parent / "sendinput-helper.c").read_text()
        self.assertIn("static void press_at_pointer(void)", source)
        self.assertIn("press down=%u up=%u", source)
        press_body = source.split("static void press_at_pointer(void) {", 1)[1]
        press_body = press_body.split("\n}", 1)[0]
        self.assertNotIn("move_to", press_body)
        self.assertNotIn("SetCursorPos", press_body)
        self.assertIn("left_down()", press_body)
        self.assertIn("left_up()", press_body)

    def test_drive_accepts_both_join_modes(self) -> None:
        result = subprocess.run(
            [sys.executable, str(MODULE_PATH), "--help"],
            capture_output=True, text=True, check=True,
        )
        self.assertIn("--join-mode", result.stdout)
        self.assertIn("hover-press", result.stdout)
        self.assertIn("dclick", result.stdout)


if __name__ == "__main__":
    unittest.main()
