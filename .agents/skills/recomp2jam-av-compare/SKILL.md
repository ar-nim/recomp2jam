---
name: recomp2jam-av-compare
description: Align and compare O2Jam 7.55 Wine captures with recomp2jam Rust audio/video output. Use for screenshot or recording diffs, SSIM/PSNR/VMAF, audio loudness or drift, scene thresholds, A/V batch manifests, and automated parity reports.
---

# recomp2jam A/V Compare

Use media metrics to locate differences, not to replace behavioral evidence.
A high aggregate score cannot turn an `OPEN` or known `FAIL` scene cell into
`PASS`.

## Prepare comparable inputs

1. Follow `recomp2jam-reference-capture` for provenance, native 800x600 frames,
   raw media preservation, and contained audio. Read the scene's current
   verdict in `docs/parity.md`.
2. Keep scenario, art vintage, settings, input trace, and timeline equivalent.
   Separate expected differences such as modern font rasterization from
   geometry, layering, timing, and gameplay differences.
3. Match resolution, aspect ratio, frame rate, audio rate, channel layout, and
   trim only through recorded transformations. Avoid scaling away native-pixel
   placement errors.
4. Align on a verified scene transition, sound cue, or input event. Check a
   short segment visually and audibly before computing full metrics.

## Run the scripts

Run commands from the repository root. Scripts require Python 3 and ffmpeg;
VMAF is optional.

```bash
python3 .agents/skills/recomp2jam-av-compare/scripts/compare_av.py \
  --ref "$HOME/scratch/captures/reference-waiting.mp4" \
  --test "$HOME/scratch/captures/rust-waiting.mp4" \
  --out-dir "$HOME/scratch/reports/waiting" \
  --label "STATE_WAITING" \
  --width 800 --height 600 \
  --audio-rate 48000 \
  --offset 0.0 --trim-start 0.0 --duration 10.0
```

Use `--no-vmaf` when libvmaf is unavailable. Do not add `--fps` unless the
capture plan explicitly requires resampling.

```bash
python3 .agents/skills/recomp2jam-av-compare/scripts/check_summary.py \
  "$HOME/scratch/reports/waiting/summary.json" \
  --thresholds .agents/skills/recomp2jam-av-compare/references/default-thresholds.json \
  --strict
```

For several scenes, use `scripts/batch_compare_av.py --strict` and the schema
in `references/av-batch-manifest.md`. Keep manifests and reports with
proprietary captures outside the repository.

Run the helper regression tests after changing the scripts:

```bash
python3 -m unittest discover \
  -s .agents/skills/recomp2jam-av-compare/scripts -p 'test_*.py' -v
```

Current script limit: both inputs need an audio stream. `--strict` fails
missing enabled metrics but exempts VMAF explicitly disabled by `--no-vmaf`.
Extend the scripts with synthetic tests if video-only comparison is needed.

## Interpret the result

1. Review SSIM/PSNR and optional VMAF per scene and region. Inspect aligned
   frames behind outliers; a single mean can hide a small but critical HUD or
   note error.
2. Compare integrated loudness and true peak for audio level differences.
   Measure timeline offset, drift, missing sounds, and note judgement timing
   separately; EBU R128 does not measure rhythm parity.
3. Treat `references/default-thresholds.json` as an example starting point,
   not an O2Jam parity specification. Derive scene thresholds from reviewed
   baselines and record why each tolerance is acceptable.
4. Classify residuals as implementation gap, evidence gap, capture fault,
   scenario/vintage delta, or approved presentation choice. Preserve the
   existing `docs/parity.md` verdict until its evidence requirements are met.
5. Save the summary, threshold decision, aligned excerpts, and top mismatch
   locations together so another engineer can reproduce the review.

A comparison is complete when alignment is verified, required metrics are
present, every threshold has a rationale, and each reported mismatch links to
inspectable evidence.
