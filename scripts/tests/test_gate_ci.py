"""One check script for CI and the merge gate (issue #741).

Fake cargo proves which arms run and that --hidden is no longer a no-op.
The CI test pins that the workflow is a thin caller: no duplicated cargo
command list.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
REPO_DIR = SCRIPTS_DIR.parent
GATE_SCRIPT = SCRIPTS_DIR / "gate-check.sh"
CI_WORKFLOW = REPO_DIR / ".github" / "workflows" / "ci.yml"


class GateCiArmTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        (self.repo / "scripts" / "lib").mkdir(parents=True)
        (self.repo / "fixtures").mkdir()
        shutil.copy(GATE_SCRIPT, self.repo / "scripts" / "gate-check.sh")
        for lib in (SCRIPTS_DIR / "lib").glob("*.sh"):
            shutil.copy(lib, self.repo / "scripts" / "lib" / lib.name)
        (self.repo / "fixtures" / "full-pack-sample.sha256").write_text("", encoding="utf-8")

        self.bin = self.repo / "bin"
        self.bin.mkdir()
        (self.bin / "cargo").write_text(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$*\" >> \"$FAKE_CARGO_LOG\"\n",
            encoding="utf-8",
        )
        (self.bin / "cargo").chmod(0o755)
        self.log = self.repo / "cargo.log"
        self.env = dict(os.environ)
        self.env.update(PATH=f"{self.bin}:{self.env['PATH']}", FAKE_CARGO_LOG=str(self.log))

    def run_gate(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", str(self.repo / "scripts" / "gate-check.sh"), "--no-push", *args],
            capture_output=True,
            text=True,
            env=self.env,
            cwd=self.repo,
            timeout=60,
        )

    def calls(self) -> str:
        return self.log.read_text(encoding="utf-8")

    def test_quick_arm_runs_fmt_clippy_suite_and_build(self) -> None:
        result = self.run_gate("--arm", "quick")
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.calls()
        self.assertIn("fmt --check", calls)
        self.assertIn("clippy --workspace --all-targets", calls)
        self.assertIn("test --workspace", calls)
        self.assertIn("build --workspace", calls)
        self.assertNotIn("push", result.stdout)

    def test_hidden_arm_runs_the_data_probe_and_the_hidden_suite(self) -> None:
        probe_log = self.repo / "probe.log"
        with (self.repo / "scripts" / "lib" / "data-root.sh").open("a", encoding="utf-8") as handle:
            handle.write(
                "\nassert_hidden_data_unreachable() { "
                f'echo probe >> "{probe_log}"; return 0; }}\n'
            )
        result = self.run_gate("--arm", "hidden")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(probe_log.is_file())
        self.assertIn("test --workspace", self.calls())

    def test_hidden_flag_is_not_a_no_op(self) -> None:
        probe_log = self.repo / "probe.log"
        with (self.repo / "scripts" / "lib" / "data-root.sh").open("a", encoding="utf-8") as handle:
            handle.write(
                "\nassert_hidden_data_unreachable() { "
                f'echo probe >> "{probe_log}"; return 1; }}\n'
            )
        result = self.run_gate("--hidden")
        self.assertEqual(result.returncode, 1)
        self.assertTrue(probe_log.is_file())

    def test_full_arm_runs_both_suites_and_the_build(self) -> None:
        result = self.run_gate("--arm", "full")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls().count("test --workspace"), 2)
        self.assertIn("build --workspace", self.calls())

    def test_ci_is_a_thin_caller_of_the_two_arms(self) -> None:
        workflow = CI_WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("scripts/gate-check.sh --arm quick", workflow)
        self.assertIn("scripts/gate-check.sh --arm hidden", workflow)
        self.assertNotIn("run: cargo ", workflow)


if __name__ == "__main__":
    unittest.main()
