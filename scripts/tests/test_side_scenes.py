"""Seam tests for the #330 side-scene drive stages.

The exe packs are the live proof; these tests pin only what the drive owns:
which aim fires first, which guard refuses a wrong frame, and that every
target ends on the scene its name promises (rules 7-8).
"""
from __future__ import annotations

import argparse
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import numpy as np

SCRIPTS = Path(__file__).resolve().parents[1]
MODULE_PATH = SCRIPTS / "wine-scene.py"
SPEC = importlib.util.spec_from_file_location("wine_scene_side", MODULE_PATH)
scene = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules["wine_scene_side"] = scene
SPEC.loader.exec_module(scene)


class FakeLog:
    def __init__(self) -> None:
        self.events: list[str] = []

    def event(self, message: str) -> None:
        self.events.append(message)

    def scene(self, name: str, mean: float, cue: str, shot: str) -> None:
        self.events.append(f"frame 1 screen={name} cue={cue} shot={shot}")


class FakeInjector:
    def __init__(self) -> None:
        self.calls: list[tuple] = []
        self.cited: set[str] = set()

    def cite(self, aim: str) -> tuple[int, int]:
        self.cited.add(aim)
        return 0, 0

    def send_click(self, aim: str) -> None:
        self.calls.append(("click", aim))

    def send_key(self, name: str, direction: str) -> None:
        self.calls.append(("key", name, direction))


class FakeFrames:
    """Frames that pass the guard only for the named scene."""

    def __init__(self, passing: bool = True) -> None:
        self.passing = passing
        self.saved: list[str] = []

    def grab(self) -> np.ndarray:
        return np.zeros((8, 8, 3), dtype=np.int16)

    def side_scene_is_up(self, name: str) -> bool:
        return self.passing

    def side_guard_fraction(self, name: str) -> float:
        return 0.75 if self.passing else 0.0

    def capture(self, path: Path) -> float:
        self.saved.append(path.name)
        return 0.0


class SideTargetPlanTest(unittest.TestCase):
    def test_each_side_target_ends_on_its_named_scene(self) -> None:
        for target, plan in scene.SIDE_PLANS.items():
            with self.subTest(target=target):
                self.assertEqual(target, plan.steps[-1].scene)
                self.assertTrue(plan.steps[-1].aim)

    def test_side_targets_are_accepted_by_the_cli(self) -> None:
        result = subprocess.run(
            [sys.executable, str(MODULE_PATH), "--help"],
            capture_output=True, text=True, check=True,
        )
        for target in ("options", "quit-confirm", "myroom", "itemshop",
                       "cash-charge", "musicshop", "tutorial", "messenger"):
            with self.subTest(target=target):
                self.assertIn(target, result.stdout)

    def test_every_side_aim_cites_live_boot_evidence(self) -> None:
        for plan in scene.SIDE_PLANS.values():
            for step in plan.steps:
                with self.subTest(aim=step.aim):
                    x, y, evidence = scene.AIMS[step.aim]
                    self.assertTrue(0 <= x <= 800 and 0 <= y <= 600)
                    self.assertIn("b1-explore", evidence)


