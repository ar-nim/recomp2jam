#!/usr/bin/env python3
"""One consumable interface for O2Jam's `[u16 len][u16 id][body]` wire frames.

The format was pinned in `scripts/wine-room-host.py:107-130` and reimplemented
in eleven scratch proxies. This module owns it: proxies supply rule data,
drivers supply directions, and the framing bytes stay in one place.

`FrameStream` keeps incomplete bytes across reads, so a TCP chunk boundary
cannot split a frame. `finish()` refuses a stream that ends mid-frame instead
of silently treating the partial bytes as evidence.
"""

from __future__ import annotations

import re
import struct
import sys
import unittest
from dataclasses import dataclass
from typing import Mapping, Sequence

HEADER_SIZE = 4
MAX_BODY_SIZE = 0xFFFF - HEADER_SIZE
WIRE_BODY_PREVIEW = 128

TAP_LINE = re.compile(
    r"^(?P<time>\d\d:\d\d:\d\d) (?P<direction>C2S|S2C) opcode=(?P<opcode>\d+) "
    r"\((?P<opcode_hex>[0-9A-Fa-f]{4})\) len=(?P<length>\d+) "
    r"hex=(?P<body>[0-9a-fA-F]*)(?P<detail>.*)$"
)


class WireFrameError(ValueError):
    """Base error for wire bytes that cannot be trusted as evidence."""


class TruncatedFrameError(WireFrameError):
    """The stream ended before a declared frame was complete."""


class InvalidFrameError(WireFrameError):
    """A frame's header or rewrite rule is invalid."""


@dataclass(frozen=True)
class Frame:
    opcode: int
    body: bytes

    def encoded(self) -> bytes:
        return encode_frame(self.opcode, self.body)


@dataclass(frozen=True)
class WireLine:
    """One rendered tap line, including a possibly preview-only body."""

    timestamp: str
    direction: str
    frame: Frame
    declared_body_length: int
    detail: str = ""

    def render(self) -> str:
        return format_wire_line(
            self.timestamp,
            self.direction,
            self.frame,
            body_length=self.declared_body_length,
            detail=self.detail,
        )


def encode_frame(opcode: int, body: bytes) -> bytes:
    """Build one frame; reject values the u16 header cannot carry."""
    if not 0 <= opcode <= 0xFFFF:
        raise InvalidFrameError(f"opcode {opcode} is outside u16")
    if len(body) > MAX_BODY_SIZE:
        raise InvalidFrameError(f"body of {len(body)} bytes exceeds {MAX_BODY_SIZE}")
    return struct.pack("<HH", HEADER_SIZE + len(body), opcode) + body


def decode_frame(data: bytes) -> Frame:
    """Decode exactly one complete frame; partial or trailing bytes fail."""
    if len(data) < HEADER_SIZE:
        raise TruncatedFrameError(
            f"incomplete frame header: {len(data)} byte(s), need {HEADER_SIZE}"
        )
    length, opcode = struct.unpack_from("<HH", data)
    if length < HEADER_SIZE:
        raise InvalidFrameError(f"frame length {length} is below {HEADER_SIZE}")
    if len(data) < length:
        raise TruncatedFrameError(
            f"incomplete frame: declared {length} byte(s), got {len(data)}"
        )
    if len(data) > length:
        raise InvalidFrameError(f"extra bytes after frame: {len(data) - length}")
    return Frame(opcode, data[HEADER_SIZE:])


class FrameStream:
    """Incremental frame reader that retains unparsed bytes."""

    def __init__(self) -> None:
        self._pending = b""

    @property
    def pending(self) -> bytes:
        return self._pending

    def feed(self, chunk: bytes) -> list[Frame]:
        self._pending += chunk
        frames: list[Frame] = []
        while len(self._pending) >= HEADER_SIZE:
            (length,) = struct.unpack_from("<H", self._pending)
            if length < HEADER_SIZE:
                raise InvalidFrameError(f"frame length {length} is below {HEADER_SIZE}")
            if len(self._pending) < length:
                break
            frames.append(decode_frame(self._pending[:length]))
            self._pending = self._pending[length:]
        return frames

    def finish(self) -> None:
        if self._pending:
            raise TruncatedFrameError(
                f"incomplete frame: {len(self._pending)} trailing byte(s) at stream end"
            )


