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

    def scene_label(self) -> str:
        return "room"


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


class WaitingObservationTest(unittest.TestCase):
    def test_distinct_frames_are_saved_with_wire_times_without_a_scene_claim(self) -> None:
        import numpy as np

        class ChangingFrames:
            def __init__(self) -> None:
                self.frames = [
                    np.zeros((8, 8, 3), dtype=np.int16),
                    np.full((8, 8, 3), 90, dtype=np.int16),
                ]
                self.saved: list[str] = []
                self.grab_count = 0

            def grab(self):
                frame = self.frames[min(self.grab_count, 1)]
                self.grab_count += 1
                self.last = frame
                return frame

            def capture(self, path: Path) -> float:
                self.saved.append(path.name)
                return 0.0

            def scene_label(self) -> str:
                return "room"

            def mean_luma(self) -> float:
                return 0.0

        class JoinedWire:
            def opcodes_since(self, opcode: int) -> list[str]:
                assert 3002 == opcode
                return ["C2S:3002@+0.0s", "S2C:3003@+0.0s"]

        frames, log = ChangingFrames(), FakeLog()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "sleep"), patch.object(
                scene.time, "time", side_effect=[0, 0.1, 1.1, 1.2, 2.1, 2.2]
            ):
                scene.observe_waiting_transition(
                    frames, JoinedWire(), log, Path(tmp), 2.0)
        self.assertEqual(
            ["waiting-observe-001.png", "waiting-observe-002.png"],
            frames.saved,
        )
        self.assertEqual(2, len(log.events))
        self.assertTrue(all("waiting observe t=+" in line for line in log.events))
        self.assertTrue(all("wire=C2S:3002@+0.0s,S2C:3003@+0.0s" in line
                            for line in log.events))
        self.assertFalse(any(line.startswith("frame ") for line in log.events))


class WaitingSignatureTest(unittest.TestCase):
    """Seam proof for the live chain rig-wine-run -> wine-scene.drive ->
    capture_scene -> Frames.scene_label; the qualification pack proves the
    boot-time reachability, so these tests need no game process.
    """

    @staticmethod
    def scene_frame(is_waiting: bool):
        import numpy as np

        frame = np.zeros((600, 800, 3), dtype=np.int16)
        frame[40:140, :] = 120      # bright tab strip, as in the room stage
        frame[555:595, :] = 110     # menu luma above BOARD_MENU_LUMA
        if is_waiting:
            frame[15:40, 570:780] = (40, 200, 60)
            frame[125:165, 625:780] = (220, 140, 40)
            frame[190:420, 625:780] = 170
        else:
            frame[15:40, 570:780] = (40, 70, 140)
            frame[125:165, 625:780] = (40, 70, 140)
            frame[190:420, 625:780] = (40, 70, 120)
        return frame

    def waiting_frames(self, is_waiting: bool):
        import numpy as np

        frames = scene.Frames(":299", Path(tempfile.mkdtemp()))
        frames.last = self.scene_frame(is_waiting)
        return frames

    def test_waiting_pixels_outrank_the_room_menu_rule(self) -> None:
        frames = self.waiting_frames(is_waiting=True)
        green, orange, bright = frames.waiting_metrics()
        self.assertGreater(green, scene.WAITING_GREEN_TOP_FRACTION)
        self.assertGreater(orange, scene.WAITING_READY_PILL_FRACTION)
        self.assertGreater(bright, scene.WAITING_SKILL_PANEL_FRACTION)
        self.assertEqual("waiting", frames.scene_label())

    def test_room_pixels_are_not_relabelled_waiting(self) -> None:
        frames = self.waiting_frames(is_waiting=False)
        self.assertEqual("room", frames.scene_label())

    def test_unproven_waiting_scene_writes_no_screenshot(self) -> None:
        frames, log = FakeFrames(), FakeLog()
        with tempfile.TemporaryDirectory() as tmp:
            captured = scene.capture_scene(
                frames, log, Path(tmp), "waiting", "server:0BBA")
            self.assertFalse(captured)
            self.assertEqual([], list(Path(tmp).glob("*.png")))
        self.assertTrue(any("stage waiting NOT captured" in line
                            for line in log.events))

    def test_evidence_line_names_metrics_and_pack_lines(self) -> None:
        line = scene.waiting_evidence_line((0.615, 0.298, 0.681))
        self.assertIn("green=0.615", line)
        self.assertIn("orange=0.298", line)
        self.assertIn("bright=0.681", line)
        self.assertIn("c1-waiting-observe/drive.log:36", line)
        self.assertIn("c1-waiting-observe/wire/room-stage.log:21-22", line)


