#!/usr/bin/env python3
"""Synthetic regression tests for the recomp2jam A/V helper scripts."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPTS_DIR = Path(__file__).resolve().parent


def load_module(name: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS_DIR / f"{name}.py")
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot load {name}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


compare_av = load_module("compare_av")
batch_compare_av = load_module("batch_compare_av")


class ParsePsnrTests(unittest.TestCase):
    def test_identical_frames_keep_infinite_psnr_as_samples(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            log = Path(temp_dir) / "psnr.log"
            log.write_text("n:1 psnr_avg:inf\nn:2 psnr_avg:inf\n")

            result = compare_av.parse_psnr(log)

            self.assertEqual(result["samples"], 2)
            self.assertEqual(result["infinite_samples"], 2)
            self.assertIsNone(result["average"])

    def test_mixed_frames_record_finite_average_and_infinite_count(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            log = Path(temp_dir) / "psnr.log"
            log.write_text("n:1 psnr_avg:inf\nn:2 psnr_avg:40.0\n")

            result = compare_av.parse_psnr(log)

            self.assertEqual(result["samples"], 2)
            self.assertEqual(result["infinite_samples"], 1)
            self.assertEqual(result["average"], 40.0)


class ParseAudioTests(unittest.TestCase):
    def test_ffmpeg_true_peak_dbfs_output_is_parsed(self) -> None:
        output = """
    I:         -21.8 LUFS
  True peak:
    Peak:      -18.1 dBFS
"""

        result = compare_av.parse_ebur128(output)

        self.assertEqual(result["integrated_lufs"], -21.8)
        self.assertEqual(result["true_peak_dbtp"], -18.1)


class CheckSummaryTests(unittest.TestCase):
    def run_check(self, summary: dict) -> tuple[int, dict]:
        with tempfile.TemporaryDirectory() as temp_dir:
            summary_path = Path(temp_dir) / "summary.json"
            summary_path.write_text(json.dumps(summary))
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS_DIR / "check_summary.py"),
                    str(summary_path),
                    "--strict",
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            report = json.loads(summary_path.with_name("pass_fail.json").read_text())
            return result.returncode, report

    def complete_summary(self) -> dict:
        return {
            "label": "synthetic",
            "settings": {"vmaf": False},
            "video": {
                "ssim": {"average": 1.0},
                "psnr": {"average": None, "infinite_samples": 2},
                "vmaf": None,
            },
            "audio": {
                "reference": {"integrated_lufs": -20.0, "true_peak_dbtp": -2.0},
                "test": {"integrated_lufs": -20.0, "true_peak_dbtp": -2.0},
            },
        }

    def test_strict_check_accepts_identical_psnr_and_disabled_vmaf(self) -> None:
        returncode, report = self.run_check(self.complete_summary())

        self.assertEqual(returncode, 0)
        self.assertEqual(report["status"], "pass")
        checks = {check["metric"]: check for check in report["checks"]}
        self.assertEqual(checks["psnr_avg"]["value"], "inf")
        self.assertEqual(checks["vmaf_avg"]["status"], "disabled")

    def test_strict_check_rejects_missing_enabled_metrics(self) -> None:
        summary = self.complete_summary()
        summary["video"]["ssim"] = {"average": None}

        returncode, report = self.run_check(summary)

        self.assertEqual(returncode, 1)
        self.assertEqual(report["status"], "fail")


class BatchManifestTests(unittest.TestCase):
    def test_rejects_an_unrelated_manifest_schema(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            manifest = Path(temp_dir) / "manifest.json"
            manifest.write_text(json.dumps({"schema_version": "v1", "scenes": []}))

            with self.assertRaises(batch_compare_av.BatchError):
                batch_compare_av.load_manifest(manifest)
                with patch.object(
                    sys,
                    "argv",
                    ["batch_compare_av.py", str(manifest)],
                ):
                    batch_compare_av.main()

    def test_failed_comparison_does_not_check_a_stale_summary(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifest = root / "manifest.json"
            output_dir = root / "out"
            output_dir.mkdir()
            (output_dir / "summary.json").write_text("stale")
            manifest.write_text(
                json.dumps(
                    {
                        "schema_version": "recomp2jam-av-v1",
                        "scenes": [
                            {
                                "id": "waiting",
                                "ref": "ref.mp4",
                                "test": "test.mp4",
                                "out_dir": "out",
                                "thresholds": "thresholds.json",
                            }
                        ],
                    }
                )
            )

            with patch.object(batch_compare_av, "run_command", return_value=1) as run:
                with patch.object(sys, "argv", ["batch_compare_av.py", str(manifest)]):
                    status = batch_compare_av.main()

            self.assertEqual(status, 1)
            self.assertEqual(run.call_count, 1)

    def test_strict_flag_reaches_the_threshold_checker(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            manifest = root / "manifest.json"
            manifest.write_text(
                json.dumps(
                    {
                        "schema_version": "recomp2jam-av-v1",
                        "scenes": [
                            {
                                "id": "result",
                                "ref": "ref.mp4",
                                "test": "test.mp4",
                                "out_dir": "out",
                                "thresholds": "thresholds.json",
                            }
                        ],
                    }
                )
            )

            with patch.object(batch_compare_av, "run_command", return_value=0) as run:
                with patch.object(
                    sys,
                    "argv",
                    ["batch_compare_av.py", str(manifest), "--strict"],
                ):
                    status = batch_compare_av.main()

            self.assertEqual(status, 0)
            self.assertEqual(run.call_count, 2)
            self.assertIn("--strict", run.call_args_list[1].args[0])


if __name__ == "__main__":
    unittest.main()
