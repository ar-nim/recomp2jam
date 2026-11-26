"""Gate arm orchestration with a stub cargo (issues #737, #741).

These tests prove which commands the gate issues without running the
suite: a fresh stamp keeps the full sweep out, an absent stamp puts it
in, and the hidden arm refuses to run when its data probe sees archives.
"""
from __future__ import annotations

import hashlib
import os
import re
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS_DIR.parent
GATE_SCRIPT = SCRIPTS_DIR / "gate-check.sh"
PRESENT_CANARY_OK = (
    "test live_art::live_art_tests::"
    "live_matches_declaration_on_walked_screens_when_archives_present ... ok"
)
REQUIRED_MARKER = "GATE_REQUIRE_ARCHIVES=1"
UNSET_MARKER = "GATE_REQUIRE_ARCHIVES=unset"
FAKE_SKIP_LINE = "SKIP some_real_data_test: install-root absent (unreachable)"


class GateArmTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        (self.repo / "scripts" / "lib").mkdir(parents=True)
        (self.repo / "fixtures").mkdir()
        shutil.copy(GATE_SCRIPT, self.repo / "scripts" / "gate-check.sh")
        shutil.copy(
            SCRIPTS_DIR / "check-guard-evidence.sh",
            self.repo / "scripts" / "check-guard-evidence.sh",
        )
        for lib in (SCRIPTS_DIR / "lib").glob("*.sh"):
            shutil.copy(lib, self.repo / "scripts" / "lib" / lib.name)
        guards = self.repo / "crates" / "fixture" / "guards.rs"
        guards.parent.mkdir(parents=True)
        guards.write_text("#[test]\nfn computes_expected_score() {}\n", encoding="utf-8")

        music = self.repo / "install-root" / "O2Jam" / "Music"
        music.mkdir(parents=True)
        chart = b"sample chart bytes"
        (music / "o2ma100.ojn").write_bytes(chart)
        digest = hashlib.sha256(chart).hexdigest()
        (self.repo / "fixtures" / "full-pack-sample.sha256").write_text(
            f"o2ma100.ojn {digest}\n", encoding="utf-8"
        )

        self.bin = self.repo / "bin"
        self.bin.mkdir()
        (self.bin / "cargo").write_text(
            "#!/usr/bin/env bash\n"
            "printf 'GATE_REQUIRE_ARCHIVES=%s :: %s\\n' \"${GATE_REQUIRE_ARCHIVES-unset}\" "
            "\"$*\" >> \"$FAKE_CARGO_LOG\"\n"
            "if [ -n \"${FAKE_CANARY_LINE:-}\" ]; then\n"
            "  printf '%s\\n' \"$FAKE_CANARY_LINE\"\n"
            "fi\n"
            "if [ -n \"${FAKE_SKIP_LINE:-}\" ]; then\n"
            "  case \" $* \" in\n"
            "    *' --show-output '*) printf '%s\\n' \"$FAKE_SKIP_LINE\" ;;\n"
            "  esac\n"
            "fi\n",
            encoding="utf-8",
        )
        (self.bin / "cargo").chmod(0o755)
        self.log = self.repo / "cargo.log"
        self.env = dict(os.environ)
        self.env.update(PATH=f"{self.bin}:{self.env['PATH']}", FAKE_CARGO_LOG=str(self.log))
        self.env.pop("GATE_REQUIRE_ARCHIVES", None)
        self.env.pop("FAKE_CANARY_LINE", None)
        self.env.pop("FAKE_SKIP_LINE", None)

    def run_gate(
        self, *args: str, fake_canary: bool = True
    ) -> subprocess.CompletedProcess[str]:
        env = dict(self.env)
        if fake_canary:
            env["FAKE_CANARY_LINE"] = PRESENT_CANARY_OK
        return subprocess.run(
            ["bash", str(self.repo / "scripts" / "gate-check.sh"), "--no-push", *args],
            capture_output=True,
            text=True,
            env=env,
            cwd=self.repo,
            timeout=60,
        )

    def workspace_suite_markers(self) -> list[str]:
        lines = self.log.read_text(encoding="utf-8").splitlines()
        return [
            line.split(" :: ", 1)[0]
            for line in lines
            if "test --workspace" in line
        ]

    def test_absent_stamp_runs_the_full_sweep_then_stamps(self) -> None:
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.log.read_text(encoding="utf-8")
        self.assertIn("test -p o2jam-file real_ojn_covers_extract_and_decode", calls)
        self.assertIn("test -p o2jam-file --test ojn_bit26_sweep", calls)
        self.assertTrue((self.repo / ".gate-full-sweep-stamp").is_file())

    def test_fresh_stamp_keeps_the_full_sweep_out(self) -> None:
        stamp = self.repo / ".gate-full-sweep-stamp"
        stamp.write_text("fresh\n", encoding="utf-8")
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.log.read_text(encoding="utf-8")
        self.assertNotIn("test -p o2jam-file", calls)

    def test_old_stamp_runs_the_full_sweep_again(self) -> None:
        stamp = self.repo / ".gate-full-sweep-stamp"
        stamp.write_text("old\n", encoding="utf-8")
        old = time.time() - 8 * 24 * 60 * 60
        os.utime(stamp, (old, old))
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("real_ojn_covers_extract_and_decode", self.log.read_text(encoding="utf-8"))

    def test_unit_suite_excludes_the_real_window_click_proof(self) -> None:
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.log.read_text(encoding="utf-8")
        self.assertIn("--skip x11_arm_routes_click_through_real_game", calls)

    def test_rig_lane_names_the_lock_and_the_real_window_proof(self) -> None:
        script = SCRIPTS_DIR / "rig-x11-click-proof.sh"
        result = subprocess.run(
            ["bash", str(script), "--dry-run", "--timeout", "7"],
            capture_output=True,
            text=True,
            env=dict(os.environ),
            cwd=self.repo,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("owner=x11-click-proof timeout=7", result.stdout)
        self.assertIn(
            "cargo test -p input-drive x11_arm_routes_click_through_real_game -- --exact",
            result.stdout,
        )

    def test_hidden_arm_stops_when_the_probe_reports_reachable_data(self) -> None:
        # The probe itself is unit-tested; here we prove the gate CALLS it.
        probe_lib = self.repo / "scripts" / "lib" / "data-root.sh"
        probe_log = self.repo / "probe.log"
        with probe_lib.open("a", encoding="utf-8") as handle:
            handle.write(
                "\nassert_hidden_data_unreachable() { "
                f'echo probe >> "{probe_log}"; return 1; }}\n'
            )
        result = self.run_gate()
        self.assertEqual(result.returncode, 1)
        self.assertTrue(probe_log.is_file())

    def test_full_arm_requires_archives_only_in_the_present_suite(self) -> None:
        result = self.run_gate(fake_canary=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.workspace_suite_markers(), [REQUIRED_MARKER, UNSET_MARKER]
        )

    def test_quick_arm_leaves_archive_requirement_unset(self) -> None:
        result = self.run_gate("--arm", "quick", fake_canary=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.workspace_suite_markers(), [UNSET_MARKER])

    def test_hidden_arm_leaves_archive_requirement_unset(self) -> None:
        result = self.run_gate("--arm", "hidden", fake_canary=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.workspace_suite_markers(), [UNSET_MARKER])

    def test_gate_output_shows_named_skip_lines_from_the_workspace_suite(self) -> None:
        # Live chain: scripts/gate-check.sh --arm hidden ->
        # cargo test --workspace -- --show-output -> captured SKIP line.
        # Without --show-output cargo hides printed SKIP lines, so the
        # official gate log cannot show why a real-data test skipped.
        env = dict(self.env)
        env["FAKE_SKIP_LINE"] = FAKE_SKIP_LINE
        result = subprocess.run(
            ["bash", str(self.repo / "scripts" / "gate-check.sh"),
             "--no-push", "--arm", "hidden"],
            capture_output=True,
            text=True,
            env=env,
            cwd=self.repo,
            timeout=60,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(FAKE_SKIP_LINE, result.stdout)
        suite_calls = [line for line in self.log.read_text(encoding="utf-8").splitlines()
                       if "test --workspace" in line]
        self.assertTrue(suite_calls, "the gate must run a workspace suite")
        for call in suite_calls:
            self.assertIn("--show-output", call, call)


def function_body(source: str, name: str) -> str:
    """One test function's body, from its opening brace."""
    start = source.index(f"fn {name}(")
    opening = source.index("{", start)
    depth = 0
    for index in range(opening, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[opening:index + 1]
    raise AssertionError(f"unbalanced braces in {name}")


class RealDataSkipTest(unittest.TestCase):
    """Seam: gate routing is source behavior, not a player-visible path."""

    ROUTED_TESTS = (
        ("crates/recomp2jam/src/loading.rs",
         "loading_cover_decodes_the_walked_chart"),
        ("crates/o2jam-shell/src/playing.rs",
         "from_archive_binds_all_seven_lanes"),
        ("crates/o2jam-shell/src/screens/game/mod.rs",
         "staged_catalog_and_selection_load_the_walked_cover"),
    )

    def test_named_real_data_tests_route_through_the_pack_gate(self) -> None:
        # Gate chain: cargo test --workspace -> real-data test ->
        # o2jam_file::data_root::pack_dir_for_test -> `SKIP <test name>`
        # -> gate log (`--show-output`). A silent `return` hides the
        # reason, so it fails this guard.
        for relative_path, test_name in self.ROUTED_TESTS:
            with self.subTest(test=test_name):
                source = (REPO_ROOT / relative_path).read_text(encoding="utf-8")
                body = function_body(source, test_name)
                self.assertIn("installed_file_bytes(", body)
                self.assertIn(f"stringify!({test_name})", body,
                              "the SKIP label must name the test function")
                self.assertNotRegex(
                    body, r"let Ok\([^)]*\) = std::fs::read",
                    "a missing pack must report SKIP, not return silently",
                )
                self.assertIn("pack_dir_for_test(&root, test_name)", source,
                              "the local helper must reach the shared pack gate")
                self.assertIn("PACK GAP {test_name}", source,
                              "a present pack with a missing file must fail")


def mask_rust_source(source: str) -> str:
    """Blank comments and string bodies; keep code offsets intact."""
    masked = list(source)
    index = 0
    mode = "code"
    while index < len(source):
        char = source[index]
        following = source[index + 1] if index + 1 < len(source) else ""
        if mode == "code":
            if char == "/" and following == "/":
                mode = "line"
                masked[index] = masked[index + 1] = " "
                index += 2
                continue
            if char == "/" and following == "*":
                mode = "block"
                masked[index] = masked[index + 1] = " "
                index += 2
                continue
            if char == "'":
                closing = source.find("'", index + 1)
                if 0 < closing - index <= 10:
                    for pos in range(index, closing + 1):
                        if source[pos] != "\n": masked[pos] = " "
                    index = closing + 1
                    continue
            if char == '"':
                mode = "string"
                masked[index] = " "
            index += 1
        elif mode == "line":
            if char == "\n":
                mode = "code"
            else:
                masked[index] = " "
            index += 1
        elif mode == "block":
            if char == "*" and following == "/":
                mode = "code"
                masked[index] = masked[index + 1] = " "
                index += 2
                continue
            if char != "\n":
                masked[index] = " "
            index += 1
        else:
            if char == "\\":
                masked[index] = " "
                index += 2 if index + 1 < len(source) else 1
                continue
            if char == '"':
                mode = "code"
            if char != "\n":
                masked[index] = " "
            index += 1
    return "".join(masked)


def rust_test_functions(source: str):
    """Yield (test name, body) for each #[test] function in one source."""
    code = mask_rust_source(source)
    lines = source.splitlines(keepends=True)
    code_lines = code.splitlines(keepends=True)
    for index, line in enumerate(code_lines):
        if "#[test" not in line:
            continue
        function_line = index + 1
        match = None
        while function_line < min(index + 40, len(code_lines)):
            candidate = code_lines[function_line]
            if "#[" in candidate or not candidate.strip():
                function_line += 1
                continue
            match = re.search(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(", candidate)
            if match:
                break
            function_line += 1
        if match is None:
            continue
        opening = source.index("{", sum(len(item) for item in lines[:function_line]) + match.end())
        depth = 0
        in_string = False
        escaped = False
        for position in range(opening, len(source)):
            char = source[position]
            if in_string:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    in_string = False
                continue
            if char == '"':
                in_string = True
            elif char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    yield match.group(1), source[opening:position + 1]
                    break


SKIP_PRINT_RE = re.compile(r'(?:println|eprintln)!\(\s*"([^"\n]*(?:SKIP|skip)[^"\n]*)"')
PACK_NAME_RE = re.compile(r'pack(?:_dir_for_test|_skip_report)\(\s*[^,]*?,\s*"([^"]+)"', re.S)


def skip_print_labels(body: str) -> list[str]:
    """Literal diagnostic labels a test can print."""
    return SKIP_PRINT_RE.findall(body)


def literal_pack_labels(body: str) -> list[str]:
    """Fixed test names passed to the shared pack gate."""
    return PACK_NAME_RE.findall(body)


DATA_GATE_TOKENS = (
    "install_root(",

    "archives_present(",
    "root_has_game_data(",
    "game_data_dir(",
    "pack_dir_for_test(",
    "pack_skip_report(",
    "installed_file_bytes(",
    "installed_data_dir(",
    "interface_archive(",
    "key_art(",
    "install_archive(",
    "stage_shop_art(",
    "stage_side_art(",
    "installed_catalog_songs(",
    "skip_without_archives(",
    "playing_render_archives_present(",
)
ROUTE_HELPERS = (
    "pack_dir_for_test(",
    "pack_skip_report(",
    "installed_file_bytes(",
    "installed_data_dir(",
    "install_archive(",
    "stage_shop_art(",
)
NOT_REAL_DATA = {
    ("crates/o2jam-file/src/data_root.rs",
     "empty_override_falls_back_to_the_checkout_root"):
        "path arithmetic only; no archive read",
    ("crates/o2jam-file/src/data_root.rs",
     "unset_override_falls_back_to_the_checkout_root"):
        "path arithmetic only; no archive read",
    ("crates/o2jam-file/src/data_root.rs",
     "lowercase_data_folder_is_recognised"):
        "synthetic temporary directory; no installed archive",
    ("crates/o2jam-file/src/data_root.rs",
     "uppercase_data_folder_is_recognised"):
        "synthetic temporary directory; no installed archive",
    ("crates/o2jam-file/src/data_root.rs",
     "root_without_a_data_folder_has_no_game_data"):
        "negative path probe; no installed archive",
    ("crates/recomp2jam/src/shots.rs",
     "real_data_arm_is_not_silently_absent"):
        "gate canary; absence is the assertion, not a skip",
    ("crates/recomp2jam/src/shots.rs",
     "missing_archives_stage_nothing"):
        "asserts the no-archive fallback; no data required",
    ("crates/recomp2jam/src/shots.rs",
     "play_skin_keeps_fallback_without_archive"):
        "asserts the no-archive fallback; no data required",
    ("crates/recomp2jam/src/data_dir_guard.rs",
     "no_tracked_path_may_shadow_the_git_ignored_data_dirs"):
        "git ls-files checks the ignored directory name, not archive bytes",
    ("crates/o2jam-shell/tests/install_root_probe.rs",
     "explicit_data_root_probe_fails_when_a_hidden_arm_can_reach_archives"):
        "environment probe; no archive read",
    ("crates/o2jam-shell/src/screens/paint.rs",
     "featured_price_row_inks_a_green_won_glyph_and_a_gold_gem_glyph"):
        "system-font fallback; no install-root art",
}


def names_data_gate(body: str, test_name: str) -> bool:
    """True when a real-data test passes its own name into a gate helper."""
    compact = re.sub(r"\s+", "", body)
    quoted = f'"{test_name}"'
    stringified = f"stringify!({test_name})"
    return (quoted in compact or stringified in compact) and any(
        helper in compact for helper in ROUTE_HELPERS
    )


class WorkspaceSkipTest(unittest.TestCase):
    """Seam: the source audit is the whole SKIP-label behavior."""

    def test_every_real_data_test_reports_a_named_skip(self) -> None:
        # Gate chain: real-data test -> named SKIP -> cargo --show-output ->
        # official gate log. A silent return or a generic label leaves the
        # hidden arm indistinguishable from a passing test (issue #914).
        violations: list[str] = []
        for path in sorted((REPO_ROOT / "crates").rglob("*.rs")):
            source = path.read_text(encoding="utf-8")
            relative = path.relative_to(REPO_ROOT).as_posix()
            for test_name, body in rust_test_functions(source):
                if not any(token in body for token in DATA_GATE_TOKENS):
                    continue
                if (relative, test_name) in NOT_REAL_DATA:
                    continue
                if f"SKIP {test_name}:" in body:
                    continue
                if names_data_gate(body, test_name):
                    continue
                violations.append(f"{relative}::{test_name}")
        self.assertEqual([], violations, "real-data tests must report a named SKIP")

    def test_every_data_skip_label_names_its_test_function(self) -> None:
        # Gate chain: real-data test -> SKIP line -> cargo --show-output ->
        # official gate log. A label that names a group, not the #[test],
        # cannot be grepped back to the test (reviewer finding 4, #914).
        violations: list[str] = []
        for path in sorted((REPO_ROOT / "crates").rglob("*.rs")):
            source = path.read_text(encoding="utf-8")
            relative = path.relative_to(REPO_ROOT).as_posix()
            for test_name, body in rust_test_functions(source):
                for label in skip_print_labels(body):
                    if not label.startswith(f"SKIP {test_name}:"):
                        violations.append(f"{relative}::{test_name}: {label!r}")
                for label in literal_pack_labels(body):
                    if label != test_name:
                        violations.append(f"{relative}::{test_name}: pack label {label!r}")
        self.assertEqual([], violations, "SKIP labels must name their #[test] function")


if __name__ == "__main__":
    unittest.main()
