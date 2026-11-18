#!/usr/bin/env python3
"""Assemble a local-only native 800x600 A/B evidence pack.

Every input is a local capture path; this script copies bytes only into the
caller's pack directory under ~/scratch. The repository receives no art,
music, or capture bytes.
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))
from native_ab import (  # noqa: E402  (local helper module, loaded after sys.path)
    DEFAULT_REGION_SCALE,
    SCENES,
    require_native_png,
    scale_rect,
    sha256_of,
    still_media_command,
    write_json,
)

REPO_ROOT = Path(__file__).resolve().parents[1]
LOADING_REFERENCE_FRAME = "waiting-cue-012.png"
TOKEN_PLACEHOLDER = "<redacted: read from Memoryer O2JAM.db>"


def tool_version(command: list[str]) -> str:
    """Read the first line of a tool's version output."""
    result = subprocess.run(command, capture_output=True, text=True, check=True)
    return result.stdout.splitlines()[0] if result.stdout else result.stderr.strip()


def reference_frame(wine_lobby: Path, wine_loading: Path, scene: str) -> Path:
    """Find the pinned Wine frame for one scene."""
    if scene == "loading":
        # The source pack mislabels this loading frame as a waiting cue.
        return wine_loading / "artifacts/screenshots" / LOADING_REFERENCE_FRAME
    return wine_lobby / "artifacts/screenshots" / f"{scene}.png"


def rust_scene_frame(rust_run: Path, scene: str) -> Path:
    """Find the one frame stamped with a scene name by the replay driver."""
    matches = sorted((rust_run / "frames").glob(f"frame-*-{scene}.png"))
    if len(matches) != 1:
        raise ValueError(
            f"{rust_run}/frames: want exactly one {scene} frame, found {len(matches)}"
        )
    return matches[0]


def copy_frame(source: Path, destination: Path) -> str:
    """Copy one native frame byte-for-byte and return its source hash."""
    require_native_png(source)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    return sha256_of(source)


def wrap_frame(frame: Path, media: Path) -> None:
    """Wrap one still in lossless FFV1 with a silent carrier."""
    media.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(still_media_command(frame, media), check=True)


def frame_sources(wine_lobby: Path, wine_loading: Path, rust_run: Path,
                  pack: Path) -> dict[str, dict[str, str]]:
    """Copy and hash every native frame pair in the pack."""
    sources: dict[str, dict[str, str]] = {}
    for scene in SCENES:
        ref_source = reference_frame(wine_lobby, wine_loading, scene)
        test_source = rust_scene_frame(rust_run, scene)
        sources[scene] = {
            "ref_source": str(ref_source),
            "ref_source_sha256": copy_frame(ref_source, pack / "frames/ref" / f"{scene}.png"),
            "test_source": str(test_source),
            "test_source_sha256": copy_frame(test_source, pack / "frames/test" / f"{scene}.png"),
        }
        wrap_frame(pack / "frames/ref" / f"{scene}.png", pack / "media/ref" / f"{scene}.mkv")
        wrap_frame(pack / "frames/test" / f"{scene}.png", pack / "media/test" / f"{scene}.mkv")
    return sources


def build_region_map(legacy_map: Path, scale: int) -> dict[str, Any]:
    """Rescale every #332 rect while keeping its class and rationale."""
    legacy = json.loads(legacy_map.read_text())
    scenes = {}
    for scene, regions in legacy["scenes"].items():
        scenes[scene] = []
        for region in regions:
            native_region = dict(region)
            native_region["rect"] = scale_rect(region["rect"], scale)
            native_region["rect_origin"] = f"{legacy_map.name} rect x{scale} (no re-derivation)"
            scenes[scene].append(native_region)
    return {
        "schema": "ab334-native-region-map-v1",
        "source_map": str(legacy_map),
        "source_map_sha256": sha256_of(legacy_map),
        "scale": scale,
        "classes": legacy["classes"],
        "frames": {
            "ref": "frames/ref/<scene>.png (Wine 7.55, native 800x600)",
            "test": "frames/test/<scene>.png (Rust --frame-dump, native 800x600)",
        },
        "scenes": scenes,
    }