class LoadingSignatureTest(unittest.TestCase):
    """Seam proof for the live chain rig-wine-run -> wine-scene.drive ->
    wait_until(loading_is_up) -> capture_scene -> Frames.scene_label. The
    qualification boot's pack is the live proof; these tests need no game.
    """

    @staticmethod
    def frame(with_warm: bool = True, with_badge: bool = True,
              with_logo: bool = True):
        import numpy as np

        frame = np.full((600, 800, 3), 60, dtype=np.int16)
        if with_warm:
            frame[100:500, 100:350] = (190, 140, 100)
        if with_badge:
            frame[410:490, 700:800] = (240, 240, 240)
        if with_logo:
            frame[0:40, 700:800] = (240, 30, 20)
        return frame

    def frames_for(self, frame):
        frames = scene.Frames(":299", Path(tempfile.mkdtemp()))
        frames.last = frame
        return frames

    def test_loading_pixels_outrank_the_planet_rule(self) -> None:
        frames = self.frames_for(self.frame())
        warm, badge, logo = frames.loading_metrics()
        self.assertGreater(warm, scene.LOADING_WARM_ART_FRACTION)
        self.assertGreater(badge, scene.LOADING_BADGE_BRIGHT_FRACTION)
        self.assertGreater(logo, scene.LOADING_LOGO_SATURATION)
        self.assertEqual("loading", frames.scene_label())

    def test_planet_frame_is_not_relabelled_loading(self) -> None:
        import numpy as np

        planet = np.full((600, 800, 3), 60, dtype=np.int16)
        planet[50:140, :200] = 150     # bright emblem strip, dark MENU bar
        self.assertEqual("planet", self.frames_for(planet).scene_label())

    def test_one_missing_box_is_not_a_loading_scene(self) -> None:
        for missing in ("with_warm", "with_badge", "with_logo"):
            frame = self.frame(**{missing: False})
            self.assertNotEqual("loading", self.frames_for(frame).scene_label())

    def test_unproven_loading_scene_writes_no_screenshot(self) -> None:
        frames, log = FakeFrames(), FakeLog()
        with tempfile.TemporaryDirectory() as tmp:
            captured = scene.capture_scene(
                frames, log, Path(tmp), "loading", "server:4012")
            self.assertFalse(captured)
            self.assertEqual([], list(Path(tmp).glob("*.png")))
        self.assertTrue(any("stage loading NOT captured" in line
                            for line in log.events))

    def test_evidence_line_names_metrics_and_pack_files(self) -> None:
        line = scene.loading_evidence_line((0.284, 0.278, 54.0))
        self.assertIn("warm=0.284", line)
        self.assertIn("badge=0.278", line)
        self.assertIn("logo_sat=54.0", line)
        self.assertIn("c1-start-cue/drive.log:63", line)
        self.assertIn("c1-start-cue/wire/room-stage.log:28-31", line)
        self.assertIn("c1-start-cue/artifacts/screenshots/waiting-cue-012.png",
                      line)


