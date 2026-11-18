"""Scenario tests for the paired-latency statistics CLI."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
STATS = SCRIPTS / "pair-stats.py"


class PairStatsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.injector = self.directory / "injector.log"
        self.judged = self.directory / "judged.log"
        # One lane per press; the judged clock starts at a wall-clock base.
        self.injector.write_text(
            "1000 lane-0\n1010 lane-1\n1020 lane-2\n", encoding="utf-8"
        )
        self.judged.write_text(
            "judged 1025 lane 0\njudged 1040 lane 1\njudged 1048 lane 2\n",
            encoding="utf-8",
        )
        self.stats = self.directory / "stats.json"

    def run_stats(self, *options: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(STATS), str(self.injector), str(self.judged),
             "--out", str(self.stats), *options],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_min_offset_removes_the_wall_clock_base(self) -> None:
        result = self.run_stats("--min-offset")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("offset_ms=25", result.stdout)
        document = json.loads(self.stats.read_text(encoding="utf-8"))
        self.assertEqual(document["injections"], 3)
        self.assertEqual(document["judged"], 3)
        self.assertEqual(document["offset_ms"], 25)
        self.assertEqual(document["p50_ms"], 3.0)
        self.assertEqual(document["max_ms"], 5.0)

    def test_raw_deltas_are_unchanged_without_normalization(self) -> None:
        result = self.run_stats()
        self.assertEqual(result.returncode, 0, result.stderr)
        document = json.loads(self.stats.read_text(encoding="utf-8"))
        self.assertEqual(document["offset_ms"], 0)
        self.assertEqual(document["p50_ms"], 28.0)

    def test_conflicting_normalizations_are_refused(self) -> None:
        result = self.run_stats("--relative", "--min-offset")
        self.assertEqual(result.returncode, 2)
        self.assertIn("cannot be combined", result.stderr)


if __name__ == "__main__":
    unittest.main()
