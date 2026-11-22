"""Scenario tests for the one blessed wave driver.

The replay leg consumes cited #582 evidence offline: it proves the recipe
drives the same clicks, cues, and evidence strings as the forked driver.
The live SendInput seam is checked as a command builder (no Wine boot here).
"""

from __future__ import annotations

import importlib.util
import json
import socket
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS / "lib"))
sys.path.insert(0, str(SCRIPTS / "tests"))

from test_wave_recipe import MIGRATED_RECIPE, leg_recipe, write_recipe  # noqa: E402
from test_wire_frames import CHANNEL_BODY, REWRITTEN_CHANNEL_BODY  # noqa: E402
from wave_recipe import load_recipe  # noqa: E402
from wire_frames import encode_frame, parse_tap_lines  # noqa: E402

DRIVER = SCRIPTS / "rig-wave.py"
RECIPE = SCRIPTS / "recipes" / "row-vanish-582.json"

LEAVE_LINE = "15:40:35 C2S opcode=3005 (0BBD) len=6 hex=000000000000"
REMOVAL_LINE = "15:40:35 S2C opcode=2007 (07D7) len=4 hex=00000000"
BEFORE_TAP = (
    "15:40:33 S2C opcode=2003 (07D3) len=2420 hex="
    "640000000000000001525635383244495342414e4400006103"
)
AFTER_TAP = (
    "15:40:39 S2C opcode=2003 (07D3) len=2404 hex="
    "64000000000000000000000000000200010000000000000000"
)
TAP = "\n".join([LEAVE_LINE, REMOVAL_LINE, BEFORE_TAP, AFTER_TAP]) + "\n"

EXPECTED_DRIVER_LINES = (
    "15:40:35 BACK pre-probe blue=0.811 label=waiting",
    "15:40:35 BACK post-probe blue=0.811 label=waiting",
    "15:40:35 SendInput BACK at (751, 580); cue=W8 wa-pick-okback2 C2S 3005 body 0000",
    "15:40:37 post-BACK label=room",
)
EXPECTED_EVIDENCE_LINES = (
    "[15:40:33] 2003 BEFORE: count=100 titles=['RV582DISBAND']",
    LEAVE_LINE,
    REMOVAL_LINE,
    "[15:40:39] 2003 AFTER: count=100 titles=[]",
)
READY_LINE = "15:41:02 C2S opcode=4008 (0FA8) len=6 hex=000000000000"
ACK_TEXT = "host start sent 4010+4012"


def fake_now(times: list[float]):
    """A clock that advances through scripted instants, then holds the last."""
    remaining = iter(times)

    def now() -> float:
        return next(remaining, times[-1])

    return now


def no_sleep(_seconds: float) -> None:
    return None


class FakeTap:
    """Stand-in for the wire-tap child process; never touches a socket."""

    def __init__(self) -> None:
        self.terminated = False

    def poll(self) -> int | None:
        return None

    def terminate(self) -> None:
        self.terminated = True

    def wait(self, timeout: float) -> None:
        return None


class FakeProcesses:
    """Records rig commands and can refuse one to prove loud failure."""

    def __init__(self, fail_on: str = "") -> None:
        self.commands: list[list[str]] = []
        self.ready_texts: list[str] = []
        self.fail_on = fail_on

    def run(self, argv: list[str]) -> subprocess.CompletedProcess[str]:
        self.commands.append(argv)
        code = 13 if self.fail_on and self.fail_on in " ".join(argv) else 0
        return subprocess.CompletedProcess(argv, code, "", "preflight refused")

    def spawn(self, argv: list[str]) -> FakeTap:
        self.commands.append(argv)
        return FakeTap()

    def wait_for_text(self, path: Path, text: str, timeout: float) -> None:
        self.ready_texts.append(text)


