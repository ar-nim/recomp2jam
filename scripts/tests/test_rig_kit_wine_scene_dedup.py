"""Guards for distinct observation PNGs in the Wine drive.

A scene that appears, changes, then returns must not write a duplicate
screenshot: ``run-manifest.sh verify`` rejects a pack with repeated images.
Both live chains are named on their test classes; no display or exe is used.
"""
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import numpy as np

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
MODULE_PATH = SCRIPTS_DIR / "wine-scene.py"
_spec = importlib.util.spec_from_file_location("wine_scene_dedup_gate", MODULE_PATH)
scene = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
sys.modules[_spec.name] = scene
_spec.loader.exec_module(scene)


class RepeatingFrames:
    """A, B, A pixels; capture() saves the frame that grab() just returned."""

    def __init__(self) -> None:
        self.frames = [np.full((8, 8, 3), value, dtype=np.int16)
                       for value in (10, 20, 10)]
        self.index = 0
        self.last = self.frames[0]
        self.saved: list[str] = []

    def grab(self):
        frame = self.frames[min(self.index, len(self.frames) - 1)]
        self.index += 1
        self.last = frame
        return frame

    def capture(self, path: Path) -> float:
        self.saved.append(path.name)
        return 0.0

    def scene_label(self) -> str:
        return "waiting"

    def mean_luma(self) -> float:
        return 0.0


class QuietWire:
    def opcodes_since(self, opcode: int) -> list[str]:
        return []

    def saw(self, opcode: int) -> bool:
        return False


class FakeLog:
    def __init__(self) -> None:
        self.events: list[str] = []

    def event(self, message: str) -> None:
        self.events.append(message)


class WaitingObservationDedupGuard(unittest.TestCase):
    """Live chain: rig-wine-run.sh::main -> phase_drive -> wine-scene.py::drive
    -> observe_waiting_transition. The observation log is the whole behavior.
    """

    def test_returning_waiting_frame_is_not_saved_twice(self) -> None:
        frames, log = RepeatingFrames(), FakeLog()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"), patch.object(
                scene.time, "time", side_effect=(0, 0, 1, 1.1, 2, 2.1, 3, 3.1)
            ):
                scene.observe_waiting_transition(
                    frames, QuietWire(), log, Path(tmp), 3.0)
        self.assertEqual(["waiting-observe-001.png", "waiting-observe-002.png"],
                         frames.saved)


class StartCueDedupGuard(unittest.TestCase):
    """Live chain: rig-wine-run.sh::main -> phase_drive -> wine-scene.py::drive
    -> observe_start_cue -> StartCueObserver.poll.
    """

    def test_returning_waiting_frame_is_not_saved_twice(self) -> None:
        frames, log = RepeatingFrames(), FakeLog()
        with tempfile.TemporaryDirectory() as tmp:
            observer = scene.StartCueObserver(
                frames, QuietWire(), None, None, log, Path(tmp))
            for elapsed in (0.0, 1.0, 2.0):
                observer.poll(elapsed)
        self.assertEqual(["waiting-cue-001.png", "waiting-cue-002.png"],
                         frames.saved)
        self.assertEqual(2, observer.captures)


if __name__ == "__main__":
    unittest.main()
