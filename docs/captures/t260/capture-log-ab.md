# #260 — capture-log A/B across the adapter move

This note pins one Rig-backed input trace through the native #334 harness
before and after moving capture plumbing behind the input seam. The question
is narrow: did the window capture log change when routing stopped owning the
sink? It is not a pixel-parity verdict.

## Method

Use `scripts/capture-log-diff.py BEFORE/input.log AFTER/input.log --judged-first` for the log-order contract, then rerun with `--full` for the whole-log diff; this applies the filters below without hand-rolled `sed`.

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

- Worktree at `4b515ec`, after `refactor(shell): put capture plumbing behind
  the input seam`.
- Evidence: `~/scratch/p260-ab/after/` (same file set as the BEFORE leg).
- Driver exit 0; `applied_events=19`; same five scenes in the same order.
  Capture log: 159 lines, 105 frame lines, 4 judged lines.
- Containment: `route=sink`, `preboot_breach=0`, `on_default_sink=0`,
  `client_streams=1`, peak `-3.3 dBFS`; same server boot identity.
- Binary SHA-256: `f562bc17aff41618cdde2883fd8421547c932d11dedfc9fed20068db26690bc2`.

## Judged lines first (log-order contract)

```bash
cd ~/scratch/p260-ab
grep -E '^(event|judged) ' before/input.log \
  | sed -E 's/^(event|judged) [0-9]+/\1 T/' > before/judged-first.log
grep -E '^(event|judged) ' after/input.log \
  | sed -E 's/^(event|judged) [0-9]+/\1 T/' > after/judged-first.log
diff -u before/judged-first.log after/judged-first.log
# exit 0: no output
```

The 4 judged lines keep their lanes and their positions relative to the
routed event lines: BEFORE `judged … lane 0, 3, 3, 5`, AFTER the same
`0, 3, 3, 5`. Wall stamps differ only because the two boots start at
different times (`15429/19690/19872/19933` vs `15390/19642/19824/19885`).
No judged line moved across a drain, and the driver applied all 19 events.

## Full capture log

```bash
cd ~/scratch/p260-ab
normalize() {
  sed -E \
    -e 's/^(event|judged) [0-9]+/\1 T/' \
    -e 's/^frame [0-9]+ wall_ms=[0-9]+/frame F wall_ms=T/' \
    -e 's/ state=[0-9a-f]+/ state=HASH/' \
    -e 's/ content=[0-9a-f]+/ content=HASH/' "$1"
}
normalize before/input.log > before/normalized.log
normalize after/input.log > after/normalized.log
diff -u before/normalized.log after/normalized.log
# exit 1: three frame-only shifts, shown below
```

```diff
@@ -1,7 +1,6 @@
 frame F wall_ms=T screen=planet state=HASH content=HASH
 frame F wall_ms=T screen=planet state=HASH content=HASH
 frame F wall_ms=T screen=planet state=HASH content=HASH
-frame F wall_ms=T screen=planet state=HASH content=HASH
 event T text s
 event T key-down KeyS
 judged T lane 0
@@ -42,8 +41,8 @@
 frame F wall_ms=T screen=room state=HASH content=HASH
 event T key-down Enter
 frame F wall_ms=T screen=waiting state=HASH content=HASH
-event T key-up Enter
 frame F wall_ms=T screen=waiting state=HASH content=HASH
+event T key-up Enter
 frame F wall_ms=T screen=waiting state=HASH content=HASH
 frame F wall_ms=T screen=waiting state=HASH content=HASH
 frame F wall_ms=T screen=waiting state=HASH content=HASH
@@ -63,6 +62,7 @@
 frame F wall_ms=T screen=waiting state=HASH content=HASH
 event T text u
 event T key-down KeyU
+frame F wall_ms=T screen=waiting state=HASH content=HASH
 event T key-up KeyU
 frame F wall_ms=T screen=waiting state=HASH content=HASH
 event T text 1
```

Verdict: the judged/event line stream is byte-for-byte equal after
normalizing the wall stamp (`diff -u` exit 0). The full log has the same
159 lines and the same screen sequence, but three identical frame lines sit
on the other side of an event line because live Xvfb paint cadence differs
(`diff -u` exit 1). No event or judged line changed order, and no line was
lost; the frame gate itself moved unchanged into `CaptureAdapter`.

Both legs are reproducible with `~/scratch/p260-ab/run-leg.sh OUT DISPLAY
TRACE BINARY`; it boots the guest with `rig-run.sh run`, starts a host room,
replays the trace with `--attach-rig --sound`, and stops only its own PIDs.
Raw logs, frames and audio stay under `~/scratch/p260-ab/`; no capture bytes
enter the repository.

Final gates at `d62c683`: `cargo fmt --check` exit 0,
`cargo clippy --workspace --all-targets -- -D warnings` exit 0,
`cargo test --workspace` exit 0 with `install-root` present (2673 tests) and
with it hidden (2673 tests).
