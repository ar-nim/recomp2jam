#!/usr/bin/env python3
"""rig-provenance.py - capture provenance manifests (docs/reversing.md section 6).

A capture is exe evidence only when its manifest names the producing
binary, the tool versions, the display and command, and the capture's
own hash. This module writes that manifest (`emit`), enforces it
(`check`), and prints a ready-to-paste citation (`cite`).

`scripts/rig-provenance.sh` probes the tools from the shell and calls
`emit`; `scripts/run-manifest.sh verify` calls `check`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

SCHEMA_ID = "rig-provenance-v1"
MANIFEST_NAME = "provenance.json"
MANIFEST_HASH_KEY = "manifest_sha256"

# A committed capture record is a hash-pinned pointer: copyrighted bytes
# stay in the local, git-ignored run dir.
RECORD_HASHES_NAME = "record-hashes.txt"
RECORD_COPIES = ("wine-provenance.json", "wine-manifest.json", "wine-citation.txt")

EXE_KIND = "exe"
CLIENT_BUILD_KIND = "client-build"
PRODUCER_KINDS = (EXE_KIND, CLIENT_BUILD_KIND)

# A capture is pixels or sound: the classes section 6 governs.
MEDIA_BY_SUFFIX = {
    ".png": "image",
    ".jpg": "image",
    ".jpeg": "image",
    ".webp": "image",
    ".bmp": "image",
    ".ppm": "image",
    ".pam": "image",
    ".xwd": "image",
    ".gif": "image",
    ".mp4": "video",
    ".mkv": "video",
    ".webm": "video",
    ".wav": "audio",
    ".flac": "audio",
    ".ogg": "audio",
    ".opus": "audio",
}

# Tool roles that must be pinned whenever a manifest uses them.
ROLE_PYTHON = "python3"
ROLE_CAPTURE = "capture"
ROLE_DISPLAY_SERVER = "xvfb"
ROLE_WINE = "wine"
ROLE_INJECTOR = "injector"

EXIT_INCOMPLETE = 1
EXIT_USAGE = 2


class EmitError(Exception):
    """The caller supplied input that cannot become a manifest."""


# --- hashing ---------------------------------------------------------------


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def describe_file(run_dir: Path, relative: str) -> dict:
    path = run_dir / relative
    return {
        "path": relative,
        "sha256": sha256_of(path),
        "size_bytes": path.stat().st_size,
        "media": MEDIA_BY_SUFFIX.get(path.suffix.lower(), "other"),
    }


def canonical_digest(document: dict) -> str:
    without_hash = {k: v for k, v in document.items() if k != MANIFEST_HASH_KEY}
    encoded = json.dumps(without_hash, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(encoded.encode("utf-8")).hexdigest()


# --- discovery -------------------------------------------------------------


def is_capture(path: Path) -> bool:
    return path.is_file() and path.suffix.lower() in MEDIA_BY_SUFFIX


def capture_files(run_dir: Path) -> list[str]:
    return [
        path.relative_to(run_dir).as_posix()
        for path in sorted(run_dir.rglob("*"))
        if is_capture(path)
    ]


def read_traditional_manifest(run_dir: Path) -> dict:
    path = run_dir / "manifest.json"
    if not path.is_file():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {}


def build_run_block(args: argparse.Namespace, run_dir: Path) -> dict:
    traditional = read_traditional_manifest(run_dir)
    run = {"out_dir": str(run_dir)}
    for field in ("pair", "build", "seed"):
        given = getattr(args, field)
        recorded = traditional.get(field, "")
        if given and recorded and str(given) != str(recorded):
            raise EmitError(
                f"run.{field} {given!r} contradicts manifest.json {recorded!r}"
            )
        run[field] = str(given or recorded or "")
    run["manifest_json"] = "manifest.json" if traditional else ""
    return run


# --- emit ------------------------------------------------------------------


def build_producer(args: argparse.Namespace) -> dict:
    if args.producer_kind not in PRODUCER_KINDS:
        raise EmitError(
            f"producer kind must be one of {', '.join(PRODUCER_KINDS)} "
            f"(got {args.producer_kind!r})"
        )
    producer_path = Path(args.producer)
    if not producer_path.is_file():
        raise EmitError(f"producer binary not found: {producer_path}")
    producer = {
        "kind": args.producer_kind,
        "name": producer_path.name,
        "path": str(producer_path.resolve()),
        "sha256": sha256_of(producer_path),
        "size_bytes": producer_path.stat().st_size,
    }
    if args.expect_producer_sha256 and producer["sha256"] != args.expect_producer_sha256:
        raise EmitError(
            f"producer sha256 {producer['sha256']} does not match the pinned "
            f"expectation {args.expect_producer_sha256}"
        )
    if args.producer_kind == EXE_KIND:
        if not args.producer_version:
            raise EmitError(f"an {EXE_KIND} producer needs --producer-version (e.g. 7.55)")
        producer["version"] = args.producer_version
    else:
        producer["cargo_profile"] = args.cargo_profile
        producer["git_commit"] = args.git_commit
        producer["git_dirty"] = bool(args.git_dirty)
    return producer


def build_tools(args: argparse.Namespace) -> dict:
    tools = {}
    for role, name, version, available in args.tool:
        tools[role] = {"name": name, "version": version, "available": available == "1"}
    return tools


def build_artifacts(args: argparse.Namespace, run_dir: Path) -> list[dict]:
    listed = capture_files(run_dir)
    for extra in args.artifact:
        if extra not in listed:
            listed.append(extra)
    return [describe_file(run_dir, relative) for relative in sorted(listed)]


def build_session(args: argparse.Namespace) -> dict:
    return {
        "display": args.display,
        "wayland_display": args.wayland_display,
        "xdg_session_type": args.xdg_session_type,
        "pulse_sink": args.pulse_sink,
        "command": list(args.command_arg),
    }


def build_document(args: argparse.Namespace, run_dir: Path) -> dict:
    document = {
        "schema": SCHEMA_ID,
        "created_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "run": build_run_block(args, run_dir),
        "producer": build_producer(args),
        "session": build_session(args),
        "tools": build_tools(args),
        "artifacts": build_artifacts(args, run_dir),
    }
    document[MANIFEST_HASH_KEY] = canonical_digest(document)
    return document


def cmd_emit(args: argparse.Namespace) -> int:
    run_dir = Path(args.run_dir).resolve()
    if not run_dir.is_dir():
        return usage_error(f"run dir not found: {run_dir}")
    try:
        document = build_document(args, run_dir)
    except EmitError as error:
        return usage_error(str(error))
    manifest_path = run_dir / MANIFEST_NAME
    manifest_path.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    print(
        f"rig-provenance: wrote {manifest_path} "
        f"({len(document['artifacts'])} artifact(s), manifest sha256 "
        f"{document[MANIFEST_HASH_KEY][:12]})"
    )
    return 0


# --- check -----------------------------------------------------------------


def required_roles(document: dict) -> list[str]:
    roles = [ROLE_PYTHON]
    if document.get("artifacts"):
        roles.append(ROLE_CAPTURE)
    session = document.get("session", {})
    if session.get("display") or session.get("wayland_display"):
        roles.append(ROLE_DISPLAY_SERVER)
    if document.get("producer", {}).get("kind") == EXE_KIND:
        roles.append(ROLE_WINE)
    if ROLE_INJECTOR in document.get("tools", {}):
        roles.append(ROLE_INJECTOR)
    return roles


def missing_fields(document: dict) -> list[str]:
    problems = []
    if document.get("schema") != SCHEMA_ID:
        problems.append(f"schema: expected {SCHEMA_ID!r}")
    if not document.get("created_at"):
        problems.append("field: created_at")
    run = document.get("run", {})
    for field in ("pair", "build"):
        if not run.get(field):
            problems.append(f"field: run.{field}")
    return problems


def producer_problems(document: dict) -> list[str]:
    producer = document.get("producer", {})
    kind = producer.get("kind")
    problems = []
    if kind not in PRODUCER_KINDS:
        problems.append("field: producer.kind")
    if not producer.get("path"):
        problems.append("field: producer.path")
    digest = producer.get("sha256", "")
    if len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        problems.append("field: producer.sha256 (64 hex characters)")
    if not producer.get("size_bytes"):
        problems.append("field: producer.size_bytes")
    if kind == EXE_KIND and not producer.get("version"):
        problems.append("field: producer.version (the exe version pin)")
    if kind == CLIENT_BUILD_KIND:
        if not producer.get("cargo_profile"):
            problems.append("field: producer.cargo_profile")
        if not producer.get("git_commit"):
            problems.append("field: producer.git_commit")
    return problems


def session_problems(document: dict) -> list[str]:
    session = document.get("session", {})
    problems = []
    if not session.get("display") and not session.get("wayland_display"):
        problems.append("field: session.display")
    if not session.get("command"):
        problems.append("field: session.command (the full command line)")
    return problems


def tool_problems(document: dict) -> list[str]:
    tools = document.get("tools", {})
    problems = []
    for role in required_roles(document):
        entry = tools.get(role)
        if not isinstance(entry, dict):
            problems.append(f"tool: {role} not recorded")
        elif not entry.get("available"):
            problems.append(f"tool: {role} unavailable ({entry.get('name', role)})")
        elif not entry.get("version"):
            problems.append(f"tool: {role} has no version")
    return problems


def artifact_problems(document: dict, run_dir: Path) -> list[str]:
    entries = document.get("artifacts", [])
    on_disk = capture_files(run_dir)
    if not isinstance(entries, list) or not entries:
        if on_disk:
            return ["artifact: the run dir holds captures but the manifest lists none"]
        return []
    problems = []
    listed = {}
    for entry in entries:
        relative = entry.get("path", "")
        if not relative:
            problems.append("artifact: entry without a path")
            continue
        if relative in listed:
            problems.append(f"artifact: {relative} listed twice")
            continue
        listed[relative] = entry
        problems += artifact_entry_problems(entry, run_dir / relative)
    problems += [
        f"artifact: {relative} has no provenance entry"
        for relative in on_disk
        if relative not in listed
    ]
    return problems


def artifact_entry_problems(entry: dict, full: Path) -> list[str]:
    relative = entry["path"]
    if not full.is_file():
        return [f"artifact: {relative} listed but missing on disk"]
    problems = []
    if sha256_of(full) != entry.get("sha256"):
        problems.append(f"artifact: {relative} bytes changed since the manifest")
    if full.stat().st_size != entry.get("size_bytes"):
        problems.append(f"artifact: {relative} size changed since the manifest")
    return problems


def producer_binary_problems(document: dict) -> list[str]:
    producer = document.get("producer", {})
    path = Path(producer.get("path", ""))
    if not path.is_file():
        return []
    if sha256_of(path) != producer.get("sha256"):
        return [f"producer: {path.name} bytes differ from the recorded sha256"]
    return []


def manifest_hash_problems(document: dict) -> list[str]:
    recorded = document.get(MANIFEST_HASH_KEY, "")
    if not recorded:
        return ["field: manifest_sha256"]
    if recorded != canonical_digest(document):
        return ["manifest: manifest_sha256 does not match the manifest content"]
    return []


def load_manifest(run_dir: Path) -> dict:
    return json.loads((run_dir / MANIFEST_NAME).read_text(encoding="utf-8"))


def check(run_dir: Path, require_manifest: bool = False) -> tuple[list[str], list[str]]:
    """Return (problems, notes); no problems means the captures may be cited."""
    if not (run_dir / MANIFEST_NAME).is_file():
        if require_manifest or capture_files(run_dir):
            return ([f"manifest: no {MANIFEST_NAME} in {run_dir}"], [])
        return ([], ["no captures: provenance not required"])
    try:
        document = load_manifest(run_dir)
    except json.JSONDecodeError as error:
        return ([f"manifest: {MANIFEST_NAME} is not valid JSON ({error})"], [])
    problems = (
        missing_fields(document)
        + producer_problems(document)
        + session_problems(document)
        + tool_problems(document)
        + artifact_problems(document, run_dir)
        + producer_binary_problems(document)
        + manifest_hash_problems(document)
    )
    notes = []
    if not Path(document.get("producer", {}).get("path", "")).is_file():
        notes.append("producer binary not on disk: hash recorded at emit time")
    return (problems, notes)


def report(problems: list[str], notes: list[str], ok_message: str) -> int:
    for note in notes:
        print(f"rig-provenance: note: {note}")
    if not problems:
        if ok_message:
            print(ok_message)
        return 0
    return report_incomplete(problems)


def report_incomplete(problems: list[str]) -> int:
    print("rig-provenance: PROVENANCE INCOMPLETE", file=sys.stderr)
    for problem in problems:
        print(f"  missing: {problem}", file=sys.stderr)
    print(
        "rig-provenance: an artifact without provenance is a lead, not exe "
        "evidence (docs/reversing.md section 6)",
        file=sys.stderr,
    )
    return EXIT_INCOMPLETE


def cmd_check(args: argparse.Namespace) -> int:
    run_dir = Path(args.run_dir).resolve()
    if not run_dir.is_dir():
        return usage_error(f"run dir not found: {run_dir}")
    problems, notes = check(run_dir)
    return report(problems, notes, "rig-provenance: manifest complete")


# --- committed record -------------------------------------------------------


def invalid_sha256(digest: str) -> bool:
    return len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest.lower())


def read_record_pins(path: Path) -> tuple[dict[str, str], list[str]]:
    """Read the pinned sha256 values; a missing pin is a record failure."""
    if not path.is_file():
        return {}, [f"record: no hash pins at {path}"]
    pins: dict[str, str] = {}
    problems = []
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split()
        if len(fields) != 2 or invalid_sha256(fields[0]):
            problems.append(f"record: malformed pin on line {number}")
            continue
        digest, name = fields
        if "/" in name or "\\" in name:
            problems.append(f"record: pin on line {number} is not a file name")
        elif name in pins:
            problems.append(f"record: duplicate pin for {name}")
        else:
            pins[name] = digest
    for name in RECORD_COPIES:
        if name not in pins:
            problems.append(f"record: no pin for {name}")
    return pins, problems


def record_copy_problems(record_dir: Path, pins: dict[str, str]) -> list[str]:
    problems = []
    for name in RECORD_COPIES:
        digest = pins.get(name)
        if digest is None:
            continue
        path = record_dir / name
        if not path.is_file():
            problems.append(f"record: {name} missing from the committed record")
            continue
        actual = sha256_of(path)
        if actual != digest:
            problems.append(f"record: {name} sha256 {actual} != pinned {digest}")
    return problems


def citation_link_problems(record_dir: Path) -> list[str]:
    """The citation must quote the pinned provenance, not merely exist."""
    try:
        document = json.loads((record_dir / RECORD_COPIES[0]).read_text(encoding="utf-8"))
        citation = (record_dir / RECORD_COPIES[2]).read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        return [f"record: cannot read the pinned provenance or citation ({error})"]
    problems = []
    digest = document.get(MANIFEST_HASH_KEY, "")
    if digest != canonical_digest(document):
        problems.append("record: provenance manifest_sha256 does not match its content")
    if f"manifest sha256 {digest}" not in citation:
        problems.append("record: citation does not quote the pinned manifest sha256")
    artifacts = document.get("artifacts") or []
    if not isinstance(artifacts, list) or not artifacts:
        problems.append("record: pinned provenance lists no artifacts")
    else:
        for entry in artifacts:
            name = entry.get("path", "")
            phrase = f"{name} sha256 {entry.get('sha256', '')}"
            if phrase not in citation:
                problems.append(f"record: citation omits artifact hash {name}")
    return problems


def report_record_problems(problems: list[str]) -> int:
    print("rig-provenance: RECORD CHECK FAILED", file=sys.stderr)
    for problem in problems:
        print(f"  {problem}", file=sys.stderr)
    print(
        "rig-provenance: a drifting pointer is not evidence "
        "(docs/captures/t240/findings.md)",
        file=sys.stderr,
    )
    return EXIT_INCOMPLETE


def cmd_check_record(args: argparse.Namespace) -> int:
    record_dir = Path(args.record_dir).resolve()
    if not record_dir.is_dir():
        return usage_error(f"record dir not found: {record_dir}")
    pins, problems = read_record_pins(record_dir / RECORD_HASHES_NAME)
    problems += record_copy_problems(record_dir, pins)
    if not problems:
        problems += citation_link_problems(record_dir)
    if problems:
        return report_record_problems(problems)
    for name in RECORD_COPIES:
        print(f"rig-provenance: {name} sha256 {pins[name]} (pinned)")
    print("rig-provenance: record check OK; citation quotes the pinned provenance")
    return 0


# --- cite ------------------------------------------------------------------


def tool_label(tools: dict, role: str) -> str:
    entry = tools.get(role)
    if not entry:
        return ""
    return f"{entry.get('name', role)} {entry.get('version', '')}".strip()


def citation(document: dict, claim: str) -> str:
    run = document["run"]
    producer = document["producer"]
    session = document["session"]
    tools = document["tools"]
    session_name = session.get("display") or session.get("wayland_display", "")
    artifact_phrases = ", ".join(
        f"{entry['path']} sha256 {entry['sha256']} ({entry['size_bytes']} B)"
        for entry in document["artifacts"]
    )
    parts = [
        f"Provenance {document['schema']} (manifest sha256 "
        f"{document[MANIFEST_HASH_KEY]}), emitted {document['created_at']}",
        f"run pair `{run.get('pair', '')}` on {session_name} "
        f"({tool_label(tools, ROLE_DISPLAY_SERVER)}), sink "
        f"{session.get('pulse_sink') or 'unset'}, command "
        f"`{' '.join(session.get('command', []))}`",
        f"producer {producer['kind']} {producer['name']} "
        f"{producer.get('version') or producer.get('cargo_profile', '')}, "
        f"{producer['size_bytes']} B, sha256 {producer['sha256']}",
        f"tools {tool_label(tools, ROLE_WINE) or 'no wine'}, capture "
        f"{tool_label(tools, ROLE_CAPTURE)}, injector "
        f"{tool_label(tools, ROLE_INJECTOR) or 'none'}",
        f"artifacts: {artifact_phrases or 'none'}",
    ]
    if claim == "exe":
        parts.append(
            "these pixels are exe evidence only through this manifest; any "
            "claim cites it (#281, docs/reversing.md section 6)"
        )
    return "; ".join(parts) + "."


def claim_problems(document: dict, claim: str) -> list[str]:
    if claim != "exe":
        return []
    problems = []
    producer_kind = document.get("producer", {}).get("kind")
    if producer_kind != EXE_KIND:
        problems.append(
            f"claim exe: the producer kind is {producer_kind!r}, not {EXE_KIND!r} "
            "(the exe did not draw these pixels)"
        )
    if not document.get("tools", {}).get(ROLE_WINE, {}).get("available"):
        problems.append("claim exe: no wine version recorded")
    return problems


def cmd_cite(args: argparse.Namespace) -> int:
    run_dir = Path(args.run_dir).resolve()
    if not run_dir.is_dir():
        return usage_error(f"run dir not found: {run_dir}")
    problems, notes = check(run_dir, require_manifest=True)
    document = None if problems else load_manifest(run_dir)
    if document is not None:
        problems = claim_problems(document, args.claim)
    if problems:
        return report(problems, notes, "")
    print(citation(document, args.claim))
    return 0

# --- cli -------------------------------------------------------------------


def usage_error(message: str) -> int:
    print(f"rig-provenance: {message}", file=sys.stderr)
    return EXIT_USAGE


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="rig-provenance.py")
    commands = parser.add_subparsers(dest="command", required=True)

    emit = commands.add_parser("emit", help="write <run-dir>/provenance.json")
    emit.add_argument("run_dir")
    emit.add_argument("--producer", required=True)
    emit.add_argument("--producer-kind", default=CLIENT_BUILD_KIND)
    emit.add_argument("--producer-version", default="")
    emit.add_argument("--cargo-profile", default="")
    emit.add_argument("--git-commit", default="")
    emit.add_argument("--git-dirty", action="store_true")
    emit.add_argument("--expect-producer-sha256", default="")
    emit.add_argument("--pair", default="")
    emit.add_argument("--build", default="")
    emit.add_argument("--seed", default="")
    emit.add_argument("--display", default="")
    emit.add_argument("--wayland-display", default="")
    emit.add_argument("--xdg-session-type", default="")
    emit.add_argument("--pulse-sink", default="")
    emit.add_argument("--command-arg", action="append", default=[])
    emit.add_argument("--tool", nargs=4, action="append", default=[],
                      metavar=("ROLE", "NAME", "VERSION", "AVAILABLE"))
    emit.add_argument("--artifact", action="append", default=[])

    check = commands.add_parser("check", help="enforce a run dir's provenance")
    check.add_argument("run_dir")

    record = commands.add_parser("check-record", help="verify a hash-pinned committed record")
    record.add_argument("record_dir")

    cite = commands.add_parser("cite", help="print a citation paragraph")
    cite.add_argument("run_dir")
    cite.add_argument("--claim", choices=("none", "exe"), default="none")
    return parser


def main(argv: list[str]) -> int:
    args = build_parser().parse_args(argv[1:])
    handler = {
        "emit": cmd_emit,
        "check": cmd_check,
        "check-record": cmd_check_record,
        "cite": cmd_cite,
    }[args.command]
    return handler(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
