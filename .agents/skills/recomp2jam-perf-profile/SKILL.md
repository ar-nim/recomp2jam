---
name: recomp2jam-perf-profile
description: Profile recomp2jam frame pacing, load time, audio timing, CPU/GPU use, and memory across supported platforms. Use for Rust performance regressions, stutter, latency budgets, release-build benchmarks, or carefully bounded comparisons with the O2Jam 7.55 Wine client.
---

# recomp2jam Performance Profile

Measure a repeatable Rust scenario and attribute its costs. Wine resource use
is not a native-performance oracle for the original Windows client.

## Define the experiment

1. Name the scene, user-visible symptom, target platform, and Rust subsystem.
   Use `docs/architecture.md` to map work to file parsing, audio, game logic,
   rendering, shell, networking, or the client binary.
2. Pin hardware, OS, power state, build commit, dirty-tree state, compiler,
   features, and build profile. Use an optimized build for user-facing claims.
3. Reuse a deterministic scene and input trace. Pin song/chart, difficulty,
   speed, art set, resolution, audio settings, and network conditions.
4. Set separate targets for frame time, load time, audio clock behavior, input
   latency, and resource use. Do not infer correctness from faster execution.

## Capture metrics

- Record the full frame-time distribution: median, p95, p99, maximum, and
  missed presentation deadlines. Average FPS alone hides rhythm-game stutter.
- Measure stage durations for archive read/parse, sprite decode, OJN decrypt,
  Vorbis decode, scene staging, and first interactive frame when loading is in
  scope.
- Record audio callback underruns, sample-clock drift, and event-to-sound or
  event-to-judgement latency when applicable. Keep test audio on a null or
  virtual device.
- Capture CPU, GPU, memory, disk I/O, and network wait only where they help
  attribute the observed symptom. Mark shader compilation, cold cache, and
  warm cache runs explicitly.
- Save raw samples and tool versions. A summary without its underlying trace
  is difficult to reproduce.

## Compare fairly

1. Warm up and run repeated trials. Report variance and identical settings;
   investigate a one-off spike before treating it as a regression.
2. Compare native Rust against a previous native Rust baseline on the same
   platform. Maintain separate baselines for materially different OS or
   hardware targets.
3. Use the original under Wine only for observable budgets such as scene
   duration, frame pacing, or audio behavior, with capture overhead recorded.
   Do not require native Rust CPU/GPU totals to equal Wine totals.
4. Map spikes to a subsystem using targeted timings or a profile. Network wait,
   audio decode, game update, rendering, and presentation need separate clocks.

## Report and gate

Provide a table per scenario with baseline, candidate, delta, threshold,
sample count, and verdict. Link the raw profile and identify the suspected
subsystem and next discriminating measurement.

A performance claim is complete when it is reproducible on the named platform,
uses equivalent stimuli, and does not silently exchange behavior parity for
speed. Keep proprietary assets and captures outside the repository; CI-only
checks must use synthetic fixtures or skip cleanly when local data is absent.
