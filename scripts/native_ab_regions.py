#!/usr/bin/env python3
"""Score native 800x600 gameplay regions from a local A/B pack.

Both inputs are unscaled RGB PNGs from the same native capture path. Unlike
#332, no bicubic/point-sample mismatch remains in the comparison.
"""
from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
from pathlib import Path
from typing import Any

from PIL import Image, ImageChops, ImageDraw, ImageEnhance

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))
from native_ab import (  # noqa: E402  (local helper module, loaded after sys.path)
    aggregate_gameplay_gate,
    parse_psnr,
    parse_ssim,
    sha256_of,
    write_json,
)

LEGACY_BASELINE = {
    "planet": (0.751759, 20.465),
    "channel": (0.674972, 14.839),
    "room": (0.772145, 20.514),
    "waiting": (0.361756, 12.976),
    "loading": (0.335887, 11.419),
}


def crop_metric(ref: Path, test: Path, rect: list[int], metric: str) -> tuple[float | None, str]:
    """Measure SSIM or PSNR on identical crops of two native frames."""
    x, y, width, height = rect
    crop = f"crop={width}:{height}:{x}:{y}"
    result = subprocess.run(
        [
            "ffmpeg", "-hide_banner", "-nostats", "-i", str(ref), "-i", str(test),
            "-lavfi", f"[0:v]{crop}[r];[1:v]{crop}[t];[r][t]{metric}",
            "-f", "null", "-",
        ],
        capture_output=True, text=True, check=True,
    )
    return parse_ssim(result.stderr) if metric == "ssim" else parse_psnr(result.stderr)


def save_region_crops(region: dict[str, Any], ref: Image.Image, test: Image.Image,
                      heat: Image.Image, destination: Path) -> dict[str, str]:
    """Persist the pixels behind each region number for later review."""
    x, y, width, height = region["rect"]
    box = (x, y, x + width, y + height)
    difference = ImageChops.difference(ref.crop(box), test.crop(box))
    crops = {
        "ref": ref.crop(box),
        "rust": test.crop(box),
        "heat": heat.crop(box),
        "diff": ImageEnhance.Contrast(difference).enhance(4),
    }
    hashes = {}
    for label, image in crops.items():
        path = destination / f"{region['name']}__{label}.png"
        image.save(path)
        hashes[label] = sha256_of(path)
    return hashes


def write_contact_sheet(scene: str, regions: list[dict[str, Any]], destination: Path) -> Path:
    """Build one review sheet per scene."""
    thumb_width, thumb_height, label_width, gap = 400, 300, 180, 8
    row_height = thumb_height + 24
    sheet = Image.new(
        "RGB",
        (label_width + 2 * thumb_width + gap + 10, row_height * len(regions) + gap),
        "white",
    )
    draw = ImageDraw.Draw(sheet)
    for row, region in enumerate(regions):
        top = row * row_height
        draw.text(
            (4, top + 4),
            f"{region['name']}\n{region['rect']}\n{region['class']}",
            fill="black",
        )
        for column, label in enumerate(("ref", "rust")):
            image = Image.open(destination / f"{region['name']}__{label}.png").convert("RGB")
            image.thumbnail((thumb_width, thumb_height), Image.Resampling.LANCZOS)
            left = label_width + column * (thumb_width + gap)
            draw.text((left, top + 4), label, fill="black")
            sheet.paste(image, (left, top + 20))
    path = destination.parent / f"region-map-native-{scene}.png"
    sheet.save(path)
    return path


def json_metric(value: float | None) -> float | str | None:
    """Encode an infinite PSNR as text so the report stays strict JSON."""
    if value is not None and not math.isfinite(value):
        return "inf"
    return value


def legacy_baseline(legacy: dict[str, Any], scene: str) -> dict[str, Any]:
    """Describe the old #332 numbers, or mark a new matched state unbaselined."""
    numbers = legacy.get(scene)
    if numbers is None:
        return {
            "ssim": None,
            "psnr": None,
            "reason": "no #332 downsampled baseline for this matched state",
        }
    return {"ssim": numbers[0], "psnr": numbers[1]}


