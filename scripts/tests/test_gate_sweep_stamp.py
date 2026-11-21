"""Full-pack sweep staleness (issue #737).

The per-merge gate must not walk 515 charts. These seam tests pin the
seven-day stamp rule; they use temporary stamps, never the real one.
"""
from __future__ import annotations

import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
SWEEP_STAMP_LIB = SCRIPTS_DIR / "lib" / "sweep-stamp.sh"


class GateSweepStampTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.stamp = Path(self.tmp.name) / ".gate-full-sweep-stamp"

    def is_stale(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", "-c", f'. "{SWEEP_STAMP_LIB}"; if sweep_stamp_is_stale "{self.stamp}"; then echo stale; else echo fresh; fi'],
            capture_output=True,
            text=True,
            env=dict(os.environ),
            timeout=30,
        )

    def age_stamp(self, seconds: int) -> None:
        self.stamp.write_text("swept\n", encoding="utf-8")
        old = time.time() - seconds
        os.utime(self.stamp, (old, old))

    def test_absent_stamp_requests_a_full_sweep(self) -> None:
        result = self.is_stale()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "stale")

    def test_fresh_stamp_skips_the_full_sweep(self) -> None:
        self.age_stamp(6 * 24 * 60 * 60)
        result = self.is_stale()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "fresh")

    def test_week_old_stamp_requests_a_full_sweep(self) -> None:
        self.age_stamp(8 * 24 * 60 * 60)
        result = self.is_stale()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "stale")


if __name__ == "__main__":
    unittest.main()
