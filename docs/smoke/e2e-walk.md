# End-to-end walk — smoke note (2026-11-10)

Ticket #23: one walk from boot to Result passing through every
landed State, driven headless through the same `ScreenGame` seam
the winit window runs (no display, no gateway, null audio — sound
contained, no speakers). Synthetic seeds only; genuine archive art
stages underneath when `install-root` is present and the walk keeps
the flat fallbacks when it is absent (both arms green in CI).

## Walk path

`avi → planet → channel → room → waiting → loading → playing →
result`, then the Result retry and leave edges back into the lobby
(`result → waiting → room`). Along the way the walk opens the
Options dialog through the gear disc and Escapes out, visits all
six side states from Waiting
(`Shop`, `MusicShop`, `MyRoom`, `Lanking`, `Payment`, `Tutorial`)
and returns each time, and commits the host song pick
(id 757, HX, X3.5) that arms the Loading gate.

Evidence: 26 steps, one frame per step (all 26 carry ink, minimum
58 unique colors with game data staged), screen trace plus state
hash per step. Result instant hash `40a617e1c9b1e5b5`; walk-end
`room` hash `b04304aac506dd41`. A twin game driven the same way
ends on the same hashes. Frames plus `trace.json` are written by
`cargo run -q -p recomp2jam -- e2e --out-dir <dir>` (default
`/home/arnim/scratch/e2e-walk`); the PNGs stay out of the repo —
genuine-art renders beyond the shots oracle are not committed.
Guards: `e2e_walk_boots_from_splash_to_result_through_every_landed_state`,
`e2e_walk_replays_to_the_same_final_hash`,
`e2e_walk_side_states_open_and_return_to_waiting`,
`e2e_walk_options_opens_and_closes_on_the_board`
(all in `crates/recomp2jam/src/e2e.rs`).

## Deltas found (new rows, never silent fixes)

1. **No broken wiring.** Every landed edge landed on its screen
   first try — the walk fixes nothing and changes no painter, so
   the shots oracle must show zero unintended deltas (proven by
   `scripts/verify-shots.sh` before/after this ticket).
2. **Live-window replay stays OPEN.** This walk is headless; the
   same step table replayed through the real window
   (`fixtures/input-trace/replay_window.py`) against a live
   gateway on the rig is the named next step.
3. **Side-visit ink rides staged art.** With `install-root`
   absent the side frames paint the flat fallback while the
   wiring asserts still hold — the shots oracle (not this walk)
   proves side ink.
