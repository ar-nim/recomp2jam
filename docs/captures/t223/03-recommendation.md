# T223 batch 3 — smoothing recommendation, conditioned on batches 1+2 (2026-11-14)

## Condition

- Batch 1: live contract is 250 ms tick + 250 ms pulse = 4 fps, one frame
  per repaint. Pulse is the binding visible constraint.
- Batch 2: exe cadence OPEN; no pin to consume; invented smoothing is out
  of scope per the brief.

## Recommendation (in order)

1. **Pin first.** Run the named capture (`02`, arm 1 or 2). No code changes
   until it lands. Expected outcomes:
   - Exe ≈ 100–125 ms/frame (8–10 fps): set `AVATAR_BREATHING_TICK_MS` to the
     pinned value AND lower the Room/Waiting `idle_repaint_interval` pulse to
     at most the tick (pulse must outrun the tick per the brief), then guard
     the new contract red-first. Both clocks move together or not at all
     (batch 1: fixing one alone cannot smooth).
   - Exe ≈ 250 ms/frame (4 fps parity): the jaggedness IS exe behavior.
     Close the smoothness expectation as wontfix-parity and re-scope any
     further work as interpolation — which needs an explicit owner decision,
     since it invents poses the exe never drew.
2. **Never interpolate without a pin.** Blending between the 12 breathing
   frames (cross-fade or anchor lerp) would smooth the motion but invents
   visuals with no exe evidence. Out of scope until the owner says otherwise.
3. **Pulse-only quick win is cosmetic, not smoothing.** Raising the repaint
   rate alone (e.g. 60 Hz) while the tick stays 250 ms only cuts input
   latency on avatar scenes; frames still step at 4 fps. Do not sell it as
   the fix.
4. **Guard shape for the execution ticket** (red-proved, archives-absent-safe):
   extend `avatar_breathing_frame_clock_guard` with the pinned cadence and
   extend the Room/Waiting pulse tests to assert
   `idle_repaint_interval <= tick`; wall-proof via the batch-1 input-log
   method (median `wall_ms` delta ≈ pulse, content changes ≈ tick).

## What the execution half needs

One Wine session (arms in `02`), then a ≤50-line change:
`AVATAR_BREATHING_TICK_MS`, two `idle_repaint_interval` arms, guards.
No painter redesign: `paint_centered_pixels` already selects one run frame
per `avatar_breathing_frame(now_ms)` (`avatar.rs:523-533`).
