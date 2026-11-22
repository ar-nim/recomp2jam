"""Scenario tests for the consumable O2Jam wire-framing interface.

These are seam tests: no live client, Memoryer, or game data is used. The
golden vectors come from the cited #582 tap and the digits-rig proxy.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
LIB = SCRIPTS / "lib"
sys.path.insert(0, str(LIB))

from wave_recipe import load_recipe  # noqa: E402
from wire_frames import (  # noqa: E402
    Frame,
    FrameStream,
    TruncatedFrameError,
    apply_rules,
    decode_frame,
    encode_frame,
    parse_tap_line,
)

CREATE_BODY = bytes.fromhex("525635383244495342414e4400000000310000000000")
CREATE_FRAME_HEX = "1a00d407525635383244495342414e4400000000310000000000"
LEAVE_LINE = "15:40:35 C2S opcode=3005 (0BBD) len=6 hex=000000000000"
ROOM_LIST_LINE = "15:40:33 S2C opcode=2003 (07D3) len=2420 hex=640000000000000001525635383244495342414e440000610300000001080100000100000000000000000000000000000000000000000200010000000000000000000000000000000000000000000200010000000000000000000000000000000000000000000200010000000000000000000000000000000000000000000200"
# A 10-row ChannelList body: the digits proxy rewrites only complete rows.
CHANNEL_BODY = bytes.fromhex(
    "0a00000000000000640000000000000001000001006400000000000000010000020064000000000000000100000300640000000000000001000004006400000000000000010000050000000000000000000000000600000000000000000000000007000000000000000000000000080000000000000000000000000900640000000000000001"
)
REWRITTEN_CHANNEL_BODY = bytes.fromhex(
    "0a00000000000500640000000000000001000006006400000000000000010000070064000000000000000100000800640000000000000001000009006400000000000000010000000000000000000000000000000100000000000000000000000002000000000000000000000000030000000000000000000000000400640000000000000001"
)
RULE = {
    "direction": "S2C",
    "opcode": 1003,
    "action": "rotate_u16_rows",
    "count_at": 0,
    "row_at": 4,
    "stride": 13,
    "field_at": 2,
    "shift": 5,
}


class WireFramesTests(unittest.TestCase):
    def test_encode_and_decode_preserve_the_2004_create_room_frame(self) -> None:
        encoded = encode_frame(2004, CREATE_BODY)

        self.assertEqual(encoded.hex(), CREATE_FRAME_HEX)
        self.assertEqual(decode_frame(encoded), Frame(2004, CREATE_BODY))

    def test_fragmented_stream_yields_one_frame_only_when_the_body_arrives(self) -> None:
        stream = FrameStream()
        frame = bytes.fromhex(CREATE_FRAME_HEX)

        self.assertEqual(stream.feed(frame[:3]), [])
        self.assertEqual(stream.feed(frame[3:7]), [])
        self.assertEqual(stream.feed(frame[7:]), [Frame(2004, CREATE_BODY)])
        self.assertEqual(stream.pending, b"")
        stream.finish()

    def test_truncated_frame_is_refused_at_stream_boundary(self) -> None:
        stream = FrameStream()
        stream.feed(bytes.fromhex("0a00d407") + CREATE_BODY[:5])

        self.assertEqual(stream.feed(b""), [])
        with self.assertRaisesRegex(TruncatedFrameError, "incomplete frame"):
            stream.finish()

    def test_tap_lines_round_trip_byte_identically_including_preview(self) -> None:
        for line in (LEAVE_LINE, ROOM_LIST_LINE):
            with self.subTest(line=line[:40]):
                parsed = parse_tap_line(line)
                self.assertIsNotNone(parsed)
                self.assertEqual(parsed.render(), line)

    def test_channel_rule_reproduces_the_digits_proxy_rewrite(self) -> None:
        frame = Frame(1003, CHANNEL_BODY)

        rewritten = apply_rules(frame, "S2C", [RULE])

        self.assertEqual(rewritten.body, REWRITTEN_CHANNEL_BODY)
        self.assertEqual(apply_rules(frame, "C2S", [RULE]), frame)

    def test_rewrite_rule_from_recipe_data_is_accepted(self) -> None:
        recipe_dir = SCRIPTS / "recipes"
        rule_path = recipe_dir / "channel-ids-1003.json"
        rule = json.loads(rule_path.read_text(encoding="utf-8"))

        self.assertEqual(apply_rules(Frame(1003, CHANNEL_BODY), "S2C", [rule]).body,
                         REWRITTEN_CHANNEL_BODY)
        self.assertIsNotNone(load_recipe(recipe_dir / "row-vanish-582.json"))


if __name__ == "__main__":
    unittest.main()
