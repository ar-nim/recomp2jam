"""Gate arm orchestration with a stub cargo (issues #737, #741).

These tests prove which commands the gate issues without running the
suite: a fresh stamp keeps the full sweep out, an absent stamp puts it
in, and the hidden arm refuses to run when its data probe sees archives.
"""
from __future__ import annotations

import hashlib
import os
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
GATE_SCRIPT = SCRIPTS_DIR / "gate-check.sh"
PRESENT_CANARY_OK = (
    "test live_art::live_art_tests::"
    "live_matches_declaration_on_walked_screens_when_archives_present ... ok"
)
REQUIRED_MARKER = "GATE_REQUIRE_ARCHIVES=1"
UNSET_MARKER = "GATE_REQUIRE_ARCHIVES=unset"


class GateArmTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        (self.repo / "scripts" / "lib").mkdir(parents=True)
        (self.repo / "fixtures").mkdir()
        shutil.copy(GATE_SCRIPT, self.repo / "scripts" / "gate-check.sh")
        shutil.copy(
            SCRIPTS_DIR / "check-guard-evidence.sh",
            self.repo / "scripts" / "check-guard-evidence.sh",
        )
        for lib in (SCRIPTS_DIR / "lib").glob("*.sh"):
            shutil.copy(lib, self.repo / "scripts" / "lib" / lib.name)
        guards = self.repo / "crates" / "fixture" / "guards.rs"
        guards.parent.mkdir(parents=True)
        guards.write_text("#[test]\nfn computes_expected_score() {}\n", encoding="utf-8")

        music = self.repo / "install-root" / "O2Jam" / "Music"
        music.mkdir(parents=True)
        chart = b"sample chart bytes"
        (music / "o2ma100.ojn").write_bytes(chart)
        digest = hashlib.sha256(chart).hexdigest()
        (self.repo / "fixtures" / "full-pack-sample.sha256").write_text(
            f"o2ma100.ojn {digest}\n", encoding="utf-8"
        )

        self.bin = self.repo / "bin"
        self.bin.mkdir()
        (self.bin / "cargo").write_text(
            "#!/usr/bin/env bash\n"
            "printf 'GATE_REQUIRE_ARCHIVES=%s :: %s\\n' \"${GATE_REQUIRE_ARCHIVES-unset}\" "
            "\"$*\" >> \"$FAKE_CARGO_LOG\"\n"
            "if [ -n \"${FAKE_CANARY_LINE:-}\" ]; then\n"
            "  printf '%s\\n' \"$FAKE_CANARY_LINE\"\n"
            "fi\n",
            encoding="utf-8",
        )
        (self.bin / "cargo").chmod(0o755)
        self.log = self.repo / "cargo.log"
        self.env = dict(os.environ)
        self.env.update(PATH=f"{self.bin}:{self.env['PATH']}", FAKE_CARGO_LOG=str(self.log))
        self.env.pop("GATE_REQUIRE_ARCHIVES", None)
        self.env.pop("FAKE_CANARY_LINE", None)

    def run_gate(
        self, *args: str, fake_canary: bool = False
    ) -> subprocess.CompletedProcess[str]:
        env = dict(self.env)
        if fake_canary:
            env["FAKE_CANARY_LINE"] = PRESENT_CANARY_OK
        return subprocess.run(
            ["bash", str(self.repo / "scripts" / "gate-check.sh"), "--no-push", *args],
            capture_output=True,
            text=True,
            env=env,
            cwd=self.repo,
            timeout=60,
        )

    def workspace_suite_markers(self) -> list[str]:
        lines = self.log.read_text(encoding="utf-8").splitlines()
        return [
            line.split(" :: ", 1)[0]
            for line in lines
            if "test --workspace" in line
        ]

    def test_absent_stamp_runs_the_full_sweep_then_stamps(self) -> None:
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.log.read_text(encoding="utf-8")
        self.assertIn("test -p o2jam-file real_ojn_covers_extract_and_decode", calls)
        self.assertIn("test -p o2jam-file --test ojn_bit26_sweep", calls)
        self.assertTrue((self.repo / ".gate-full-sweep-stamp").is_file())

    def test_fresh_stamp_keeps_the_full_sweep_out(self) -> None:
        stamp = self.repo / ".gate-full-sweep-stamp"
        stamp.write_text("fresh\n", encoding="utf-8")
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.log.read_text(encoding="utf-8")
        self.assertNotIn("test -p o2jam-file", calls)

    def test_old_stamp_runs_the_full_sweep_again(self) -> None:
        stamp = self.repo / ".gate-full-sweep-stamp"
        stamp.write_text("old\n", encoding="utf-8")
        old = time.time() - 8 * 24 * 60 * 60
        os.utime(stamp, (old, old))
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("real_ojn_covers_extract_and_decode", self.log.read_text(encoding="utf-8"))

    def test_unit_suite_excludes_the_real_window_click_proof(self) -> None:
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.log.read_text(encoding="utf-8")
        self.assertIn("--skip x11_arm_routes_click_through_real_game", calls)

    def test_rig_lane_names_the_lock_and_the_real_window_proof(self) -> None:
        script = SCRIPTS_DIR / "rig-x11-click-proof.sh"
        result = subprocess.run(
            ["bash", str(script), "--dry-run", "--timeout", "7"],
            capture_output=True,
            text=True,
            env=dict(os.environ),
            cwd=self.repo,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("owner=x11-click-proof timeout=7", result.stdout)
        self.assertIn(
            "cargo test -p input-drive x11_arm_routes_click_through_real_game -- --exact",
            result.stdout,
        )

    def test_hidden_arm_stops_when_the_probe_reports_reachable_data(self) -> None:
        # The probe itself is unit-tested; here we prove the gate CALLS it.
        probe_lib = self.repo / "scripts" / "lib" / "data-root.sh"
        probe_log = self.repo / "probe.log"
        with probe_lib.open("a", encoding="utf-8") as handle:
            handle.write(
                "\nassert_hidden_data_unreachable() { "
                f'echo probe >> "{probe_log}"; return 1; }}\n'
            )
        result = self.run_gate()
        self.assertEqual(result.returncode, 1)
        self.assertTrue(probe_log.is_file())

    def test_full_arm_requires_archives_only_in_the_present_suite(self) -> None:
        result = self.run_gate(fake_canary=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.workspace_suite_markers(), [REQUIRED_MARKER, UNSET_MARKER]
        )

    def test_quick_arm_leaves_archive_requirement_unset(self) -> None:
        result = self.run_gate("--arm", "quick", fake_canary=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.workspace_suite_markers(), [UNSET_MARKER])

    def test_hidden_arm_leaves_archive_requirement_unset(self) -> None:
        result = self.run_gate("--arm", "hidden", fake_canary=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.workspace_suite_markers(), [UNSET_MARKER])


if __name__ == "__main__":
    unittest.main()
