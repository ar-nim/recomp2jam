"""Seam tests for the #240 hover-press join stimulus.

The exe leg is the live proof; these tests pin the stimulus ORDER and the
no-SetCursorPos press shape at the seam the drive owns. A green test never
substitutes for a qualified capture (rules 7-8).
"""
from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
MODULE_PATH = SCRIPTS_DIR / "wine-scene.py"
HOST_PATH = SCRIPTS_DIR / "wine-room-host.py"
RUNNER_PATH = SCRIPTS_DIR / "rig-wine-run.sh"


def load_module(name: str, path: Path):
    module_spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(module_spec)
    assert module_spec.loader is not None
    sys.modules[name] = module
    module_spec.loader.exec_module(module)
    return module


scene = load_module("wine_scene", MODULE_PATH)
host = load_module("wine_room_host", HOST_PATH)


class FakeLog:
    def __init__(self) -> None:
        self.events: list[str] = []

    def event(self, message: str) -> None:
        self.events.append(message)


class FakeFrames:
    def __init__(self) -> None:
        import numpy as np
        self.rest = np.zeros((600, 800, 3), dtype=np.int16)
        self.hovered = self.rest.copy()
        self.hovered[85:120, 40:330] = 200
        self.saved: list[Path] = []

    def grab(self):
        return self.rest if not self.saved else self.hovered

    def capture(self, path: Path) -> float:
        self.saved.append(path)
        return 0.0

    def row_highlight_delta(self, rest) -> float:
        return 42.0


class FakeServer:
    def __init__(self, joined: bool = True) -> None:
        self.joined = joined
        self.waited: list[tuple[str, float]] = []

    def wait_for(self, opcode: str, timeout: float) -> bool:
        self.waited.append((opcode, timeout))
        return self.joined


class FakeInjector:
    def __init__(self) -> None:
        self.calls: list[tuple] = []

    def cite(self, aim: str) -> tuple[int, int]:
        self.calls.append(("cite", aim))
        return 100, 100

    def warp_pointer(self, x: int, y: int) -> None:
        self.calls.append(("warp", x, y))

    def send_press(self) -> None:
        self.calls.append(("press",))


class HoverPressStimulusTest(unittest.TestCase):
    def test_warp_then_capture_highlight_then_one_press(self) -> None:
        frames, injector, server, log = (
            FakeFrames(), FakeInjector(), FakeServer(), FakeLog())
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"):
                joined = scene.hover_press_join(
                    frames, injector, server, log, Path(tmp), 12.0)
        self.assertTrue(joined)
        self.assertEqual(
            [call[0] for call in injector.calls],
            ["cite", "warp", "press"],
        )
        self.assertEqual(("warp", 100, 100), injector.calls[1])
        self.assertEqual([("0BBA", 12.0)], server.waited)
        self.assertEqual(1, len(frames.saved))
        self.assertEqual("room-row-hover.png", frames.saved[0].name)
        self.assertTrue(any("hover delta=" in line for line in log.events))

    def test_no_join_keeps_the_single_stimulus(self) -> None:
        frames, injector, server, log = (
            FakeFrames(), FakeInjector(), FakeServer(joined=False), FakeLog())
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"):
                joined = scene.hover_press_join(
                    frames, injector, server, log, Path(tmp), 12.0)
        self.assertFalse(joined)
        self.assertEqual(1, sum(1 for c in injector.calls if c[0] == "press"))


class NoJoinLogTest(unittest.TestCase):
    def test_hover_press_failure_names_one_stimulus(self) -> None:
        self.assertEqual(
            "stage waiting FAILED reason=no 0BBA after one hover-press",
            scene.no_join_failure_message("hover-press"),
        )

    def test_dclick_failure_keeps_two_join_wording(self) -> None:
        self.assertEqual(
            "stage waiting FAILED reason=no 0BBA after two joins",
            scene.no_join_failure_message("dclick"),
        )


