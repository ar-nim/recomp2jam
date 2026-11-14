# T223 batch 1 — live paint-cadence method + contract numbers (2026-11-14)

## Method (exact, reproducible)

The client already ships the wall-clock paint logger. No new
instrumentation is needed.

1. Build the worktree client, start Memoryer (`~/memoryer-local`),
   start Xvfb on `:99`.
2. Run with the trace-harness pair (both required together):
   `recomp2jam online --window --input-log /tmp/t223/input.log --frame-dump /tmp/t223/frames --no-sound`
   (`--no-sound` keeps audio on the `wine_null` sink; never the speakers).
   `--input-log` alone also arms the plan (empty dump dir skips PNGs, keeps counting).
3. Drive login → Room (or Waiting) with a worn figure staged
   (tester2 wears the full kit per `docs/equip.md`), then park 20 s with no input.
4. Parse: every paint appends one line
   `frame N wall_ms=<t> screen=<s> state=<h> content=<h>`
   (`crates/o2jam-shell/src/window.rs`, `capture_frame`).
   Filter `screen=room|waiting`; the wall cadence is the median delta
   of consecutive `wall_ms`; a `content` hash change marks a breathing-frame
   advance. Breathing fps = content-changes per second.

## Contract numbers (measured, seam-green, 2026-11-14)

`cargo test -p o2jam-shell -- idle_pulse_arms avatar_breathing_frame_clock_guard`
→ 3 passed (`avatar_breathing_frame_clock_guard`,
`room_idle_pulse_arms_while_the_worn_figure_shows`,
`waiting_idle_pulse_arms_while_a_seat_figure_shows`).

- Tick: `AVATAR_BREATHING_TICK_MS = 250`
  (`crates/o2jam-shell/src/avatar.rs:47`); frame `= (now_ms/250) % 12`.
- Pulse: `idle_repaint_interval` returns `Some(250 ms)` on bare
  Room/Waiting boards with a staged figure
  (`crates/o2jam-shell/src/screens/game/mod.rs`, `idle_repaint_interval`).
- Scheduler: `plan_repaint` (`crates/o2jam-shell/src/window.rs`)
  repaints when `now >= due` and re-arms `due = now + gap`,
  so wall cadence is 250 ms plus event-loop jitter on an idle park.

Arithmetic: 12 frames at 250 ms = one 3000 ms breathing cycle at 4 fps.
Tick and pulse are the same 250 ms, so the figure advances exactly one
frame per repaint. The pulse is the binding visible constraint: even a
faster tick could not show more than 4 repaints/s through this pulse.

## Live wall run: NOT yet executed

Rig state today: `:99` down (Xvfb can start it), `wine_null` sink present,
Memoryer DOWN, and the login→room SendInput navigation is the known
blocker (#176 round 1 never completed it). A parked 20 s wall sample is
the next rig session, not this batch. The contract numbers above are the
ceiling the wall sample must confirm (expect ~250 ms median, jitter ±20 ms).

## Dominant steppiness source (named)

The stacked pair: 250 ms tick (frame-step size) plus 250 ms pulse
(repaint rate). Both coarse, both the same value, both OPEN placeholders.
Fixing only one cannot smooth motion: a faster tick with the 250 ms pulse
still shows 4 repaints/s; a faster pulse with the 250 ms tick repaints the
same frame.
