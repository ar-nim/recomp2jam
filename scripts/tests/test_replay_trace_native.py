"""Scenario-named seam tests for the committed native replay driver (#334).

The qualified proof is the live windowed walk; these guards pin the launch
contract that keeps it on native pixels, private X11, and null sound.
"""
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
DRIVER_PATH = SCRIPTS_DIR / "replay-trace-native.py"


def load_driver():
    """Import the hyphenated driver module for seam tests."""
    spec = importlib.util.spec_from_file_location("replay_trace_native", DRIVER_PATH)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class ReplayLaunchTests(unittest.TestCase):
    """Guard containment and frame-dump plumbing without spawning a client."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.driver = load_driver()

    def options(self, *, online: bool) -> object:
        return self.driver.ReplayOptions(
            binary=Path("/bin/recomp2jam"),
            online=online,
            host="127.0.0.1",
            port=15010,
            user_id=3,
            token="secret",
            gateway_id=0,
            channel_id=0,
            music_id=757,
            frames_dir=Path("/scratch/run/frames"),
            input_log=Path("/scratch/run/input.log"),
        )

    def test_online_command_keeps_null_sound_and_native_frame_dump(self) -> None:
        command = self.driver.build_client_command(self.options(online=True))
        self.assertEqual(command[1], "online")
        self.assertIn("--no-sound", command)
        self.assertIn("--frame-dump", command)
        self.assertIn("/scratch/run/frames", command)
        self.assertIn("--input-log", command)
        self.assertIn("--token", command)

    def test_offline_command_stays_windowed_and_silent(self) -> None:
        command = self.driver.build_client_command(self.options(online=False))
        self.assertEqual(command[1:3], ["play", "--window"])
        self.assertIn("--no-sound", command)
        self.assertNotIn("--token", command)

    def test_driver_owned_boot_wraps_client_in_the_live_lane_lock(self) -> None:
        command = self.driver.build_client_command(self.options(online=False))
        wrapped = self.driver.wrap_in_live_lock(command, ":350")
        self.assertEqual(wrapped[1:3], ["run", "--owner"])
        self.assertIn("replay-trace-native:350", wrapped[3])
        self.assertEqual(wrapped[-len(command):], command)

    def test_child_environment_removes_wayland_and_pins_x11(self) -> None:
        with patch.dict(
            "os.environ",
            {"WAYLAND_DISPLAY": "wayland-1", "WAYLAND_SOCKET": "/tmp/wayland-1"},
        ):
            env = self.driver.child_environment(":350")
        self.assertNotIn("WAYLAND_DISPLAY", env)
        self.assertNotIn("WAYLAND_SOCKET", env)
        self.assertEqual(env["WINIT_UNIX_BACKEND"], "x11")
        self.assertEqual(env["DISPLAY"], ":350")

    def test_gateway_without_port_uses_the_memoryer_default(self) -> None:
        self.assertEqual(self.driver.parse_gateway("127.0.0.1"), ("127.0.0.1", 15010))

    def test_manifest_records_the_replay_command_for_provenance(self) -> None:
        source = DRIVER_PATH.read_text()
        self.assertIn('"command": command', source)

    def test_injection_delay_never_rewinds_the_trace_timeline(self) -> None:
        self.assertEqual(self.driver.injection_delay(1000, 500, 0.2), 0.3)
        self.assertEqual(self.driver.injection_delay(1000, 500, 0.8), 0.0)

    def test_lane_events_select_the_playing_scene_prelude(self) -> None:
        trace = {"events": [{"control": "menu-confirm", "action": "press"}]}
        self.assertFalse(self.driver.trace_has_lane_events(trace))
        trace["events"].append({"control": "lane-3", "action": "press"})
        self.assertTrue(self.driver.trace_has_lane_events(trace))

    def test_applied_event_record_preserves_scenario_and_status(self) -> None:
        event = {
            "sequence": 7,
            "control": "await-screen",
            "action": "reach",
            "time_ms": 400,
            "screen": "planet",
        }
        applied = []
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "applied.log"
            entry = self.driver.record_applied_event(applied, log, event, "accepted")
            self.assertEqual(entry["sequence"], 7)
            self.assertIn("status=accepted", log.read_text())
            self.assertEqual(len(applied), 1)


class CaptureLogSeamTests(unittest.TestCase):
    """Guard screen sync from the driver's own applied-event log."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.driver = load_driver()

    def test_latest_frame_line_sets_the_current_screen(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.log"
            path.write_text(
                "event 1 key-down KeyS\n"
                "frame 39 wall_ms=100 screen=planet state=aa content=bb\n"
                "frame 57 wall_ms=200 screen=channel state=cc content=dd\n"
            )
            log = self.driver.CaptureLog(path)
            self.assertEqual(log.current_screen(), "channel")

    def test_trace_validation_rejects_an_unreleased_lane_press(self) -> None:
        trace = {
            "schema_version": "recomp2jam-input-v1",
            "events": [{"sequence": 0, "time_ms": 0, "control": "lane-0", "action": "press"}],
        }
        errors = self.driver.validate(trace)
        self.assertTrue(any("never released" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
