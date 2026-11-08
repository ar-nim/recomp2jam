# recomp2jam Validation Artifact Layout

Keep this workspace under `$HOME/scratch` or another git-ignored root. Separate
inputs, raw evidence, derived metrics, and reports so a failed run cannot be
mistaken for a later successful run.

```text
workspace/
  manifests/
    waiting-linux-2026-11-08.json
  inputs/
    provenance/
      o2jam-7.55.toml
    traces/
      waiting-host-ready.json
    references/
      waiting-host-ready.mp4
  builds/
    linux-x86_64/
      logs/build.log
  runs/
    waiting-host-ready-linux/
      metadata.json
      logs/
        reference.log
        rust.log
        cleanup.log
      captures/
        reference-raw.mp4
        rust-raw.mp4
        reference-aligned.mp4
        rust-aligned.mp4
      metrics/
        ssim.log
        psnr.log
        vmaf.json
        ref_ebur128.log
        test_ebur128.log
      reports/
        summary.json
        pass_fail.json
      perf/
        frame_times.csv
        audio_clock.json
```

## Stages

1. **Preflight:** validate the manifest, reference identity, input availability,
   and tool versions. Allocate isolated displays, prefixes, sinks, and ports.
2. **Build:** record the commit and dirty state; run the requested Cargo gates.
3. **Run and capture:** apply the trace, capture raw logs/media, and save event
   and clock records. Keep audio contained.
4. **Compare:** produce aligned derivatives and metrics without overwriting raw
   evidence. Never threshold-check a summary from a failed comparison.
5. **Profile:** save raw frame, resource, loading, or audio-clock samples.
6. **Clean up:** stop only processes owned by the run, release allocated
   resources, and record cleanup failures. Preserve artifacts for triage.
7. **Report:** finalize each row as passed, failed, skipped, or needs_review;
   include skip reasons and links to residual evidence.
