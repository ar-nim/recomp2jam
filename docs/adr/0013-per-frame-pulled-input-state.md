# ADR-0013: Per-frame pulled input state (exe-parity input architecture)

## Status

Accepted (owner ratification, 2026-11-12), per the research in
`docs/input-architecture-128.md` and the recommendation on #128.

## Context

The windowed client delivers input as a winit PUSH through a forwarding
chain (`WindowHandler` → `Shell` → `Box<dyn GameLoop>` → session → game).
That shape produced the #118 incident (a forwarding gap silently dropped
every press/release/motion while keyboard kept working) and, more
fundamentally, cannot reproduce the exe's observable input semantics: the
exe POLLS `GetCursorPos` once per frame, sweeps placed rects itself
(`PtInRect`), and drains queued discrete events at the frame boundary
(claim A46; no `WM_MOUSEMOVE` handler exists in the image). Push and
pulled state are different observables — SDL's own documentation states
the two disagree on press-release pairs between frames — and matching a
polling oracle requires matching its observable.

## Decision

The game consumes input as a **per-frame snapshot**:

1. winit events are demoted to pure OS transport. `window.rs` caches the
   latest pointer position and button/keyboard state into a frame
   snapshot and queues discrete events; at each frame boundary the cache
   is frozen and the queue drained — exactly one consumption point.
2. States own hit-testing: the snapshot carries positions and state;
   scenes do their own rect sweep per A46 (hover = pointer position at
   paint time; presses arm/release against the same snapshot).
3. No forwarding chain: every scene reads the same snapshot type. The
   `GameLoop` event methods are replaced by snapshot-shaped seams as the
   migration lands.

## Consequences

- Hover/press semantics become frame-exact BY CONSTRUCTION, matching the
  oracle; the #118 forwarding class is structurally impossible.
- One snapshot per frame feeds replay and the trace harness (determinism);
  the #76 latency budget gets exactly one measurable sampling point.
- Risks (from the research): the intra-frame tap edge queue (press and
  release between two snapshots must still register as a click — the
  exe's per-frame dispatch has the same edge; the queue preserves it),
  and the backend-silence residual (a dead backend starves the snapshot
  too — the existing seam guards cover this).
- The migration is guard-backed and green-to-green; scenario tests that
  drive the game directly keep working (they construct snapshots).
