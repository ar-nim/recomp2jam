# Architecture — Rust target

Goal: behavior-compatible recompilation of the 7.55 gameplay client.
Clean-room: describe original behavior in docs, re-implement here.

```text
┌─────────────┐   ┌────────────┐   ┌────────────┐
│ o2jam-dump  │──▶│ o2jam-file │   │ recomp2jam │  (binaries)
└─────────────┘   └─────┬──────┘   └─────┬──────┘
                        │                │
              ┌─────────▼────────────────▼─────────┐
              │            o2jam-game               │  pure logic:
              │ chart · timing · judgement · score │  no OS, fully tested
              └─────────┬────────────────┬────────┘
                        │                │
              ┌─────────▼──────┐ ┌───────▼────────┐
              │  o2jam-audio   │ │ renderer/input │  (planned)
              │ lewton + trait │ │ winit+wgpu+cpal│
              └────────────────┘ └────────────────┘
```

## Crates

- **`o2jam-file`** (today: magic sniff + documented stubs; next: verified
  OJM/OJN parsers). Pure Rust, `thiserror` only. No OS calls so WASM/test
  harnesses stay easy.
- **`o2jam-audio`** (today: `AudioBackend` trait + `NullBackend` +
  `lewton` decode helper). Later: `cpal` backend, mixer, latency probe.
- **`o2jam-game`** (today: `Note`, `Chart`, `TimingWindows`, `Judgement`,
  `Score`, song-clock). Later: 7-key scroll, long-note state machine,
  replay-compare tests.
- **`recomp2jam`** (today: CLI `--help`, version, log init). Later: the
  windowed client (fixed-timestep loop anchored to the audio clock).
- **`o2jam-dump`** (today: `identify` + hexdump helper). Later: full
  `dump-ojm` / `dump-ojn` pretty printers.

## Key decisions

1. **Audio clock owns song time.** CDX-era games often used wall-clock
   (`timeGetTime`) for scroll; rhythm parity needs sample-accurate audio
   time. The game loop interpolates render positions from the audio clock.
2. **Parsers are total and bounded.** All file parsing is fallible,
   length-checked, and fuzzable (`cargo fuzz` later). No `unwrap` on
   untrusted bytes.
3. **Boost-isms become ownership.** `shared_ptr` webs become explicit
   owners + `Arc` only where threads share. Threads: `std::thread` now,
   `tokio` only if networking/loading needs it.
4. **CDX-isms become data.** Sprites/scenes become plain data + a wgpu
   renderer later — never Win32 calls inside game logic.

## Milestones

1. File parsers verified against installed 7.55 data (hashes recorded).
2. Null-backend autoplay of a chart reaches expected score deterministically.
3. Windowed client hits notes with measured audio+input latency budget.
