---
name: recomp2jam-input-replay
description: Design deterministic O2Jam input traces and replay tests for recomp2jam. Use for seven-lane key capture, press/release timing, long-note replay, menu navigation, deterministic state hashes, or comparing Wine and cross-platform Rust runs with the same stimuli.
---

# recomp2jam Input Replay

Replay the same user actions at a portable input boundary. Determinism makes
regressions visible; it does not prove that the behavior matches 7.55.

## Choose the clock and boundary

1. Read the relevant behavior in `docs/gameplay.md`, `docs/lifecycle.md`, and
   `docs/architecture.md`. Use `docs/parity.md` to identify known shell gaps.
2. Use song position for gameplay events and a monotonic clock for menus.
   Record units explicitly. Do not assume a fixed frame rate or confuse the
   original `timeGetTime` path with the Rust audio-clock design.
3. Inject routine tests at the portable game/shell input API, not through OS
   key injection. Reserve DirectInput/Wine and native window injection for
   end-to-end probes of that integration layer. For pointer probes, verify the
   native 800x600 canvas coordinates and focus before `SendInput` or XTEST.
4. Record the event's intended timestamp separately from the applied timestamp
   and observed song position. This exposes queueing and latency errors.

## Define a stable trace

Use ordered events with a sequence number, integer timestamp, lane or control,
and action. Include both `press` and `release`; long-note tails depend on key
release and cannot be tested with key-down-only traces.

```json
{
  "schema_version": "recomp2jam-input-v1",
  "scenario": "hold-tail-release",
  "time_base": { "name": "song_position", "unit": "microseconds" },
  "events": [
    { "sequence": 0, "time_us": 1000000, "control": "lane-1", "action": "press" },
    { "sequence": 1, "time_us": 1506000, "control": "lane-1", "action": "release" }
  ]
}
```

- Use lanes consistently and document whether the external label is one-based.
- Validate monotonic sequence/time, allowed actions, complete press/release
  pairs, and range bounds before a run.
- Store chart/scenario ID, difficulty, speed, modifiers, expected RNG behavior,
  and build identity beside the trace. Inject an explicit random source only
  where the Rust model provides that boundary.
- Commit only small synthetic traces. Keep captures derived from proprietary
  charts or sessions in local-only storage.

## Replay and verify

1. Log every accepted, rejected, and applied event in sequence. Reject or hold
   out-of-order events according to a documented harness rule rather than
   silently reordering them.
2. Run the same trace twice in the same build. Compare event logs, state
   transitions, note judgements, score, combo, jam, life, pill stock, hold
   state, and final state hash as applicable.
3. Run reference and test scenarios with equivalent stimuli. Compare observable
   outcomes with 7.55-derived tolerances, especially the inclusive 6/18/25 ms
   judgement windows where those windows apply.
4. Cover press, release, early/late hold release, simultaneous lanes, duplicate
   events, focus loss, pause/transition boundaries, and chart end when relevant.
5. Keep network time and external server behavior outside the deterministic
   core unless the harness explicitly controls them.

## CI and completion

Synthetic replay tests must run without `install-root/` or a live gateway.
Tests that require real archives must anchor paths to `CARGO_MANIFEST_DIR` and
return early when data is absent. Live tests remain opt-in. Audio-producing
runs must use a null or virtual output.

A replay harness is complete when it is non-interactive, rejects malformed
traces, demonstrates repeatability, and reports behavioral differences without
labeling an unverified 7.55 assumption as a parity failure.
