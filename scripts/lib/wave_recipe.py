#!/usr/bin/env python3
"""Validated wave recipes: aims, steps, leg payloads, and proxy rules as data.

One recipe replaces a forked scene driver. Coordinates carry their evidence
strings; popup faces are references to `lib/popup_state.py`, never copied
coordinates. A leg step carries its action-specific fields under `leg`:
boot-attach, scene-sync, host-marker, and set-injector. The driver reads a
recipe; it does not grow per-wave branches.
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping

from popup_state import POPUP_AIMS, aim as published_aim

RECIPE_SCHEMA = 1
ACTIONS = frozenset(
    {"probe-popup", "dismiss-popup", "click", "wire-cue", "capture", "wait", "note",
     "boot-attach", "scene-sync", "host-marker", "set-injector"}
)
REWRITE_ACTIONS = frozenset({"rotate_u16_rows"})
POPUP_ACTIONS = frozenset({"probe-popup", "dismiss-popup"})
LEG_ACTIONS = frozenset({"boot-attach", "scene-sync", "host-marker", "set-injector"})
INJECTORS = frozenset({"sendinput", "xtest"})
BOOT_CLIENTS = frozenset({"exe", "rust"})
BOOT_MODES = frozenset({"boot", "attach"})
SCENE_SYNC_MODES = frozenset({"screen", "state"})


class RecipeError(ValueError):
    """A recipe cannot be trusted to drive a wave."""


@dataclass(frozen=True)
class Aim:
    key: str
    evidence: str
    x: int = 0
    y: int = 0
    popup_key: str = ""

    def point(self) -> tuple[int, int]:
        if self.popup_key:
            return published_aim(self.popup_key).as_pair()
        return self.x, self.y


@dataclass(frozen=True)
class Step:
    id: str
    action: str
    aim: Aim | None = None
    time: str = ""
    label: str = ""
    phase: str = ""
    blue: float | None = None
    scene: str = ""
    text: str = ""
    name: str = ""
    direction: str = ""
    opcode: int | None = None
    body_contains: str = ""
    body_absent: str = ""
    seconds: float = 0.0
    max_tries: int = 2
    evidence: Mapping[str, object] = field(default_factory=dict)
    leg: Mapping[str, object] = field(default_factory=dict)


@dataclass(frozen=True)
class WaveRecipe:
    id: str
    ticket: int
    trace_id: str
    source: str
    aims: Mapping[str, Aim]
    steps: tuple[Step, ...]
    proxy_rules: tuple[Mapping[str, object], ...]
    evidence: Mapping[str, object]
    injector: str = "sendinput"

    def aim(self, key: str) -> Aim:
        try:
            return self.aims[key]
        except KeyError as error:
            raise RecipeError(f"unknown aim {key!r}") from error

    def aim_point(self, key: str) -> tuple[int, int]:
        return self.aim(key).point()


def _read_aim(key: str, data: Mapping[str, Any]) -> Aim:
    evidence = str(data.get("evidence", ""))
    if "ref" in data:
        popup_key = str(data["ref"])
        if popup_key not in POPUP_AIMS:
            raise RecipeError(f"aim {key!r} names unknown popup face {popup_key!r}")
        if "x" in data or "y" in data:
            raise RecipeError(f"popup face {key!r} must use ref, not copied coordinates")
        return Aim(key=key, evidence=evidence, popup_key=popup_key)
    if key in POPUP_AIMS:
        raise RecipeError(f"popup face {key!r} must use ref, not copied coordinates")
    if not evidence:
        raise RecipeError(f"aim {key!r} is missing its evidence string")
    x, y = data.get("x"), data.get("y")
    if not isinstance(x, int) or not isinstance(y, int):
        raise RecipeError(f"aim {key!r} needs integer x and y")
    return Aim(key=key, evidence=evidence, x=x, y=y)


def _is_command(value: object) -> bool:
    return isinstance(value, list) and bool(value) and all(
        isinstance(word, str) and word for word in value)


def _has_positive_timeout(leg: Mapping[str, Any]) -> bool:
    try:
        return float(leg.get("timeout_seconds", 0.0)) > 0
    except (TypeError, ValueError):
        return False


def _read_leg(data: Mapping[str, Any], step_id: str, action: str) -> dict[str, Any]:
    leg = data.get("leg")
    if not isinstance(leg, dict):
        raise RecipeError(f"step {step_id!r} {action} needs a leg object")
    return dict(leg)


def _read_boot_attach_leg(leg: dict[str, Any], step_id: str) -> dict[str, Any]:
    client = str(leg.get("client", ""))
    if client not in BOOT_CLIENTS:
        raise RecipeError(f"step {step_id!r} boot-attach needs a published client")
    mode = str(leg.get("mode", "boot"))
    if mode not in BOOT_MODES:
        raise RecipeError(f"step {step_id!r} boot-attach mode {mode!r} is not published")
    if not str(leg.get("target", "")):
        raise RecipeError(f"step {step_id!r} boot-attach needs a login target")
    if mode == "boot":
        if not isinstance(leg.get("wire_port"), int):
            raise RecipeError(f"step {step_id!r} boot-attach needs a wire_port")
        if client == "exe" and not isinstance(leg.get("user_index_id"), int):
            raise RecipeError(f"step {step_id!r} boot-attach needs a user_index_id")
        if client == "rust" and not _is_command(leg.get("launch")):
            raise RecipeError(f"step {step_id!r} boot-attach needs a launch command")
    elif client == "rust" and not _is_command(leg.get("login")):
        raise RecipeError(f"step {step_id!r} boot-attach needs a login command")
    leg.setdefault("mode", mode)
    leg.setdefault("upstream_port", 15010)
    leg.setdefault("login_timeout_seconds", 180.0)
    return leg


def _read_scene_sync_leg(leg: dict[str, Any], step_id: str) -> dict[str, Any]:
    mode = str(leg.get("mode", ""))
    if mode not in SCENE_SYNC_MODES:
        raise RecipeError(f"step {step_id!r} scene-sync needs mode screen or state")
    if mode == "screen" and not str(leg.get("scene", "")):
        raise RecipeError(f"step {step_id!r} scene-sync screen mode needs a scene")
    if mode == "state" and (not str(leg.get("state_file", ""))
                            or not str(leg.get("state_text", ""))):
        raise RecipeError(f"step {step_id!r} scene-sync state mode needs state_file and state_text")
    if not _has_positive_timeout(leg):
        raise RecipeError(f"step {step_id!r} scene-sync needs a positive timeout_seconds")
    return leg


def _read_host_marker_leg(leg: dict[str, Any], step_id: str) -> dict[str, Any]:
    for field_name in ("marker", "command", "ack_text"):
        if not str(leg.get(field_name, "")):
            raise RecipeError(f"step {step_id!r} host-marker needs {field_name}")
    if not isinstance(leg.get("gate_opcode"), int):
        raise RecipeError(f"step {step_id!r} host-marker needs a gate_opcode")
    leg.setdefault("timeout_seconds", 60.0)
    return leg


def _read_injector_leg(leg: dict[str, Any], step_id: str) -> dict[str, Any]:
    name = str(leg.get("name", ""))
    if name not in INJECTORS:
        raise RecipeError(
            f"step {step_id!r} injector {name!r} is not published: {sorted(INJECTORS)}")
    return leg


LEG_VALIDATORS = {
    "boot-attach": _read_boot_attach_leg,
    "scene-sync": _read_scene_sync_leg,
    "host-marker": _read_host_marker_leg,
    "set-injector": _read_injector_leg,
}


def _read_step(data: Mapping[str, Any], aims: Mapping[str, Aim]) -> Step:
    step_id = str(data.get("id", ""))
    action = str(data.get("action", ""))
    if not step_id:
        raise RecipeError("every step needs an id")
    if action not in ACTIONS:
        raise RecipeError(f"step {step_id!r} has unknown action {action!r}")
    aim_key = data.get("aim")
    if aim_key is not None and aim_key not in aims:
        raise RecipeError(f"step {step_id!r} names unknown aim {aim_key!r}")
    if action in POPUP_ACTIONS and (aim_key is None or aims[aim_key].popup_key == ""):
        raise RecipeError(f"step {step_id!r} must reference a published popup face")
    if action == "click" and aim_key is None:
        raise RecipeError(f"step {step_id!r} needs an aim")
    if action == "probe-popup" and not str(data.get("label", "")):
        raise RecipeError(f"step {step_id!r} needs a probe label")
    blue = data.get("blue")
    if blue is not None and not isinstance(blue, (int, float)):
        raise RecipeError(f"step {step_id!r} blue must be a number")
    if action == "wire-cue" and (
        data.get("direction") not in ("C2S", "S2C")
        or not isinstance(data.get("opcode"), int)
        or not str(data.get("text", ""))
    ):
        raise RecipeError(f"step {step_id!r} needs direction, opcode, and text")
    if action == "capture" and (not data.get("name") or not data.get("evidence")):
        raise RecipeError(f"step {step_id!r} needs a name and capture evidence")
    if action == "note" and not data.get("text"):
        raise RecipeError(f"step {step_id!r} needs text")
    leg = _read_leg(data, step_id, action) if action in LEG_ACTIONS else {}
    if leg:
        leg = LEG_VALIDATORS[action](leg, step_id)
    return Step(
        id=step_id,
        action=action,
        aim=aims[aim_key] if aim_key is not None else None,
        time=str(data.get("time", "")),
        label=str(data.get("label", "")),
        phase=str(data.get("phase", "")),
        blue=data.get("blue"),
        scene=str(data.get("scene", "")),
        text=str(data.get("text", "")),
        name=str(data.get("name", "")),
        direction=str(data.get("direction", "")),
        opcode=data.get("opcode"),
        body_contains=str(data.get("body_contains", "")),
        body_absent=str(data.get("body_absent", "")),
        seconds=float(data.get("seconds", 0.0)),
        max_tries=int(data.get("max_tries", 2)),
        evidence=data.get("evidence", {}),
        leg=leg,
    )


def load_recipe(path: Path) -> WaveRecipe:
    """Read and validate one recipe; invalid data fails before any live action."""
    try:
        data: Mapping[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RecipeError(f"cannot read recipe {path}: {error}") from error
    if data.get("schema") != RECIPE_SCHEMA:
        raise RecipeError(f"recipe {path} needs schema {RECIPE_SCHEMA}")
    aims_data = data.get("aims")
    steps_data = data.get("steps")
    if not isinstance(aims_data, dict) or not isinstance(steps_data, list) or not steps_data:
        raise RecipeError(f"recipe {path} needs aims and a non-empty step list")
    aims = {key: _read_aim(key, value) for key, value in aims_data.items()}
    seen: set[str] = set()
    steps = []
    for item in steps_data:
        step = _read_step(item, aims)
        if step.id in seen:
            raise RecipeError(f"duplicate step id {step.id!r}")
        seen.add(step.id)
        steps.append(step)
    rules = data.get("proxy_rules", [])
    if not isinstance(rules, list):
        raise RecipeError("proxy_rules must be a list")
    for rule in rules:
        if not isinstance(rule, dict) or rule.get("action") not in REWRITE_ACTIONS:
            raise RecipeError(f"unsupported proxy rule {rule!r}")
    injector = str(data.get("injector", "sendinput"))
    if injector not in INJECTORS:
        raise RecipeError(f"recipe injector {injector!r} is not published: {sorted(INJECTORS)}")
    return WaveRecipe(
        id=str(data.get("id", "")),
        ticket=int(data.get("ticket", 0)),
        trace_id=str(data.get("trace_id", "")),
        source=str(data.get("source", "")),
        aims=aims,
        steps=tuple(steps),
        proxy_rules=tuple(rules),
        evidence=data.get("evidence", {}),
        injector=injector,
    )


class WaveRecipeTests(unittest.TestCase):
    """The lib selftest: validation refuses data that would fork behavior."""

    def test_valid_minimal_recipe_loads(self) -> None:
        recipe = {
            "schema": RECIPE_SCHEMA,
            "id": "selftest",
            "aims": {"ok": {"ref": "normal-mode-ok", "evidence": "popup_state.py"}},
            "steps": [
                {"id": "dismiss", "action": "dismiss-popup", "aim": "ok", "text": "OK"}
            ],
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "recipe.json"
            path.write_text(json.dumps(recipe), encoding="utf-8")
            loaded = load_recipe(path)
        self.assertEqual(loaded.aim_point("ok"), published_aim("normal-mode-ok").as_pair())

    def test_copied_popup_coordinates_are_refused(self) -> None:
        with self.assertRaises(RecipeError):
            _read_aim("normal-mode-ok", {"x": 400, "y": 365, "evidence": "copied"})

    def test_leg_actions_are_published_in_the_driver_vocabulary(self) -> None:
        self.assertEqual(LEG_ACTIONS, {"boot-attach", "scene-sync",
                                       "host-marker", "set-injector"})
        self.assertEqual(LEG_ACTIONS - ACTIONS, set())


if __name__ == "__main__":
    if sys.argv[1:] == ["selftest"]:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(WaveRecipeTests)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        raise SystemExit(0 if result.wasSuccessful() else 1)
    print(__doc__)
