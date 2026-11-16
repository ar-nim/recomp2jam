# Retro queue

Items parked for the next retrospective (owner calls the retro skill).

## 2026-11-13 — #183 backfill evidence typo (#167, ea4e098b vs ea4f098)

- The ## Guards (backfill #183) comment on #167 cited evidence commit
  `ea4e098b`. That object does not exist in any branch.
- The real fix commit is `ea4f098b975c16a94a7ea43885b60388acd62f83`
  (`feat(shell): pulse the planet promo carousel from the repaint heartbeat`).
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
