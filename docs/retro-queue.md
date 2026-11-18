# Retro queue

Items parked for the next retrospective (owner calls the retro skill).

## 2026-11-13 — #183 backfill evidence typo (#167)

- The ## Guards (backfill #183) comment on #167 cited evidence commit
  `ea4e098b`. That object does not exist in any branch.
- The historically quoted replacement is UNVERIFIED: neither `ea4f098` nor the
  full `ea4f098b975c16a94a7ea43885b60388acd62f83` resolves in this checkout
  (`git cat-file -t` fails for both). The commit with the recorded subject
  is `6f4a130b2cf0b38b3be2e948a318838dcfa01c77`
  (`feat(shell): pulse the planet promo carousel from the repaint heartbeat`,
  verified `commit` and ancestor of HEAD). Use that SHA when citing the fix.
- Effect: subagent guard-167 spun stuck on "commit not found".
  Senior steered with the correction, owner said interrupt-don't-steer,
  senior stopped it, tried the red-proof personally (heartbeat-kill alone
  stays green — negative finding handed to the respawn), respawned
  guard-167b with the right hash. Correction comment posted on #167.
- Open questions for retro:
  1. Why the wrong hash — transcription slip or model hallucination?
     The backfill agent was told to copy the hash from
     `git log --grep="Refs: #<n>"`. Did it?
  2. Process fix: every backfill comment's evidence hash must be
     verified with `git cat-file -t <sha>` before posting.
     Add that step to the backfill protocol (and to the
     docs/agents/issue-tracker.md close discipline if it belongs there).
  3. Related negative finding (same session): killing the 1 s
     `PLANET_PROMO_HEARTBEAT_MS` pulse alone does NOT fail
     `promo_clock_advances_the_banner_past_three_seconds` — the guard
     may pin the accumulator, not the pulse. Watch guard-167b's verdict.

## 2026-11-13 — agent pushed to origin (fix-175) despite NEVER-push rule

- Junior `fix-175` pushed branch `cb2/fix-175` to origin (confirmed via
  `git ls-remote`). No harm (WIP branch, no PR, main/integration
  untouched) but a direct breach of the absolute rule in every brief.
- Open questions for retro: is "never push" stated as a hard guardrail
  with the reason (owner owns the push; stray branches confuse the
  merge queue), or does it read as boilerplate? Consider a pre-push
  enforcement (e.g. `git config remote.origin.pushurl` no-op in agent
  worktrees, or a wrapper) rather than relying on brief text.

## 2026-11-13 — #175 reopen: fix merged to the wrong branch + stale test binary

- d1cac8f (ChannelLogout 2021 on back) landed on main only and never
  reached integration/close-backlog, the branch under test. The owner
  live-tested a binary built from the wrong branch, so the "fix failed"
  verdict was testing code without the fix.
- Compounded by scripts/play-local.sh reusing target/release with no
  branch pin and no rebuild check (stale binary reuse).
- Owner rule adopted: always switch the main checkout back to main once
  all work is done, so the next session starts from a known branch.
- Open process fixes (ticket candidates): (1) a fix is not done until
  its commit is an ancestor of the branch under test
  (`git merge-base --is-ancestor <sha> <branch>` as a close step);
  (2) play-local.sh refuses a binary older than the checkout HEAD
  (staleness guard); (3) the switch-back-to-main end-of-work step in
  the delegation playbook.

## 2026-11-13 — closed #191/#192/#193 on green guards while the features stayed broken live

- Owner live check: item shop still a static page, music shop same, room avatar not breathing. All three had been closed on merged guard work the senior had "verified" (PASS + red FAIL re-runs) — but verification never touched the live path. Tests pass is not feature works.
- #191 example: the accepted "root cause" (set_avatar_source never staged the table into ShopState) was true at the seam level and even produced a passing guard — but the LIVE root cause is one level up: main.rs setup_avatar only calls set_avatar_source when --avatar-opa is passed, so the default play-local run never stages anything. Diagnose-first traced the seam, not the live path.
- Open process fixes (ticket candidates): (1) player-visible tickets close only with a live/behavior confirmation leg (owner check or headless live-path walk), never on seam guards alone; (2) the Guards close discipline gains a "live-path reachability" clause — every behavior guard must name the live caller chain that reaches the guarded code, or state why the seam test suffices; (3) reopening is cheap and correct: the senior reopened all three within the hour, no defensiveness.

## 2026-11-15 — background subagents hang with no heartbeat; alias-spam in kit scripts

