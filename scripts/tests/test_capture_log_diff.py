"""Scenario tests for the capture-log A/B diff CLI.

This is a CLI seam test: it pins the t260 log-normalization contract, not a
player-visible behavior reached from boot. A/B notes supply the live logs.
The golden pair is optional so the suite also runs without scratch evidence.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
DIFF = SCRIPTS / "capture-log-diff.py"

BASE_LOG = (
    "frame 1 wall_ms=34 screen=planet state=aaaa1111 content=bbbb2222\n"
    "event 15429 text s\n"
    "event 15429 key-down KeyS\n"
    "judged 15429 lane 0\n"
    "frame 2 wall_ms=15517 screen=planet state=aaaa1111 content=bbbb2222\n"
)


class CaptureLogDiffTests(unittest.TestCase):
    def setUp(self) -> None:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.directory = Path(temp.name)
        self.before = self.directory / "before.log"
        self.after = self.directory / "after.log"

    def write_logs(self, before_text: str, after_text: str) -> None:
        self.before.write_text(before_text, encoding="utf-8")
        self.after.write_text(after_text, encoding="utf-8")

    def run_diff(
        self,
        *options: str,
        before: Path | None = None,
        after: Path | None = None,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(DIFF), str(before or self.before),
             str(after or self.after), *options],
            capture_output=True,
            text=True,
            check=False,
        )

    def changed_lines(self, result: subprocess.CompletedProcess[str]) -> list[str]:
        return [
            line for line in result.stdout.splitlines()
            if line.startswith(("+", "-")) and not line.startswith(("+++", "---"))
        ]

    def golden_logs(self) -> tuple[Path, Path]:
        before = Path.home() / "scratch/p260-ab/before/input.log"
        after = Path.home() / "scratch/p260-ab/after/input.log"
        if not (before.is_file() and after.is_file()):
            self.skipTest("t260 golden logs are not installed")
        return before, after

    def test_wall_stamps_and_hashes_normalize_to_equal_logs(self) -> None:
        shifted = (BASE_LOG.replace("frame 1 wall_ms=34", "frame 7 wall_ms=900")
                   .replace("state=aaaa1111", "state=cafe1234")
                   .replace("content=bbbb2222", "content=dead5678"))
        self.write_logs(BASE_LOG, shifted)

        result = self.run_diff()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "")

    def test_t260_normalization_keeps_every_line_and_replaces_any_hex_field(self) -> None:
        self.write_logs(
            "event 12 text s\n"
            "judged 34 lane 0\n"
            "frame 5 wall_ms=6 screen=planet state=ABCDEF content=abc123\n"
            "note 7 state=ff00ff\n",
            "event T text s\n"
            "judged T lane 0\n"
            "frame F wall_ms=T screen=planet state=ABCDEF content=HASH\n"
            "note 7 state=HASH\n",
        )

        result = self.run_diff()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "")

    def test_default_mode_compares_frame_lines(self) -> None:
        self.write_logs(BASE_LOG, BASE_LOG.replace("screen=planet", "screen=room", 1))

        result = self.run_diff()

        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(
            self.changed_lines(result),
            ["-frame F wall_ms=T screen=planet state=HASH content=HASH",
             "+frame F wall_ms=T screen=room state=HASH content=HASH"],
        )

    def test_full_flag_selects_the_default_mode(self) -> None:
        self.write_logs(BASE_LOG, BASE_LOG.replace("screen=planet", "screen=room", 1))

        implicit = self.run_diff()
        explicit = self.run_diff("--full")

        self.assertEqual(explicit.returncode, 1, explicit.stderr)
        self.assertEqual(explicit.stdout, implicit.stdout)

    def test_judged_first_mode_ignores_a_changed_frame_line(self) -> None:
        self.write_logs(BASE_LOG, BASE_LOG.replace("screen=planet", "screen=room", 1))

        result = self.run_diff("--judged-first")

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "")

    def test_judged_first_mode_detects_a_changed_judged_lane(self) -> None:
        self.write_logs(BASE_LOG, BASE_LOG.replace("judged 15429 lane 0", "judged 15429 lane 3"))

        result = self.run_diff("--judged-first")

        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(
            self.changed_lines(result), ["-judged T lane 0", "+judged T lane 3"]
        )

    def test_golden_t260_pair_is_equal_in_judged_first_mode(self) -> None:
        before, after = self.golden_logs()

        result = self.run_diff("--judged-first", before=before, after=after)

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "")

    def test_golden_t260_pair_has_only_three_frame_shifts_in_full_mode(self) -> None:
        before, after = self.golden_logs()

        result = self.run_diff("--full", before=before, after=after)

        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(result.stdout.count("@@ "), 3)
        changed = self.changed_lines(result)
        self.assertEqual(len(changed), 4)
        self.assertTrue(all(line.lstrip("+-").startswith("frame ") for line in changed))
        self.assertFalse(any("event " in line or "judged " in line for line in changed))

    def test_missing_log_exits_two_with_the_path(self) -> None:
        self.write_logs(BASE_LOG, BASE_LOG)
        missing = self.directory / "missing.log"

        result = self.run_diff(before=missing)

        self.assertEqual(result.returncode, 2)
        self.assertIn(str(missing), result.stderr)

    def test_conflicting_modes_exit_two(self) -> None:
        self.write_logs(BASE_LOG, BASE_LOG)

        result = self.run_diff("--judged-first", "--full")

        self.assertEqual(result.returncode, 2)
        self.assertIn("not allowed with argument", result.stderr)


if __name__ == "__main__":
    unittest.main()
