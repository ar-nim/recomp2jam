"""Scenario tests for the one blessed wave driver.

The replay leg consumes cited #582 evidence offline: it proves the recipe
drives the same clicks, cues, and evidence strings as the forked driver.
The live SendInput seam is checked as a command builder (no Wine boot here).
"""

from __future__ import annotations

import importlib.util
import json
import socket
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS / "lib"))
sys.path.insert(0, str(SCRIPTS / "tests"))

from test_wire_frames import CHANNEL_BODY, REWRITTEN_CHANNEL_BODY  # noqa: E402
from wave_recipe import load_recipe  # noqa: E402
from wire_frames import encode_frame  # noqa: E402

DRIVER = SCRIPTS / "rig-wave.py"
RECIPE = SCRIPTS / "recipes" / "row-vanish-582.json"

LEAVE_LINE = "15:40:35 C2S opcode=3005 (0BBD) len=6 hex=000000000000"
REMOVAL_LINE = "15:40:35 S2C opcode=2007 (07D7) len=4 hex=00000000"
BEFORE_TAP = (
    "15:40:33 S2C opcode=2003 (07D3) len=2420 hex="
    "640000000000000001525635383244495342414e4400006103"
)
AFTER_TAP = (
    "15:40:39 S2C opcode=2003 (07D3) len=2404 hex="
    "64000000000000000000000000000200010000000000000000"
)
TAP = "\n".join([LEAVE_LINE, REMOVAL_LINE, BEFORE_TAP, AFTER_TAP]) + "\n"

EXPECTED_DRIVER_LINES = (
    "15:40:35 BACK pre-probe blue=0.811 label=waiting",
    "15:40:35 BACK post-probe blue=0.811 label=waiting",
    "15:40:35 SendInput BACK at (751, 580); cue=W8 wa-pick-okback2 C2S 3005 body 0000",
    "15:40:37 post-BACK label=room",
)
EXPECTED_EVIDENCE_LINES = (
    "[15:40:33] 2003 BEFORE: count=100 titles=['RV582DISBAND']",
    LEAVE_LINE,
    REMOVAL_LINE,
    "[15:40:39] 2003 AFTER: count=100 titles=[]",
)


def load_driver_module() -> object:
    spec = importlib.util.spec_from_file_location("rig_wave", DRIVER)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load {DRIVER}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class RigWaveTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.tap = self.directory / "tap.log"
        self.tap.write_text(TAP, encoding="utf-8")
        self.out = self.directory / "out"

    def run_driver(self, *, tap: Path | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(DRIVER), "--recipe", str(RECIPE),
             "--mode", "replay", "--frames", str(tap or self.tap),
             "--out", str(self.out)],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_replay_reproduces_the_cited_row_vanish_evidence(self) -> None:
        result = self.run_driver()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        driver_log = (self.out / "driver.log").read_text(encoding="utf-8").splitlines()
        evidence = (self.out / "evidence.log").read_text(encoding="utf-8").splitlines()
        self.assertEqual(driver_log, list(EXPECTED_DRIVER_LINES))
        self.assertEqual(evidence, list(EXPECTED_EVIDENCE_LINES))
        captures = json.loads((self.out / "captures.json").read_text(encoding="utf-8"))
        self.assertEqual(captures["before"]["full_sha256"],
                         "3ecfd8f55743b05b4233145514312c228694e755f5ba83741927d6aac58bdb23")
        self.assertEqual(captures["after"]["crops"]["row-zero"]["sha256"],
                         "503d758b175443f705bf5436b417d4a4fa73acaa44e3f9d27280063a647ee985")

    def test_replay_names_the_missing_wire_cue_and_refuses_to_claim_evidence(self) -> None:
        quiet_tap = self.directory / "quiet.log"
        quiet_tap.write_text(TAP.replace(LEAVE_LINE + "\n", ""), encoding="utf-8")

        result = self.run_driver(tap=quiet_tap)

        self.assertEqual(result.returncode, 2, result.stdout)
        self.assertIn("missing wire cue leave-3005", result.stderr)
        self.assertFalse((self.out / "evidence.log").exists())

    def test_tap_applies_the_recipe_rule_to_s2c_frames(self) -> None:
        module = load_driver_module()
        rule = json.loads((SCRIPTS / "recipes" / "channel-ids-1003.json").read_text())
        upstream = socket.socket()
        upstream.bind(("127.0.0.1", 0))
        upstream.listen(1)
        upstream.settimeout(5)
        self.addCleanup(upstream.close)
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            listen_port = probe.getsockname()[1]
        tap = module.RewritingTap(listen_port, upstream.getsockname()[1],
                                  [rule], self.directory / "tap.log")
        tap.start()
        self.addCleanup(tap.stop)
        client = socket.create_connection(("127.0.0.1", listen_port), timeout=5)
        self.addCleanup(client.close)
        accepted, _ = upstream.accept()
        self.addCleanup(accepted.close)

        accepted.sendall(encode_frame(1003, CHANNEL_BODY))

        expected = encode_frame(1003, REWRITTEN_CHANNEL_BODY)
        forwarded = b""
        while len(forwarded) < len(expected):
            forwarded += client.recv(len(expected) - len(forwarded))
        self.assertEqual(forwarded, expected)
        self.assertIn("opcode=1003", (self.directory / "tap.log").read_text())

    def test_click_command_uses_the_sendinput_helper_not_xtest(self) -> None:
        module = load_driver_module()
        recipe = load_recipe(RECIPE)

        command = module.SendInputClicker.command(recipe.aim("back"))

        self.assertEqual(command[-4:], ["send", "click", "751", "580"])
        self.assertTrue(command[0].endswith("rig-wine.sh"))


if __name__ == "__main__":
    unittest.main()
