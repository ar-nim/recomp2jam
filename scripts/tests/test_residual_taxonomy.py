"""Guards for the #250 residual taxonomy and the read-only triage report.

Every case uses synthetic metrics and annotations: a green report never stands
in for a qualified exe/Rust capture pair.
"""
from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any

SCRIPTS_DIR = Path(__file__).resolve().parents[1]


def load_script(name: str, directory: Path = SCRIPTS_DIR):
    """Load a helper or script module by file name."""
    path = directory / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def synthetic_metrics() -> dict[str, Any]:
    """Two regions: one failing scored region and one excluded glyph crop."""
    return {
        "schema": "ab334-native-region-metrics-v1",
        "thresholds": {"ssim_min": 0.95, "psnr_min": 35.0},
        "scenes": {
            "planet": {
                "regions": [
                    {
                        "name": "emblem_strip",
                        "rect": [0, 0, 800, 152],
                        "class": "gameplay",
                        "score": True,
                        "ssim": 0.729063,
                        "psnr": 16.616024,
                        "crops_sha256": {
                            "ref": "a" * 64,
                            "rust": "b" * 64,
                            "heat": "c" * 64,
                            "diff": "d" * 64,
                        },
                    },
                    {
                        "name": "promo_banner",
                        "rect": [40, 152, 720, 148],
                        "class": "text",
                        "score": False,
                        "ssim": 0.90717,
                        "psnr": 21.179485,
                        "crops_sha256": {"ref": "e" * 64},
                    },
                ]
            }
        },
    }


def emblem_annotation() -> dict[str, Any]:
    """A pinned implementation-gap annotation with an exe capture and bug."""
    return {
        "schema": "ab334-residual-taxonomy-v1",
        "regions": {
            "planet.emblem_strip": {
                "divergence_class": "art-selection",
                "residual_class": "implementation-gap",
                "evidence": [
                    "https://github.com/ar-nim/recomp2jam/issues/361",
                    "docs/captures/journey-audit/288-planet.md#P3",
                ],
                "linked_bug": 361,
                "exe_capture": "235f53521171be4f1d4c725d16143332fa47b66813f9e1c5b8e8b09fe68ddc21",
                "note": "Rest badge art is bright in Rust and dimmed in the exe.",
            }
        },
    }