def write_batch_manifest(pack: Path, thresholds: Path) -> None:
    """Write an av-compare manifest with no scaling or frame-rate override."""
    scenes = [
        {
            "id": scene,
            "label": f"STATE_{scene.upper()} (native 800x600 still A/B)",
            "ref": f"media/ref/{scene}.mkv",
            "test": f"media/test/{scene}.mkv",
            "out_dir": f"reports/{scene}",
            "audio_rate": 48000,
            "duration": 1,
            "no_vmaf": True,
            "thresholds": "thresholds/skill-default.json",
        }
        for scene in SCENES
    ]
    write_json(pack / "av-batch-native.json", {
        "schema_version": "recomp2jam-av-v1",
        "scenes": scenes,
    })
    destination = pack / "thresholds/skill-default.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(thresholds, destination)


def source_run_identity(run_dir: Path) -> dict[str, Any]:
    """Capture the producer identity recorded by a rig pack."""
    provenance = json.loads((run_dir / "provenance.json").read_text())
    manifest = json.loads((run_dir / "manifest.json").read_text())
    return {
        "out_dir": provenance["run"]["out_dir"],
        "pair": provenance["run"]["pair"],
        "build": provenance["run"]["build"],
        "seed": provenance["run"]["seed"],
        "exe": provenance["producer"],
        "session_display": provenance["session"]["display"],
        "pulse_sink": provenance["session"]["pulse_sink"],
        "tools": provenance["tools"],
        "manifest_raw_sha256": sha256_of(run_dir / "manifest.json"),
        "manifest_canonical_sha256": provenance["manifest_sha256"],
        "scene_sequence": manifest.get("scene_sequence", []),
    }


def redact_token(command: list[str]) -> list[str]:
    """Replace a CLI session token before any manifest is written."""
    safe = list(command)
    if "--token" in safe:
        safe[safe.index("--token") + 1] = TOKEN_PLACEHOLDER
    return safe


def git_commit() -> str:
    """Record the exact producer source revision."""
    result = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD"],
        capture_output=True, text=True, check=True,
    )
    return result.stdout.strip()


def write_provenance(pack: Path, wine_lobby: Path, wine_loading: Path,
                     rust_run: Path, trace: Path,
                     frames: dict[str, dict[str, str]]) -> None:
    """Record source identities, commands, tools, and transformations."""
    rust_manifest = json.loads((rust_run / "manifest.json").read_text())
    recorded_command = rust_manifest.get("command")
    command = (
        redact_token(recorded_command) if recorded_command
        else ["<not recorded: rerun with the committed replay-trace-native.py>"]
    )
    document = {
        "schema": "ab334-pack-provenance-v1",
        "created_at": rust_manifest["finished_at"],
        "reference_captures": {
            "planet/channel/room/waiting": source_run_identity(wine_lobby),
            "loading": source_run_identity(wine_loading),
        },
        "test_capture": {
            "run_dir": str(rust_run),
            "driver": str(REPO_ROOT / "scripts/replay-trace-native.py"),
            "trace": str(trace),
            "trace_sha256": sha256_of(trace),
            "binary": rust_manifest["binary"],
            "binary_sha256": sha256_of(Path(rust_manifest["binary"])),
            "git_commit": git_commit(),
            "display": rust_manifest["display"],
            "command": command,
            "applied_events": rust_manifest["applied_events"],
            "manifest_raw_sha256": sha256_of(rust_run / "manifest.json"),
            "audio": "--no-sound; no sink opened, no game-audio parity claim",
        },
        "frame_sources": frames,
        "tools": {
            "python3": tool_version([sys.executable, "--version"]),
            "ffmpeg": tool_version(["ffmpeg", "-version"]),
            "rustc": tool_version(["rustc", "--version"]),
            "pillow": Image.__version__,
        },
        "transformations": [
            "Native frames copied byte-for-byte; no crop, scale, or resample.",
            "Each still wrapped once as FFV1 bgr0 + silent PCM (1 s, 48 kHz stereo) "
            "because compare_av.py requires an audio stream.",
            "Region rects from the legacy map multiplied by the declared factor; "
            "classes and exclusions unchanged.",
        ],
        "wine_boot_this_leg": False,
    }
    write_json(pack / "provenance.json", document)


