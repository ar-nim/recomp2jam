# Payment/Tutorial hand session — smoke note (2026-11-10)

Seam: `ScreenGame` directly (no window binding stages side
screens — the windowed binary and the shots walk reach sides only
through `visit_side`; the hand steps below rode
`visit_side`/`leave_side` in test
`payment_tutorial_hand_session_visits_both_and_returns`, null
audio, synthetic placements).
Trace `fixtures/input-trace/payment-tutorial-hand-session.json`
records the lobby holds in `recomp2jam-input-v1` (side overlays
keep the flow on Waiting, so the trace holds one `await-screen`
per overlay visit); the test executes the same sequence with
assertions.

## What was stepped, what happened

Waiting → `visit_side(Payment)` opens the billing overlay over
two synthetic staged rects. The overlay inks the unwired-billing
notice (`BILLING UNWIRED payment_input.aspx gift_input.aspx`)
into the largest staged rect and leaves the small rect blank
(guard `payment_side_paints_the_unwired_billing_notice`); with no
staged rects nothing paints (guard
`payment_side_without_staged_rects_paints_no_notice` — ADR-0003,
never a literal fallback rect). Entry queues nothing on any send
seam (equip/gift-claim/gift-list/create/join/music/team/ready all
`None` — guard `payment_entry_queues_no_wire_call`), so the run
stays silent and honest. `leave_side` returns to Waiting.
`visit_side(Tutorial)` opens the guided-play overlay (backdrop
only — no staged figures in the synthetic run, nothing to click);
`leave_side` returns to Waiting.

## Wire queued (take-seam — none)

No wire call exists behind either view: the exe opens web billing
views here (lifecycle §12, paths `payment_input.aspx` /
`gift_input.aspx`), and this client has no web bridge. The screen
states the stall instead of staying a silent dead view.

## Paint seen

White notice ink inside the largest staged rect; the small rect
stays black. Tutorial paints its staged backdrop alone
(`tutorial.png` byte-identical in the shots oracle — no new ink).

## Deltas found (new rows, never silent fixes)

1. **Real billing is OPEN.** The notice cites lifecycle-verified
   paths; no host, session, or web bridge is pinned. Next step:
   live gateway replay through the billing endpoints naming the
   host/session join, then queue through the session bridge.
2. **Tutorial entry path is OPEN.** No dossier offset names the
   `CStateTutorial` entry dispatch (`TUTORIAL_ENTRY_NEXT_STEP`).
   Next step: SendInput sweep of the lobby emblem buttons per the
   `~/scratch/planet-verify/` recipe plus a REA trace of the entry
   dispatch.
3. **Notice placement is ours-by-design.** Largest staged rect is
   the stand-in anchor; the exe's web-view layout rect is unpinned.
   Next step: rig capture of the live billing view mount.
