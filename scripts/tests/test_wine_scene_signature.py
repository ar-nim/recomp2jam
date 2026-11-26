"""CLI selftest for the wine-scene scene-signature gate.

The review leg feeds a synthetic frame to wine-scene.py itself: a frame that
misses the measured channel signature must be refused with a nonzero exit,
and a frame that matches must be labelled. No Wine, X server, Memoryer or game
data is used.
"""
from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
WINE_SCENE = Path(os.environ.get(
    "WINE_SCENE_SCRIPT", str(SCRIPTS_DIR / "wine-scene.py")))
REFUSAL_LINE = (
    "stage channel NOT captured: screen shows 'unknown' cue=selftest:mismatch"
)


def run_signature_case(case: str, out_dir: Path) -> subprocess.CompletedProcess[str]:
    """Run one CLI selftest case; the script owns the refusal and the exit."""
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
    return subprocess.run(
        [sys.executable, str(WINE_SCENE), "--attach", str(out_dir),
         "--display", ":299", "--signature-selftest", case],
        capture_output=True, text=True, env=env, check=False,
    )


class SignatureGateSelftest(unittest.TestCase):
    def test_mismatched_frame_is_refused_with_nonzero_exit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run_signature_case("mismatch", Path(tmp))
            shots = list((Path(tmp) / "selftest-shots").glob("*.png"))
        self.assertEqual(1, result.returncode, result.stderr)
        self.assertIn(REFUSAL_LINE, result.stdout)
        self.assertEqual([], shots)

    def test_matching_frame_passes_the_gate_with_zero_exit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run_signature_case("match", Path(tmp))
            shot_written = (
                Path(tmp) / "selftest-shots" / "channel.png").is_file()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("frame 1 screen=channel", result.stdout)
        self.assertTrue(shot_written)


if __name__ == "__main__":
    unittest.main()
