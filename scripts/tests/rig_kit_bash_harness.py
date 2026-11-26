"""Hermetic runner for the real functions in ``scripts/rig-wine-run.sh``.

Each test states the live caller chain it pins. This helper only extracts one
function from the shipped script and runs it with explicit stub collaborators;
it never boots Wine or opens a display.
"""
from __future__ import annotations

import subprocess
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
RUNNER_SCRIPT = SCRIPTS_DIR / "rig-wine-run.sh"


def runner_function_source(name: str, required: bool = True) -> str:
    """Return one function definition from the shipped runner verbatim."""
    source = RUNNER_SCRIPT.read_text(encoding="utf-8")
    marker = f"{name}() {{"
    if marker not in source:
        if required:
            raise AssertionError(f"runner has no {name} function")
        return ""
    start = source.index(marker)
    end = source.index("\n}\n", start) + len("\n}\n")
    return source[start:end]


def run_runner_function(name: str, stubs: str, env: dict[str, str]
                        ) -> subprocess.CompletedProcess[str]:
    """Run a real runner function with the caller's stubs in scope."""
    body = "\n".join((
        "set -euo pipefail",
        stubs,
        runner_function_source(name),
        name,
        "",
    ))
    return subprocess.run(["bash", "-c", body], env=env, capture_output=True,
                          text=True, check=False)
