# Delegation playbook: senior and junior

How the orchestrator (senior) delegates work to subagents (junior)
in this repo, and the review bar the senior applies to their output.
Read this with `docs/agents/issue-tracker.md` (brief template, merge
protocol) and `docs/agents/triage-labels.md`.

## Roles

- **Senior**: owns the plan, the evidence, the gates, and the close.
  Spawns juniors, reviews their diffs, re-runs their gates, merges,
  and pushes always: merged work is pushed immediately, never
  accumulated unpushed (owner decision 2026-11-16).
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

## Scoped tracker-write grants

Some programs let a junior write to the tracker (for example spec bodies).
The senior puts the grant in the brief; every write outside it stays senior work.

- **Scope statement:** name the exact objects up front: `Tracker grant scope
  only: bodies of #178/#179; no comments, labels, closes, new issues, or
  pushes`. Repeat the line in the report's tracker section.
- **One edit per object:** fetch the body, build the new body as an additive
  change, then run one `gh issue edit <n> --body-file <file>` per object. Keep
  existing text byte-for-byte; a second edit to one object needs a stated
  reason.
- **Read-back:** after each edit, run `gh issue view <n> --json body --jq .body`
  and compare the original prefix and checklist counts (GitHub adds one
  trailing LF). Record byte counts and sha256 in the findings. The write is
  done only when read-back matches.