class WireEvidenceTest(unittest.TestCase):
    def test_wire_tap_splits_partial_frames_without_loss(self) -> None:
        first = host.frame(1002, b"ab")
        second = host.frame(2005, b"xyz")
        frames, pending = host.split_wire_frames(first + second[:5])
        self.assertEqual([first], frames)
        self.assertEqual(second[:5], pending)
        frames, pending = host.split_wire_frames(pending + second[5:])
        self.assertEqual([second], frames)
        self.assertEqual(b"", pending)

    def test_wire_log_counts_only_downstream_room_created_events(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "room-stage.log"
            path.write_text(
                "12:00:00 C2S opcode=2005 (07D5) len=4 hex=01000000\n"
                "12:00:01 S2C opcode=1003 (03EB) len=8 hex=00\n"
                "12:00:02 S2C opcode=2005 (07D5) len=4 hex=00000000 "
                "ROOM_CREATED room=00000000\n",
                encoding="utf-8",
            )
            wire = scene.WireTapLog(path)
            self.assertEqual(1, len(wire.take()))
            self.assertEqual(1, wire.room_created_count)
            self.assertEqual([], wire.take())
            path.write_text(path.read_text() +
                            "12:00:03 S2C opcode=2005 (07D5) len=4 hex=01000000\n")
            self.assertEqual(1, len(wire.take()))
            self.assertEqual(2, wire.room_created_count)

    def test_paint_wait_keeps_popup_when_dismissal_disabled(self) -> None:
        class PopupFrames(FakeFrames):
            def popup_is_up(self):
                return True

            def grab(self):
                return self.rest

            def row_painted(self):
                return False

        frames, injector, log = PopupFrames(), FakeInjector(), FakeLog()
        wire = scene.WireTapLog(Path(tempfile.gettempdir()) / "absent-wire.log")
        with tempfile.TemporaryDirectory() as tmp:
            wire.path = Path(tmp) / "wire.log"
            wait = scene.RoomPaintWait(frames, injector, wire, log, "none")
            self.assertFalse(wait.run(0.1))
        self.assertEqual([], [c for c in injector.calls if c[0] == "press"])
        self.assertFalse(any(c[0] == "click" for c in injector.calls))
        self.assertTrue(any("popup_up=True" in line for line in log.events))


class BoundaryInstrumentationTest(unittest.TestCase):
    def test_wire_boundary_names_first_frames_since_channel_enter(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "wire.log"
            path.write_text(
                "20:56:23 C2S opcode=1004 (03EC) len=4 hex=\n"
                "20:56:43 S2C opcode=2005 (07D5) len=21 hex=\n"
                "20:56:59 C2S opcode=2010 (07DA) len=0 hex=\n"
                "20:56:59 C2S opcode=2024 (07E8) len=30 hex=\n",
                encoding="utf-8",
            )
            log = FakeLog()
            scene.WireTapLog(path).note_boundaries(log, "popup-up")
        self.assertEqual(1, len(log.events))
        line = log.events[0]
        self.assertIn("03EC@+0.0s", line)
        self.assertIn("2005@+20.0s", line)
        self.assertIn("07DA@+36.0s", line)
        self.assertIn("07E8@+36.0s", line)
        self.assertIn("frames=C2S:3/S2C:1", line)

    def test_host_boundary_reports_pulses_without_changing_host_state(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out_dir = Path(tmp)
            (out_dir / "host").mkdir()
            (out_dir / "host" / "host.log").write_text(
                "20:56:20 host room created number=0 title=WINEJOIN\n"
                "20:56:30 host keepalive sent 6001 count=1 t+10.0s\n"
                "20:56:41 host recreate start t+21.0s\n"
                "20:56:43 host recreate done t+23.0s (fuse suppression)\n",
                encoding="utf-8",
            )
            log = FakeLog()
            fake_host = scene.Host(out_dir, 757, 3, log)
            fake_host.note_boundaries(log, "row-paint")
        self.assertEqual(4, len(log.events))
        self.assertTrue(all("host boundary row-paint" in line for line in log.events))


class ManifestVerdictTest(unittest.TestCase):
    @staticmethod
    def join_fields(out_dir: Path, wire_log: Path) -> str:
        source = RUNNER_PATH.read_text()
        start = source.index("join_outcome_fields() {")
        end = source.index("\n}\n", start) + 3
        script = source[start:end] + "\njoin_outcome_fields\n"
        result = subprocess.run(
            ["bash", "-c", script],
            env={"OUT_DIR": str(out_dir), "WIRE_LOG": str(wire_log)},
            capture_output=True, text=True, check=True,
        )
        return result.stdout.strip()

    def test_manifest_records_absent_join_and_one_press(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out_dir, wire_log = Path(tmp), Path(tmp) / "wire.log"
            wire_log.write_text(
                "12:00:00 S2C opcode=2005 (07D5) len=21\n", encoding="utf-8")
            (out_dir / "drive.log").write_text(
                "12:00:01 inject send-press down=1 up=1\n", encoding="utf-8")
            self.assertEqual(
                "join_0BBA=absent join_press_count=1",
                self.join_fields(out_dir, wire_log),
            )
            with wire_log.open("a", encoding="utf-8") as handle:
                handle.write("12:00:02 C2S opcode=3002 (0BBA) len=0\n")
            self.assertEqual(
                "join_0BBA=present join_press_count=1",
                self.join_fields(out_dir, wire_log),
            )


class IdleRelayTest(unittest.TestCase):
    def test_idle_timeout_does_not_end_the_wire_relay(self) -> None:
        import socket as socket_module

        relay_frame = host.frame(1003, b"ok")

        class TimeoutThenFrameSource:
            def __init__(self) -> None:
                self.recv_count = 0

            def recv(self, size: int) -> bytes:
                self.recv_count += 1
                if self.recv_count == 1:
                    raise socket_module.timeout("idle")
                if self.recv_count == 2:
                    return relay_frame
                return b""

        class RecordingTarget:
            def __init__(self) -> None:
                self.sent: list[bytes] = []

            def sendall(self, data: bytes) -> None:
                self.sent.append(data)

            def shutdown(self, how: int) -> None:
                pass

        class RecordingWireLog:
            def __init__(self) -> None:
                self.frames: list[tuple[str, bytes]] = []
                self.connections: list[str] = []

            def frame(self, direction: str, data: bytes) -> None:
                self.frames.append((direction, data))

            def connection(self, message: str) -> None:
                self.connections.append(message)

        source, target, wire_log = (
            TimeoutThenFrameSource(), RecordingTarget(), RecordingWireLog())
        host.relay_wire(source, target, "S2C", wire_log)
        self.assertEqual([relay_frame], target.sent)
        self.assertEqual([("S2C", relay_frame)], wire_log.frames)
        self.assertFalse(any("timed out" in line for line in wire_log.connections))


class PressShapeTest(unittest.TestCase):
    def test_press_sends_no_cursor_move(self) -> None:
        source = (MODULE_PATH.parent / "sendinput-helper.c").read_text()
        self.assertIn("static void press_at_pointer(void)", source)
        self.assertIn("press down=%u up=%u", source)
        press_body = source.split("static void press_at_pointer(void) {", 1)[1]
        press_body = press_body.split("\n}", 1)[0]
        self.assertNotIn("move_to", press_body)
        self.assertNotIn("SetCursorPos", press_body)
        self.assertIn("left_down()", press_body)
        self.assertIn("left_up()", press_body)

    def test_drive_accepts_both_join_modes(self) -> None:
        result = subprocess.run(
            [sys.executable, str(MODULE_PATH), "--help"],
            capture_output=True, text=True, check=True,
        )
        self.assertIn("--join-mode", result.stdout)
        self.assertIn("hover-press", result.stdout)
        self.assertIn("dclick", result.stdout)


if __name__ == "__main__":
    unittest.main()
