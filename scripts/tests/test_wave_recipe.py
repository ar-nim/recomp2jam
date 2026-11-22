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
