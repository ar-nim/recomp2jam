---
name: recomp2jam-scope-plan
description: Plan or re-scope recomp2jam behavior-parity work for the O2Jam 7.55 Windows client and its cross-platform Rust reimplementation. Use when choosing an RE target, defining acceptance criteria, building a validation matrix, planning a scene or subsystem, or rebooting the project roadmap.
---

# recomp2jam Scope Plan

Define a reviewable behavior-parity slice. The target is the O2Jam 7.55
Windows client, not a Nintendo Switch title, an emulator, or a byte-identical
executable.

## Establish the current state

1. Read `AGENTS.md`, `docs/architecture.md`, and `docs/reversing.md`.
2. Read `docs/progress.md` for implementation status and `docs/parity.md`
   for scene verdicts. Consult `docs/handoff.md` when current environment or
   recent work matters.
3. Treat newer, evidence-backed findings as corrections to stale roadmap or
   overview text. Preserve existing user work and distinguish committed state
   from uncommitted state.

## Bound the slice

1. Name one user-visible behavior, file format, scene, protocol exchange, or
   performance question. Split unrelated goals into separate slices.
2. Pin the reference as O2Jam 7.55. For binary evidence, record the file name,
   size, hash, file offsets, and REA evidence IDs required by the project.
3. Identify the Rust crate boundary from `docs/architecture.md`. Keep game
   rules independent of Win32, Wine, winit, audio devices, and network I/O.
4. Mark launcher/patch behavior, server-computed rules, and unsupported side
   states as in scope or out of scope explicitly.
5. Ask which target platforms matter when that choice changes acceptance
   criteria. Do not claim that one Linux run proves every platform.

## Preserve the clean-room boundary

- Require a legally obtained, user-supplied 7.55 install for real-data work.
- Keep original executables, archives, music, artwork, captures, and traces
  containing them under local-only paths such as `install-root/`, `extracted/`,
  or `$HOME/scratch`.
- Put only synthetic fixtures and behavior descriptions in the repository.
- Describe findings in new words. File offsets and evidence IDs belong in
  notes; original disassembly and decompiled code do not.
- Treat Memoryer as a clean-room interoperability reference and corroboration.
  Treat player reports as hypotheses. The 7.55 executable remains the client
  behavior oracle.

## Build the validation matrix

Use one row per observable behavior and these columns where applicable:

| Behavior | 7.55 evidence | Rust boundary | Synthetic check | Real-data or live check | Residual |
|---|---|---|---|---|---|
| Named scenario | Doc, offset, evidence ID | Crate/API | Assertion and tolerance | Command, capture, or skip rule | OPEN/FAIL item |

Include the relevant dimensions: parsing bounds, state transitions, input
press/release, judgement and score, audio clock, rendering geometry, protocol
ordering, timing, and stability. Mark unknown cells `OPEN`; mark known gaps
`FAIL`. Reserve `PASS` for 7.55 evidence, an implementation, and a passing
test together. A successful harness run alone does not meet that bar.

## Set exit criteria

1. Give each criterion a stimulus, observable result, and explicit tolerance
   or exact expected value. Prefer the 7.55-derived value over an invented
   similarity threshold.
2. Require a synthetic regression test for implemented behavior. Real-data
   tests must skip cleanly when `install-root/` is absent; live tests must be
   opt-in and separately reported.
3. For captures, require verified alignment and scene-specific review. For
   performance, name the hardware, OS, build profile, and baseline.
4. Include the project gates: `cargo fmt --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`, and
   `cargo test --workspace`.
5. Keep reference audio contained to a virtual or null audio path during
   automated probes. Never route test audio to the user's speakers.

## Deliverable

Return or write the requested plan with: scope, current evidence, crate map,
validation matrix, artifact locations, skip rules, exit criteria, and the
smallest next evidence-gathering step. Do not implement an unverified rule
merely to make the plan look complete.
