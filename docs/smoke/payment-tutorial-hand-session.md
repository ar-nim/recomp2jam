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
two synthetic staged rects. Both billing views stage their bound
chrome and the embedded-IE client area stays blank by owner pin
(`docs/agents/environment.md`; guard
`payment_views_leave_the_embedded_web_client_blank`, install-backed
guard `billing_and_gift_web_sheets_stage_only_their_bound_chrome`);
with no staged rects nothing paints (guard
`payment_side_without_staged_rects_paints_nothing` — ADR-0003,
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
stays black. Tutorial paints its staged backdrop plus the B7-pinned
frame-0 instruction bubble (`1/13` baked in the art) and coach figure;
chart-timed note bodies remain OPEN under #326. The shots oracle holds
the regenerated `tutorial.png` (sha256
`01b6a4bfd92636e47e47f954713442f5c293cec3dfcc93a8d359cdd5d92ca24e`).

## Deltas found (new rows, never silent fixes)

1. **Real billing is OPEN.** The notice cites lifecycle-verified
   paths; no host, session, or web bridge is pinned. Next step:
   live gateway replay through the billing endpoints naming the
   host/session join, then queue through the session bridge.
2. **Tutorial entry path is PINNED from Room FIRSTSTEP.** Qualified
   pack `~/scratch/side-scenes/b7-tutorial/` (manifest sha256
   `064bec85c998dbeb5d9073f312ef949d490a4f8a2cc9d62adfccdb0c1183272d`)
   records the live click; static pin `S-PLAY-B3` in
   `docs/captures/static-play/pins.md` names `0x4902bb` → `0x495110`
   (chart globals + `ChangeState(0xb)`) → `CStateTutorial` ctor
   `0x4f07f0`. Our FIRSTSTEP route installs those globals. The dimmed
   Planet TUTORIAL plate has no dispatch: the remaining next step is
   `PLANET_TUTORIAL_PLATE_NEXT_STEP`, a live click sweep of that plate
   only (and the still-OPEN chart/audio join under #326).
3. **Notice placement is ours-by-design.** Largest staged rect is
   the stand-in anchor; the exe's web-view layout rect is unpinned.
   Next step: rig capture of the live billing view mount.
