"""Scenario tests for the issue-creation process guard.

CLI seam tests: they pin `scripts/issue-create.sh` behavior with a fake `gh`
and a temp fixture title list. No live caller chain from boot exists — this is
a developer-workflow script, so the seam is the whole surface.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
ISSUE_CREATE = SCRIPTS / "issue-create.sh"

FAKE_GH = '''#!/usr/bin/env python3
"""Test double: reads the fixture title list; records every gh invocation."""
import json
import os
import sys

argv = sys.argv[1:]
with open(os.environ["ISSUE_CREATE_TEST_GH_LOG"], "a", encoding="utf-8") as log:
    log.write(json.dumps(argv) + "\\n")

command = argv[:2]
if command == ["issue", "list"]:
    with open(os.environ["ISSUE_CREATE_TEST_TITLES"], encoding="utf-8") as titles:
        for line in titles:
            if line.strip():
                print(line.rstrip("\\n"))
elif command == ["issue", "create"]:
    created_title = argv[argv.index("--title") + 1]
    with open(os.environ["ISSUE_CREATE_TEST_CREATED_TITLE"], "w", encoding="utf-8") as title_file:
        title_file.write(created_title)
    print("https://github.com/ar-nim/recomp2jam/issues/999")
elif command == ["issue", "view"]:
    with open(os.environ["ISSUE_CREATE_TEST_CREATED_TITLE"], encoding="utf-8") as title_file:
        created_title = title_file.read()
    print(json.dumps({
        "number": 999,
        "title": created_title,
        "issueType": {"name": "Bug"},
        "labels": [{"name": "needs-triage"}],
    }))
else:
    sys.exit(2)
'''


class IssueCreateTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.titles = self.directory / "open-titles.tsv"
        self.gh_log = self.directory / "gh-calls.jsonl"
        self.created_title = self.directory / "created-title.txt"
        self.body = self.directory / "body.md"
        self.body.write_text("Synthetic issue body.\n", encoding="utf-8")

        fake_bin = self.directory / "bin"
        fake_bin.mkdir()
        fake_gh = fake_bin / "gh"
        fake_gh.write_text(FAKE_GH, encoding="utf-8")
        fake_gh.chmod(0o755)

        self.env = os.environ.copy()
        self.env["PATH"] = f"{fake_bin}{os.pathsep}{self.env['PATH']}"
        self.env["ISSUE_CREATE_TEST_TITLES"] = str(self.titles)
        self.env["ISSUE_CREATE_TEST_GH_LOG"] = str(self.gh_log)
        self.env["ISSUE_CREATE_TEST_CREATED_TITLE"] = str(self.created_title)

    def write_open_titles(self, *titles: str) -> None:
        """Fixture rows are `number<TAB>title`, the shape gh --jq emits."""
        self.titles.write_text(
            "".join(f"{number}\t{title}\n" for number, title in enumerate(titles, 431)),
            encoding="utf-8",
        )

    def run_create(self, title: str, *extra_args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                str(ISSUE_CREATE),
                "--title", title,
                "--body-file", str(self.body),
                "--type", "Bug",
                *extra_args,
            ],
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )

    def gh_calls(self) -> list[list[str]]:
        if not self.gh_log.exists():
            return []
        return [json.loads(line) for line in self.gh_log.read_text(encoding="utf-8").splitlines()]

    def calls_for(self, *command: str) -> list[list[str]]:
        return [call for call in self.gh_calls() if call[:2] == list(command)]

    def list_search_query(self) -> str:
        list_calls = self.calls_for("issue", "list")
        self.assertEqual(len(list_calls), 1, list_calls)
        return list_calls[0][list_calls[0].index("--search") + 1]

    def test_doubled_bug_prefix_is_normalized_before_create(self) -> None:
        self.write_open_titles()

        result = self.run_create("Bug: Bug: Stop duplicate recon ticket floods")

        self.assertEqual(result.returncode, 0, result.stderr)
        create_calls = self.calls_for("issue", "create")
        self.assertEqual(len(create_calls), 1, create_calls)
        self.assertEqual(
            create_calls[0][create_calls[0].index("--title") + 1],
            "Bug: Stop duplicate recon ticket floods",
        )
        self.assertIn("Bug: Stop duplicate recon ticket floods", result.stdout)

    def test_doubled_task_prefix_is_normalized_before_create(self) -> None:
        self.write_open_titles()

        result = self.run_create("Task: Task: Replay the issue creation gate")

        self.assertEqual(result.returncode, 0, result.stderr)
        create_calls = self.calls_for("issue", "create")
        self.assertEqual(
            create_calls[0][create_calls[0].index("--title") + 1],
            "Task: Replay the issue creation gate",
        )

    def test_mixed_case_repeated_prefixes_keep_one_canonical_prefix(self) -> None:
        self.write_open_titles()

        result = self.run_create("bug: TASK:  Document the dedup rule")

        self.assertEqual(result.returncode, 0, result.stderr)
        create_calls = self.calls_for("issue", "create")
        self.assertEqual(
            create_calls[0][create_calls[0].index("--title") + 1],
            "Bug: Document the dedup rule",
        )

    def test_open_issue_with_same_normalized_title_is_rejected(self) -> None:
        self.write_open_titles("Bug: Stop duplicate recon ticket floods")

        result = self.run_create("bug:  stop   duplicate recon ticket floods")

        self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
        self.assertIn("duplicate open issue #431", result.stderr)
        self.assertEqual(self.calls_for("issue", "create"), [])

    def test_doubled_prefix_does_not_hide_a_duplicate(self) -> None:
        self.write_open_titles("Bug: Stop duplicate recon ticket floods")

        result = self.run_create("Bug: Bug: Stop duplicate recon ticket floods")

        self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
        self.assertIn("duplicate open issue #431", result.stderr)
        self.assertEqual(self.calls_for("issue", "create"), [])

    def test_unrelated_open_title_does_not_block_creation(self) -> None:
        self.write_open_titles("431\tBug: Repair a separate crash in planet")

        result = self.run_create("Bug: Stop duplicate recon ticket floods")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(self.calls_for("issue", "create")), 1)

    def test_duplicate_search_uses_normalized_phrase_and_only_open_issues(self) -> None:
        self.write_open_titles()

        result = self.run_create("Bug: Bug: Stop duplicate recon ticket floods")

        self.assertEqual(result.returncode, 0, result.stderr)
        list_call = self.calls_for("issue", "list")[0]
        self.assertIn("--state", list_call)
        self.assertEqual(list_call[list_call.index("--state") + 1], "open")
        self.assertEqual(
            self.list_search_query(),
            '"stop duplicate recon ticket floods" in:title',
        )

    def test_parent_and_blocker_flags_reach_gh_create(self) -> None:
        self.write_open_titles()

        result = self.run_create("Bug: Stop duplicate recon ticket floods",
                                 "--parent", "487", "--blocked-by", "486")

        self.assertEqual(result.returncode, 0, result.stderr)
        create_call = self.calls_for("issue", "create")[0]
        self.assertEqual(create_call[create_call.index("--parent") + 1], "487")
        self.assertEqual(create_call[create_call.index("--blocked-by") + 1], "486")


if __name__ == "__main__":
    unittest.main()
