# recomp2jam Batch Manifest Schema

Use JSON or TOML with these canonical keys. This schema describes one project's
scene/platform validation matrix, not a catalog of games.

## Top-level

- `schema_version` (string, required): `recomp2jam-batch-v1`.
- `batch_id` (string, required): stable run identifier.
- `created_at` (string, required): ISO 8601 timestamp.
- `reference` (object, required): 7.55 identity and provenance.
- `toolchain` (object, required): pipeline and tool versions.
- `global_defaults` (object, optional): shared scenario settings.
- `runs` (array, required): scene/platform records.

## reference

- `client_version` (string): `7.55`.
- `file_name` (string): reference executable name.
- `size_bytes` (number): expected file size.
- `sha256` (string): full reference hash.
- `provenance_record` (string): local-only provenance file path.

## toolchain

Record applicable values: `pipeline_version`, `rustc_version`,
`cargo_version`, `wine_version`, `xvfb_version`, `ffmpeg_version`,
`memoryer_version`, and `libvmaf_version`. Include the build commit and whether
the tree was dirty in each report.

## global_defaults

- `resolution`: normally `800x600` for native-client scene comparison.
- `audio_rate`: explicit sample rate when audio is captured.
- `renderer_settings`: renderer/backend and relevant options.
- `metrics_thresholds`: reviewed regression thresholds, if any.
- `artifact_root`: local-only output root.

## runs[]

- `id` (string, required): stable scenario/platform identifier.
- `scene` (string, required): for example `STATE_WAITING` or `STATE_PLAYING`.
- `platform` (string, required): Rust target or `windows-7.55-wine` reference.
- `build` (object, required): profile, features, and artifact path if used.
- `inputs` (object, required): `input_trace`, `reference_captures`, and
  `required_install_paths` arrays.
- `validation` (object, required): `checks`, `targets`, and optional
  `metrics_thresholds`.
- `status` (object, required): `state`, `last_updated`, and optional `notes`.
- `artifacts` (object, optional): report, metrics, captures, logs, and profile
  paths.

Valid states are `pending`, `running`, `passed`, `failed`, `skipped`, and
`needs_review`. Every `skipped` state needs a reason. A known mismatch stays
`failed` or `needs_review` according to the requested gate; it is not silently
reclassified as passed.

## Example

```json
{
  "schema_version": "recomp2jam-batch-v1",
  "batch_id": "waiting-linux-2026-11-08",
  "created_at": "2026-11-08T09:20:00Z",
  "reference": {
    "client_version": "7.55",
    "file_name": "AngelO2Jam.exe",
    "size_bytes": 5029888,
    "sha256": "<full-local-reference-hash>",
    "provenance_record": "inputs/provenance/o2jam-7.55.toml"
  },
  "toolchain": {
    "pipeline_version": "1",
    "rustc_version": "<recorded>",
    "wine_version": "<recorded>",
    "ffmpeg_version": "<recorded>"
  },
  "global_defaults": {
    "resolution": "800x600",
    "audio_rate": 48000,
    "artifact_root": "runs"
  },
  "runs": [
    {
      "id": "waiting-host-ready-linux",
      "scene": "STATE_WAITING",
      "platform": "linux-x86_64",
      "build": { "profile": "release", "features": [] },
      "inputs": {
        "input_trace": ["traces/waiting-host-ready.json"],
        "reference_captures": ["references/waiting-host-ready.mp4"],
        "required_install_paths": ["install-root/O2Jam/Image/Interface1.opi"]
      },
      "validation": {
        "checks": ["synthetic", "real-data", "av-compare"],
        "targets": { "resolution": "800x600", "audio_rate": 48000 }
      },
      "status": {
        "state": "pending",
        "last_updated": "2026-11-08T09:20:00Z"
      },
      "artifacts": {
        "report_path": "runs/waiting-host-ready-linux/reports/summary.json",
        "metrics_dir": "runs/waiting-host-ready-linux/metrics",
        "captures_dir": "runs/waiting-host-ready-linux/captures"
      }
    }
  ]
}
```
