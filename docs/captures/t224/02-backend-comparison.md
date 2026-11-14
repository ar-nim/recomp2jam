# T224 leg 2 — backend comparison for this workload

Workload: CPU-side 800x600 XRGB software frames, menu-driven pacing
(60 Hz interp clock, audio sample clock owns song time per ADR-0001),
Linux X11 + Wayland, Windows, macOS. Current pins: winit 0.30.13,
softbuffer 0.4.8 (`Cargo.lock`), `o2jam-shell` + `o2jam-render`.

## winit + softbuffer (as-is)

- Hardware acceleration: none required. The shell paints XRGB words
  into a software backbuffer and presents with `buffer_mut` +
  `present` (`crates/o2jam-shell/src/window.rs`). Runs anywhere a
  window with shared-memory buffers works (X11 shm, Wayland shm,
  Windows GDI-class surfaces).
- Frame pacing: solved in-tree. winit 0.30.13's frame-callback gate
  engages only via `pre_present_notify` (`window.rs`, citing winit
  #4668-adjacent and the winit-diligence sweep). Without it the
  Wayland loop busy-repaints at 100% CPU (reproduced on KWin 6.8.2,
  2026-11-12; `~/scratch/winit-4668/findings.md`). With it the loop
  blocks on the compositor callback and idles; `ControlFlow::Wait` /
  `WaitUntil` otherwise (`window.rs`).
- Input parity: needs met. winit keymap (SDF SpaceJKL) ships; the
  scripted-key probe through the real windowed dispatch path measures
  shell-added latency 0 ticks, gated at
  `INPUT_LATENCY_BUDGET_TICKS = 0`
  (`crates/o2jam-shell/src/input_latency.rs`, #100). Exe-side
  DirectInput numbers stay OPEN (next: ADR-0009 rig pair, #76).
- Already suffered: XTEST injection never reaches a DirectDraw canvas
  (motion only) — the Wine rig drives clicks with a winegcc-built
  SendInput helper instead; Wayland decoration behavior needed a
  rig workaround; the ARGB-shm alpha byte needed forcing opaque
  (white speckles / transparent bars without `0xFF00_0000`).
- Maintenance: two crates.io deps, no C toolchain, no GPU backend
  quirks. Platform matrix covered by softbuffer's shm/DXGI/CG paths.

## SDL2

- Hardware acceleration: not needed by this workload; SDL2 would add
  a C dependency for no rendering gain (software surface path would
  mirror today's backbuffer).
- Frame pacing: mature game loop, but its own backend quirks replace
  winit's — the in-tree gate, XTEST, and decoration lessons would
  need re-proof on SDL2's X11/Wayland backends.
- Input parity: keyboard + mouse in one crate, plus audio in the same
  crate — attractive only if cpal ever disappoints (it has not:
  `CpalBackend::samples_played` clock + 53.8 s bgRoom parity).
- Maintenance: C dependency (SDL2 dev packages per platform, build
  complexity on all three OSes). Wins only if the project wants one
  crate for window + input + audio.
- Verdict: lateral move with added build cost; no evidenced payoff.

## wgpu

- Hardware acceleration: required (GPU device, swapchain). Nothing in
  the evidenced 7.55 path calls for it: fixed 800x600x16 blits,
  source colorkey, no lighting, no blended shaders, no scaling
  transform in `Draw`/`DrawClip` (`docs/claims.md` A38).
- Frame pacing: swapchain present modes give real vsync control, but
  today's menu-driven pacing already idles on the compositor
  callback — nothing to fix.
- Input parity: none (rendering only; winit still needed for window
  + input).
- Maintenance: shader pipeline, adapter-selection failures on
  headless/VM rigs, screenshot-diff surface changes. The tree already
  scopes it correctly: `wgpu` upload is OPEN as a future upload path
  (`docs/progress.md` renderer row, `docs/architecture.md`), not a
  replacement.
- Verdict: wins only if GPU effects ever enter scope (animated
  shaders, blended lighting). No 7.55 menu or highway art calls for
  them.

## Raw platform backends (Win32 + X11 + Cocoa hand-rolled)

- Hardware acceleration: none needed, same as softbuffer.
- Frame pacing: maximum control (own vsync/threading), but every
  gate the tree already solved (frame callback, decorations, shm
  lifetime) becomes hand-rolled per platform.
- Input parity: own key maps per OS; the DirectInput bind table is
  OPEN either way, so no head start.
- Maintenance: all backend cost on us, three platforms × window +
  input + present. Highest cost, zero evidenced benefit for 800x600
  software frames.
- Verdict: never justified for this workload.

## Scorecard (this workload only)

| Backend | HW accel | Pacing | Input | Maintenance | Matrix |
|---|---|---|---|---|---|
| winit+softbuffer | none needed, none used | solved (gate) | probe-green | low | X11/Wayland/Win/macOS today |
| SDL2 | none needed, C dep added | re-proof needed | same crate as audio | medium (C toolchain) | same, own quirks |
| wgpu | GPU required, unused | nothing to fix | n/a (render only) | high (shaders, adapters) | headless/VM risk |
| raw backends | none needed | max control, hand-rolled | hand-rolled maps | highest | all cost on us |
