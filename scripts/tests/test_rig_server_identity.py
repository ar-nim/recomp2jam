"""Server-identity provenance for rig packs (issue #345).

A pack must name the Memoryer process it used, so two legs run across a
silent server restart are distinguishable from the packs alone. These seam
tests pin the preflight record and its propagation into manifest params;
they never touch the live server on 127.0.0.1:15010.
"""
from __future__ import annotations

import os
import socket
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
PREFLIGHT = SCRIPTS_DIR / "lib" / "rig-preflight.sh"
RIG_RUN = SCRIPTS_DIR / "rig-run.sh"
RIG_AFK = SCRIPTS_DIR / "rig-afk.sh"
RIG_WINE_RUN = SCRIPTS_DIR / "rig-wine-run.sh"
MEMORYER_START = SCRIPTS_DIR / "memoryer-start.sh"


def source_preflight(script: str, env: dict[str, str], cwd: Path) -> subprocess.CompletedProcess[str]:
    """Run a shell snippet with the shared preflight sourced."""
    return subprocess.run(
        ["bash", "-c", f'. "{PREFLIGHT}"; {script}'],
        capture_output=True,
        text=True,
        env=env,
        cwd=cwd,
        timeout=30,
    )


class ServerIdentityTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.proc = self.root / "proc"
        (self.proc / "1234").mkdir(parents=True)
        fields = ["0"] * 30
        fields[0] = "S"
        fields[19] = "12345"  # /proc/<pid>/stat field 22: starttime ticks
        (self.proc / "1234" / "stat").write_text(
            f'1234 (dotnet) {" ".join(fields)}\n', encoding="utf-8"
        )
        (self.proc / "stat").write_text("cpu 1 2 3 4\nbtime 1700000000\n", encoding="utf-8")
        self.ss_output = self.root / "ss.txt"
        self.ss_output.write_text(
            'LISTEN 0 1000 127.0.0.1:15010 0.0.0.0:* users:(("dotnet",pid=1234,fd=141))\n',
            encoding="utf-8",
        )
        self.env = dict(os.environ)
        self.env.update(
            {
                "RIG_PROC_ROOT": str(self.proc),
                "RIG_SS_FIXTURE": str(self.ss_output),
                "RIG_MEMORYER_PORT": "15010",
            }
        )

    def test_preflight_records_live_server_pid_and_process_boot_time(self) -> None:
        out_dir = self.root / "pack"
        out_dir.mkdir()
        result = source_preflight(f'rig_record_server_identity "{out_dir}"', self.env, self.root)
        self.assertEqual(result.returncode, 0, result.stderr)
        recorded = (out_dir / "server-identity").read_text(encoding="utf-8").strip()
        self.assertIn("memoryer_pid=1234", recorded)
        self.assertIn("memoryer_boot_epoch=1700000123", recorded)
        self.assertIn("memoryer_boot_utc=2023-11-14T22:15:23Z", recorded)

    def test_pack_params_carry_the_recorded_identity(self) -> None:
        out_dir = self.root / "pack"
        out_dir.mkdir()
        (out_dir / "server-identity").write_text(
            "memoryer_pid=1234 memoryer_boot_epoch=1700000123 memoryer_boot_utc=2023-11-14T22:15:23Z\n",
            encoding="utf-8",
        )
        result = source_preflight(f'rig_server_identity_params "{out_dir}"', self.env, self.root)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("memoryer_pid=1234", result.stdout)
        self.assertIn("memoryer_boot_epoch=1700000123", result.stdout)

    def test_unidentifiable_server_fails_loudly(self) -> None:
        self.ss_output.write_text("LISTEN 0 5 127.0.0.1:9999 0.0.0.0:*\n", encoding="utf-8")
        out_dir = self.root / "pack"
        out_dir.mkdir()
        result = source_preflight(f'rig_record_server_identity "{out_dir}"', self.env, self.root)
        self.assertNotEqual(result.returncode, 0, "no listener must not yield a silent unknown")
        self.assertIn("server-identity", result.stderr)

    def test_skipped_server_check_records_unknown(self) -> None:
        self.env["RIG_EXPECT_MEMORYER"] = "0"
        out_dir = self.root / "pack"
        out_dir.mkdir()
        result = source_preflight(f'rig_record_server_identity "{out_dir}"', self.env, self.root)
        self.assertEqual(result.returncode, 0, result.stderr)
        recorded = (out_dir / "server-identity").read_text(encoding="utf-8").strip()
        self.assertIn("memoryer_pid=unknown", recorded)
        self.assertIn("memoryer_boot_epoch=unknown", recorded)


class MemoryerStartWrapperTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.env = dict(os.environ)
        self.env["RIG_MEMORYER_PORT"] = "59997"
        self.env["MEMORYER_DIR"] = str(self.root / "server")
        (self.root / "server" / "bin").mkdir(parents=True)
        (self.root / "server" / "bin" / "Memoryer.Encore.dll").write_text("stub", encoding="utf-8")

    def test_refuses_a_second_instance_when_the_port_is_taken(self) -> None:
        listener = socket.socket()
        listener.bind(("127.0.0.1", 59997))
        listener.listen(1)
        self.addCleanup(listener.close)
        result = subprocess.run(
            ["bash", str(MEMORYER_START), "--check-only"],
            capture_output=True,
            text=True,
            env=self.env,
            timeout=30,
        )
        self.assertEqual(result.returncode, 13, result.stderr)
        self.assertIn("refusing a second instance", result.stderr)

    def test_start_exports_managed_dump_environment(self) -> None:
        observed = self.root / "dump-env.txt"
        stub = self.root / "dotnet-stub"
        stub.write_text(
            f"#!/bin/sh\nenv | grep '^DOTNET_Dbg' > '{observed}'\n", encoding="utf-8"
        )
        stub.chmod(0o755)
        self.env["MEMORYER_DOTNET"] = str(stub)
        self.env["MEMORYER_DUMP_DIR"] = str(self.root / "dumps")
        result = subprocess.run(
            ["bash", str(MEMORYER_START)],
            capture_output=True,
            text=True,
            env=self.env,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        for _ in range(40):
            if observed.exists():
                break
            time.sleep(0.1)
        self.assertTrue(observed.exists(), "the start wrapper never ran the dotnet host")
        written = observed.read_text(encoding="utf-8")
        self.assertIn("DOTNET_DbgEnableMiniDump=1", written)
        self.assertIn("DOTNET_DbgMiniDumpType=2", written)
        self.assertIn("DOTNET_DbgMiniDumpName=", written)


class LiveCallerChainTest(unittest.TestCase):
    def test_rig_run_preflight_calls_the_recorder(self) -> None:
        source = RIG_RUN.read_text(encoding="utf-8")
        self.assertIn("rig_record_server_identity", source)
        preflight = source.split("preflight_run_checks()", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("rig_check_memoryer", preflight)
        self.assertIn("rig_record_server_identity", preflight)

    def test_both_pack_writers_add_identity_to_manifest_params(self) -> None:
        for writer in (RIG_AFK, RIG_WINE_RUN):
            with self.subTest(writer=writer.name):
                source = writer.read_text(encoding="utf-8")
                self.assertIn("rig_server_identity_params", source)
                self.assertIn("--params", source)

    def test_runbook_documents_the_record_and_the_restart_signal(self) -> None:
        runbook = (SCRIPTS_DIR.parent / "docs" / "rig-runbook.md").read_text(encoding="utf-8")
        self.assertIn("server-identity", runbook)
        self.assertIn("memoryer_boot_epoch", runbook)
        self.assertIn("DOTNET_DbgEnableMiniDump", runbook)
        self.assertIn("DOTNET_DbgMiniDumpType", runbook)
        self.assertIn("/var/lib/systemd/coredump/", runbook)
        self.assertIn("~/scratch/memoryer-dumps/", runbook)

    def test_play_local_start_hint_points_to_the_dump_wrapper(self) -> None:
        hint = (SCRIPTS_DIR / "play-local.sh").read_text(encoding="utf-8")
        self.assertIn("memoryer-start.sh", hint)
        self.assertIn("DOTNET_DbgEnableMiniDump=1", hint)
        self.assertIn("DOTNET_DbgMiniDumpType=2", hint)
        self.assertIn("DOTNET_DbgMiniDumpName", hint)


if __name__ == "__main__":
    unittest.main()
