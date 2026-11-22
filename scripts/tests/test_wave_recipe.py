"""Scenario tests for recipe validation and aim resolution.

A recipe must carry its own evidence and may only name popup faces that
`lib/popup_state.py` publishes. No client or display is needed here.
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS / "lib"))

from popup_state import aim as published_aim  # noqa: E402
from wave_recipe import RecipeError, load_recipe  # noqa: E402

RECIPE = SCRIPTS / "recipes" / "row-vanish-582.json"
MIGRATED_RECIPE = SCRIPTS / "recipes" / "w4-shop-goshop-861.json"


def leg_recipe(*steps: dict) -> dict:
    """Build a minimal recipe around one or more leg payloads."""
    return {
        "schema": 1,
        "id": "leg-selftest",
        "aims": {},
        "steps": list(steps),
    }


def write_recipe(directory: Path, data: dict) -> Path:
    path = directory / "recipe.json"
    path.write_text(json.dumps(data), encoding="utf-8")
    return path


class WaveRecipeLegTests(unittest.TestCase):
    """Validation rules for the four wave-4 leg kinds."""

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)

    def test_boot_attach_scene_sync_host_marker_and_injector_load(self) -> None:
        data = leg_recipe(
            {"id": "boot", "action": "boot-attach", "evidence": "w4-plan-shop-goshop",
             "leg": {"mode": "boot", "client": "exe", "user_index_id": 1,
                      "target": "room", "wire_port": 15012}},
            {"id": "sync", "action": "scene-sync", "evidence": "w4-shop-goshop-rust/room.png",
             "leg": {"mode": "screen", "scene": "room", "timeout_seconds": 30}},
            {"id": "start", "action": "host-marker",
             "evidence": "w4-result-exe-e host.cmd start-sent.ready",
             "leg": {"marker": "start", "command": "start", "gate_opcode": 4008,
                      "ack_text": "host start sent 4010+4012"}},
            {"id": "native", "action": "set-injector",
             "evidence": "wine-boot.md Attempts 9-12; rig-xtest.py",
             "leg": {"name": "xtest"}},
        )
        data["injector"] = "xtest"

        recipe = load_recipe(write_recipe(self.directory, data))

        self.assertEqual(recipe.injector, "xtest")
        self.assertEqual([step.action for step in recipe.steps],
                         ["boot-attach", "scene-sync", "host-marker", "set-injector"])
        self.assertEqual(recipe.steps[0].leg["target"], "room")
        self.assertEqual(recipe.steps[1].leg["timeout_seconds"], 30.0)
        self.assertEqual(recipe.steps[2].leg["gate_opcode"], 4008)
        self.assertEqual(recipe.steps[3].leg["name"], "xtest")

    def test_boot_attach_requires_a_login_target(self) -> None:
        data = leg_recipe({"id": "boot", "action": "boot-attach",
                           "evidence": "w4-plan-shop-goshop",
                           "leg": {"client": "exe", "wire_port": 15012}})

        with self.assertRaisesRegex(RecipeError, "boot-attach.*target"):
            load_recipe(write_recipe(self.directory, data))

    def test_screen_sync_requires_a_scene_and_a_positive_timeout(self) -> None:
        data = leg_recipe({"id": "sync", "action": "scene-sync",
                           "evidence": "w4-shop-goshop-rust/room.png",
                           "leg": {"mode": "screen", "timeout_seconds": 0}})

        with self.assertRaisesRegex(RecipeError, "scene-sync.*scene"):
            load_recipe(write_recipe(self.directory, data))
        data["steps"][0]["leg"]["scene"] = "room"
        with self.assertRaisesRegex(RecipeError, "scene-sync.*timeout"):
            load_recipe(write_recipe(self.directory, data))

    def test_host_marker_requires_a_gate_opcode_and_an_ack(self) -> None:
        data = leg_recipe({"id": "start", "action": "host-marker",
                           "evidence": "w4-result host.cmd",
                           "leg": {"marker": "start", "command": "start",
                                   "ack_text": "host start sent 4010+4012"}})

        with self.assertRaisesRegex(RecipeError, "host-marker.*gate_opcode"):
            load_recipe(write_recipe(self.directory, data))
        data["steps"][0]["leg"]["gate_opcode"] = 4008
        data["steps"][0]["leg"]["ack_text"] = ""
        with self.assertRaisesRegex(RecipeError, "host-marker.*ack_text"):
            load_recipe(write_recipe(self.directory, data))

    def test_nonnumeric_scene_sync_timeout_is_refused(self) -> None:
        data = leg_recipe({"id": "sync", "action": "scene-sync",
                           "evidence": "w4-shop-goshop-rust/room.png",
                           "leg": {"mode": "screen", "scene": "room",
                                   "timeout_seconds": "soon"}})

        with self.assertRaisesRegex(RecipeError, "scene-sync.*timeout"):
            load_recipe(write_recipe(self.directory, data))

    def test_unpublished_injector_is_refused(self) -> None:
        data = leg_recipe({"id": "note", "action": "note", "text": "x"})
        data["injector"] = "blind-xtest"

        with self.assertRaisesRegex(RecipeError, "injector"):
            load_recipe(write_recipe(self.directory, data))


class WaveRecipeTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)

    def test_row_vanish_recipe_resolves_back_and_published_popup_aims(self) -> None:
        recipe = load_recipe(RECIPE)

        self.assertEqual(recipe.id, "row-vanish-582")
        self.assertEqual(recipe.aim_point("back"), (751, 580))
        self.assertEqual(recipe.aim_point("normal-mode-ok"),
                         published_aim("normal-mode-ok").as_pair())
        self.assertEqual(recipe.proxy_rules, ())

    def test_migrated_two_aim_entry_recipe_keeps_both_pins(self) -> None:
        recipe = load_recipe(MIGRATED_RECIPE)

        self.assertEqual(recipe.aim_point("myroom"), (517, 16))
        self.assertEqual(recipe.aim_point("goshop"), (89, 320))
        self.assertEqual(recipe.steps[0].leg["target"], "room")
        self.assertEqual([step.opcode for step in recipe.steps if step.action == "wire-cue"],
                         [5006])
        self.assertEqual(recipe.injector, "sendinput")

    def test_popup_aim_must_be_a_reference_not_a_copied_coordinate(self) -> None:
        data = json.loads(RECIPE.read_text(encoding="utf-8"))
        data["aims"]["normal-mode-ok"] = {
            "x": 400,
            "y": 365,
            "evidence": "copied from an old wave",
        }
        path = self.directory / "copied-popup.json"
        path.write_text(json.dumps(data), encoding="utf-8")

        with self.assertRaisesRegex(RecipeError, "popup face.*ref"):
            load_recipe(path)

    def test_unknown_step_action_fails_loudly(self) -> None:
        data = json.loads(RECIPE.read_text(encoding="utf-8"))
        data["steps"][0]["action"] = "blind-escape"
        path = self.directory / "unknown-action.json"
        path.write_text(json.dumps(data), encoding="utf-8")

        with self.assertRaisesRegex(RecipeError, "unknown action"):
            load_recipe(path)

    def test_unknown_aim_reference_fails_loudly(self) -> None:
        data = json.loads(RECIPE.read_text(encoding="utf-8"))
        for step in data["steps"]:
            if step["action"] == "click":
                step["aim"] = "not-in-recipe"
        path = self.directory / "unknown-aim.json"
        path.write_text(json.dumps(data), encoding="utf-8")

        with self.assertRaisesRegex(RecipeError, "unknown aim"):
            load_recipe(path)


if __name__ == "__main__":
    unittest.main()