class SideSceneGuardTest(unittest.TestCase):
    @staticmethod
    def frames_with(pixels: np.ndarray) -> scene.Frames:
        frames = scene.Frames(":299", Path(tempfile.mkdtemp()))
        frames.last = pixels
        return frames

    @staticmethod
    def painted(box: tuple[int, int, int, int], color: tuple[int, int, int]):
        frame = np.zeros((600, 800, 3), dtype=np.int16)
        left, top, right, bottom = box
        frame[top:bottom, left:right] = color
        return frame

    def test_options_orange_plate_is_recognized(self) -> None:
        box, _, _ = scene.SIDE_SIGNATURES["options"]
        frames = self.frames_with(self.painted(box, (241, 151, 52)))
        self.assertTrue(frames.side_scene_is_up("options"))

    def test_quit_confirm_green_tab_is_recognized(self) -> None:
        box, _, _ = scene.SIDE_SIGNATURES["quit-confirm"]
        frames = self.frames_with(self.painted(box, (90, 186, 16)))
        self.assertTrue(frames.side_scene_is_up("quit-confirm"))

    def test_blank_frame_is_not_labelled_a_side_scene(self) -> None:
        blank = np.zeros((600, 800, 3), dtype=np.int16)
        for name in scene.SIDE_SIGNATURES:
            with self.subTest(scene=name):
                self.assertFalse(self.frames_with(blank).side_scene_is_up(name))

    def test_blank_billing_page_still_shows_the_cash_charge_title(self) -> None:
        frame = np.zeros((600, 800, 3), dtype=np.int16)
        left, top, right, bottom = scene.CASH_TITLE_BOX
        frame[top:bottom, left:right] = (90, 110, 170)
        frames = self.frames_with(frame)
        self.assertTrue(frames.side_scene_is_up("cash-charge"))
        self.assertFalse(frames.side_scene_is_up("messenger"))

    def test_crash_report_box_is_detected_from_its_blue_title_and_grey_body(self) -> None:
        frame = np.zeros((600, 800, 3), dtype=np.int16)
        frame[245:265, 380:420] = (50, 150, 250)
        frame[280:300, 380:420] = (245, 245, 245)
        self.assertTrue(self.frames_with(frame).crash_report_is_up())

    def test_board_is_not_mistaken_for_a_crash_report(self) -> None:
        frame = np.zeros((600, 800, 3), dtype=np.int16)
        frame[245:265, 380:420] = (8, 0, 99)
        frame[280:300, 380:420] = (0, 0, 0)
        self.assertFalse(self.frames_with(frame).crash_report_is_up())


class SideSceneCaptureTest(unittest.TestCase):
    def test_unproven_side_scene_writes_no_screenshot(self) -> None:
        frames, log = FakeFrames(passing=False), FakeLog()
        with tempfile.TemporaryDirectory() as tmp:
            captured = scene.capture_side_scene(
                frames, log, Path(tmp), "options", "click:option-gear")
            self.assertFalse(captured)
            self.assertEqual([], list(Path(tmp).glob("*.png")))
        self.assertTrue(any("stage options NOT captured" in line
                            for line in log.events))

    def test_proven_side_scene_logs_the_click_cue_and_measured_fraction(self) -> None:
        frames, log = FakeFrames(), FakeLog()
        with tempfile.TemporaryDirectory() as tmp:
            captured = scene.capture_side_scene(
                frames, log, Path(tmp), "myroom", "click:room-myroom")
            self.assertTrue(captured)
            self.assertEqual(["myroom.png"], frames.saved)
        self.assertTrue(any("screen=myroom" in line and "click:room-myroom" in line
                            for line in log.events))
        self.assertTrue(any("myroom signature" in line and "0.750" in line
                            for line in log.events))


