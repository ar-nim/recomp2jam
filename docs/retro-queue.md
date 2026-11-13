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
