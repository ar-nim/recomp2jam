# Residual taxonomy in region metrics (#250)

This is the method half of #250: every region in a native A/B pack gets a
classification and inspectable evidence. The script does not file issues.
Pixels and captures stay in the local pack; only these scripts and notes are
tracked.

## Schema (backward-compatible)

`scripts/native_ab_regions.py` keeps the existing `schema` string
(`ab334-native-region-metrics-v1`) and every existing region key. It adds these
fields to each region:

| Field | Values | Meaning |
|---|---|---|
| `divergence_class` | `art-selection`, `geometry`, `font-presentation`, `scenario`, `server-data`, `unpinned` | What kind of pixel difference was observed. |
| `residual_class` | `implementation-gap`, `evidence-gap`, `capture-fault`, `vintage-delta`, `approved-presentation` | What still closes the story (#250 vocabulary). |
| `evidence` | list of doc anchors, hashes, or URLs | Inspectable backing for the class. |
| `linked_bug` | issue number, URL, or `null` | Bug that owns an implementation gap; `null` means not filed. |
| `exe_capture` | hash or local path | Exe-side capture pin; required for `implementation-gap`. |
| `note` | text | Why this class was chosen, or the next evidence step. |

A top-level `taxonomy` block records `ab334-residual-taxonomy-v1`, the
annotation sidecar path, and its SHA-256. Old readers ignore the new keys.

Classes stay on two axes on purpose. `server-data` describes a value that came
from a reply; `scenario` describes a different state or stimulus. Neither is an
exe behavior mismatch until a matched-window pair pins it. `font-presentation`
and `approved-presentation` record the owner's rasterizer decision; they do not
silently excuse geometry or art faults. `unpinned` / `evidence-gap` is the only
honest default: a region nobody annotated has no verdict.

## Annotate a pack

Evidence is supplied by a pack-local sidecar, never by a guess in the script:

```json
{
  "schema": "ab334-residual-taxonomy-v1",
  "regions": {
    "planet.emblem_strip": {
      "divergence_class": "art-selection",
      "residual_class": "implementation-gap",
      "evidence": [
        "docs/captures/journey-audit/288-planet.md#P3",
        "https://github.com/ar-nim/recomp2jam/issues/361"
      ],
      "linked_bug": 361,
      "exe_capture": "235f53521171be4f1d4c725d16143332fa47b66813f9e1c5b8e8b09fe68ddc21",
      "note": "Rest badges are bright in Rust and dimmed in the exe."
    }
  }
}
```

Keys are `scene.region` from the region map. Validation rejects unknown keys
(stale annotations after a map change), unknown classes, classified regions
without evidence, and an `implementation-gap` without an exe capture pin.
Existing region-map fields pass through; the sidecar overrides them.

```bash
python3 scripts/native_ab_regions.py --pack "$PACK" \
  --annotations "$PACK/residual-annotations.json"

python3 scripts/native_ab_regions.py --pack "$PACK" --triage \
  --annotations "$PACK/residual-annotations.json" \
  --output "$PACK/residual-report.md"

python3 scripts/native_ab_regions.py --pack "$PACK" --triage \
  --group-by residual
```

`--triage` reads `region-metrics.json` and writes markdown. It needs no
frames, ffmpeg, or tracker access, and never rewrites the metrics. The report
groups regions by divergence class by default; `--group-by residual` groups by
the #250 action vocabulary. Each row carries gate status, rect, rationale,
evidence links, aligned crop paths and full crop hashes, exe capture pin, and
linked Bug. The rollup counts both axes, and the filing queue lists every
implementation gap with its Bug or `NOT FILED`.

## Filing an implementation gap

The report is a queue, not a filing agent. For each `NOT FILED` row:

1. Open the exe capture and aligned crops named in the row. Confirm the same
   scenario and that the divergence is not an unpinned wire/server value.
2. Write the bug body with the metrics path and hash, scene/region, rect, both
   SSIM/PSNR values, the exe capture pin, and the evidence links. A 7.55
   assumption without a pin stays `evidence-gap`, not a parity failure.
3. Create **one** issue per row, sequentially:
   `scripts/issue-create.sh --title "..." --body-file BODY --type Bug
   --label needs-triage --label component:<scene>`. Read back number, type and
   labels. Triage is a separate step; it may promote to `ready-for-agent` in
   one pass with the record visible on the issue
   (`docs/agents/triage-labels.md`).
4. Add the read-back number to the sidecar's `linked_bug` and rerun `--triage`.
   The queue must show the link before the residual is called filed.

The senior verifies each #250 acceptance criterion against the report and the
tracker before closing; green script guards alone do not close it.

## Guards and their reachability

`scripts/tests/test_residual_taxonomy.py` uses synthetic metrics/annotations
only. seam: the taxonomy is a pure transformation plus a read-only CLI over an
already-scored local pack; no live boot reaches this code. A green guard proves
schema compatibility, validation, and report grouping. It does not prove any
pixel pair, exe capture, or filed Bug. Those need the live evidence described
above and the journey-audit matrices.
