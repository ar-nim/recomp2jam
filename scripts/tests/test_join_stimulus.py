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

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
MODULE_PATH = SCRIPTS_DIR / "wine-scene.py"
HOST_PATH = SCRIPTS_DIR / "wine-room-host.py"


def load_module(name: str, path: Path):
    module_spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(module_spec)
    assert module_spec.loader is not None
    sys.modules[name] = module
    module_spec.loader.exec_module(module)
    return module


scene = load_module("wine_scene", MODULE_PATH)
host = load_module("wine_room_host", HOST_PATH)


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


class WireEvidenceTest(unittest.TestCase):
    def test_wire_tap_splits_partial_frames_without_loss(self) -> None:
        first = host.frame(1002, b"ab")
        second = host.frame(2005, b"xyz")
        frames, pending = host.split_wire_frames(first + second[:5])
        self.assertEqual([first], frames)
        self.assertEqual(second[:5], pending)
        frames, pending = host.split_wire_frames(pending + second[5:])
        self.assertEqual([second], frames)
        self.assertEqual(b"", pending)

    def test_wire_log_counts_only_downstream_room_created_events(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "room-stage.log"
            path.write_text(
                "12:00:00 C2S opcode=2005 (07D5) len=4 hex=01000000\n"
                "12:00:01 S2C opcode=1003 (03EB) len=8 hex=00\n"
                "12:00:02 S2C opcode=2005 (07D5) len=4 hex=00000000 "
                "ROOM_CREATED room=00000000\n",
                encoding="utf-8",
            )
            wire = scene.WireTapLog(path)
            self.assertEqual(1, len(wire.take()))
            self.assertEqual(1, wire.room_created_count)
            self.assertEqual([], wire.take())
            path.write_text(path.read_text() +
                            "12:00:03 S2C opcode=2005 (07D5) len=4 hex=01000000\n")
            self.assertEqual(1, len(wire.take()))
            self.assertEqual(2, wire.room_created_count)

    def test_paint_wait_keeps_popup_when_dismissal_disabled(self) -> None:
        class PopupFrames(FakeFrames):
            def popup_is_up(self):
                return True

            def grab(self):
                return self.rest

            def row_painted(self):
                return False

        frames, injector, log = PopupFrames(), FakeInjector(), FakeLog()
        wire = scene.WireTapLog(Path(tempfile.gettempdir()) / "absent-wire.log")
        with tempfile.TemporaryDirectory() as tmp:
            wire.path = Path(tmp) / "wire.log"
            wait = scene.RoomPaintWait(frames, injector, wire, log, "none")
            self.assertFalse(wait.run(0.1))
        self.assertEqual([], [c for c in injector.calls if c[0] == "press"])
        self.assertFalse(any(c[0] == "click" for c in injector.calls))
        self.assertTrue(any("popup_up=True" in line for line in log.events))


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