def load_driver_module() -> object:
    spec = importlib.util.spec_from_file_location("rig_wave", DRIVER)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load {DRIVER}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class RigWaveTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.tap = self.directory / "tap.log"
        self.tap.write_text(TAP, encoding="utf-8")
        self.out = self.directory / "out"

    def run_driver(self, *, tap: Path | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(DRIVER), "--recipe", str(RECIPE),
             "--mode", "replay", "--frames", str(tap or self.tap),
             "--out", str(self.out)],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_replay_reproduces_the_cited_row_vanish_evidence(self) -> None:
        result = self.run_driver()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        driver_log = (self.out / "driver.log").read_text(encoding="utf-8").splitlines()
        evidence = (self.out / "evidence.log").read_text(encoding="utf-8").splitlines()
        self.assertEqual(driver_log, list(EXPECTED_DRIVER_LINES))
        self.assertEqual(evidence, list(EXPECTED_EVIDENCE_LINES))
        captures = json.loads((self.out / "captures.json").read_text(encoding="utf-8"))
        self.assertEqual(captures["before"]["full_sha256"],
                         "3ecfd8f55743b05b4233145514312c228694e755f5ba83741927d6aac58bdb23")
        self.assertEqual(captures["after"]["crops"]["row-zero"]["sha256"],
                         "503d758b175443f705bf5436b417d4a4fa73acaa44e3f9d27280063a647ee985")

    def test_replay_reproduces_the_migrated_two_aim_entry_wave(self) -> None:
        tap = self.directory / "migrated-tap.log"
        tap.write_text("21:41:16 C2S opcode=5006 (138E) len=0 hex=\n", encoding="utf-8")

        result = subprocess.run(
            [sys.executable, str(DRIVER), "--recipe", str(MIGRATED_RECIPE),
             "--mode", "replay", "--frames", str(tap),
             "--out", str(self.out)],
            capture_output=True, text=True, check=False,
        )

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        driver_log = (self.out / "driver.log").read_text(encoding="utf-8")
        self.assertIn("w3 step=s1-myroom aim=(517,16)", driver_log)
        self.assertIn("w3 step=s2-goshop-entry aim=(89,320)", driver_log)
        self.assertIn("21:41:16 C2S opcode=5006",
                      (self.out / "evidence.log").read_text(encoding="utf-8"))
        captures = json.loads((self.out / "captures.json").read_text(encoding="utf-8"))
        self.assertEqual(set(captures), {"s1-myroom", "s2-goshop-entry"})

    def test_replay_names_the_missing_wire_cue_and_refuses_to_claim_evidence(self) -> None:
        quiet_tap = self.directory / "quiet.log"
        quiet_tap.write_text(TAP.replace(LEAVE_LINE + "\n", ""), encoding="utf-8")

        result = self.run_driver(tap=quiet_tap)

        self.assertEqual(result.returncode, 2, result.stdout)
        self.assertIn("missing wire cue leave-3005", result.stderr)
        self.assertFalse((self.out / "evidence.log").exists())

    def test_tap_applies_the_recipe_rule_to_s2c_frames(self) -> None:
        module = load_driver_module()
        rule = json.loads((SCRIPTS / "recipes" / "channel-ids-1003.json").read_text())
        upstream = socket.socket()
        upstream.bind(("127.0.0.1", 0))
        upstream.listen(1)
        upstream.settimeout(5)
        self.addCleanup(upstream.close)
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            listen_port = probe.getsockname()[1]
        tap = module.RewritingTap(listen_port, upstream.getsockname()[1],
                                  [rule], self.directory / "tap.log")
        tap.start()
        self.addCleanup(tap.stop)
        client = socket.create_connection(("127.0.0.1", listen_port), timeout=5)
        self.addCleanup(client.close)
        accepted, _ = upstream.accept()
        self.addCleanup(accepted.close)

        accepted.sendall(encode_frame(1003, CHANNEL_BODY))

        expected = encode_frame(1003, REWRITTEN_CHANNEL_BODY)
        forwarded = b""
        while len(forwarded) < len(expected):
            forwarded += client.recv(len(expected) - len(forwarded))
        self.assertEqual(forwarded, expected)
        self.assertIn("opcode=1003", (self.directory / "tap.log").read_text())

    def test_click_command_uses_the_sendinput_helper_not_xtest(self) -> None:
        module = load_driver_module()
        recipe = load_recipe(RECIPE)

        command = module.SendInputCommand.command(recipe.aim("back"))

        self.assertEqual(command[-4:], ["send", "click", "751", "580"])
        self.assertTrue(command[0].endswith("rig-wine.sh"))


