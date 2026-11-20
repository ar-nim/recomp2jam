#!/usr/bin/env python3
"""Assert the Channel board's painted fill digits against the 1003 ids.

The 7.55 exe paints ``channel_id + 1`` in the two `CH_Num` wells
(docs/channel-fill.md, proxy runs +5/+10). This probe reads a native
800x600 board frame from the shots walk, decodes the 13x18 digit frames
from `ChannelInfo_CH_Num.ojs`, and checks every visible row's two wells
pixel for pixel. No game bytes enter the repo: the archive stays under
the local install-root, and a hidden-data run skips cleanly.

Usage:
  scripts/rig-channel-digits-probe.py \
      --frame docs/screenshots/channel-digits-native.png \
      --ids 0,1,2,3 \
      --install-root install-root
"""

from __future__ import annotations

import argparse
import struct
import sys
from pathlib import Path

from PIL import Image

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INSTALL_ROOT = REPO_ROOT / "install-root"
INTERFACE_ARCHIVE = Path("O2Jam/Image/Interface1.opi")
DIGIT_SPRITE = "ChannelInfo_CH_Num.ojs"

# Dialog_ChannelInfo.bnd rects 2..5, plus the bar inset and digit locals
# (docs/channel-fill.md, geometry-audit.md). Four viewport slots, in
# wire-list order; the shots walk stages ids 0..3 in these four rows.
ROW_TOPS = (224, 275, 324, 375)
ROW_LEFT = 216
BAR_INSET = 1
DIGIT_LOCALS = ((50, 11), (63, 11))
DIGIT_SIZE = (13, 18)
SPRITE55_MAGIC = 0x0555
OPI_ENTRY_LEN = 152


class ProbeError(RuntimeError):
    """A frame, archive, or digit well did not match the pinned rule."""


def read_opi_entry(archive: Path, name: str) -> bytes:
    """Return one named body from a flat OPI archive (docs/file-formats/opi.md)."""
    data = archive.read_bytes()
    if len(data) < 16:
        raise ProbeError(f"short OPI archive: {archive}")
    version, count = struct.unpack_from("<II", data, 0)
    if version != 2:
        raise ProbeError(f"OPI version {version}, want 2")
    table_start = len(data) - OPI_ENTRY_LEN * count
    if table_start < 16:
        raise ProbeError(f"OPI entry table overruns {archive}")
    for index in range(count):
        entry = table_start + index * OPI_ENTRY_LEN
        kind = struct.unpack_from("<I", data, entry)[0]
        stored_name = data[entry + 4 : entry + 132].split(b"\0", 1)[0]
        if kind != 1 or stored_name.decode("ascii", "replace") != name:
            continue
        offset, size = struct.unpack_from("<II", data, entry + 132)
        body = data[offset : offset + size]
        if len(body) != size:
            raise ProbeError(f"truncated {name} body in {archive}")
        return body
    raise ProbeError(f"no {name} in {archive}")


def expand_rgb555(word: int) -> tuple[int, int, int]:
    """Decode one 5-5-5 word the same way o2jam_render::expand_rgb555 does."""
    parts = ((word >> 10) & 31, (word >> 5) & 31, word & 31)
    return tuple((part << 3) | (part >> 2) for part in parts)  # type: ignore[return-value]


def digit_templates(body: bytes) -> list[bytes]:
    """Return ten raw 13x18 RGB templates in stored (digit 0..9) order."""
    if len(body) < 8:
        raise ProbeError("short sprite container")
    _tag, magic, count, _colorkey = struct.unpack_from("<HHHH", body, 0)
    if magic != SPRITE55_MAGIC:
        raise ProbeError(f"sprite magic 0x{magic:04x}, want 0x{SPRITE55_MAGIC:04x}")
    blob_start = 8 + 20 * count
    width, height = DIGIT_SIZE
    templates: list[bytes] = []
    for index in range(count):
        rec = 8 + 20 * index
        x, y, w, h, offset, size, _ = struct.unpack_from("<HHHHIII", body, rec)
        if (x, y, w, h) != (0, 0, width, height) or size != width * height * 2:
            raise ProbeError(f"digit frame {index} is not a raw {width}x{height} image")
        start = blob_start + offset
        words = struct.unpack_from(f"<{width * height}H", body, start)
        templates.append(b"".join(bytes(expand_rgb555(word)) for word in words))
    if len(templates) < 10:
        raise ProbeError(f"sprite holds {len(templates)} digit frames, want 10")
    return templates


