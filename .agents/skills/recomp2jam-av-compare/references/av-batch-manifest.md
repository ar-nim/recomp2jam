# recomp2jam A/V Batch Manifest

This manifest drives `scripts/batch_compare_av.py`. Paths resolve relative to
the manifest. Keep manifests that reference proprietary captures under the
same local-only artifact root.

## Top-level object

- `schema_version` (string, required): exactly `recomp2jam-av-v1`.
- `scenes` (array, required): one object per scene comparison.

## Scene keys

- `id` (string, required): stable scene/scenario identifier.
- `label` (string, optional): human-readable label.
- `ref` (string, required): original 7.55 reference capture.
- `test` (string, required): recomp2jam capture.
- `out_dir` (string, required): per-scene report directory.
- `width`, `height` (numbers, optional): use 800 and 600 for native-canvas
  normalization when required.
- `fps` (number, optional): omit unless resampling is part of the capture plan.
- `audio_rate` (number, optional): defaults to 48000 in the compare script.
- `offset` (number, optional): seconds added to the reference input timeline.
- `trim_start`, `duration` (numbers, optional): segment bounds in seconds.
- `no_vmaf` (boolean, optional): disable VMAF explicitly.
- `thresholds` (string, optional): scene-specific thresholds JSON.

Both inputs currently require audio streams. A batch comparison failure never
runs threshold checks against an old summary. Use the batch `--strict` option
for CI so missing required metrics fail; deliberately disabled VMAF remains
exempt from that strict check.

## Thresholds JSON

- `ssim_min` (number)
- `psnr_min` (number)
- `vmaf_min` (number)
- `audio_lufs_delta_max` (number)
- `audio_peak_delta_max` (number)

These are regression thresholds, not proof of 7.55 behavior. Infinite PSNR on
identical frames is a passing value. Finite averages exclude infinite frames;
the summary records their count.

## Example

```json
{
  "schema_version": "recomp2jam-av-v1",
  "scenes": [
    {
      "id": "waiting-host-ready",
      "label": "STATE_WAITING host ready",
      "ref": "captures/reference/waiting.mp4",
      "test": "captures/rust/waiting.mp4",
      "out_dir": "reports/waiting",
      "width": 800,
      "height": 600,
      "audio_rate": 48000,
      "offset": 0.25,
      "trim_start": 2.0,
      "duration": 10.0,
      "no_vmaf": true,
      "thresholds": "thresholds/waiting.json"
    }
  ]
}
```
