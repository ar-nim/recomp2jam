# T223 batch 4 — live re-observe against the 66 ms pin

## Verdict

The tick and pulse are already at the exe's pinned 66 ms
(`crates/o2jam-shell/src/avatar.rs:47`,
`crates/o2jam-shell/src/screens/game/mod.rs:6869`). A release session on
the live rig paints Waiting every **66 ms** (median, 451 frames in the
final pack), so the pinned cadence reaches the screen and no interpolation
is needed.
The jaggy debug playtest is **paint-bound**: a debug paint costs about
151 ms (temporary instrumented median) versus 11 ms in release, so no
66 ms tick can show every step there. The heartbeat scheduler also used
to add that paint cost to every deadline; it now follows the fixed grid
and skips steps a slow paint already missed.

Owner live-confirm is still OPEN: this is a rig measurement, not a human
playtest. Debug-build smoothness is a separate performance arm, not a
cadence contract change.

## Evidence IDs

- `EV-P223-LIVE-1`: post-fix release pack
  `~/scratch/p223-cadence/rust-final`, input log SHA-256
  `3b65040922478ee61f3475072f61842971786c1d4c560e4dbfd7f8af50966a0f`,
  provenance `manifest_sha256`
  `e57f6446284ef37abfa9c2bf80808ecda7b1eae59b49c0788c671b372184742f`.
  `run-manifest verify`, `rig-audio verify`, and
  `rig-provenance.sh cite --claim none` all exited 0. The pack is
  git-ignored; only these hashes are committed.
- `EV-P223-PAINT-1`: one instrumented debug run and one instrumented
  release run (`~/scratch/p223-cadence/rust-probe{,-release}`), temporary
  `tracing` probes around `present()` (removed before the commit).
  Median paint: debug 151 ms (252 samples), release 11 ms (768 samples).
- `EV-P229-DWELL-1`: exe step 66 ms, 12-frame cycle ~800 ms
  (`docs/captures/t-pin-cadence/04-wine-dwell-pin.md`).

## Root cause and file:line

1. Tick source: `ScreenGame::now_ms`
   (`crates/o2jam-shell/src/screens/game/mod.rs:1425`) reads the audio
   sample clock for every screen; `avatar_animation_phase`
   (`crates/o2jam-shell/src/avatar.rs:58`) turns that into the 66 ms
   phase. This is not the dominant steppiness in a sound run: content
   changed on every paint in both profiles.
   Separate OPEN arm: `--no-sound` uses `NullBackend::samples_played`
   (`crates/o2jam-audio/src/lib.rs:194`), which only advances when PCM
   is pushed, so a silent park freezes the phase. Next step: give menu
   breathing a wall/sample-independent clock; out of scope here.
2. Dominant arm: paint cost. `WindowHandler::present`
   (`crates/o2jam-shell/src/window.rs:83`) and the software blit are
   ~14x slower in a debug build (151 ms vs 11 ms). A 66 ms heartbeat
   cannot show a new pose faster than one paint.
3. Schedule arm: `plan_repaint`
   (`crates/o2jam-shell/src/window.rs:580`) re-armed the heartbeat from
   post-paint `now`, adding paint cost to the step. It now re-arms from
   the missed deadline through `next_heartbeat_deadline`
   (`crates/o2jam-shell/src/window.rs:555`), skipping grid points that
   already passed. Input paints still re-arm from the input instant.

## Red before green

- Guard `avatar_step_survives_a_paint_that_eats_the_deadline_slack`:
  red, `left (true, Some(now+66+45))`, `right (true, Some(due+66))`.
- Guard `heartbeat_skips_deadlines_missed_by_a_long_paint`: red,
  `left (true, Some(painted+66))`, `right (true, Some(due+2*66))`.
- Both green after the one `plan_repaint` change; all 36
  `window::tests::` pass.

Live red/green (all with `--input-log` only, sound on an owned null
sink, private Xvfb `:164`, `route=sink`, `on_default_sink=0`,
`preboot_breach=0`):

| Run | Code | Median Waiting wall gap | Content changes |
|---|---|---|---|
| `rust-clean` | debug, before grid fix | 117 ms | 879/879 |
| `rust-fixed` | debug, after grid fix | 112 ms | 538/538 |
| `rust-release` | release, before grid fix | 66 ms | 938/939 |
| `rust-final` | release, after grid fix | 66 ms | 448/450 |

Interpretation: the grid fix removes scheduler drift but cannot beat a
151 ms debug paint; release is already at exe cadence and stays there.
No frame interpolation was added.

## Guards that pin the contract

- `avatar_step_survives_a_paint_that_eats_the_deadline_slack` and
  `heartbeat_skips_deadlines_missed_by_a_long_paint` (new, synthetic,
  live caller chain named in each test).
- `avatar_animation_phase_clock_guard` (66 ms, 48-step wrap) and
  `room_idle_pulse_arms_while_the_worn_figure_shows` (pulse == tick)
  from commit `7857d38` remain green.

## Next steps

- Owner: live-confirm the release build on a real seat; this rig cannot
  supply that verdict.
- Debug-build smoothness: separate performance ticket; the 151 ms paint
  is a profile problem, not a breathing-cadence problem.
- Silent-menu breathing: clock-source ticket named in root cause 1.