class SideSceneWalkTest(unittest.TestCase):
    def test_options_walk_repeats_the_gear_click_after_a_stale_report(self) -> None:
        log, injector = FakeLog(), FakeInjector()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"):
                reached = scene.execute_side_plan(
                    scene.SIDE_PLANS["options"], FakeFrames(),
                    injector, log, Path(tmp))
        self.assertEqual(["options"], reached)
        self.assertEqual([("click", "option-gear"),
                          ("click", "option-gear"),
                          ("key", "escape", "down"),
                          ("key", "escape", "up")], injector.calls)

    def test_item_shop_walk_visits_myroom_before_the_shop(self) -> None:
        log, injector = FakeLog(), FakeInjector()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"):
                reached = scene.execute_side_plan(
                    scene.SIDE_PLANS["itemshop"], FakeFrames(),
                    injector, log, Path(tmp))
        self.assertEqual(["myroom", "itemshop"], reached)
        self.assertEqual([("click", "room-myroom"),
                          ("click", "room-myroom"),
                          ("click", "myroom-itemshop"),
                          ("click", "myroom-itemshop")], injector.calls)

    def test_tutorial_walk_returns_from_the_shop_before_the_second_click(self) -> None:
        log, injector = FakeLog(), FakeInjector()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"):
                reached = scene.execute_side_plan(
                    scene.SIDE_PLANS["tutorial"], FakeFrames(),
                    injector, log, Path(tmp))
        self.assertEqual(["musicshop", "tutorial"], reached)
        self.assertEqual([("click", "room-musicshop"),
                          ("click", "room-musicshop"),
                          ("click", "side-back"),
                          ("click", "room-firststep"),
                          ("click", "room-firststep")], injector.calls)

    def test_a_failed_guard_stops_the_chain_without_a_false_scene(self) -> None:
        log, injector = FakeLog(), FakeInjector()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"):
                reached = scene.execute_side_plan(
                    scene.SIDE_PLANS["itemshop"], FakeFrames(passing=False),
                    injector, log, Path(tmp))
        self.assertEqual([], reached)
        self.assertEqual([("click", "room-myroom"),
                          ("click", "room-myroom")], injector.calls)

    def test_stale_crash_report_dismissal_uses_the_two_pinned_cancel_aims(self) -> None:
        log, injector = FakeLog(), FakeInjector()

        class ReportThenBoard:
            def __init__(self) -> None:
                self.checks = 0

            def grab(self) -> None:
                pass

            def crash_report_is_up(self) -> bool:
                self.checks += 1
                return self.checks == 1

        with patch.object(scene.time, "sleep"):
            dismissed = scene.dismiss_stale_crash_report(
                ReportThenBoard(), injector, log)
        self.assertTrue(dismissed)
        self.assertEqual([("click", "crash-cancel")], injector.calls)
        self.assertTrue(any("stale crash report" in line for line in log.events))


    def test_cash_chain_clears_the_messenger_popup_before_the_shop(self) -> None:
        log, injector = FakeLog(), FakeInjector()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"):
                reached = scene.execute_side_plan(
                    scene.SIDE_PLANS["cash-charge"], FakeFrames(),
                    injector, log, Path(tmp))
        self.assertEqual(["myroom", "messenger", "itemshop", "cash-charge"],
                         reached)
        clicks = [call[1] for call in injector.calls]
        self.assertEqual("side-dialog-ok", clicks[clicks.index("room-messenger") + 2])
        self.assertLess(clicks.index("side-dialog-ok"),
                        clicks.index("myroom-itemshop"))


class BoardPrimeTest(unittest.TestCase):
    def test_board_side_plans_prime_the_canvas_before_the_first_control(self) -> None:
        for target in ("options", "quit-confirm"):
            with self.subTest(target=target):
                plan = scene.SIDE_PLANS[target]
                self.assertEqual("board-prime", plan.prime_aim)
                self.assertFalse(scene.SIDE_PLANS["myroom"].prime_aim)

    def test_room_side_plans_do_not_prime_after_the_lobby_walk(self) -> None:
        for target in ("myroom", "itemshop", "cash-charge", "musicshop",
                       "tutorial", "messenger"):
            with self.subTest(target=target):
                self.assertEqual("", scene.SIDE_PLANS[target].prime_aim)


class ArgumentShapeTest(unittest.TestCase):
    def test_side_target_parser_keeps_the_existing_drive_defaults(self) -> None:
        arguments = scene.build_argument_parser().parse_args(
            ["--attach", "out", "--display", ":9", "--target", "tutorial"])
        self.assertEqual("tutorial", arguments.target)
        self.assertEqual("loading", scene.build_argument_parser().parse_args(
            ["--attach", "out", "--display", ":9"]).target)
        self.assertIsInstance(arguments, argparse.Namespace)


if __name__ == "__main__":
    unittest.main()