class BootAttachLegTests(unittest.TestCase):
    """Boot/attach leg: login argv, standard preflight, wire tap, login flow."""

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.pack = self.directory / "pack"
        self.step = load_recipe(write_recipe(self.directory, leg_recipe(
            {"id": "boot", "action": "boot-attach", "evidence": "w4-plan-shop-goshop",
             "leg": {"mode": "boot", "client": "exe", "user_index_id": 1,
                      "target": "room", "wire_port": 15012}},
        ))).steps[0]

    def test_plan_mints_login_argv_taps_the_wire_and_drives_login(self) -> None:
        module = load_driver_module()

        commands = module.boot_attach_commands(self.step, self.pack, ":163")

        self.assertEqual([command.label for command in commands],
                         ["mint-login-argv", "wire-tap", "client-boot", "login-flow"])
        self.assertTrue(any("rig-mint-launch-argv.sh" in word for word in commands[0].argv))
        self.assertTrue(any("wine-room-host.py" in word for word in commands[1].argv))
        self.assertTrue(any("rig-run.sh" in word for word in commands[2].argv))
        self.assertTrue(any("rig-wine.sh" in word for word in commands[2].argv))
        self.assertTrue(any("wine-scene.py" in word for word in commands[3].argv))
        self.assertEqual(commands[3].argv[commands[3].argv.index("--target") + 1], "room")

    def test_refused_preflight_stops_before_the_login_flow(self) -> None:
        module = load_driver_module()
        processes = FakeProcesses(fail_on="rig-run.sh")

        with self.assertRaisesRegex(module.WaveFailure, "preflight|client-boot"):
            module.launch_boot_attach(self.step, self.pack, ":163", processes)

        self.assertFalse(any("wine-scene.py" in command for command in processes.commands))

    def test_failed_tap_readiness_keeps_the_session_owned_for_cleanup(self) -> None:
        module = load_driver_module()

        class RefusingReadyProcesses(FakeProcesses):
            def wait_for_text(self, path: Path, text: str, timeout: float) -> None:
                raise module.WaveFailure("tap never listened")

        processes = RefusingReadyProcesses()
        session = module.BootSession(self.pack)

        with self.assertRaisesRegex(module.WaveFailure, "tap never listened"):
            module.launch_boot_attach(self.step, self.pack, ":163", processes, session)

        tap = session.tap
        self.assertIsNotNone(tap)
        session.stop(processes)
        self.assertTrue(tap.terminated)

    def test_stop_releases_the_rig_and_the_wire_tap(self) -> None:
        module = load_driver_module()
        processes = FakeProcesses()
        session = module.launch_boot_attach(self.step, self.pack, ":163", processes)
        self.tap = session.tap

        session.stop(processes)

        self.assertTrue(any("rig-run.sh" in " ".join(command) and "stop" in command
                            for command in processes.commands))
        self.assertTrue(self.tap.terminated)


class SceneSyncLegTests(unittest.TestCase):
    """Scene-sync leg: wait for a named screen or a state marker."""

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.module = load_driver_module()

    def screen_step(self, timeout: float = 5.0):
        return load_recipe(write_recipe(self.directory, leg_recipe(
            {"id": "sync-room", "action": "scene-sync",
             "evidence": "w4-shop-goshop-rust/room.png",
             "leg": {"mode": "screen", "scene": "room", "timeout_seconds": timeout}},
        ))).steps[0]

    def test_waits_until_the_named_screen_is_painted(self) -> None:
        labels = iter(["unknown", "channel", "room"])
        sync = self.module.SceneSync(self.screen_step(), self.directory,
                                     probe=lambda: next(labels),
                                     now=fake_now([0.0, 1.0, 2.0]),
                                     sleep=no_sleep)

        line = sync.wait()

        self.assertIn("scene-sync sync-room screen=room", line)

    def test_timeout_names_the_scene_that_never_appeared(self) -> None:
        sync = self.module.SceneSync(self.screen_step(timeout=0.5), self.directory,
                                     probe=lambda: "channel",
                                     now=fake_now([0.0, 0.2, 0.6]),
                                     sleep=no_sleep)

        with self.assertRaisesRegex(self.module.WaveFailure, "scene-sync sync-room.*room"):
            sync.wait()

    def test_missing_state_file_times_out_instead_of_crashing(self) -> None:
        step = load_recipe(write_recipe(self.directory, leg_recipe(
            {"id": "sync-state", "action": "scene-sync",
             "evidence": "matched-ab-rust/applied.log",
             "leg": {"mode": "state", "state_file": "absent.log",
                      "state_text": "name=base-ready status=captured",
                      "timeout_seconds": 0.5}},
        ))).steps[0]
        sync = self.module.SceneSync(step, self.directory, probe=None,
                                     now=fake_now([0.0, 0.2, 0.6]), sleep=no_sleep)

        with self.assertRaisesRegex(self.module.WaveFailure, "base-ready"):
            sync.wait()

    def test_state_mode_waits_for_the_named_marker_text(self) -> None:
        marker = self.directory / "applied.log"
        marker.write_text("seq=1 name=base-ready status=captured\n", encoding="utf-8")
        step = load_recipe(write_recipe(self.directory, leg_recipe(
            {"id": "sync-state", "action": "scene-sync",
             "evidence": "matched-ab-rust/applied.log",
             "leg": {"mode": "state", "state_file": marker.name,
                      "state_text": "name=base-ready status=captured",
                      "timeout_seconds": 5}},
        ))).steps[0]
        sync = self.module.SceneSync(step, self.directory, probe=None,
                                     now=fake_now([0.0]), sleep=no_sleep)

        line = sync.wait()

        self.assertIn("state=base-ready", line)


