"""Scenario-named seam guards for the #339 matched-state A/B kit.

The live boots are the proof; these tests pin the command encoding, trace
schema and wire-state extraction that make the two legs comparable.
"""
from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS_DIR.parent


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


host = load_module("matched_ab_host", SCRIPTS_DIR / "wine-room-host.py")
driver = load_module("matched_ab_replay", SCRIPTS_DIR / "replay-trace-native.py")
scenario = load_module("matched_ab_scenario", SCRIPTS_DIR / "matched-ab-scenario.py")


class HostStateCommandTests(unittest.TestCase):
    """Guard that a member's music state rides real gateway opcodes."""

    def test_member_music_list_encodes_count_then_ids(self) -> None:
        opcode, body = host.parse_scenario_command("music-list=100,200")
        self.assertEqual(opcode, 2024)
        self.assertEqual(body, b"\x02\x00\x00\x00\x64\x00\xc8\x00")

    def test_downloading_state_and_sync_use_the_server_broadcast(self) -> None:
        state_opcode, state_body = host.parse_scenario_command("music-state=4")
        sync_opcode, sync_body = host.parse_scenario_command("sync-member=0")
        self.assertEqual((state_opcode, state_body), (4025, b"\x04"))
        self.assertEqual((sync_opcode, sync_body), (4036, b"\x00"))

    def test_unknown_state_command_is_rejected(self) -> None:
        with self.assertRaises(ValueError):
            host.parse_scenario_command("delete=all")


class SharedTraceTests(unittest.TestCase):
    """Guard the trace controls that keep both clients in one state."""

    def test_native_driver_accepts_state_shots_and_marker_waits(self) -> None:
        trace = json.loads(
            (REPO_ROOT / "fixtures" / "input-trace" / "matched-ab-339.json").read_text()
        )
        self.assertEqual(driver.validate(trace), [])
        names = [event.get("name") for event in trace["events"]
                 if event["control"] == "state-shot"]
        self.assertEqual(names[0], "channel")

    def test_rig_owned_boot_can_omit_the_null_sound_fence(self) -> None:
        options = driver.ReplayOptions(
            binary=Path("/bin/recomp2jam"), online=True, host="127.0.0.1",
            port=15010, user_id=1, token="test", gateway_id=0, channel_id=0,
            music_id=757, frames_dir=Path("/tmp/frames"),
            input_log=Path("/tmp/input.log"), silent=False,
        )
        command = driver.build_client_command(options)
        self.assertNotIn("--no-sound", command)
        self.assertIn("--frame-dump", command)

    def test_state_shot_name_reaches_the_applied_log(self) -> None:
        event = {"sequence": 3, "control": "state-shot", "action": "capture",
                 "name": "base-ready", "time_ms": 10}
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "applied.log"
            driver.record_applied_event([], log, event, "captured")
            self.assertIn("name=base-ready status=captured", log.read_text())


class WireProofTests(unittest.TestCase):
    """Guard that the state proof ignores process-specific frames only."""

    def test_scenario_trace_keeps_state_opcodes_and_drops_auth_noise(self) -> None:
        frames = [
            scenario.WireFrame("C2S", 1000, "", "12:00:00"),
            scenario.WireFrame("C2S", 2024, "02000000", "12:00:01"),
            scenario.WireFrame("S2C", 4037, "0004", "12:00:02"),
            scenario.WireFrame("C2S", 6001, "", "12:00:03"),
        ]
        self.assertEqual(
            scenario.scenario_trace(frames),
            ["C2S:2024", "S2C:4037"],
        )

    def test_wire_window_uses_the_leg_clock_bounds(self) -> None:
        frames = [
            scenario.WireFrame("C2S", 2024, "", "12:00:10"),
            scenario.WireFrame("S2C", 4037, "", "12:00:20"),
        ]
        self.assertEqual(scenario.wire_window(frames), ("12:00:10", "12:00:20"))


if __name__ == "__main__":
    unittest.main()
