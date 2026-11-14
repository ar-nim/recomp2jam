# Delegation playbook: senior and junior

How the orchestrator (senior) delegates work to subagents (junior)
in this repo, and the review bar the senior applies to their output.
Read this with `docs/agents/issue-tracker.md` (brief template, merge
protocol) and `docs/agents/triage-labels.md`.

## Roles

- **Senior**: owns the plan, the evidence, the gates, and the close.
  Spawns juniors, reviews their diffs, re-runs their gates, merges.
- **Junior**: works in one assigned worktree on one ticket. Reports
  done with `git status` clean of unintended files. Never pushes,
  never comments on or closes issues.

## Skip bound

A "skip" is any decision not to run, not to verify, or not to
implement something that the ticket or the gates imply.

- Every skip **names the sources checked** (file, test, command, or
  evidence id) that justified skipping. A bare "skipped" is not
  acceptable.
- **Max 3 skips per wave.** A wave is one delegated batch (one set of
  parallel juniors or one senior pass). After 3 skips, stop and report
  to the senior instead of continuing.
- Skips are recorded in the junior's report and re-checked by the
  senior during review.

## Steer vs respawn

- **Steer** for narrow corrections: a typo, a wrong constant, a missed
  import, a misnamed symbol. Send the correction into the existing
  junior; the premise of the work is sound.
- **Respawn** when the premise is poisoned: wrong commit hash, wrong
  assumption about the exe, a fundamentally flawed approach, or a
  correction that would force the junior to redo substantial work.
  Stop the junior (interrupt, do not steer), then spawn a fresh one
  with the corrected premise. A poisoned premise does not recover by
  steering.
- Rule of thumb: if the correction changes what to do, respawn; if it
  changes only how, steer.

## Show two, then delegate

Before delegating a batch of similar items (guards, backfills, ports):

1. The senior completes (or has a junior complete) **two ground-truth
   exemplars** first and posts them where the rest of the batch can see
   them (commit, issue comment, or doc entry).
2. The exemplars pin the exact shape: naming, structure, evidence
   references, and the verification command.
3. Only then are the remaining items delegated, each brief quoting the
   exemplar references.

This prevents drift across a batch and gives the junior a concrete
template instead of a description.

## Thinking escalation

- Start every junior at **medium** thinking.
- Escalate to a higher thinking level only on a miss: a failed gate, a
  wrong result, or a review rejection.
- Do not start high by default; reserve the escalation for the case
  that needs it, and record the trigger in the report.

## Verify, don't trust

The senior re-runs **every** PASS/FAIL line a junior reports.

- Re-run the gates via `scripts/gate-check.sh` (fmt, clippy, both
  test arms, push, origin-verify) on the junior's commit, not on the
  junior's word. Equivalent manual form: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (present + hidden install-root arms).
- Re-run any named guard or test the junior claims green.
- A junior's "PASS" is a claim, not evidence, until the senior
  reproduces it.
- If a reported PASS does not reproduce, treat it as a FAIL and
  respawn or steer per the rule above.

## Brief requirements

Each brief follows `docs/agents/issue-tracker.md` (implementer brief
template) and adds:

- The thinking level (start medium).
- The skip budget remaining for this wave (default 3).
- Exemplar references when the item is part of a batch.

## Audit

A delegation is healthy when: no skip lacks named sources, no wave
exceeds 3 skips, every batch had 2 exemplars posted before delegation,
every reported PASS/FAIL was re-run by the senior, and every respawn
was triggered by a poisoned premise (not by a narrow correction).