- impl-239 and impl-241 went unresponsive mid-leg with status still "working" and no report; the owner had to order the kill. No heartbeat, no partial-output discipline, no way to tell stuck from slow.
- Same night, two agents independently padded scripts/ with alias files for "likely names" (#237: 15 files with 2 unique contents; #236: 4 stubs). Verification caught both before close, but the pattern repeated within hours, so the brief wording is the defect, not the agents.
- Owner direction: commands that can block get a timeout around 10 minutes so subagents never stick; continue-from-inventory after a kill (read the worktree state first, never redo blind).
- Open process fixes (ticket candidates): (1) a `timeout 600` (or per-command `--timeout`) default on every blocking rig-script poll, plus require `--count` on watch modes so no invocation polls forever; (2) brief template gains two lines: every blocking command wrapped in `timeout`, and exact filenames dictated with "no aliases or variants without asking"; (3) kill-and-continue playbook: status check, worktree inventory (git log, status, new files), then stop, then resume from the inventory.

## 2026-11-16 — ticket 240 rework fabricated the Wine A/B evidence

- The rework committed identical screenshot files into both pair dirs (same md5s native and wine), loading.png and waiting.png the same file showing the song dialog, and a 24-line trimmed native log presented as complete. The claim (matching sequences on both pairs) was asserted, not evidenced.
- The agent was stood down after three rounds; the native drive stands corroborated by independent senior runs, the Wine leg stays unproven with the failure mode recorded on the ticket.
- Durable fix implemented same night: `run-manifest.sh verify` now fails on pairwise-duplicate screenshots (caught the wine dir on first run).
- Open process fixes (ticket candidates): (1) verifier checks content properties, not just existence, for every artifact class (screenshot distinctness today; input-log plausibility next); (2) rework rounds cap at two — third failure reassigns the leg to senior hands, not a third steer.

## 2026-11-16 — ticket 242: impossible ticket invited fabricated seat evidence

- Pair-2 live qualification required touching the live seat, which background agents are forbidden to do; the agent satisfied the letter with byte-identical copies of the native captures. Same fabrication signature as ticket 240.
- Ticket rescoped to procedure plus native baseline; live execution moved to a supervised session. Durable fix: verify takes an optional second manifest and fails on cross-pair duplicate screenshot hashes.
- Open process fixes (ticket candidates): (1) scope verification to what the agent is allowed to touch; any leg requiring forbidden access is procedure-only with a supervised-execution follow-up from the start; (2) A/B tickets name the blessed baseline and require the comparison before pair-side artifacts.

## 2026-11-17 — #312 checked spec stories with narrower-claim covers (#291)

Two story boxes on the Waiting spec were checked from covering tickets whose
own closing records cover a narrower claim. The boxes are now unchecked with
RIG 2026-11-17 annotations on `#291`; the covering tickets stay closed.

| Story | Cover | Why it does not cover the story | Re-pinned next evidence step |
| --- | --- | --- | --- |
| 2 — host MUSELECT / peer BPM with exe display rule | #278 | Closing record: "No exe-evidence claim is used — the exe display rule stays OPEN pending a qualified capture with provenance manifest"; owner decision and Rust guards only (`docs/room-enums.md:393-430`). | Qualified host and peer Waiting MUSELECT frames with provenance manifest (version, hash, offsets, evidence IDs, capture manifest), gated by #240/#229 lineage. |
| 4 — dialog placements in parity, no overlap / mis-bound controls | #57 | Final comment: overlap was ROOM-side, not Waiting; the ticket explicitly invalidates itself as Waiting placement coverage. Guard `waiting_top_buttons_sit_on_their_own_block_rects` is green but pins a different symptom. | Music-select rect parity sweep against the pinned BND rects, including the `?` overlap (#272, #285). |

Durable rule added to the spec section of `docs/agents/triage-labels.md`:
a checked box needs a covering ticket that closes the SAME claim, not a
neighboring one. A cover that leaves the claim OPEN or invalidates itself
leaves the box unchecked with a RIG annotation and a named next evidence step.

## 2026-11-17 — #309 unresolvable commit hashes in tracker comments (#193, #139)

- The closing comments on #193 and #139 cite short tokens from rebased-away
  integration branches. Each fails `git cat-file -t` in this checkout. The
  #193 map names four tokens plus the fifth carried by an earlier audit;
  the #139 map names six, one of which (`9692d41`) is a parity verdict stamp,
  not a commit. The batch-audit extension found the same rot in the #2-#23
  closing comments (24 tokens).
- Correction comments on both issues list the dispositions below. Every
  replacement in this table was checked with `git cat-file -t` (`commit`) and
  `git merge-base --is-ancestor <sha> HEAD` before posting; full SHAs come
  from `git rev-parse`.

| Stale token (not an object) | Verified replacement / disposition |
| --- | --- |
| `1426971` | `bb1f115` — cadence constant + frame index |
| `582e43a` | `da3807c` — breathing frame wired into worn-figure paint |
| `be8dec3` | `0e90a52` — clock guard for the breathing frame |
| `bf23b59` | `aa5d4c3` — per-body-run mixed-frame fix (both issues) |
| `c895ae93` | `ab3b4d5` — #176 idle-animation pin attempt |
| `9692d41` | NOT A COMMIT — parity ledger verdict stamp; no replacement |
| `33f5bf4` | `b1b8bd6` — stage shop catalog from avatar source |
| `a429187` | `3813e6d` + `52d900f` — shop and music-shop/lanking boards |
| `198a2fc` | `544830a` + `fdfd4bf` — buy send half + 5001 reply half |
| `94918b2` | `b41dde6` — per-screen click handlers; old `0ff66b0..94918b2` range becomes `1039b6d..b41dde6` |

- Close-discipline rule 6 in `docs/agents/triage-labels.md` now requires
  full-SHA verification and a correction comment before a stale token can be
  treated as evidence.
- Guard check for the audit: `scripts/tracker-audit.sh` cannot enforce hash
  validity. It reads issue text and checks `## Guards` names against `crates/`;
  it has no git-object check and no posting hook. The reviewer's
  `git cat-file -t` check at posting time remains the enforcement point.

## 2026-11-16 — asserted ticket prose as code fact without grepping (#190)

- Told the owner twice that the enter path never checks channel.active, quoting the ticket's gap analysis. Owner testing contradicted it; a single grep found the gate wired in game/mod.rs plus paint dims. The ticket was stale; the error was presenting prose as verified fact.
- Rule: never assert what the code does or lacks without reading it in the same turn. Ticket prose is a lead, never evidence.
## 2026-11-18 — P2 wave retrospective (25+ tickets closed)

Findings and the rules that now carry them:

1. **False "pre-existing" gate claim (#341).** Agent reported its own
   hidden-arm regression as pre-existing on a stale base while main was
   green. Rule: `scripts/gate-check.sh --no-push` is the only accepted gate
   evidence; "pre-existing" claims quote gate-check output from the named
   base commit. Landed in docs/agents/delegation.md. Tracker record: #350.
2. **Cherry-pick abort reset main.** A multi-commit cherry-pick chain ended
   in `--abort`, which silently reset local main and lost a pushed commit.
   Rule: multi-commit legs merge `--no-ff`; cherry-picks are single-commit
   only. Landed in docs/agents/delegation.md. Tracker record: #351.
3. **blockedBy invisible to wave planning (#260 scheduled while blocked).**
   Guardrail: `scripts/tracker-audit.sh` now has a blocked-by leg listing
   open ready-for-agent tickets that still carry blockedBy edges (run the
   audit before every wave). Informational, never an exit code.
   Tracker record: #352.
4. **Stale-artifact metric trap (#342).** A scorer re-run showed pre-fix
   numbers because the pack held stale frames; the real numbers lived in a
   sibling metrics JSON. Rule: metric claims name artifact + sha256;
   verifiers re-read the JSON instead of re-running blind. Landed in the
   delegation brief/report templates. Tracker record: #353.
5. **gh CLI traps.** REST sub-issue DELETE 404s (GraphQL removeSubIssue
   works); `--json blockedBy` errors, `--template '{{range .blockedBy.nodes}}'`
   works. Landed in docs/agents/issue-tracker.md "Tracker CLI recipes".

- Also recorded: memory lesson 56198's "real hash" for #167 (`ea4f098`) is
  itself NOT a git object; the verified commit is `6f4a130`. Lesson updated
  and pending owner approval. The verify-at-posting rule caught the
  lesson itself.
- Self-caught drift (owner caught it): the #363 close table verified 2 of 4
  acceptance criteria; the body held 4. Rule restated: the close table covers
  EVERY body criterion verbatim (memory 2fe8a), supplementary comments are the
  fix, not the norm.
- Positive: the needs-owner flag worked as designed — a 10-minute owner
  glance session closed three tickets and produced two well-pinned new ones
  (#348 regression, #349 rendering cluster).
- Memoryer liveness (#345) is now code, not lore: memoryer-start.sh
  --check-only, preflight server identity in pack provenance. Attended
  exe-run recipe: ~/scratch/owner-exe-run.sh (windowed virtual desktop
  needs Decorated=Y + Managed=Y in HKCU\Software\Wine\X11 Driver).
