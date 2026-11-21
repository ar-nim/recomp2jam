"""Gate data-root contract (issue #736).

These seam tests pin the runtime root resolution and the hidden-arm probe.
They never touch the real install-root: every root is a temporary directory.
"""
from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
DATA_ROOT_LIB = SCRIPTS_DIR / "lib" / "data-root.sh"


class GateDataRootTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        self.env = dict(os.environ)
        self.env.pop("O2JAM_INSTALL_ROOT", None)

    def source(self, snippet: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", "-c", f'. "{DATA_ROOT_LIB}"; {snippet}'],
            capture_output=True,
            text=True,
            env=env or self.env,
            cwd=self.repo,
            timeout=30,
        )

    def test_root_uses_env_override_at_runtime(self) -> None:
        override = self.repo / "borrowed-root"
        override.mkdir()
        env = dict(self.env, O2JAM_INSTALL_ROOT=str(override))
        result = self.source("gate_data_root /checkout", env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), str(override))

    def test_root_falls_back_to_checkout_install_root(self) -> None:
        result = self.source("gate_data_root /checkout")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "/checkout/install-root")

    def test_hidden_probe_fails_when_the_hidden_root_still_exists(self) -> None:
        leftover = self.repo / ".install-root.gate-hidden"
        (leftover / "O2Jam").mkdir(parents=True)
        result = self.source(f'assert_hidden_data_unreachable "{leftover}" /absent')
        self.assertEqual(result.returncode, 1)
        self.assertIn("hidden data root still exists", result.stderr)

    def test_hidden_probe_fails_when_the_fallback_still_has_archives(self) -> None:
        fallback = self.repo / "install-root"
        (fallback / "O2Jam").mkdir(parents=True)
        result = self.source(
            f'assert_hidden_data_unreachable "{self.repo}/absent" "{fallback}"'
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("hidden arm can still reach fallback archives", result.stderr)

    def test_hidden_probe_accepts_dangling_link_and_missing_fallback(self) -> None:
        hidden = self.repo / ".install-root.gate-hidden"
        fallback = self.repo / "install-root"
        fallback.symlink_to(self.repo / "no-such-slot")
        result = self.source(f'assert_hidden_data_unreachable "{hidden}" "{fallback}"')
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_dangling_link_is_recognized_without_archive_presence(self) -> None:
        link = self.repo / "install-root"
        link.symlink_to(self.repo / "no-such-slot")
        result = self.source(
            f'if data_root_is_dangling_link "{link}"; then echo yes; else echo no; fi'
        )
        self.assertEqual(result.stdout.strip(), "yes")

    def test_dangling_data_links_are_hidden_then_restored(self) -> None:
        (self.repo / "install-root").symlink_to(self.repo / "no-such-slot")
        (self.repo / "extracted").symlink_to(self.repo / "no-such-slot")
        snippet = f"""
            hide_dangling_data_links "{self.repo}"
            find "{self.repo}" -maxdepth 1 -name '*gate-dangling' -print
            restore_dangling_data_links
        """
        result = self.source(snippet)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(".install-root.gate-dangling", result.stdout)
        self.assertIn(".extracted.gate-dangling", result.stdout)
        self.assertTrue((self.repo / "install-root").is_symlink())
        self.assertTrue((self.repo / "extracted").is_symlink())
        self.assertFalse((self.repo / ".install-root.gate-dangling").exists())

    def test_live_data_symlink_is_not_hidden(self) -> None:
        target = self.repo / "data"
        target.mkdir()
        (self.repo / "install-root").symlink_to(target)
        snippet = f"""
            hide_dangling_data_links "{self.repo}"
            test -L "{self.repo}/install-root" && echo retained
        """
        result = self.source(snippet)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "retained")


if __name__ == "__main__":
    unittest.main()
