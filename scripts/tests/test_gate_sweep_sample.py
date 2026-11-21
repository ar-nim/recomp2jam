"""Hash-pinned sample for the per-merge gate (issue #737).

The sample stands in for the 515-chart pack: it must fail loudly when a
pinned chart changes, and skip cleanly when no archives are installed.
"""
from __future__ import annotations

import hashlib
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
SAMPLE_LIB = SCRIPTS_DIR / "lib" / "sweep-sample.sh"


class GateSweepSampleTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        self.root = self.repo / "install-root"
        (self.root / "O2Jam" / "Music").mkdir(parents=True)
        self.manifest = self.repo / "sample.sha256"

    def verify(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                "bash",
                "-c",
                f'. "{SAMPLE_LIB}"; verify_sweep_sample "{self.root}" "{self.manifest}"',
            ],
            capture_output=True,
            text=True,
            env=dict(os.environ),
            cwd=self.repo,
            timeout=30,
        )

    def pin(self, name: str, payload: bytes) -> None:
        (self.root / "O2Jam" / "Music" / name).write_bytes(payload)
        digest = hashlib.sha256(payload).hexdigest()
        with self.manifest.open("a", encoding="utf-8") as handle:
            handle.write(f"{name} {digest}\n")

    def test_missing_archives_skip_cleanly(self) -> None:
        self.root = self.repo / "no-root"
        self.manifest.write_text("o2ma100.ojn deadbeef\n", encoding="utf-8")
        result = self.verify()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("skipped (no archives)", result.stdout)

    def test_matching_sample_passes(self) -> None:
        self.pin("o2ma100.ojn", b"chart bytes")
        result = self.verify()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_changed_chart_fails_with_its_name(self) -> None:
        self.pin("o2ma100.ojn", b"chart bytes")
        (self.root / "O2Jam" / "Music" / "o2ma100.ojn").write_bytes(b"tampered")
        result = self.verify()
        self.assertEqual(result.returncode, 1)
        self.assertIn("o2ma100.ojn", result.stderr)


if __name__ == "__main__":
    unittest.main()
