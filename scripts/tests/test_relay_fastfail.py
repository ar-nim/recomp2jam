"""Seam tests for the dead-relay fast-fail interposer (#321/#323/#324).

The qualified capture is the live proof (b8-relay-fastfail: 0BBA in the
same boot); these tests pin the kit contract at the seams the scripts own:
the interposer refuses ONLY the pinned relay target, the launch loads it
through LD_PRELOAD (prepend, never clobber), and the run records its
presence in the pack. A green test never substitutes for a qualified
capture (rules 7-8).
"""
from __future__ import annotations

import shutil
import subprocess
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
INTERPOSER_SOURCE = SCRIPTS_DIR / "rig-relay-fastfail.c"
WINE_SCRIPT = SCRIPTS_DIR / "rig-wine.sh"
RUNNER_PATH = SCRIPTS_DIR / "rig-wine-run.sh"


class InterposerSourceTest(unittest.TestCase):
    def test_source_pins_the_dead_relay_target_and_port(self) -> None:
        source = INTERPOSER_SOURCE.read_text()
        self.assertIn("203, 238, 136, 52", source)
        self.assertIn("0x1e, 0x6c", source)  # 7788, per exe site 0x59370b

    def test_source_compiles(self) -> None:
        if shutil.which("gcc") is None:
            self.skipTest("gcc not available")
        result = subprocess.run(
            ["gcc", "-fsyntax-only", str(INTERPOSER_SOURCE)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_refusal_is_scoped_to_the_relay_connect(self) -> None:
        source = INTERPOSER_SOURCE.read_text()
        self.assertIn("targets_dead_relay", source)
        self.assertIn("ECONNREFUSED", source)
        # Anything that is not the relay pair must reach the real connect.
        self.assertIn('dlsym(RTLD_NEXT, "connect")', source)


class LaunchInjectionTest(unittest.TestCase):
    def test_launch_preloads_the_built_interposer(self) -> None:
        source = WINE_SCRIPT.read_text()
        self.assertIn("build-relay-fastfail", source)
        self.assertIn("export_relay_fastfail", source)
        self.assertIn("LD_PRELOAD", source)
        # Prepend keeps a caller-supplied LD_PRELOAD alive; the escape
        # hatch keeps the falsification arm (b6 vs b8) reachable.
        self.assertIn('${LD_PRELOAD:+:$LD_PRELOAD}', source)
        self.assertIn('RIG_RELAY_FASTFAIL:-1', source)

    def test_launch_usage_names_the_new_subcommand(self) -> None:
        source = WINE_SCRIPT.read_text()
        self.assertIn(
            "{build-injector|build-relay-fastfail|launch|send|reset}", source
        )


class RunPackagingTest(unittest.TestCase):
    def test_run_exports_the_pack_scoped_interposer_log(self) -> None:
        source = RUNNER_PATH.read_text()
        self.assertIn('RELAY_FASTFAIL_LOG="$OUT_DIR/relay-fastfail.log"', source)
        self.assertIn("export RELAY_FASTFAIL_LOG", source)

    def test_run_builds_once_and_records_the_artifact(self) -> None:
        source = RUNNER_PATH.read_text()
        self.assertIn("ensure_relay_fastfail", source)
        self.assertIn("build-relay-fastfail", source)
        self.assertIn("relay_fastfail=$relay_sha", source)
        self.assertIn('relay_args=(--artifact relay-fastfail.log)', source)

    def test_manifest_params_report_the_interposer_state(self) -> None:
        source = RUNNER_PATH.read_text()
        self.assertIn('local relay_sha=off', source)


if __name__ == "__main__":
    unittest.main()
