"""Residual classification for native A/B region metrics (#250).

The vocabulary has two independent axes. ``divergence_class`` says what kind
of pixel difference was observed. ``residual_class`` says what still closes the
story. An annotation must carry inspectable evidence; guesswork stays
``unpinned`` / ``evidence-gap`` instead of becoming a parity failure.
"""
from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

TAXONOMY_SCHEMA = "ab334-residual-taxonomy-v1"
DIVERGENCE_CLASSES = (
    "art-selection",
    "geometry",
    "font-presentation",
    "scenario",
    "server-data",
    "unpinned",
)
RESIDUAL_CLASSES = (
    "implementation-gap",
    "evidence-gap",
    "capture-fault",
    "vintage-delta",
    "approved-presentation",
)
GROUP_BY_DIMENSIONS = ("divergence", "residual")
TAXONOMY_FIELDS = (
    "divergence_class",
    "residual_class",
    "evidence",
    "linked_bug",
    "exe_capture",
    "note",
)
ISSUE_URL_ROOT = "https://github.com/ar-nim/recomp2jam/issues/"
CROP_LABELS = ("ref", "rust", "heat", "diff")


def default_annotation() -> dict[str, Any]:
    """Describe a region nobody has classified yet."""
    return {
        "divergence_class": "unpinned",
        "residual_class": "evidence-gap",
        "evidence": [],
        "linked_bug": None,
        "exe_capture": None,
        "note": "No annotation: evidence step still required.",
    }


def validate_annotation(key: str, annotation: dict[str, Any]) -> None:
    """Reject classes or claims that cannot be backed by evidence."""
    if not isinstance(annotation, dict):
        raise ValueError(f"{key}: annotation must be an object")
    unknown = sorted(set(annotation) - set(TAXONOMY_FIELDS))
    if unknown:
        raise ValueError(f"{key}: unknown annotation fields {unknown}")
    divergence = annotation.get("divergence_class", "unpinned")
    if divergence not in DIVERGENCE_CLASSES:
        raise ValueError(f"{key}: unknown divergence_class {divergence!r}")
    residual = annotation.get("residual_class", "evidence-gap")
    if residual not in RESIDUAL_CLASSES:
        raise ValueError(f"{key}: unknown residual_class {residual!r}")

    evidence = annotation.get("evidence", [])
    if not isinstance(evidence, list) or not all(isinstance(item, str) for item in evidence):
        raise ValueError(f"{key}: evidence must be a list of links")
    if divergence != "unpinned" and not evidence:
        raise ValueError(f"{key}: classified divergence needs at least one evidence link")
    if residual == "implementation-gap" and not annotation.get("exe_capture"):
        raise ValueError(f"{key}: implementation-gap needs an exe_capture pin")
    if not isinstance(annotation.get("note", ""), str):
        raise ValueError(f"{key}: note must be text")
    linked_bug = annotation.get("linked_bug")
    if linked_bug is not None and not isinstance(linked_bug, (int, str)):
        raise ValueError(f"{key}: linked_bug must be an issue number or URL")


def load_annotations(path: Path) -> dict[str, dict[str, Any]]:
    """Read a pack-local annotation sidecar and validate every region key."""
    document = json.loads(path.read_text(encoding="utf-8"))
    if document.get("schema") != TAXONOMY_SCHEMA:
        raise ValueError(f"{path}: schema must be {TAXONOMY_SCHEMA}")
    regions = document.get("regions")
    if not isinstance(regions, dict):
        raise ValueError(f"{path}: regions must be an object")
    for key, annotation in regions.items():
        validate_annotation(key, annotation)
    return regions


def region_key(scene: str, region: dict[str, Any]) -> str:
    """Identify one region across scenes."""
    return f"{scene}.{region['name']}"


def annotation_for_region(
    scene: str, region: dict[str, Any], annotations: dict[str, dict[str, Any]]
) -> dict[str, Any]:
    """Merge sidecar evidence into a copy of one region's taxonomy fields."""
    key = region_key(scene, region)
    merged = default_annotation()
    merged.update({name: region[name] for name in TAXONOMY_FIELDS if name in region})
    if key in annotations:
        validate_annotation(key, annotations[key])
        merged.update(annotations[key])
    validate_annotation(key, merged)
    return merged


def apply_taxonomy_to_metrics(
    metrics: dict[str, Any], annotations: dict[str, dict[str, Any]]
) -> dict[str, Any]:
    """Add taxonomy fields to every region without mutating the input."""
    enriched = copy.deepcopy(metrics)
    known = {
        region_key(scene, region)
        for scene, data in enriched["scenes"].items()
        for region in data["regions"]
    }
    unknown = sorted(set(annotations) - known)
    if unknown:
        raise ValueError(f"annotation keys have no region: {unknown}")
    for scene, data in enriched["scenes"].items():
        for region in data["regions"]:
            region.update(annotation_for_region(scene, region, annotations))
    return enriched


def as_number(value: Any) -> float | None:
    """Read a JSON metric, including the text sentinel for infinite PSNR."""
    if value is None:
        return None
    if value == "inf":
        return float("inf")
    return float(value)


def format_metric(value: Any, places: int) -> str:
    """Render one metric or its missing-value marker."""
    number = as_number(value)
    if number is None:
        return "n/a"
    return f"{number:.{places}f}"


