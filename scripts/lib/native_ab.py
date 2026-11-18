"""Pure helpers shared by the native 800x600 A/B capture and score scripts.

The helpers keep proprietary pixels in local scratch directories: they only
hash, validate, and describe frames and regions. No media bytes enter the repo.
"""
from __future__ import annotations

import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any

from PIL import Image

NATIVE_SIZE = (800, 600)
SCENES = ("planet", "channel", "room", "waiting", "loading")
DEFAULT_REGION_SCALE = 2
STILL_DURATION_SECONDS = 1
AUDIO_RATE_HZ = 48000
SSIM_ALL = re.compile(r"All:([^\s(]+)")
SSIM_LUMA = re.compile(r"\bY:([0-9.]+)")
PSNR_AVERAGE = re.compile(r"average:([^\s]+)")


def sha256_of(path: Path) -> str:
    """Hash a file without loading it all into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, document: dict[str, Any]) -> None:
    """Write one deterministic, human-readable JSON record."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(document, indent=2) + "\n")


def require_native_png(path: Path) -> None:
    """Reject a frame that is not a native-resolution RGB still."""
    with Image.open(path) as image:
        if image.size != NATIVE_SIZE or image.mode not in ("RGB", "RGBA"):
            raise ValueError(
                f"{path}: want {NATIVE_SIZE} RGB, got {image.size} {image.mode}"
            )


def still_media_command(frame: Path, media: Path) -> list[str]:
    """Wrap a still losslessly; silence only satisfies compare_av.py."""
    return [
        "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
        "-loop", "1", "-framerate", "1", "-i", str(frame),
        "-f", "lavfi", "-i", f"anullsrc=r={AUDIO_RATE_HZ}:cl=stereo",
        "-t", str(STILL_DURATION_SECONDS),
        "-c:v", "ffv1", "-level", "3", "-pix_fmt", "bgr0",
        "-c:a", "pcm_s16le", "-shortest", str(media),
    ]


def scale_rect(rect: list[int], factor: int = DEFAULT_REGION_SCALE) -> list[int]:
    """Rescale a normalized 400x300 rect into native frame coordinates."""
    if len(rect) != 4 or any(value < 0 for value in rect):
        raise ValueError(f"invalid rect: {rect!r}")
    return [value * factor for value in rect]


def parse_number(text: str) -> float | None:
    """Read a finite, infinite, or missing ffmpeg metric value."""
    try:
        return float(text)
    except ValueError:
        return None


def parse_ssim(ffmpeg_output: str) -> tuple[float | None, str]:
    """Read SSIM, falling back to luma when a crop has constant chroma."""
    candidates = SSIM_ALL.findall(ffmpeg_output)
    if not candidates:
        raise ValueError("ffmpeg output has no SSIM value")
    value = parse_number(candidates[-1])
    if value is not None and math.isfinite(value):
        return value, "All"
    luma = SSIM_LUMA.findall(ffmpeg_output)
    if not luma:
        raise ValueError("ffmpeg output has no luma SSIM fallback")
    return float(luma[-1]), f"Y (All={candidates[-1]}: constant chroma)"


def parse_psnr(ffmpeg_output: str) -> tuple[float | None, str]:
    """Read PSNR; identical crops report an infinite value."""
    candidates = PSNR_AVERAGE.findall(ffmpeg_output)
    if not candidates:
        raise ValueError("ffmpeg output has no PSNR value")
    value = parse_number(candidates[-1])
    if value is not None and math.isinf(value):
        return None, "inf (identical crop)"
    if value is None or not math.isfinite(value):
        raise ValueError(f"unusable PSNR value: {candidates[-1]}")
    return value, "average"


def aggregate_gameplay_gate(
    regions: list[dict[str, Any]], thresholds: dict[str, float]
) -> dict[str, Any]:
    """Gate every scored region; weight means by native pixel area."""
    scored = [region for region in regions if region["score"]]
    if not scored:
        return {"scored_regions": 0}
    pixel_count = 0
    mse_total = 0.0
    ssim_total = 0.0
    for region in scored:
        width, height = region["rect"][2], region["rect"][3]
        area = width * height
        psnr = region["psnr"]
        mse = 0.0 if psnr is None else 255.0**2 / 10.0 ** (psnr / 10.0)
        mse_total += mse * area
        ssim_total += region["ssim"] * area
        pixel_count += area
    weighted_psnr = (
        math.inf if mse_total == 0
        else 10.0 * math.log10(255.0**2 / (mse_total / pixel_count))
    )
    minimum_ssim = min(region["ssim"] for region in scored)
    minimum_psnr = min(
        (region["psnr"] for region in scored if region["psnr"] is not None),
        default=None,
    )
    return {
        "scored_regions": len(scored),
        "min_ssim": minimum_ssim,
        "min_psnr": minimum_psnr,
        "area_weighted_ssim": ssim_total / pixel_count,
        "area_weighted_psnr_from_mse": weighted_psnr,
        "scored_pixels": pixel_count,
        "all_scored_regions_pass": (
            minimum_ssim >= thresholds["ssim_min"]
            and (minimum_psnr is None or minimum_psnr >= thresholds["psnr_min"])
        ),
    }
