---
name: recomp2jam-regression-triage
description: Triage recomp2jam test failures and visual, audio, input, timing, network, or performance regressions against O2Jam 7.55 evidence. Use when a parity check breaks, a scene differs, CI and local results disagree, or root-cause investigation is needed.
---

# recomp2jam Regression Triage

Find the smallest evidence-backed cause. Distinguish a Rust regression from a
bad reference assumption, a capture fault, missing local data, or concurrent
work.

## Preserve and reproduce

1. Record the commit, branch, dirty files, command, environment, platform, and
   exact failure. Protect existing edits; do not stash, reset, or overwrite
   another worker's changes.
2. Read the relevant finding and current status in `docs/progress.md`,
   `docs/parity.md`, `docs/gameplay.md`, `docs/reversing.md`, or the format
   specification. Existing `OPEN` and `FAIL` labels are evidence boundaries.
3. Reproduce with the smallest synthetic scene, packet, chart, or input trace.
   Trim media to the shortest failing interval and replay only the needed
   events.
4. Check whether the failure requires `install-root/`, a live gateway, Wine, a
   display, or audio hardware. Missing optional data must skip cleanly; an
   absent required reference is not a pass.

## Classify and map

Classify the primary symptom:

- parser bounds, decrypt/decode, or asset lookup;
- game rule, judgement, hold press/release, score, combo, jam, pill, or life;
- scene transition, layout rect, sprite anchor, layering, color key, or text;
- audio clock, drift, missing sound, or output-device handling;
- input mapping, event order, focus, or injection;
- protocol framing, field order, server sequencing, or live-only behavior;
- frame pacing, loading, resource use, or platform integration;
- capture alignment, environment, stale artifact, or test-fixture drift.

Map the symptom to the owning crate in `docs/architecture.md`. Map original
behavior to 7.55 file offsets and REA evidence IDs in notes, using new words.
Do not paste disassembly or decompiled code into the repository or report.

## Test one hypothesis

1. Write the current observation, expected behavior, evidence source, and one
   root-cause hypothesis. List a discriminating check that can disprove it.
2. For an implementation defect, add a small scenario-named regression test
   that fails for the observed reason. Use synthetic fixtures by default.
3. Change one relevant behavior at a time. Keep platform adaptation behind the
   existing crate boundary; do not move OS calls into pure parsers or game
   logic to make a probe easier.
4. Re-run the minimal check, then the affected crate, then the project gates:

   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

5. Run real-data or live checks separately and report their actual status.
   Keep automated audio on a null or virtual output.

## Report

Return: verdict, minimal reproduction, root-cause evidence, changed files,
tests run, remaining uncertainty, and the next action. Update the relevant
finding or parity row when behavior evidence changed; do not rewrite a verdict
just because a screenshot or aggregate metric improved.

If the evidence disproves the old reference claim, correct the documentation
first and label the correction. If the cause remains unknown, preserve the
reproduction and state what evidence is still needed instead of guessing.
