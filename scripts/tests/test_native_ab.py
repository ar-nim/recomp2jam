"""Seam tests for the native A/B pack and region scripts (#334).

These guards use only synthetic files. Qualified parity still needs the live
Wine/Rust frame pairs in the local-only pack; seam-green never closes parity.
"""
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR / "lib"))
import native_ab  # noqa: E402


def load_script(name: str):
    """Load a script module by file name."""
    path = SCRIPTS_DIR / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class NativeAbHelperTests(unittest.TestCase):
    """Guard the pure transformations behind the native baseline."""

    def test_legacy_rect_doubles_every_coordinate_for_native_frames(self) -> None:
        self.assertEqual(native_ab.scale_rect([20, 220, 360, 64]), [40, 440, 720, 128])

    def test_negative_rect_is_rejected_instead_of_silently_scaling(self) -> None:
        with self.assertRaises(ValueError):
            native_ab.scale_rect([-2, 0, 10, 10])

    def test_synthetic_400x300_frame_is_rejected_by_native_guard(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            frame = Path(directory) / "downsampled.png"
            Image.new("RGB", (400, 300)).save(frame)
            with self.assertRaises(ValueError):
                native_ab.require_native_png(frame)

    def test_media_wrapper_keeps_native_pixels_and_adds_silent_carrier(self) -> None:
        command = native_ab.still_media_command(Path("ref.png"), Path("ref.mkv"))
        self.assertIn("-pix_fmt", command)
        self.assertEqual(command[command.index("-pix_fmt") + 1], "bgr0")
        self.assertIn("anullsrc", " ".join(command))
        self.assertIn("ffv1", command)
        self.assertNotIn("-vf", command)
        self.assertNotIn("scale", " ".join(command))

    def test_ssim_parser_falls_back_to_luma_when_all_is_not_a_number(self) -> None:
        value, source = native_ab.parse_ssim("SSIM Y:0.910000 All:nan (0.0)")
        self.assertEqual(value, 0.91)
        self.assertIn("constant chroma", source)

    def test_psnr_parser_marks_identical_crops_as_infinite(self) -> None:
        value, source = native_ab.parse_psnr("PSNR average:inf min:inf")
        self.assertIsNone(value)
        self.assertIn("identical", source)

    def test_gate_weights_psnr_by_region_mse_not_by_decibels(self) -> None:
        regions = [
            {"score": True, "rect": [0, 0, 10, 10], "ssim": 0.9, "psnr": 30.0},
            {"score": True, "rect": [0, 0, 30, 10], "ssim": 0.8, "psnr": 20.0},
            {"score": False, "rect": [0, 0, 100, 100], "ssim": 0.1, "psnr": 1.0},
        ]
        gate = native_ab.aggregate_gameplay_gate(
            regions, {"ssim_min": 0.95, "psnr_min": 35.0}
        )
        self.assertEqual(gate["scored_pixels"], 400)
        self.assertAlmostEqual(gate["area_weighted_ssim"], 0.825)
        self.assertAlmostEqual(gate["area_weighted_psnr_from_mse"], 21.106983, places=5)
        self.assertFalse(gate["all_scored_regions_pass"])


class PackBuilderSeamTests(unittest.TestCase):
    """Guard source lookup and provenance redaction."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.pack_builder = load_script("native_ab_pack")

    def test_loading_frame_uses_the_mislabeled_wine_cue(self) -> None:
        frame = self.pack_builder.reference_frame(
            Path("/wine/lobby"), Path("/wine/cue"), "loading"
        )
        self.assertEqual(frame, Path("/wine/cue/artifacts/screenshots/waiting-cue-012.png"))

    def test_exactly_one_scene_stamped_frame_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            frames = Path(directory) / "frames"
            frames.mkdir()
            (frames / "frame-000040-planet.png").touch()
            found = self.pack_builder.rust_scene_frame(Path(directory), "planet")
            self.assertEqual(found.name, "frame-000040-planet.png")

    def test_two_stamped_frames_are_rejected_instead_of_hand_picking(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            frames = Path(directory) / "frames"
            frames.mkdir()
            (frames / "frame-1-room.png").touch()
            (frames / "frame-2-room.png").touch()
            with self.assertRaises(ValueError):
                self.pack_builder.rust_scene_frame(Path(directory), "room")

    def test_token_is_redacted_before_provenance_is_written(self) -> None:
        command = ["client", "--token", "secret", "--no-sound"]
        safe = self.pack_builder.redact_token(command)
        self.assertNotIn("secret", safe)
        self.assertEqual(command[2], "secret")


if __name__ == "__main__":
    unittest.main()