def format_wire_line(
    timestamp: str,
    direction: str,
    frame: Frame,
    *,
    body_length: int | None = None,
    detail: str = "",
) -> str:
    """Render a wire line in the byte-exact format of `wine-room-host.py`."""
    length = len(frame.body) if body_length is None else body_length
    suffix = f" {detail}" if detail else ""
    return (
        f"{timestamp} {direction} opcode={frame.opcode} ({frame.opcode:04X}) "
        f"len={length} hex={frame.body[:WIRE_BODY_PREVIEW].hex()}{suffix}"
    )


def parse_tap_line(line: str) -> WireLine | None:
    """Parse one recorded tap line; ignore connection notes, reject bad frames."""
    if "opcode=" not in line:
        return None
    match = TAP_LINE.match(line.rstrip("\n"))
    if match is None:
        raise InvalidFrameError(f"unparsable tap line: {line!r}")
    fields = match.groupdict()
    opcode = int(fields["opcode"])
    opcode_hex = int(fields["opcode_hex"], 16)
    if opcode != opcode_hex:
        raise InvalidFrameError(f"tap opcode {opcode} disagrees with {fields['opcode_hex']}")
    body = bytes.fromhex(fields["body"])
    declared_length = int(fields["length"])
    if len(body) > declared_length:
        raise InvalidFrameError(f"tap body of {len(body)} exceeds declared {declared_length}")
    return WireLine(
        timestamp=fields["time"],
        direction=fields["direction"],
        frame=Frame(opcode, body),
        declared_body_length=declared_length,
        detail=fields["detail"],
    )


def parse_tap_lines(text: str) -> list[WireLine]:
    parsed = [parse_tap_line(line) for line in text.splitlines()]
    return [line for line in parsed if line is not None]


def _matches_rule(frame: Frame, direction: str, rule: Mapping[str, object]) -> bool:
    return direction == rule.get("direction") and frame.opcode == rule.get("opcode")


def rotate_u16_rows(body: bytes, rule: Mapping[str, object]) -> bytes:
    """Rotate one u16 field per fixed-size row, as the digits-rig proxy did."""
    count_at = int(rule["count_at"])
    row_at = int(rule["row_at"])
    stride = int(rule["stride"])
    field_at = int(rule["field_at"])
    shift = int(rule["shift"])
    if count_at < 0 or count_at + 4 > len(body):
        raise InvalidFrameError(f"row count offset {count_at} outside body")
    (count,) = struct.unpack_from("<i", body, count_at)
    if count <= 0 or row_at < 0 or row_at + count * stride > len(body):
        raise InvalidFrameError(f"row layout count={count} does not fit body")
    rewritten = bytearray(body)
    for row in range(count):
        field = row_at + row * stride + field_at
        if field + 2 > row_at + (row + 1) * stride:
            raise InvalidFrameError(f"row field offset {field_at} outside row {row}")
        (value,) = struct.unpack_from("<H", rewritten, field)
        struct.pack_into("<H", rewritten, field, (value + shift) % count)
    return bytes(rewritten)


def apply_rules(
    frame: Frame, direction: str, rules: Sequence[Mapping[str, object]]
) -> Frame:
    """Apply matching proxy rules as data; unmatched frames pass unchanged."""
    rewritten = frame
    for rule in rules:
        if not _matches_rule(rewritten, direction, rule):
            continue
        action = rule.get("action")
        if action == "rotate_u16_rows":
            rewritten = Frame(rewritten.opcode, rotate_u16_rows(rewritten.body, rule))
        else:
            raise InvalidFrameError(f"unsupported rewrite action {action!r}")
    return rewritten


class WireFramesTests(unittest.TestCase):
    """The lib selftest: golden vectors, fragmentation, and refusal."""

    def test_build_and_parse_one_frame(self) -> None:
        body = bytes.fromhex("525635383244495342414e4400000000310000000000")
        encoded = encode_frame(2004, body)
        self.assertEqual(encoded[:4].hex(), "1a00d407")
        self.assertEqual(decode_frame(encoded), Frame(2004, body))

    def test_fragmented_stream_reassembles_and_refuses_a_truncated_tail(self) -> None:
        stream = FrameStream()
        encoded = encode_frame(2004, b"RV582DISBAND")
        self.assertEqual(stream.feed(encoded[:3]), [])
        self.assertEqual(stream.feed(encoded[3:]), [Frame(2004, b"RV582DISBAND")])
        stream.feed(encoded[:5])
        with self.assertRaises(TruncatedFrameError):
            stream.finish()


if __name__ == "__main__":
    if sys.argv[1:] == ["selftest"]:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(WireFramesTests)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        raise SystemExit(0 if result.wasSuccessful() else 1)
    print(__doc__)