def write_pack_manifest(pack: Path, rust_run: Path,
                        frames: dict[str, dict[str, str]]) -> None:
    """Mirror the Wine pack manifest conventions for the native pack."""
    document = {
        "pair": "native-800x600-wine-vs-rust-window",
        "build": git_commit(),
        "params": "800x600 native frames; no resize; FFV1 bgr0 + silent PCM carrier",
        "script": "scripts/replay-trace-native.py (windowed online walk, --no-sound)",
        "seed": "u1-windowed-online first 19 events; local Memoryer user 3",
        "created_at": json.loads((rust_run / "manifest.json").read_text())["finished_at"],
        "input_log": "test-capture/input.log",
        "scene_sequence": ["avi", "planet", "channel", "room", "waiting", "loading"],
        "frames": frames,
        "artifacts": {
            "root": ".",
            "input_log": "test-capture/input.log",
            "screenshots_dir": "frames",
            "media_dir": "media",
            "batch_manifest": "av-batch-native.json",
            "region_map": "regions-v1-native.json",
            "metrics": "region-metrics.json",
        },
        "required_artifacts": [
            "manifest.json", "provenance.json", "artifact-manifest.json",
            "av-batch-native.json", "regions-v1-native.json",
            "region-metrics.json", "frames", "media", "test-capture",
        ],
    }
    write_json(pack / "manifest.json", document)


def copy_test_capture(pack: Path, rust_run: Path) -> None:
    """Keep the replay driver's own logs beside the derived pack."""
    destination = pack / "test-capture"
    destination.mkdir(parents=True, exist_ok=True)
    for name in ("manifest.json", "input.log", "applied.log", "client.log"):
        source = rust_run / name
        if source.is_file():
            shutil.copy2(source, destination / name)


def refresh_artifact_manifest(pack: Path) -> Path:
    """Hash every pack file except the hash manifest itself."""
    entries = []
    for path in sorted(pack.rglob("*")):
        if not path.is_file() or path.name == "artifact-manifest.json":
            continue
        entries.append({
            "path": path.relative_to(pack).as_posix(),
            "sha256": sha256_of(path),
            "size_bytes": path.stat().st_size,
        })
    output = pack / "artifact-manifest.json"
    write_json(output, {"schema": "ab334-artifact-manifest-v1", "root": str(pack), "files": entries})
    return output


def build_pack(args: argparse.Namespace) -> None:
    """Assemble the full pack, then hash it."""
    pack = args.pack
    frames = frame_sources(args.wine_lobby, args.wine_loading, args.rust_run, pack)
    copy_test_capture(pack, args.rust_run)
    write_json(pack / "regions-v1-native.json", build_region_map(args.legacy_map, args.scale))
    write_batch_manifest(pack, args.thresholds)
    write_provenance(pack, args.wine_lobby, args.wine_loading, args.rust_run, args.trace, frames)
    write_pack_manifest(pack, args.rust_run, frames)
    manifest = refresh_artifact_manifest(pack)
    print(f"artifact manifest: {manifest} {sha256_of(manifest)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("build", "refresh"))
    parser.add_argument("--pack", type=Path, required=True, help="local-only pack root")
    parser.add_argument("--wine-lobby", type=Path, help="Wine pack with lobby scenes")
    parser.add_argument("--wine-loading", type=Path, help="Wine pack with the loading cue")
    parser.add_argument("--rust-run", type=Path, help="replay-trace-native.py output dir")
    parser.add_argument("--trace", type=Path, help="input trace used for the Rust run")
    parser.add_argument("--legacy-map", type=Path, help="400x300 region map to rescale")
    parser.add_argument("--thresholds", type=Path, help="skill-default thresholds JSON")
    parser.add_argument("--scale", type=int, default=DEFAULT_REGION_SCALE)
    args = parser.parse_args()
    if args.action == "refresh":
        manifest = refresh_artifact_manifest(args.pack)
        print(f"artifact manifest: {manifest} {sha256_of(manifest)}")
        return 0
    required = (
        args.wine_lobby, args.wine_loading, args.rust_run,
        args.trace, args.legacy_map, args.thresholds,
    )
    if any(value is None for value in required):
        parser.error("build needs --wine-lobby --wine-loading --rust-run --trace --legacy-map --thresholds")
    build_pack(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
