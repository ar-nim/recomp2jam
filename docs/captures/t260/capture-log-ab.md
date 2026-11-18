# #260 — capture-log A/B across the adapter move

This note pins one Rig-backed input trace through the native #334 harness
before and after moving capture plumbing behind the input seam. The question
is narrow: did the window capture log change when routing stopped owning the
sink? It is not a pixel-parity verdict.

## Method

- Trace: `~/scratch/native-ab-334/native-walk-to-loading.json`, 19 events,
  SHA-256 `ab89594201fe7efad709b795b94cc42cb5d04afdd857d437ddf0876175e4ef0d`.
  Both legs replay this same file; no event is added, removed or retimed.
- Producer: `scripts/replay-trace-native.py --attach-rig --sound
  --await-timeout 60` on a private X11 display, with `rig-run.sh run` owning
  a per-run null sink. Guest account 1 joins a room hosted by account 2.
- The local Memoryer rejects the trace's HARD emblem click (136,88) with the
  blue NORMAL-mode-only notice. The same helper clears one OK click at
  (400,369) in both legs when it sees that trace click. It is capture
  plumbing outside the refactored module, not a change to the trace.
- Normalization: wall-clock stamps and environment-dependent paint/state
  hashes vary between live boots. Both legs use this filter before `diff -u`:

  ```bash
  normalize() {
    sed -E \
      -e 's/^(event|judged) [0-9]+/\1 T/' \
      -e 's/^frame [0-9]+ wall_ms=[0-9]+/frame F wall_ms=T/' \
      -e 's/ state=[0-9a-f]+/ state=HASH/' \
      -e 's/ content=[0-9a-f]+/ content=HASH/' "$1"
  }
  ```

  The filter keeps every line, every screen name and every line order. Only
  values that a live boot cannot repeat are replaced; no line is dropped.
- Judged lines are diffed first (log-order contract: judged strikes write
  before the following drain-order event lines), then the full capture log.

## BEFORE leg (pre-refactor)

- Worktree `/home/arnim/Projects/recomp2jam-wt91` at `cc8e043`.
- Evidence: `~/scratch/p260-ab/before/` (`input.log`, `applied.log`,
  `client.log`, `manifest.json`, `audio/manifest`, scene frames).
- Driver exit 0; `applied_events=19`; scenes planet → channel → room →
  waiting → loading. Capture log: 159 lines, 105 frame lines, 4 judged lines.
- Containment: `route=sink`, `preboot_breach=0`, `on_default_sink=0`,
  `client_streams=1`, peak `-3.3 dBFS`; server identity
  `memoryer_boot_epoch=1794986056` unchanged.
- Binary SHA-256: `be06bf77bcec05f00780ee640734cd162d7e3ec1f486d04bb4a920980624bb7d`.

## AFTER leg (post-refactor)

Pending the adapter move.