class PlayingSignatureTest(unittest.TestCase):
    """Seam proof for the live chain rig-wine-run -> wine-scene.drive ->
    wait_until(play observe) -> capture_scene -> Frames.scene_label. The
    Playing pack is the live proof; these tests need no game process.
    """

    @staticmethod
    def frame(is_playing: bool):
        import numpy as np

        frame = np.full((600, 800, 3), 60, dtype=np.int16)
        if is_playing:
            frame[0:520, 0:205] = 30                    # dark play highway
            frame[520:600, 200:800] = (40, 120, 220)    # blue play HUD
        return frame

    def test_play_pixels_outrank_the_loading_rule(self) -> None:
        frames = scene.Frames(":299", Path(tempfile.mkdtemp()))
        frames.last = self.frame(is_playing=True)
        lane, hud = frames.playing_metrics()
        self.assertGreater(lane, scene.PLAYING_LANE_DARK_FRACTION)
        self.assertGreater(hud, scene.PLAYING_HUD_BLUE_FRACTION)
        self.assertEqual("playing", frames.scene_label())

    def test_loading_pixels_are_not_relabelled_playing(self) -> None:
        frames = scene.Frames(":299", Path(tempfile.mkdtemp()))
        frames.last = self.frame(is_playing=False)
        self.assertNotEqual("playing", frames.scene_label())

    def test_evidence_line_names_metrics_and_wire_cue(self) -> None:
        line = scene.playing_evidence_line((0.92, 0.48))
        self.assertIn("lane_dark=0.920", line)
        self.assertIn("hud_blue=0.480", line)
        self.assertIn("cue=server:4014", line)
        self.assertIn("artifacts/screenshots/playing.png", line)

    def test_drive_accepts_playing_target(self) -> None:
        self.assertIn("playing", scene.SCENE_TARGETS)
        result = subprocess.run(
            [sys.executable, str(MODULE_PATH), "--help"],
            capture_output=True, text=True, check=True,
        )
        self.assertIn("playing", result.stdout)


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


