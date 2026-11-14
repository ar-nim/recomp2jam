# T223 batch 2 — exe-cadence evidence hunt (2026-11-14)

## Sources checked

- `docs/equip.md` read end to end (§"Worn-figure idle frames" through
  §"Avatar idle animation (ticket #176) — OPEN").
- Ticket #176 (`gh issue view 176 --comments`): full thread plus all four
  comment records (triage, round-1 OPEN report at `c895ae9`, absorption
  into #177, guard backfill #183).
- `~/scratch/` (591 entries): filename sweep for `dwell|breath|avatar|worn|176`
  → zero hits; deeper `find -maxdepth 2` for `*dwell*|*avatar*` → zero hits.
  Prior Wine sessions (`button-unblock/`, `planet-verify/findings.md`) cover
  dialog unblock and planet clicks only — no worn-figure scene captures exist.

## Verdict: exe cadence still OPEN, no new evidence

- Structure pinned: 12 chained breathing frames per worn body; generic
  sprite-frame machinery pinned (setter VA `0x41e0f0`, file `0x1e0f0`;
  frame index at control `+0x134`; frame table at `+0x120`, stride `0x14`).
- Timing OPEN: no ms/frame value, no driver (paint-clock accumulator vs
  timer tick). #176 round 1 booted the exe (`bright=4800`) but never
  completed the SendInput navigation into a worn-figure scene, so no
  frame-change timestamps were ever recorded.
- #176 is CLOSED as superseded (absorbed into the #177 progression spec);
  the evidence record in `docs/equip.md` carries forward unchanged.
  Nothing in this hunt contradicts it, so the OPEN stands, uninveted.

## Next capture that would pin it (named, not run)

Two parallel arms, either sufficient:

1. **Wine dwell capture** (cross-check): launch `OTwo.exe`
   (SHA `83a04541…72c823b`) via `scripts/rig-run.sh` on `:99`, null sink,
   local Memoryer; SendInput-drive login → Room with tester2's kitted row
   (face 35 + kit 16/30/376/383/389 per `docs/equip.md`); park 20 s on the
   room avatar region (bbox 485,50–726,255 per `docs/hover.md`) capturing at
   10 Hz; diff consecutive frames; derive ms/frame from change timestamps
   (expect a 12-cycle). Recipe: `recomp2jam-reference-capture` skill.
2. **winedbg stopped-state reads** (direct): break at the avatar paint,
   read sprite-control `+0x134` across successive stops; deltas give both
   cadence and driver. This answers the vtable-indirect case byte scans
   cannot (zero direct xrefs to `0x41e0f0`). Discipline per project memory:
   launch on `OTwo.exe` directly (never explorer), stopped-state reads only.

## Implication for #223

No exe pin exists to consume. Per the brief's hard rule (out of scope:
invented smoothing without an exe pin), the execution half must wait on
arm 1 or 2 above. See `03-recommendation.md` for the conditioned plan.
