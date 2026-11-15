# Pair B rig measurement (ticket #76) — 2026-11-15 live run srV

## Result

XTEST key press to judged-strike entry (`judged` log line), N=60,
Playing highway, solo room, song 757, debug build, Xvfb :98:

- min (hitch-free floor): 0 ms (by construction)
- p50: 71 ms
- p90: 113 ms
- p99: 122 ms
- max: 122 ms

Proposed input-only budget number: **p99 <= 125 ms** on this rig
(rounds the measured 122 ms; revisit after N>=200 and release build).

## Method

- Binary: `wt19-v232` worktree (`cb2/tt-232`), debug build, `-v`,
  `--online-music-id 757`, tester account, room `t76srV`.
- Drive: emblem double-click, face click (resident-channel swallow),
  room-row double-click, song-row pick, OK (BND rect), READY
  (BND rect `(699,144)`), START, Enter on Loading (explicit
  `set_input_focus` first — bare Xvfb does NOT focus on click).
- Injector: `/tmp/inject76.py` — cycles S/D/F/Space/J/K/L
  (lanes 0-6, verified from `judged` lines), 1.62 s spacing,
  epoch-ms recorded per press.
- Pairing: client input log `event <wall_ms> key-down <Key>` lines
  (in order) with injector epochs; offset = min(E-W) (luckiest
  delivery); deltas = (E-W) - offset. `judged` stamps equal key-down
  stamps (same frame freeze), so key-down stands in for judged.
- All 60 presses judged (lanes 0-6 all hit).

## Caveats (must fix before closing #76)

- N=60: p99 is the 2nd-largest sample. Need N>=200.
- Offset-by-min floors systematic transit into 0; absolute transit
  unknown (needs frame-clock-anchored injector, the `capture_plan`
  follow-up on `cb2/tt-76`).
- Debug build renders Playing at ~9 fps (wall 114 ms/frame);
  p50 ~71 ms is mostly half-frame quantization. Release build
  re-measure required.
- Solo room; server had already stopped the session
  ("failed to ready" from the earlier slow run) — timing is local
  only, no server contention. Multi-member room may differ.
- XTEST path, not exe SendInput; equivalence argument open
  (see exe-sampler-spec, pair A).

## Rig lessons

- Bare Xvfb + no WM: keys need explicit `set_input_focus`
  (python-xlib) before XTEST key events; clicks work unfocused.
- `RUST_LOG` is ignored; verbosity comes from `-v`/`-vv`
  (`cli.verbose` in `main.rs`).
- Loading Confirm = Enter (`MenuKey::Confirm`); mouse clicks in
  Loading do nothing. 4012 then fires, battle starts.
- The client input log can drop externally appended lines
  (seen: `inject` markers vanished) — keep injector records
  in a separate file.
