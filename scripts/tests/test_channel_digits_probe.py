"""Synthetic checks for the Channel fill-digit rig probe (no game art)."""

from __future__ import annotations

import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image, ImageDraw

SCRIPTS = Path(__file__).resolve().parents[1]
PROBE = SCRIPTS / "rig-channel-digits-probe.py"

ROW_TOPS = (224, 275, 324, 375)
DIGIT_LOCALS = ((50, 11), (63, 11))
WIDTH, HEIGHT = 13, 18
FRAME_COLOR = (80, 80, 80)
BACKGROUND = (60, 60, 60)


def rgb555(channel: int) -> int:
    return channel >> 3


def expand(value: int) -> int:
    return (value << 3) | (value >> 2)


def digit_color(digit: int) -> tuple[int, int, int]:
    # Exact 5-5-5 expansion: round-trip through `rgb555` without drift.
    return (expand(digit), expand(1), expand(2))


def sprite_body() -> bytes:
    """Ten raw 13x18 digit frames, one flat colour per digit."""
    records = bytearray()
    pixels = bytearray()
    for digit in range(10):
        red, green, blue = digit_color(digit)
        word = rgb555(red) << 10 | rgb555(green) << 5 | rgb555(blue)
        records.extend(
            struct.pack("<HHHHIII", 0, 0, WIDTH, HEIGHT, digit * WIDTH * HEIGHT * 2,
                        WIDTH * HEIGHT * 2, 0)
        )
        pixels.extend(struct.pack(f"<{WIDTH * HEIGHT}H", *([word] * (WIDTH * HEIGHT))))
    header = struct.pack("<HHHH", 0, 0x0555, 10, 0)
    return bytes(header + records + pixels)


def write_archive(directory: Path) -> None:
    archive_dir = directory / "O2Jam/Image"
    archive_dir.mkdir(parents=True)
    body = sprite_body()
    name = b"ChannelInfo_CH_Num.ojs"
    entry = struct.pack("<I", 1) + name.ljust(128, b"\0")
    entry += struct.pack("<III", 16, len(body), len(body)) + bytes(8)
    count = 1
    archive = struct.pack("<IIQ", 2, count, 0) + body + entry
    (archive_dir / "Interface1.opi").write_bytes(archive)


def write_frame(directory: Path, ids: list[int]) -> Path:
    frame = Image.new("RGB", (800, 600), BACKGROUND)
    draw = ImageDraw.Draw(frame)
    for row, channel_id in enumerate(ids):
        shown = channel_id + 1
        for (local_x, local_y), digit in zip(DIGIT_LOCALS, (shown // 10, shown % 10)):
            x = 216 + 1 + local_x
            y = ROW_TOPS[row] + 1 + local_y
            draw.rectangle((x, y, x + WIDTH - 1, y + HEIGHT - 1), fill=digit_color(digit))
    path = directory / "channel-digits-native.png"
    frame.save(path)
    return path


class ChannelDigitsProbeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)

    def run_probe(self, install_root: Path, ids: str, frame: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(PROBE), "--frame", str(frame), "--ids", ids,
             "--install-root", str(install_root)],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_one_based_digits_pass(self) -> None:
        write_archive(self.directory)
        frame = write_frame(self.directory, [0, 1, 2, 3])
        result = self.run_probe(self.directory, "0,1,2,3", frame)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("4 row(s) match channel_id + 1", result.stdout)

    def test_raw_ids_are_refuted(self) -> None:
        write_archive(self.directory)
        frame = write_frame(self.directory, [0, 1, 2, 3])
        result = self.run_probe(self.directory, "1,2,3,4", frame)
        self.assertEqual(result.returncode, 1)
        self.assertIn("digit 1 paints 1", result.stderr)
        self.assertIn("want 2", result.stderr)

    def test_missing_archive_skips(self) -> None:
        frame = write_frame(self.directory, [0])
        result = self.run_probe(self.directory / "absent", "0", frame)
        self.assertEqual(result.returncode, 0)
        self.assertIn("SKIP", result.stdout)


if __name__ == "__main__":
    unittest.main()