def score_scene(scene: str, regions: list[dict[str, Any]], pack: Path,
                thresholds: dict[str, float], legacy: dict[str, Any]) -> dict[str, Any]:
    """Score one scene and return its gate plus review artifacts."""
    ref_path = pack / "frames/ref" / f"{scene}.png"
    test_path = pack / "frames/test" / f"{scene}.png"
    with Image.open(ref_path) as ref_image, Image.open(test_path) as test_image:
        ref = ref_image.convert("RGB")
        test = test_image.convert("RGB")
        heat = ImageEnhance.Contrast(ImageChops.difference(ref, test)).enhance(4)
        destination = pack / "regions" / scene
        destination.mkdir(parents=True, exist_ok=True)
        enriched = []
        for region in regions:
            row = dict(region)
            row["ssim"], row["ssim_source"] = crop_metric(
                ref_path, test_path, region["rect"], "ssim"
            )
            row["psnr"], row["psnr_source"] = crop_metric(
                ref_path, test_path, region["rect"], "psnr"
            )
            row["crops_sha256"] = save_region_crops(row, ref, test, heat, destination)
            enriched.append(row)
    sheet = write_contact_sheet(scene, enriched, destination)
    gate = aggregate_gameplay_gate(enriched, thresholds)
    gate["area_weighted_psnr_from_mse"] = json_metric(gate["area_weighted_psnr_from_mse"])
    full_ssim, _ = crop_metric(ref_path, test_path, [0, 0, 800, 600], "ssim")
    full_psnr, _ = crop_metric(ref_path, test_path, [0, 0, 800, 600], "psnr")
    return {
        "full_frame_validation": {"ssim": full_ssim, "psnr": json_metric(full_psnr)},
        "regions": enriched,
        "gameplay_gate": gate,
        "legacy_downsampled_weighted": legacy_baseline(legacy, scene),
        "contact_sheet": str(sheet),
        "contact_sheet_sha256": sha256_of(sheet),
    }


def score_all_scenes(pack: Path, region_map: Path, thresholds_path: Path) -> dict[str, Any]:
    """Produce the native baseline with full provenance."""
    spec = json.loads(region_map.read_text())
    thresholds = json.loads(thresholds_path.read_text())
    ffmpeg_version = subprocess.run(
        ["ffmpeg", "-version"], capture_output=True, text=True, check=True
    ).stdout.splitlines()[0]
    result: dict[str, Any] = {
        "schema": "ab334-native-region-metrics-v1",
        "region_map": str(region_map),
        "region_map_sha256": sha256_of(region_map),
        "thresholds_sha256": sha256_of(thresholds_path),
        "thresholds": {
            "ssim_min": thresholds["ssim_min"],
            "psnr_min": thresholds["psnr_min"],
        },
        "substrate": "unscaled native 800x600 RGB PNG pairs; ffmpeg crop filter only",
        "comparability": (
            "#332 used 400x300 frames with different filters per side and a "
            "libx264 yuv420p substrate; deltas are method and scenario changes"
        ),
        "legacy_downsampled_baseline": LEGACY_BASELINE,
        "ffmpeg_version": ffmpeg_version,
        "gate_rule": "scene passes only when every scored gameplay region meets BOTH thresholds",
        "scenes": {},
    }
    for scene, regions in spec["scenes"].items():
        result["scenes"][scene] = score_scene(scene, regions, pack, thresholds, LEGACY_BASELINE)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", type=Path, required=True, help="local-only pack root")
    args = parser.parse_args()
    region_map = args.pack / "regions-v1-native.json"
    thresholds_path = args.pack / "thresholds/skill-default.json"
    metrics = score_all_scenes(args.pack, region_map, thresholds_path)
    output = args.pack / "region-metrics.json"
    write_json(output, metrics)
    print(f"metrics: {output} {sha256_of(output)}")
    for scene, data in metrics["scenes"].items():
        gate = data["gameplay_gate"]
        print(
            scene,
            f"min={gate['min_ssim']:.6f}/{gate['min_psnr']:.3f}",
            f"weighted={gate['area_weighted_ssim']:.6f}/{gate['area_weighted_psnr_from_mse']}",
            f"pass={gate['all_scored_regions_pass']}",
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
