"""Scenario-named guards for the #378 bot-master play chain.

Live caller chain: `wine-scene.py Host.start` launches `wine-room-host.py`;
`main` opens the room and calls `hold_session`; the `start` command reaches
`send_start`, and the server's 4017 score-submission broadcast reaches the
`hold_session` event loop. These seam tests pin the two master frames the
guest needs (4048 tick sync and a reactive 4016 submit); the qualified
two-exe Playing boot is the live proof, never these guards.
"""
from __future__ import annotations

import importlib.util
import struct
import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
MODULE_PATH = SCRIPTS_DIR / "wine-room-host.py"


def load_module(name: str, path: Path):
    module_spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(module_spec)
    assert module_spec.loader is not None
    sys.modules[name] = module
    module_spec.loader.exec_module(module)
    return module


host = load_module("master_play_chain_host", MODULE_PATH)


class RecordingSocket:
    def __init__(self) -> None:
        self.sent: list[tuple[int, bytes]] = []

    def sendall(self, data: bytes) -> None:
        (length, opcode) = struct.unpack_from("<HH", data)
        self.sent.append((opcode, data[4:length]))


class RecordingLog:
    def __init__(self) -> None:
        self.events: list[str] = []

    def write(self, message: str) -> None:
        self.events.append(message)


class MasterStartReachesTheGuestPlayGateTests(unittest.TestCase):
    def test_start_sends_eleven_master_latency_frames_with_last_flag(self) -> None:
        sock, log = RecordingSocket(), RecordingLog()

        host.send_start(sock, log)

        latency = [(opcode, body) for opcode, body in sock.sent if opcode == 4048]
        self.assertEqual(11, len(latency))
        decoded = [struct.unpack("<iiiii", body) for _, body in latency]
        self.assertEqual(list(range(11)), [frame[3] for frame in decoded])
        self.assertEqual([0] * 10 + [1], [frame[4] for frame in decoded])
        self.assertEqual({0}, {frame[0] for frame in decoded})
        self.assertTrue(any("4048" in line for line in log.events))

    def test_master_score_body_is_a_zero_idle_submission_at_full_life(self) -> None:
        body = host.encode_master_score()
        self.assertEqual(24, len(body))
        cool, good, bad, miss, max_combo, jam, max_jam = struct.unpack_from("<7H", body)
        self.assertEqual((0, 0, 0, 0, 0, 0, 0),
                         (cool, good, bad, miss, max_combo, jam, max_jam))
        score, life, speed, long_note = struct.unpack_from("<IBBi", body, 14)
        self.assertEqual((0, 255, 1, 0), (score, life, speed, long_note))


class GuestSubmissionUnblocksTheResultTests(unittest.TestCase):
    def test_guest_score_broadcast_makes_the_master_submit_once(self) -> None:
        sock, log = RecordingSocket(), RecordingLog()

        submitted = host.submit_master_score_if_guest_finished(
            sock, 4017, b"\x01", False, log)
        again = host.submit_master_score_if_guest_finished(
            sock, 4017, b"\x01", submitted, log)

        self.assertTrue(submitted)
        self.assertTrue(again)
        self.assertEqual([(4016, host.encode_master_score())], sock.sent)
        self.assertTrue(any("master score submitted" in line for line in log.events))

    def test_own_score_broadcast_or_wrong_opcode_never_submits(self) -> None:
        sock, log = RecordingSocket(), RecordingLog()

        for opcode, body in ((4017, b"\x00"), (4015, b"\x01"), (4017, b"")):
            self.assertFalse(host.submit_master_score_if_guest_finished(
                sock, opcode, body, False, log))

        self.assertEqual([], sock.sent)


if __name__ == "__main__":
    unittest.main()
