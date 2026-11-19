"""Scenario-named seam tests for the one-live-lane flock (#489).

The lock is machine-wide in production; these tests point it at a private
state dir so they never block a real boot or touch a real owner file.
"""
from __future__ import annotations

import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
LOCK_SCRIPT = SCRIPTS_DIR / "lib" / "rig-live-lock.sh"


class RigLiveLockTests(unittest.TestCase):
    """Guard serialization, owner reporting, timeout, and exit release."""

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.state = Path(self.temp.name)
        self.env = dict(os.environ, RIG_LIVE_LOCK_DIR=str(self.state / "lock"))

    def launch_holder(
        self, owner: str, command: str, extra_env: dict[str, str] | None = None
    ) -> subprocess.Popen[str]:
        process = subprocess.Popen(
            [str(LOCK_SCRIPT), "run", "--owner", owner, "--", "bash", "-c", command],
            env=dict(self.env, **(extra_env or {})),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.addCleanup(self.stop_holder, process)
        lock_file = self.state / "lock" / "lane.lock"
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if lock_file.is_file() and lock_file.read_text(errors="replace").strip() == owner:
                return process
            time.sleep(0.02)
        self.fail(f"lock owner was never written for {owner}")

    @staticmethod
    def stop_holder(process: subprocess.Popen[str]) -> None:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
        for stream in (process.stdout, process.stderr):
            if stream is not None:
                stream.close()

    def run_with_lock(self, owner: str, command: list[str], timeout: str = "3"):
        return subprocess.run(
            [str(LOCK_SCRIPT), "run", "--owner", owner, "--timeout", timeout, "--", *command],
            env=self.env,
            text=True,
            capture_output=True,
            timeout=15,
        )

    def test_competing_acquire_serializes_and_names_the_current_owner(self) -> None:
        log = self.state / "order.log"
        self.env["ORDER_LOG"] = str(log)
        first = subprocess.Popen(
            [str(LOCK_SCRIPT), "run", "--owner", "lane-A", "--", "bash", "-c",
             "echo A-start >> \"$ORDER_LOG\"; sleep 2; echo A-end >> \"$ORDER_LOG\""],
            env=self.env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        self.addCleanup(self.stop_holder, first)
        lock_file = self.state / "lock" / "lane.lock"
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline and (
            not log.exists() or lock_file.read_text(errors="replace").strip() != "lane-A"
        ):
            time.sleep(0.02)
        second = self.run_with_lock(
            "lane-B", ["bash", "-c", "echo B-start >> \"$ORDER_LOG\"; echo B-end >> \"$ORDER_LOG\""]
        )

        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertIn("live lane held by lane-A, retrying", second.stderr)
        first.wait(timeout=5)
        self.assertEqual(
            log.read_text().splitlines(), ["A-start", "A-end", "B-start", "B-end"]
        )

    def test_lock_releases_when_the_wrapper_exits(self) -> None:
        holder = self.launch_holder("lane-A", "sleep 30")
        holder.terminate()
        holder.wait(timeout=5)

        next_boot = self.run_with_lock("lane-B", ["true"])

        self.assertEqual(next_boot.returncode, 0, next_boot.stderr)
        self.assertNotIn("retrying", next_boot.stderr)

    def test_child_shell_reuses_the_inherited_lock_without_waiting(self) -> None:
        nested_result = self.state / "nested.rc"
        holder = self.launch_holder(
            "lane-A",
            '. "$LOCK_LIB"; rig_live_lock_acquire nested 1; echo $? > "$NESTED_RC"; sleep 3',
            extra_env={"LOCK_LIB": str(LOCK_SCRIPT), "NESTED_RC": str(nested_result)},
        )
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline and not nested_result.exists():
            time.sleep(0.02)

        self.assertTrue(nested_result.exists(), "nested acquire never returned")
        self.assertEqual(nested_result.read_text().strip(), "0")
        self.assertEqual((self.state / "lock" / "lane.lock").read_text().strip(), "lane-A")
        self.assertIsNone(holder.poll())
        holder.terminate()
        holder.wait(timeout=5)
        self.assertIn("inherited", holder.stderr.read())
        self.assertNotIn("retrying", holder.stderr.read())

    def test_acquire_times_out_without_taking_a_live_lane(self) -> None:
        holder = self.launch_holder("lane-A", "sleep 4")

        denied = self.run_with_lock("lane-B", ["true"], timeout="1")

        self.assertEqual(denied.returncode, 17)
        self.assertIn("live lane held by lane-A, retrying", denied.stderr)
        self.assertEqual((self.state / "lock" / "lane.lock").read_text().strip(), "lane-A")
        self.assertIsNone(holder.poll())


if __name__ == "__main__":
    unittest.main()
