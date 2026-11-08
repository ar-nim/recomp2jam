---
name: recomp2jam-batch-harness
description: Build reproducible recomp2jam validation batches across O2Jam scenes, Rust crates, and target platforms. Use for run manifests, nightly parity checks, artifact layouts, automated pass/fail gates, or orchestrating synthetic, real-data, Wine, and live-server tests.
---

# recomp2jam Batch Harness

Batch validation covers scenes, scenarios, and platforms for one O2Jam 7.55
project. It is not a multi-title console pipeline.

## Define the run matrix

1. Read `docs/architecture.md`, `docs/parity.md`, `docs/playtest.md`, and
   `docs/install-layout.md`. Select scenarios from current behavior and known
   gaps rather than inventing a separate acceptance model.
2. Give each run a stable ID and record scene, platform, build, input trace,
   reference capture, required local data, and expected verdict.
3. Use `references/manifest-schema.md` for the manifest shape. Use
   `references/pipeline-layout.md` to separate manifests, local inputs,
   captures, metrics, profiles, and reports.
4. Keep proprietary inputs and derived media under `$HOME/scratch` or another
   git-ignored root. Commit only synthetic fixtures and non-sensitive schemas.

## Order the pipeline

1. **Preflight:** verify the reference hash and tools when used; check display,
   audio sink, server, and port ownership; reject malformed manifests.
2. **Build and synthetic gates:** run formatting, clippy, and workspace tests.
   Keep live gateway credentials unset for the ordinary suite.
3. **Scenario run:** apply the selected input trace and capture state logs,
   media, and performance samples. Use isolated output directories.
4. **Comparison:** align reference/test media and apply explicit thresholds or
   exact behavioral assertions. Preserve known `FAIL` and `OPEN` residuals.
5. **Clean up:** stop only processes owned by the run and release its display,
   prefix, sink, and ports. Record cleanup failures without deleting evidence.
6. **Report:** write per-run status and a compact batch summary with links to
   raw artifacts and the top actionable mismatches.

## Make statuses honest

- Use `passed`, `failed`, `skipped`, and `needs_review` distinctly. A skip is
  never a pass, and a metric report is not automatically a parity verdict.
- Missing optional `install-root/` data must produce a clean skip in CI.
  Missing data required by an explicitly requested real-data run is a failed
  preflight.
- Keep live-server tests opt-in and report them separately from offline tests.
  Record Memoryer results as interoperability evidence, not proof of an
  unverified 7.55 client rule.
- Fail fast on manifest, build, or required-input errors. Continue independent
  scene comparisons when useful, but preserve every failure in the summary.
- Known differences need an explicit reviewed expectation and rationale. Do
  not let a manual override erase the original evidence.

## Isolate parallel work

Bound CPU, memory, and I/O. Give each Wine run its own prefix, display, scratch
copy, and virtual/null audio path. Give each live run explicit server and port
ownership. Never route test audio to the user's speakers or touch the user's
desktop. Avoid parallel writers to the same captures or reports.

## Completion

A batch is complete when every manifest row has a final status, all required
artifacts and hashes exist, thresholds and skip reasons are visible, and the
summary can be reproduced from the recorded build and tool versions. Finish
with the project gates when repository code or tests changed.