class ResidualTaxonomyTests(unittest.TestCase):
    """Guard classification values and the evidence requirements behind them."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.taxonomy = load_script("residual_taxonomy", SCRIPTS_DIR / "lib")

    def test_metrics_without_thresholds_report_a_missing_gate_instead_of_crashing(self) -> None:
        metrics = synthetic_metrics()
        metrics.pop("thresholds")
        enriched = self.taxonomy.apply_taxonomy_to_metrics(metrics, {})
        report = self.taxonomy.render_triage_report(enriched)
        self.assertIn("gate thresholds missing", report)

    def test_unannotated_regions_report_as_unpinned_evidence_gaps(self) -> None:
        metrics = self.taxonomy.apply_taxonomy_to_metrics(synthetic_metrics(), {})
        row = metrics["scenes"]["planet"]["regions"][0]
        self.assertEqual(row["divergence_class"], "unpinned")
        self.assertEqual(row["residual_class"], "evidence-gap")
        report = self.taxonomy.render_triage_report(metrics)
        self.assertIn("## Divergence: unpinned", report)
        self.assertIn("No annotation", report)

    def test_art_selection_annotation_carries_evidence_capture_and_bug(self) -> None:
        metrics = self.taxonomy.apply_taxonomy_to_metrics(
            synthetic_metrics(), emblem_annotation()["regions"]
        )
        report = self.taxonomy.render_triage_report(metrics)
        self.assertIn("## Divergence: art-selection", report)
        self.assertIn("### planet.emblem_strip — implementation-gap", report)
        self.assertIn("https://github.com/ar-nim/recomp2jam/issues/361", report)
        self.assertIn("235f5352", report)
        self.assertIn("a" * 64, report)

    def test_implementation_gap_without_exe_capture_is_rejected(self) -> None:
        annotation = emblem_annotation()["regions"]["planet.emblem_strip"]
        annotation.pop("exe_capture")
        with self.assertRaisesRegex(ValueError, "exe_capture"):
            self.taxonomy.apply_taxonomy_to_metrics(
                synthetic_metrics(), {"planet.emblem_strip": annotation}
            )

    def test_unknown_divergence_class_is_rejected(self) -> None:
        annotation = emblem_annotation()["regions"]["planet.emblem_strip"]
        annotation["divergence_class"] = "hand-wave"
        with self.assertRaisesRegex(ValueError, "divergence_class"):
            self.taxonomy.apply_taxonomy_to_metrics(
                synthetic_metrics(), {"planet.emblem_strip": annotation}
            )

    def test_stale_annotation_for_a_missing_region_is_rejected(self) -> None:
        annotations = dict(emblem_annotation()["regions"])
        annotations["planet.removed_region"] = {
            "divergence_class": "geometry",
            "residual_class": "capture-fault",
            "evidence": ["docs/captures/journey-audit/288-planet.md"],
            "note": "Stale key after the region map changed.",
        }
        with self.assertRaisesRegex(ValueError, "removed_region"):
            self.taxonomy.apply_taxonomy_to_metrics(synthetic_metrics(), annotations)

    def test_applying_taxonomy_keeps_existing_metrics_contract(self) -> None:
        original = synthetic_metrics()
        snapshot = json.dumps(original, sort_keys=True)
        enriched = self.taxonomy.apply_taxonomy_to_metrics(
            original, emblem_annotation()["regions"]
        )
        self.assertEqual(original, json.loads(snapshot))
        self.assertEqual(enriched["schema"], "ab334-native-region-metrics-v1")
        for key in ("name", "rect", "class", "score", "ssim", "psnr", "crops_sha256"):
            self.assertIn(key, enriched["scenes"]["planet"]["regions"][0])


class TriageCommandTests(unittest.TestCase):
    """Guard the read-only CLI against needing pixels, ffmpeg, or tracker writes."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.scorer = load_script("native_ab_regions")

    def write_pack(self, directory: Path, with_annotations: bool = True) -> Path:
        pack = Path(directory)
        (pack / "region-metrics.json").write_text(json.dumps(synthetic_metrics()))
        if with_annotations:
            (pack / "residual-annotations.json").write_text(
                json.dumps(emblem_annotation())
            )
        return pack

    def test_triage_writes_grouped_markdown_without_changing_metrics(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            pack = self.write_pack(Path(directory))
            before = (pack / "region-metrics.json").read_text()
            result = subprocess.run(
                [
                    sys.executable, str(SCRIPTS_DIR / "native_ab_regions.py"),
                    "--pack", str(pack), "--triage",
                    "--annotations", str(pack / "residual-annotations.json"),
                    "--group-by", "residual",
                    "--output", str(pack / "residual-report.md"),
                ],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = (pack / "residual-report.md").read_text()
            self.assertIn("## Residual class: implementation-gap", report)
            self.assertIn("### planet.emblem_strip", report)
            self.assertIn("## Residual class: evidence-gap", report)
            self.assertEqual((pack / "region-metrics.json").read_text(), before)

    def test_triage_prints_the_report_when_no_output_path_is_given(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            pack = self.write_pack(Path(directory), with_annotations=False)
            result = subprocess.run(
                [
                    sys.executable, str(SCRIPTS_DIR / "native_ab_regions.py"),
                    "--pack", str(pack), "--triage",
                ],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("# Residual triage", result.stdout)
            self.assertIn("## Divergence: unpinned", result.stdout)

    def test_scoring_refuses_triage_only_options_instead_of_ignoring_them(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            pack = self.write_pack(Path(directory), with_annotations=False)
            result = subprocess.run(
                [
                    sys.executable, str(SCRIPTS_DIR / "native_ab_regions.py"),
                    "--pack", str(pack), "--group-by", "residual",
                ],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn("--group-by needs --triage", result.stderr)

    def test_triage_refuses_an_unknown_grouping_dimension(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            pack = self.write_pack(Path(directory), with_annotations=False)
            result = subprocess.run(
                [
                    sys.executable, str(SCRIPTS_DIR / "native_ab_regions.py"),
                    "--pack", str(pack), "--triage", "--group-by", "colour",
                ],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn("group-by", result.stderr)


if __name__ == "__main__":
    unittest.main()