def painted_digit(
    pixels: object, x: int, y: int, templates: list[bytes]
) -> tuple[int, int]:
    """Return (closest digit, differing channel bytes) for one well."""
    width, height = DIGIT_SIZE
    crop = b"".join(
        bytes(pixels[x + dx, y + dy]) for dy in range(height) for dx in range(width)
    )
    ranked = sorted(
        (
            sum(left != right for left, right in zip(crop, template)),
            digit,
        )
        for digit, template in enumerate(templates[:10])
    )
    return ranked[0][1], ranked[0][0]


def shown_digits(channel_id: int) -> tuple[int, int]:
    """The exe's one-based presentation number, split into two digits."""
    shown = channel_id + 1
    return shown // 10 % 10, shown % 10


def probe(frame: Path, ids: list[int], install_root: Path) -> int:
    archive = install_root / INTERFACE_ARCHIVE
    if not archive.is_file():
        print(f"rig-channel-digits: SKIP (no {archive}; hidden-data run)")
        return 0
    if not frame.is_file():
        raise ProbeError(f"missing board frame {frame}; run the shots walk first")
    if not ids:
        raise ProbeError("no channel ids supplied")
    if len(ids) > len(ROW_TOPS):
        raise ProbeError(f"at most {len(ROW_TOPS)} rows can be probed, got {len(ids)}")
    if any(channel_id < 0 for channel_id in ids):
        raise ProbeError("channel ids must be non-negative")

    with Image.open(frame) as image:
        if image.size != (800, 600):
            raise ProbeError(f"frame is {image.size}, want (800, 600)")
        # `convert` owns its pixels, so the accessor survives the `with`.
        rgb = image.convert("RGB")
    pixels = rgb.load()
    templates = digit_templates(read_opi_entry(archive, DIGIT_SPRITE))

    for row, channel_id in enumerate(ids):
        expected = shown_digits(channel_id)
        for slot, ((local_x, local_y), wanted) in enumerate(zip(DIGIT_LOCALS, expected)):
            x = ROW_LEFT + BAR_INSET + local_x
            y = ROW_TOPS[row] + BAR_INSET + local_y
            painted, differences = painted_digit(pixels, x, y, templates)
            if differences != 0 or painted != wanted:
                raise ProbeError(
                    f"row {row} id {channel_id}: digit {slot} paints {painted} "
                    f"({differences} differing bytes), want {wanted}"
                )
        print(
            f"rig-channel-digits: row {row} id {channel_id} -> "
            f"Ch.{expected[0]}{expected[1]} OK"
        )
    print(f"rig-channel-digits: {len(ids)} row(s) match channel_id + 1")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--frame",
        type=Path,
        default=REPO_ROOT / "docs/screenshots/channel-digits-native.png",
        help="native 800x600 channel frame from the shots walk",
    )
    parser.add_argument(
        "--ids",
        default="0,1,2,3",
        help="wire channel ids in visible row order (default: 0,1,2,3)",
    )
    parser.add_argument(
        "--install-root",
        type=Path,
        default=DEFAULT_INSTALL_ROOT,
        help="install root holding O2Jam/Image/Interface1.opi",
    )
    args = parser.parse_args()
    try:
        ids = [int(part.strip()) for part in args.ids.split(",") if part.strip()]
        return probe(args.frame, ids, args.install_root)
    except (ProbeError, OSError, ValueError, struct.error) as error:
        print(f"rig-channel-digits: FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