class ServerOpcodeVocabularyTest(unittest.TestCase):
    def test_ready_and_confirm_are_hex_opcodes_not_decimal_labels(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "server.log"
            ready_line = (
                "2026-11-18T01:20:29.236+08:00 INFO  0FA8 --- "
                "[ WaitingController ] [ tester ]: ready\n")
            confirm_line = (
                "2026-11-18T01:20:45.171+08:00 INFO  0FAC --- "
                "[ WaitingController ] [ tester ]: music loaded\n")
            with patch.object(scene.time, "sleep"):
                path.write_text(ready_line, encoding="utf-8")
                hex_view = scene.ServerLog(path, "tester")
                hex_view.offset = 0      # fixture predates the reader
                self.assertTrue(hex_view.wait_for("0FA8", 0.1))
                with path.open("a", encoding="utf-8") as handle:
                    handle.write(confirm_line)
                self.assertTrue(hex_view.wait_for("0FAC", 0.1))
                decimal_view = scene.ServerLog(path, "tester")
                decimal_view.offset = 0
                self.assertFalse(decimal_view.wait_for("4008", 0.1))
                self.assertFalse(decimal_view.wait_for("4012", 0.1))


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


class StartCueDiscoveryTest(unittest.TestCase):
    """Seam proof for the live chain rig-wine-run -> wine-scene.drive ->
    observe_start_cue -> StartCueObserver. The cue boot's pack is the live
    evidence; these tests only pin the inventory/stimulus order.
    """

    class AdvancingClock:
        def __init__(self) -> None:
            self.now = 1000.0

        def time(self) -> float:
            return self.now

        def sleep(self, seconds: float) -> None:
            self.now += seconds

    class CueFrames:
        def __init__(self, changing: bool = False) -> None:
            import numpy as np

            self.changing = changing
            self.frame = np.zeros((600, 800, 3), dtype=np.int16)
            self.last = self.frame
            self.grabs = 0
            self.saved: list[str] = []

        def grab(self):
            self.grabs += 1
            if self.changing and self.grabs > 1:
                self.frame = self.frame + 1
            self.last = self.frame
            return self.frame

        def capture(self, path: Path) -> float:
            self.saved.append(path.name)
            return 0.0

        def scene_label(self) -> str:
            return "waiting"

        def mean_luma(self) -> float:
            return 100.0

    class CueWire:
        def __init__(self, gate: bool = False) -> None:
            self.gate = gate

        def opcodes_since(self, opcode: int) -> list[str]:
            assert 3002 == opcode
            return ["C2S:3002@+0.0s", "S2C:3003@+0.0s"]

        def saw(self, opcode: int) -> bool:
            return self.gate and 4011 == opcode

    class CueHost:
        def __init__(self) -> None:
            self.commands: list[str] = []
            self.boundaries: list[str] = []

        def command(self, token: str) -> None:
            self.commands.append(token)

        def note_boundaries(self, log: FakeLog, label: str) -> None:
            self.boundaries.append(label)

    class CueInjector:
        def __init__(self) -> None:
            self.aims: list[str] = []

        def send_click(self, aim: str) -> None:
            self.aims.append(aim)

    def test_affordance_inventory_names_controls_without_a_scene_claim(self) -> None:
        frames, log = self.CueFrames(), FakeLog()
        scene.waiting_affordance_lines(frames, log)
        self.assertEqual(len(scene.WAITING_AFFORDANCE_BOXES), len(log.events))
        self.assertTrue(all(line.startswith("cue affordance ") for line in log.events))
        self.assertTrue(any("ready_pill box=625,125,780,165" in line
                            for line in log.events))
        self.assertFalse(any(line.startswith("frame ") for line in log.events))

    def test_guest_ready_probe_precedes_the_host_start_command(self) -> None:
        clock = self.AdvancingClock()
        frames = self.CueFrames()
        log, host, injector = FakeLog(), self.CueHost(), self.CueInjector()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "time", clock.time), patch.object(
                scene.time, "sleep", clock.sleep
            ):
                scene.observe_start_cue(
                    frames, self.CueWire(), injector, host, log, Path(tmp), 20.0)
        stimuli = [line for line in log.events if "cue stimulus" in line]
        self.assertEqual(2, len(stimuli))
        self.assertIn("guest ready-pill", stimuli[0])
        self.assertIn("host start command", stimuli[1])
        self.assertEqual(["ready-pill"], injector.aims)
        self.assertEqual(["start"], host.commands)
        self.assertEqual(["start-cue"], host.boundaries)
        self.assertFalse(any(line.startswith("frame ") for line in log.events))

    def test_gate_wire_and_distinct_frames_are_logged(self) -> None:
        clock = self.AdvancingClock()
        frames = self.CueFrames(changing=True)
        log, host, injector = FakeLog(), self.CueHost(), self.CueInjector()
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(scene.time, "time", clock.time), patch.object(
                scene.time, "sleep", clock.sleep
            ):
                scene.observe_start_cue(
                    frames, self.CueWire(gate=True), injector, host, log,
                    Path(tmp), 3.0)
        self.assertEqual(3, len(frames.saved))
        self.assertTrue(all(line.startswith("waiting-cue-")
                            for line in frames.saved))
        self.assertTrue(any("S2C 4011 seen" in line for line in log.events))
        polls = [line for line in log.events if line.startswith("cue t=+")]
        self.assertTrue(all("wire=C2S:3002@+0.0s,S2C:3003@+0.0s" in line
                            for line in polls))

    def test_drive_accepts_cue_options(self) -> None:
        result = subprocess.run(
            [sys.executable, str(MODULE_PATH), "--help"],
            capture_output=True, text=True, check=True,
        )
        self.assertIn("--cue-discover-seconds", result.stdout)
        self.assertIn("--host-start-delay", result.stdout)


if __name__ == "__main__":
    unittest.main()