def gate_status(region: dict[str, Any], thresholds: dict[str, float]) -> str:
    """State whether this region passes the scene's threshold gate."""
    if not region.get("score"):
        return "excluded from gate"
    try:
        ssim_min, psnr_min = thresholds["ssim_min"], thresholds["psnr_min"]
    except KeyError:
        return "gate thresholds missing"
    ssim, psnr = as_number(region.get("ssim")), as_number(region.get("psnr"))
    if ssim is None:
        return "no SSIM metric"
    passes = ssim >= ssim_min and (psnr is None or psnr >= psnr_min)
    result = "within gate" if passes else "GATE FAIL"
    return (
        f"{result} (SSIM {format_metric(ssim, 6)} / "
        f"PSNR {format_metric(psnr, 3)})"
    )


def format_evidence(item: str) -> str:
    """Render a URL as a link and a local citation as code."""
    if item.startswith(("http://", "https://")):
        return f"[{item}]({item})"
    return f"`{item}`"


def format_bug(linked_bug: Any) -> str:
    """Render the linked Bug, or an explicit filing prompt."""
    if linked_bug is None:
        return "NOT FILED"
    if isinstance(linked_bug, int):
        return f"[#{linked_bug}]({ISSUE_URL_ROOT}{linked_bug})"
    if linked_bug.startswith(("http://", "https://")):
        return f"[#{linked_bug.rstrip('/').rsplit('/', 1)[-1]}]({linked_bug})"
    return f"`{linked_bug}`"


def render_region(scene: str, region: dict[str, Any], thresholds: dict[str, float]) -> list[str]:
    """Describe one residual with the numbers and evidence behind it."""
    name = region["name"]
    evidence = ", ".join(map(format_evidence, region["evidence"])) or "none"
    crop_hashes = region.get("crops_sha256", {})
    crop_text = ", ".join(
        f"`{label}={crop_hashes[label]}`" for label in CROP_LABELS if label in crop_hashes
    ) or "none"
    lines = [
        f"### {scene}.{name} — {region['residual_class']}",
        "",
        f"- Gate: {gate_status(region, thresholds)}; rect `{region['rect']}`.",
        f"- Why: {region['note']}",
        f"- Evidence: {evidence}",
        f"- Aligned crops: `regions/{scene}/{name}__{{ref,rust,heat,diff}}.png`; hashes {crop_text}.",
    ]
    if region.get("exe_capture"):
        lines.append(f"- Exe capture: `{region['exe_capture']}`")
    if region["residual_class"] == "implementation-gap" or region["linked_bug"]:
        lines.append(f"- Bug: {format_bug(region['linked_bug'])}")
    lines.append("")
    return lines


def rollup_table(metrics: dict[str, Any]) -> list[str]:
    """Count regions on both taxonomy axes."""
    regions = [
        region
        for data in metrics["scenes"].values()
        for region in data["regions"]
    ]
    lines = ["## Rollup", "", "| Divergence | Regions |", "|---|---:|"]
    for value in DIVERGENCE_CLASSES:
        lines.append(f"| {value} | {sum(r['divergence_class'] == value for r in regions)} |")
    lines += ["", "| Residual class | Regions |", "|---|---:|"]
    for value in RESIDUAL_CLASSES:
        lines.append(f"| {value} | {sum(r['residual_class'] == value for r in regions)} |")
    lines.append("")
    return lines


def render_groups(metrics: dict[str, Any], group_by: str) -> list[str]:
    """Group every region under one taxonomy axis."""
    if group_by == "divergence":
        field, heading = "divergence_class", "Divergence"
        values = DIVERGENCE_CLASSES
    else:
        field, heading = "residual_class", "Residual class"
        values = RESIDUAL_CLASSES
    thresholds = metrics.get("thresholds", {})
    lines: list[str] = []
    for value in values:
        matches = [
            (scene, region)
            for scene, data in metrics["scenes"].items()
            for region in data["regions"]
            if region[field] == value
        ]
        if not matches:
            continue
        lines += [f"## {heading}: {value}", ""]
        for scene, region in matches:
            lines += render_region(scene, region, thresholds)
    return lines


def render_filing_queue(metrics: dict[str, Any]) -> list[str]:
    """List implementation gaps for the separate, verified filing step."""
    lines = ["## Implementation-gap filing queue", ""]
    gaps = [
        (scene, region)
        for scene, data in metrics["scenes"].items()
        for region in data["regions"]
        if region["residual_class"] == "implementation-gap"
    ]
    if not gaps:
        return lines + ["No implementation-gap residuals are annotated.", ""]
    lines += [
        "| Region | Bug | Exe capture | Divergence |",
        "|---|---|---|---|",
    ]
    for scene, region in gaps:
        lines.append(
            f"| `{region_key(scene, region)}` | {format_bug(region['linked_bug'])} "
            f"| `{region['exe_capture']}` | {region['divergence_class']} |"
        )
    lines += [
        "",
        "File each NOT FILED row as its own Bug with `scripts/issue-create.sh` "
        "after checking the exe capture and evidence. This report never writes to the tracker.",
        "",
    ]
    return lines


def render_triage_report(
    metrics: dict[str, Any], group_by: str = "divergence", source: Path | None = None
) -> str:
    """Build the markdown residual report from one metrics document."""
    if group_by not in GROUP_BY_DIMENSIONS:
        raise ValueError(f"group-by must be one of {GROUP_BY_DIMENSIONS}")
    lines = [
        "# Residual triage",
        "",
        f"- Source: `{source}`" if source else "- Source: supplied metrics",
        f"- Metrics schema: `{metrics['schema']}`; taxonomy: `{TAXONOMY_SCHEMA}`.",
        f"- Grouped by: `{group_by}` (use `--group-by residual` for the #250 vocabulary).",
        "- Unannotated regions stay `unpinned` / `evidence-gap`; no assumption is a parity failure.",
        "",
    ]
    lines += rollup_table(metrics)
    lines += render_groups(metrics, group_by)
    lines += render_filing_queue(metrics)
    return "\n".join(lines).rstrip() + "\n"
