#!/usr/bin/env python3
"""Observe that a test run never connects to an owned decoy Wayland socket.

The script binds a private Unix socket, runs `cargo test -p input-drive`
with that socket as `WAYLAND_DISPLAY`, then counts queued connections and
records the result as an observation artifact. It never touches a display
or compositor it did not create.
"""

from __future__ import annotations

import os
import pathlib
import socket
import subprocess
import tempfile
from datetime import datetime, timezone

REPO = pathlib.Path(__file__).resolve().parents[1]
OUT = REPO / "docs/captures/267/decoy-env-observation.md"
DECOY_NAME = "fence-267-decoy-wayland"
LIVE_NAME = "wayland-1"


def unix_sockets() -> list[tuple[str, str]]:
    rows = []
    for line in pathlib.Path("/proc/net/unix").read_text().splitlines()[1:]:
        fields = line.split()
        if len(fields) >= 8:
            rows.append((fields[6], fields[7]))
    return rows


def socket_summary(rows: list[tuple[str, str]]) -> str:
    live = sorted(inode for inode, path in rows if LIVE_NAME in path)
    decoy = sorted(inode for inode, path in rows if DECOY_NAME in path)
    return f"live={live or '(none)'} decoy={decoy or '(none)'}"


def client_processes() -> list[str]:
    found = []
    for entry in pathlib.Path("/proc").glob("[0-9]*/cmdline"):
        try:
            cmd = entry.read_bytes().replace(b"\0", b" ").decode("utf-8", "replace").strip()
        except OSError:
            continue
        executable = cmd.split(" ", 1)[0].rsplit("/", 1)[-1]
        if executable.startswith("input_drive-") or executable in {
            "recomp2jam",
            "Xvfb",
            "kwin_wayland",
        }:
            found.append(f"{entry.parent.name}: {cmd[:160]}")
    return sorted(found)


def drain_connections(listener: socket.socket) -> int:
    connections = 0
    while True:
        try:
            client, _ = listener.accept()
            connections += 1
            client.close()
        except socket.timeout:
            return connections


def render_report(
    started: str,
    runtime_dir: pathlib.Path,
    before_sockets: str,
    after_sockets: str,
    connections: int,
    test: subprocess.CompletedProcess[str],
    spawned_processes: list[str],
    before_processes: list[str],
    passed: bool,
) -> str:
    summaries = "; ".join(
        f"`{line.strip()}`" for line in test.stdout.splitlines() if "test result:" in line
    )
    return "\n".join(
        [
            "# Decoy-Wayland observation for #267",
            "",
            f"Started (UTC): `{started}`. Worktree: `{REPO}`. Parent environment: "
            "`WAYLAND_DISPLAY=wayland-1` (owner session), `DISPLAY=:0`; child "
            f"overridden to `{runtime_dir}` and `WAYLAND_DISPLAY={DECOY_NAME}` "
            "(no real compositor).",
            "",
            "## Commands",
            "",
            "```text",
            "python3 scripts/observe-decoy-display.py",
            "```",
            "",
            "## Observation",
            "",
            "- Wayland socket inodes before test:",
            "",
            "```text",
            before_sockets,
            "```",
            "",
            "- Wayland socket inodes after test (same decoy inode = listener intact):",
            "",
            "```text",
            after_sockets,
            "```",
            "",
            f"- Connections accepted by the decoy listener after the run: `{connections}`.",
            f"- `cargo test -p input-drive` exit code: `{test.returncode}`.",
            f"- Test summaries: {summaries}.",
            "- New client/Xvfb processes during the run (empty = none spawned):",
            "",
            "```text",
            "\n".join(spawned_processes) if spawned_processes else "(none)",
            "```",
            "",
            "- Client/Xvfb processes before the run (context):",
            "",
            "```text",
            "\n".join(before_processes) if before_processes else "(none)",
            "```",
            "",
            f"Verdict: **{'PASS' if passed else 'FAIL'}** — "
            "zero decoy connections and no spawned client/Xvfb process, with the "
            "input-drive suite green under a fake ambient Wayland display.",
            "",
        ]
    )


def main() -> None:
    runtime_dir = pathlib.Path(tempfile.mkdtemp(prefix="fence-267-"))
    decoy_path = runtime_dir / DECOY_NAME
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listener.bind(str(decoy_path))
    listener.listen(8)
    listener.settimeout(1.0)

    started = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    before_sockets = socket_summary(unix_sockets())
    before_processes = client_processes()

    env = dict(os.environ)
    env["WAYLAND_DISPLAY"] = DECOY_NAME
    env["XDG_RUNTIME_DIR"] = str(runtime_dir)
    env.pop("WAYLAND_SOCKET", None)
    test = subprocess.run(
        ["cargo", "test", "-p", "input-drive"],
        cwd=REPO,
        env=env,
        capture_output=True,
        text=True,
        timeout=900,
        check=False,
    )

    connections = drain_connections(listener)
    after_sockets = socket_summary(unix_sockets())
    listener.close()
    spawned = [line for line in client_processes() if line not in before_processes]
    passed = test.returncode == 0 and connections == 0 and not spawned

    report = render_report(
        started,
        runtime_dir,
        before_sockets,
        after_sockets,
        connections,
        test,
        spawned,
        before_processes,
        passed,
    )
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(report, encoding="utf-8")
    print(report)
    raise SystemExit(0 if passed else 1)


if __name__ == "__main__":
    main()
