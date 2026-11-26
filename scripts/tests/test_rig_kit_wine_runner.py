"""Guards for the Wine runner's join report and drive verdict.

Every guard names its live caller chain. The stubs here replace only the
display-bound collaborators; the function text under test comes from
``scripts/rig-wine-run.sh`` itself, so the pack cannot drift from the guard.
"""
from __future__ import annotations

import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

from rig_kit_bash_harness import run_runner_function, runner_function_source


class JoinPressCountGuard(unittest.TestCase):
    """Live chain: rig-wine-run.sh::main -> phase_package -> join_outcome_fields.

    The manifest field is a pure log count, so a synthetic drive.log is the
    whole behavior; no exe or display is needed.
    """

    def test_dclick_and_press_lines_both_count_as_join_pressures(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out_dir = Path(tmp)
            (out_dir / "drive.log").write_text(
                "12:00:01 inject send-press down=1 up=1\n"
                "12:00:02 inject send-double-click room-row-title "
                "(100,100) detail=win32\n",
                encoding="utf-8",
            )
            result = run_runner_function(
                "join_outcome_fields", "",
                dict(os.environ, OUT_DIR=str(out_dir),
                     WIRE_LOG=str(out_dir / "wire.log")),
            )
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual("join_0BBA=absent join_press_count=2",
                         result.stdout.strip())


class WireTapUpstreamNoteGuard(unittest.TestCase):
    """Live chain: rig-wine-run.sh::main -> phase_wire_tap -> note.

    A stub proxy records the upstream argument it receives. The note must
    name that upstream, not the listen port that replaced $GATEWAY_PORT.
    """

    def test_note_names_the_upstream_after_gateway_port_reassignment(self) -> None:
        stubs = "\n".join((
            "note() { printf '%s\\n' \"$*\" >> \"$NOTE_LOG\"; }",
            "python3() { printf '%s\\n' \"$*\" >> \"$TAP_ARG_LOG\"; }",
        ))
        with tempfile.TemporaryDirectory() as tmp:
            out_dir = Path(tmp)
            wire_dir = out_dir / "wire"
            wire_dir.mkdir()
            wire_log = wire_dir / "room-stage.log"
            wire_log.write_text("wire-tap listening\n", encoding="utf-8")
            arg_log, note_log = out_dir / "tap-args.log", out_dir / "notes.log"
            result = run_runner_function(
                "phase_wire_tap", stubs,
                dict(os.environ, OUT_DIR=str(out_dir), WIRE_TAP="1",
                     WIRE_TAP_PORT="15012", GATEWAY_PORT="15012",
                     GATEWAY_UPSTREAM_PORT="15010", WIRE_LOG=str(wire_log),
                     WIRE_PID_FILE=str(wire_dir / "tap.pid"),
                     SCRIPT_DIR=str(Path(__file__).resolve().parents[1]),
                     TAP_ARG_LOG=str(arg_log), NOTE_LOG=str(note_log)),
            )
            for _ in range(50):
                if arg_log.is_file() and arg_log.stat().st_size:
                    break
                time.sleep(0.02)
            args = arg_log.read_text(encoding="utf-8").split()
            notes = note_log.read_text(encoding="utf-8").splitlines()
        self.assertEqual(0, result.returncode, result.stderr)
        listen = args[args.index("--listen-port") + 1]
        upstream = args[args.index("--upstream-port") + 1]
        note_line = next(line for line in notes
                         if line.startswith("phase wire-tap on "))
        self.assertEqual("15012", listen)
        self.assertEqual(upstream, note_line.rsplit("-> ", 1)[1])
        self.assertNotEqual(listen, note_line.rsplit("-> ", 1)[1])


class ObservationOnlyDriveVerdictGuard(unittest.TestCase):
    """Live chain: rig-wine-run.sh::main -> phase_drive -> note.

    An observation leg deliberately refuses to label Waiting; its success is
    the drive's own NOT-claimed line. A normal leg keeps the scene label as
    its only success condition, so no frame is ever relabelled.
    """

    @staticmethod
    def stubs() -> str:
        return "\n".join((
            runner_function_source("observation_only_waiting_reached",
                                   required=False),
            "note() { printf '%s\\n' \"$*\" >> \"$NOTE_LOG\"; }",
            "timeout() { shift; \"$@\"; }",
            "python3() { cp \"$FAKE_DRIVE_LOG\" \"$OUT_DIR/drive.log\"; }",
            "bash() { cat \"$FAKE_SCENE_LABEL\"; }",
        ))

    @staticmethod
    def run_drive(out_dir: Path, drive_text: str, observe_seconds: int,
                  scene_label: str = "room"
                  ) -> tuple[subprocess.CompletedProcess[str], list[str]]:
        fixture = out_dir / "drive-fixture.log"
        fixture.write_text(drive_text, encoding="utf-8")
        label = out_dir / "scene-label"
        label.write_text(scene_label + "\n", encoding="utf-8")
        notes = out_dir / "notes.log"
        result = run_runner_function(
            "phase_drive", ObservationOnlyDriveVerdictGuard.stubs(),
            dict(os.environ, OUT_DIR=str(out_dir), TARGET="waiting",
                 MUSIC_ID="757", JOIN_MODE="dclick", ROW_REPAINT="none",
                 REPAINT_DELAY="0", ROW_DISMISS="auto",
                 WIRE_LOG=str(out_dir / "wire.log"),
                 OBSERVE_WAITING_SECONDS=str(observe_seconds),
                 CUE_DISCOVER_SECONDS="0", HOST_START_DELAY="20",
                 HOST_USER_INDEX_ID="3", WINEPREFIX="/unused",
                 DISPLAY_NAME=":299", RIG_WINE_DRIVE_TIMEOUT="1",
                 SCRIPT_DIR=str(Path(__file__).resolve().parents[1]),
                 FAKE_DRIVE_LOG=str(fixture), FAKE_SCENE_LABEL=str(label),
                 NOTE_LOG=str(notes)),
        )
        return result, notes.read_text(encoding="utf-8").splitlines()

    def test_observation_line_passes_an_observation_only_leg(self) -> None:
        drive = (
            "12:00:01 frame 1 screen=room mean_luma=90 cue=server:03EC\n"
            "12:00:02 stage waiting NOT claimed reason=observation-only\n")
        with tempfile.TemporaryDirectory() as tmp:
            result, notes = self.run_drive(Path(tmp), drive, 8)
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("drive reached 'room'", notes)
        self.assertFalse(any("FAILED" in line for line in notes))

    def test_observation_leg_without_the_line_still_fails(self) -> None:
        drive = "12:00:01 frame 1 screen=room mean_luma=90 cue=server:03EC\n"
        with tempfile.TemporaryDirectory() as tmp:
            result, notes = self.run_drive(Path(tmp), drive, 8)
        self.assertEqual(1, result.returncode, result.stderr)
        self.assertTrue(any("drive FAILED to reach 'waiting'" in line
                            for line in notes))

    def test_observation_line_does_not_rescue_a_normal_leg(self) -> None:
        drive = (
            "12:00:01 frame 1 screen=room mean_luma=90 cue=server:03EC\n"
            "12:00:02 stage waiting NOT claimed reason=observation-only\n")
        with tempfile.TemporaryDirectory() as tmp:
            result, notes = self.run_drive(Path(tmp), drive, 0)
        self.assertEqual(1, result.returncode, result.stderr)
        self.assertIn("drive reached 'room'", notes)
        self.assertTrue(any("drive FAILED to reach 'waiting'" in line
                            for line in notes))

    def test_normal_leg_that_reaches_waiting_still_passes(self) -> None:
        drive = "12:00:01 frame 2 screen=waiting mean_luma=95 cue=server:0BBA\n"
        with tempfile.TemporaryDirectory() as tmp:
            result, notes = self.run_drive(Path(tmp), drive, 0, "waiting")
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("drive reached 'waiting'", notes)


if __name__ == "__main__":
    unittest.main()