- **Resume dedup:** on re-entry, list prior writes from the findings and the
  tracker before touching an object, then skip objects already written. One
  create-script resume re-ran its queue and duplicated 36 tickets (#431-#466);
  each duplicate needed a close comment and a `to-remove` label, with the lower
  number in each pair kept as the keeper. One listing at resume would have
  caught it. Create new issues one at a time with `scripts/issue-create.sh`
  and read back each number.

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
- **Delivery check:** after ANY stop, interrupt, or send, run a `status` check
  before relying on the steer. A send only queues text; it does not prove the
  junior consumed it. Confirm the agent is in the state the correction needs
  (working with the steer applied, or freshly respawned with the corrected
  premise) and quote that status output in the report.
- **Time-critical corrections get a respawn, not a queue.** Stop the agent and
  spawn a fresh one carrying the full correction; a queued follow-up in a
  stopped or finished agent has no consumer. Check status after the stop and
  after the spawn.
- Retro 2026-11-19: the owner's 640x480 desktop-size correction and later
  resume steers both sat as queued follow-ups in stopped or finished agents.
  Once the owner had to point out the agent would never receive the steer.
  The senior assumed delivery and never checked status.

1. **Two-round rework cap.** A leg that fails verification twice is
   reassigned to senior hands; no third steer. Retro #240: three steer
   rounds before the agent stood down after byte-identical captures and a
   trimmed input log; the ticket stayed open for a fresh senior-owned leg.
   Count rounds with a recorded verification verdict, not delivery checks or
   status pokes.

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

## Assumption challenge (owner decision 2026-11-20, #493)

When two technical legs fail on the same player-visible blocker, stop
spawning legs and challenge the SCENARIO assumptions the legs depend on,
not the technology. The #327/#328/DRI3 wedge burned three legs; the answer
was one owner sentence ("just play solo") attacking an assumption no
technical leg could question.

The challenge is a non-blocking menu, never a pause: one message naming the
assumptions ("my assumptions: A, B, C — correct me") as a tracker comment
or chat note. Work continues on legs that do not depend on those
assumptions; the owner answers in their own time. Legs are counted, not
hypotheses — parallel legs on the same blocker count toward the two.

## Thinking escalation

- Start every junior at **medium** thinking.
- Escalate to a higher thinking level only on a miss: a failed gate, a
  wrong result, or a review rejection.
- Do not start high by default; reserve the escalation for the case
  that needs it, and record the trigger in the report.

## Timeouts and filenames

- Every brief wraps blocking commands in `timeout` (ten minutes
  unless the ticket says otherwise), and every script poll ships
  with a default cap, so a stuck run dies loudly instead of
  hanging a subagent with status "working" and no report.
- Every brief dictates exact filenames and adds "no aliases or
  variants without asking": two agents in one night padded
  scripts/ with duplicate-named files, and verification caught
  both. Naming is senior business.
- Reuse merged kit code by dependency, never by copying a file across
  tickets: #240 vendored #239's drive script verbatim, and the fork now
  rots in two places.
- Every brief quotes the documented recipe command for the exact leg
  before adapting it, and routes all runs through kit scripts (never
  bare sleeps, pattern-kills, or hand-rolled display handling).

## Verify, don't trust

The senior re-runs **every** PASS/FAIL line a junior reports.

The senior also verifies every acceptance criterion before a merge or close;
the authoritative rule and its table live in
`docs/agents/issue-tracker.md` ("Acceptance-criteria review before any
close") and close-discipline rule 2 in `docs/agents/triage-labels.md`.

- Re-run the gates via `scripts/gate-check.sh` (fmt, clippy, both
  test arms, push, origin-verify) on the junior's commit, not on the
  junior's word. Equivalent manual form: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (present + hidden install-root arms).
- Re-run any named guard or test the junior claims green.
- Pixel verification: no screenshot, frame, or log excerpt is cited until
  the reviewer has opened that exact artifact with their own eyes. Compare
  the opened artifacts with each other as well: the fabricated #240/#242
  capture pairs had matching md5s and a loading PNG reused for Waiting, which
  an opened side-by-side view exposes before close. A path, hash, or
  second-hand description is not a view.
- A junior's "PASS" is a claim, not evidence, until the senior
  reproduces it.
- If a reported PASS does not reproduce, treat it as a FAIL and
  respawn or steer per the rule above.
- Gates: `scripts/gate-check.sh --no-push` output is the only accepted gate
  evidence (retro 2026-11-18). A "pre-existing failure" claim must quote
  gate-check output re-run FROM THE NAMED BASE COMMIT — the #341 agent
  reported its own hidden-arm regression as "pre-existing on d191bb8" while
  main was green.
- Never push from an agent worktree: the orchestrator is the only pusher.
  `scripts/worktree-setup.sh` installs a worktree-local pre-push hook as a
  reminder (`--no-verify` bypasses it); it never edits shared `remote.*` or
  `protocol.*` config (issue #488). `scripts/tracker-audit.sh` fails with exit
  1 on shared push-URL drift and lists remote-only `cb2/*` branches. The
  reviewer enforces the rule on every leg.
- Multi-commit legs arrive as branches; the senior merges `--no-ff`. Reserve
  cherry-picks for single commits — a 2026-11-18 cherry-pick abort silently
  reset local main and lost a pushed commit; the merge was needed anyway.

Close handoff rules (retro #175; rule 3 from retro #922):

1. **Ancestor check before close.** A fix is not done until
   `git merge-base --is-ancestor <fix-sha> <branch-under-test>` passes; quote
   the command and its rc=0 output in the close record. The #175 fix
   (d1cac8f) landed on main only and never reached the branch under test, so
   the owner live-tested a binary without the fix and got a false
   "fix failed".
2. **End-of-work switch-back.** The mandatory final step of every delegation
   flow returns the main checkout to `main`; confirm with
   `git -C /home/arnim/Projects/recomp2jam branch --show-current`. Owner rule
   2026-11-13 (#175): the next session starts from a known branch.
3. **Mechanical cited-hash check before any close comment.** Before every
   `gh issue comment` that closes an issue or posts a close table, write the
   comment to a file and run `scripts/check-cited-hashes.sh <file>`. Exit 0
   is required; any `STALE` token blocks the close until it is replaced with
   a verified SHA or explicitly marked UNVERIFIED. Record the command and
   its exit code in the close table. This enforces the prose rule in
   `docs/agents/triage-labels.md` rule 6.

## Senior completeness pre-flight

Run this list immediately before posting any close, retro, or merge note.
Gates and criterion verification come first; this list checks the write-up.

1. Count the acceptance criteria in the issue body and the rows in the close
   table. Different counts stop the post: add a row for every criterion.
2. Confirm a tracker record exists for the close or retro (issue created and
   linked, or the exception stated in the post).
3. Post only after both checks pass.

Retro 2026-11-19: the #363 close table verified 2 of 4 body criteria
(`docs/retro-queue.md:172-173`), and a retro post had no tracker record at
all. The owner caught both, not the senior. An automated tracker-audit leg is
the reduce-needs-owner research's first deliverable; until it lands, this
manual list stays mandatory. Close #495 when either version lands, noting
which.
## Resume discipline

A resumed agent re-lists its own prior creates before any tracker write:
`gh issue list --author @me --state all --limit 100 --json number,title,state`.
An existing ticket is a result, not another create (#487: 36 duplicates from a
resumed recon-c create step). New tickets go only through
`scripts/issue-create.sh`, which rejects duplicate open titles.

## Brief requirements

Each brief follows `docs/agents/issue-tracker.md` (implementer brief
template) and adds:

- The thinking level (start medium).
- The skip budget remaining for this wave (default 3).
- Exemplar references when the item is part of a batch.
- Implementer address: address the subagent as `implementer` (never `junior`)
  throughout the brief and its report.

## Brief and report templates

Copy these shapes verbatim. `docs/agents/issue-tracker.md` (implementer brief
template) stays authoritative for tracker and window-seam rules; the templates
below make their shape a single source instead of a per-spawn rewrite.

### Brief template

```text
Ticket: #<number> — <title> (one ticket, one leg)
Worktree: <path> | Branch: <name>
Pre-flight: git rev-parse --show-toplevel == <path>
Base SHA: <full 40-hex SHA>; git cat-file -t <sha> prints commit
Acceptance: <paste every criterion verbatim; no paraphrase>
Deliverables: <exact filenames>; no aliases or variants without asking
Evidence: <paths / evidence ids + sha256 for metrics artifacts>; reviewer
opens each artifact (pixel rule) and re-reads metrics JSONs rather than
re-running scorers blind (stale packs can hold pre-fix frames)
Gates: `bash scripts/gate-check.sh --no-push` (the only accepted evidence;
both install-root arms); wrap any ad-hoc run in timeout 600
Both arms: any lane whose guards touch data-gated behavior must run BOTH gate arms (install-root present AND hidden); a data-less worktree alone silently skips the guards (random-default incident)
Cited hashes: write each close comment to a file and run `scripts/check-cited-hashes.sh <file>` before posting; a stale token blocks the close
Merge: run the named guards before composing any conflict resolution in
guarded code (guard-first rule in "Merge-conflict discipline"); push only
after gate-check rc=0 is observed in the same output
Thinking: medium | Skips left: 3 | Exemplars: <verified SHAs or doc entries>
Never push; never comment on or close issues. Report: the template below.
```

### Report template

```text
## Answer
<answer first: what changed and why, in one or two sentences>

## Branch and commit
Branch: <name>
Base SHA: <full SHA copied from the brief>
Commit SHA: <full SHA printed by git rev-parse HEAD>
Verification: <paste git rev-parse HEAD and git cat-file -t <sha> output>

## Files
- <exact path>:<line> — <change>
<every touched file; `git status` clean of unintended files; no aliases>

## Acceptance criteria
| Criterion (verbatim) | MET / NOT MET / OPEN | Command + output or file:line |

## Gates
- <exact command> — exit <n>
- Both arms (install-root present AND hidden), when guards touch data-gated behavior; a data-less worktree alone silently skips the guards (random-default incident)
- `scripts/check-cited-hashes.sh <close-comment-file>` — exit 0, before every close comment
<PASS and FAIL lines for every named guard; no bare PASS>

## Skips
<none, or each skip with its named sources>

## Reproduce
1. <command>
2. <command>

## Evidence opened
<exact artifacts the reviewer opened, or none — docs-only>
```

A report with a short SHA, a SHA from memory, or a renamed file is rejected
on sight: two 2026-11-16 reports transposed SHAs, and a full `git rev-parse`
output plus exact filenames is what makes the transposition visible.

## Merge-conflict discipline

Retro 2026-11-12: mid-conflict mistakes cost a session. When a merge
conflicts:


- Copy both sides verbatim into the resolution before editing. Never
  reconstruct a side from memory.
- Diff each hunk against both parents before staging, one hunk at a
  time.
- Stage only the conflicted paths and regenerated files. Never
  `git add -A` — the working tree may hold someone else's live edits.
- No `git stash` mid-conflict. Stash hides the conflict state and loses
  context; resolve in place, or abort and restart.
- Run `cargo check --workspace` before committing the merge; full gates
  after.

Guarded conflicts and pushes (retro 2026-11-20, paint_channel):

1. **Guard-first resolution.** When a conflict touches guarded code, run
   the named guards on each parent before composing the resolution, then
   re-run them on the resolution before staging. A resolution never adds
   unpinned behavior: unevidenced cases stay OPEN. Worked example:
   `e54e41ddc3868959ee77770e50ba6ccaabf35c60` removed a speculative third
   arm from the channel-waiting-2 merge; that arm flipped the #348
   differential guard's empty frame to equal the held frame. The unpinned
   drained-edge case stayed OPEN.
2. **Push only after observed green.** Push only when `gate-check` rc=0 is
   observed in the same output; never `;`-chain a push after the gates. If a
   red merge reaches origin, fix-forward immediately — no force, no history
   rewrite. The paint_channel merge was pushed after `gate=101`; e54e41d was
   the fix-forward.

The issue-tracker copy is a pointer; this section is canonical.

## Standard merge path (issue #918)

`scripts/merge-lane.sh <branch>` is the standard senior merge path, run from
the main checkout. It refuses a linked worktree, a dirty tree, a missing or
already-merged branch, or a checkout not on `main`; merges with `--no-ff`;
runs `scripts/gate-check.sh --arm full` with `GATE_CHECK_NO_PUSH=1`; and
resets local main to the pre-merge SHA if any red step occurs before the
push. On green it pushes `main`, verifies `main == origin/main`, prints the
merge commit, releases the lane worktree, and deletes the merged branch.
`--no-push` leaves a green merge local; `--keep-worktree` leaves the lane
worktree in place. The script orchestrates; gate-check.sh owns every check.

A manual merge plus push is the exception, not the default: it needs a named
reason in the close report (for example, a conflict that needs hand
resolution). Pushing without an observed full-gate rc=0 stays forbidden.

## Land-or-rebase timing pointer

Land (merge) only after the full gates
are green on the implementer's commit; rebase onto the current base
before starting when behind. Do not let work sit unlanded across a
wave. If a conflict appears mid-land, follow the merge-conflict
discipline above; never `git stash`.

## Audit

A delegation is healthy when: no skip lacks named sources, no wave
exceeds 3 skips, every batch had 2 exemplars posted before delegation,
every reported PASS/FAIL was re-run by the senior, every cited artifact
was opened by the reviewer before citation, and every respawn
was triggered by a poisoned premise (not by a narrow correction).

## Standard brief lines (retro batch 2026-11-22)

Every implementer/rig brief carries, verbatim:

- "Self-verify before done: render-over-time via `paint_at` (frames must
  change where the behavior manifests), audio-over-time via the backend
  cue index, and the live input path exercised — seam-green alone never
  closes. See docs/rig-runbook.md § Self-verify." (owner principle #870)
- "Archive-gated tests must RUN, not skip, in your worktree: use
  `INSTALL_DATA_POOL=` to claim a private data slot when the shared
  symlink is absent, and report the skip count in your final report."
  (lesson 0a77d)
- "Any lane whose guards touch data-gated behavior must run BOTH gate arms
  (install-root present AND hidden) — a data-less worktree alone silently
  skips the guards (the random-default incident)." (retro #922)
- "Write every close comment to a file and run
  `scripts/check-cited-hashes.sh <file>` before posting it; a stale token
  blocks the close." (retro #922)
- "If your fix touches a spec, `bash scripts/close-spec.sh <N>` must
  exit 0 before the close comment is written." (retro #869)
