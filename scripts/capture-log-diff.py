#!/usr/bin/env python3
"""Diff two window-driver capture logs with the normalization proven in t260.

Usage: scripts/capture-log-diff.py BEFORE AFTER [--judged-first | --full]

Normalization is line-for-line identical to docs/captures/t260/capture-log-ab.md:
an event/judged index becomes T, a frame index and wall_ms become F and T,
and state/content hex becomes HASH. No line is dropped or reordered.

Modes:
  --full (default)  compare every normalized line, including frame cadence
  --judged-first    compare only event and judged lines (log-order contract)

Exit codes: 0 equal after normalization, 1 differences (unified diff on
stdout), 2 bad arguments or unreadable input.
"""

from __future__ import annotations

import argparse
import difflib
import re
from pathlib import Path

NORMALIZATIONS = (
    (re.compile(r"^(event|judged) [0-9]+"), r"\1 T"),
    (re.compile(r"^frame [0-9]+ wall_ms=[0-9]+"), "frame F wall_ms=T"),
    (re.compile(r" state=[0-9a-f]+"), " state=HASH"),
    (re.compile(r" content=[0-9a-f]+"), " content=HASH"),
)
CAPTURE_LINE_PREFIXES = ("event ", "judged ")


def normalize_line(line: str) -> str:
    """Replace only the values that a live boot cannot repeat."""
    for pattern, replacement in NORMALIZATIONS:
        line = pattern.sub(replacement, line, count=1)
    return line


def read_capture_log(path: Path, judged_only: bool) -> list[str]:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    if judged_only:
        lines = [line for line in lines if line.startswith(CAPTURE_LINE_PREFIXES)]
    return [normalize_line(line) for line in lines]


def make_unified_report(
    before: list[str], after: list[str], before_path: Path, after_path: Path
) -> str:
    return "\n".join(
        difflib.unified_diff(
            before,
            after,
            fromfile=str(before_path),
            tofile=str(after_path),
            lineterm="",
        )
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path, help="capture log from the first leg")
    parser.add_argument("after", type=Path, help="capture log from the second leg")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--judged-first",
        action="store_true",
        help="compare only event and judged lines (log-order contract)",
    )
    mode.add_argument(
        "--full",
        action="store_true",
        help="compare every line (default), including frame cadence",
    )
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    judged_only = args.judged_first
    try:
        before = read_capture_log(args.before, judged_only)
        after = read_capture_log(args.after, judged_only)
    except OSError as error:
        parser.error(str(error))

    report = make_unified_report(before, after, args.before, args.after)
    if report:
        print(report)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
