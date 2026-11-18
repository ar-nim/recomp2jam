# Ticket 242 — Pair-2 (live seat) qualification

Parent: `#233` (unified live rig kit). This ticket blesses a procedure; the
live seat never feeds a budget number. The release number stands on the
Xvfb pair (`docs/captures/t76/release-n203.md`).

## Criterion status (2026-11-18)

| Criterion from the issue | Verdict | Evidence in this leg |
| --- | --- | --- |
| Seat procedure proven by dry-run (driver commands planned, no pointer moved); native baseline drive committed with manifests that verify clean | MET | `seat-dry-run.txt`: `rig-seat-probe.sh` exit 0, planned ydotool commands, pointer `x=0 y=0` before and after. Native baseline committed in `docs/captures/t76/release-n203/`, verifier green. |
| Live supervised run on an unlocked seat; same scripted drive; judged distribution compared against native N=203 release run per spec rule; manifest plus comparison committed | NOT MET — OPEN | No owner-present supervised session was granted in this leg. `rig-seat-probe.sh` reporting `seat ready` is not owner presence or grant scope. `comparison.md` is an explicit NOT RUN template, not a fabricated result. |
| Both manifests verify clean, including cross-pair duplicate-screenshot check | NOT MET — OPEN | Only the native manifest exists. The cross-pair command needs a real seat pack; running it on a copied pack is what caused the earlier retraction. Recipe and refusal are in `comparison.md`. |

## What is ready for the supervised session

- The release native baseline is the qualified N=203 pack from #241: 7
  lanes × 29 rounds at 800 ms, min-offset stats, full audio/display
  containment, all five AFK checks green. Its text record and hash pins
  are committed; the local pack remains git-ignored.
- The ydotool dry-run proves the driver command shape without touching
  input. The planned live drive reuses the native scripted trace, so the
  stimulus stays identical across pairs (`#233` spec rule).
- The procedure in `seat-dry-run.txt` includes announce, unlock check,
  act, stop by PID, pointer restore, report, and abort conditions. It
  follows the two-path rules: the seat is owner path, and no seat figure
  enters the budget.

## Why no live run happened

The earlier #242 attempt was retracted for fabricated seat captures:
byte-identical screenshots copied from the native pack. The issue notes and
four owner comments require an owner-present supervised unlocked session.
This leg has no owner grant and runs under a no-tracker-writes rule, so it
cannot announce a seat session or obtain an explicit grant scope. A
background agent moving a pointer on the owner's compositor would violate
the two-path architecture and the seat-safety rule, even though the probe
currently says the display is unlocked. No pointer, key, or window was
touched; the dry-run output and pointer reads prove that.

## Next evidence step

The owner calls a supervised session, announces it, confirms the seat is
free and unlocked, and grants a scope. Then run step 3 of
`seat-dry-run.txt`, package the seat pack, run the cross-pair verifier, and
replace the OPEN rows in `comparison.md` with values from the two packs.
Until then this ticket stays open and ready-for-human.