class HostMarkerLegTests(unittest.TestCase):
    """Host-bot leg: queue START only after the guest's 4008 is on the wire."""

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.host_dir = self.directory / "host"
        self.host_dir.mkdir()
        self.module = load_driver_module()
        self.step = load_recipe(write_recipe(self.directory, leg_recipe(
            {"id": "host-start", "action": "host-marker",
             "evidence": "w4-result-exe-e host.cmd start-sent.ready",
             "leg": {"marker": "start", "command": "start", "gate_opcode": 4008,
                      "ack_text": ACK_TEXT, "timeout_seconds": 0.5}},
        ))).steps[0]

    def marker(self, lines: list, *, ack: bool = True) -> object:
        if ack:
            (self.host_dir / "host.log").write_text(ACK_TEXT + "\n", encoding="utf-8")
        return self.module.HostMarker(
            self.step, self.host_dir, self.directory / "out",
            lines_provider=lambda: lines, now=fake_now([0.0, 1.0, 2.0]), sleep=no_sleep)

    def test_queues_start_after_4008_and_writes_the_ready_marker(self) -> None:
        lines = parse_tap_lines(TAP + "\n" + READY_LINE + "\n")

        line = self.marker(lines).run()

        self.assertEqual((self.host_dir / "host.cmd").read_text(), "start\n")
        self.assertEqual((self.directory / "out" / "start.ready").read_text(), "ok\n")
        self.assertIn("host-marker host-start gate=4008", line)

    def test_missing_guest_4008_writes_no_marker(self) -> None:
        with self.assertRaisesRegex(self.module.WaveFailure, "4008"):
            self.marker(parse_tap_lines(TAP)).run()

        self.assertFalse((self.host_dir / "host.cmd").exists())
        self.assertFalse((self.directory / "out" / "start.ready").exists())

    def test_missing_host_ack_writes_no_ready_marker(self) -> None:
        lines = parse_tap_lines(READY_LINE + "\n")
        marker = self.marker(lines, ack=False)

        with self.assertRaisesRegex(self.module.WaveFailure, "ack"):
            marker.run()

        self.assertFalse((self.directory / "out" / "start.ready").exists())


class NativeInjectorLegTests(unittest.TestCase):
    """Injector leg: SendInput for Wine, XTEST for the native client."""

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.module = load_driver_module()

    def test_xtest_adapter_uses_rig_xtest_not_the_siclick_helper(self) -> None:
        dispatcher = self.module.ClickDispatcher(":163")
        dispatcher.select("xtest")

        command = dispatcher.command(load_recipe(RECIPE).aim("back"))

        self.assertEqual(command[:3], [str(SCRIPTS / "rig-xtest.py"), "--display", ":163"])
        self.assertEqual(command[-4:], ["click", "1", "751", "580"])
        self.assertNotIn("rig-wine.sh", command)

    def test_sendinput_adapter_keeps_the_wine_helper_path(self) -> None:
        dispatcher = self.module.ClickDispatcher(":163")
        dispatcher.select("sendinput")

        command = dispatcher.command(load_recipe(RECIPE).aim("back"))

        self.assertEqual(command[-4:], ["send", "click", "751", "580"])
        self.assertTrue(command[0].endswith("rig-wine.sh"))

    def test_unknown_adapter_fails_loudly(self) -> None:
        dispatcher = self.module.ClickDispatcher(":163")

        with self.assertRaisesRegex(self.module.WaveFailure, "injector"):
            dispatcher.select("blind-escape")

    def test_driver_clicks_with_the_injector_selected_by_the_recipe(self) -> None:
        write_recipe(self.directory, leg_recipe(
            {"id": "native", "action": "set-injector",
             "evidence": "wine-boot.md Attempts 9-12",
             "leg": {"name": "xtest"}},
            {"id": "click-back", "action": "click", "aim": "back",
             "text": "native BACK"},
        ))
        recipe_data = json.loads((self.directory / "recipe.json").read_text())
        recipe_data["aims"]["back"] = {"x": 751, "y": 580, "evidence": "W8 wa-pick-okback2"}
        (self.directory / "recipe.json").write_text(json.dumps(recipe_data))
        recipe = load_recipe(self.directory / "recipe.json")
        processes = FakeProcesses()
        rig = self.module.LiveRig(self.directory / "out", ":163",
                                  self.directory / "tap.log", processes=processes)

        result = self.module.execute(recipe, rig)

        self.assertTrue(any("rig-xtest.py" in " ".join(command)
                            for command in processes.commands))
        self.assertFalse(any("rig-wine.sh" in " ".join(command)
                             for command in processes.commands))
        self.assertIn("native BACK", result.driver_lines[-1])
        self.assertIn("via=xtest click", result.driver_lines[-1])


if __name__ == "__main__":
    unittest.main()
