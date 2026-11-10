# Architecture — Rust target

Goal: behavior-compatible recompilation of the 7.55 gameplay client.
Clean-room: describe original behavior in docs, re-implement here.
Oracle: the 7.55 exe; terms in `GLOSSARY.md`, deviations in `docs/adr/`.

```text
              ┌────────────┐  ┌────────────┐
              │ o2jam-dump │─▶│ o2jam-file │  pure parsing, no OS
              └────────────┘  └─────┬──────┘
                                    │
              ┌─────────▼────────────────▼─────────┐
              │            o2jam-game               │  pure rules:
              │ chart · timing · judgement · score │  no OS, fully tested
              │ life · pills · hold · teams · peers│
              └─────────┬────────────────┬────────┘
                        │                │
      ┌─────────────────▼──────┐ ┌───────▼────────────────┐
      │  o2jam-audio           │ │  o2jam-shell           │  window + input
      │ lewton + cpal + clock  │ │ winit keymap + scenes  │  via o2jam-render
      │ mixer pending          │ │ sample-clock Playing   │  + o2jam-text
      └────────────────────────┘ └───────────┬────────────┘
                                             │
              ┌────────────┐  ┌──────────────▼───────────┐
              │ o2jam-net  │◀─│ recomp2jam               │  binaries
              │ wire protos│  │ connect/play/shots/trace │  + live session
              └────────────┘  └──────────────────────────┘
```

Pure leaves: `o2jam-file`, `o2jam-game`, `o2jam-render`, `o2jam-text`.
OS edges: `o2jam-shell/src/window.rs`, `o2jam-audio` (`CpalBackend`).
Wire: `o2jam-net/src/frame.rs`, `message.rs`, `play.rs`, `relay.rs`.

## Crates

- **`o2jam-file`** (now: `parse_m30`/`parse_omc`/new-variant entries +
  `decode_new_variant_pcm`, `parse_opi`, `.bnd` rects, ControlList ordinal
  pairing, `decrypt_ojn_new_format`, `parse_ojn_levels`, `build_level_chart`).
  Pure Rust, `thiserror` only, no OS.
- **`o2jam-game`** (now: `TimingWindows{6,18,25}`, `Score`, `CoolStreak`,
  `PillStock`, `LifeBar`/`LifeDeltas`, `HoldState`, `RoomTeams`, `PeerBars`,
  `Sequencer`). Pure, scenario-named tests.
- **`o2jam-audio`** (now: `AudioBackend` + `NullBackend` + `lewton` decode +
  live `cpal` backend with sample clock + resampler). Next: mixer, scene cues.
- **`o2jam-render`** (now: software compositor `Frame`/`Sprite`/`Rect`, RGB555
  blit + clip + colorkey, JPEG table driver). Next: `wgpu` upload.
- **`o2jam-text`** (now: CP949 decode + `ab_glyph` + system Noto Sans CJK KR;
  missing font degrades to ASCII). No test fails without the font.
- **`o2jam-net`** (now: frame/message/rooms/play/relay/shop/gift/wallet/
  character/strings/inflate + endpoint/opcode map). Shell fold: chat + peer-life
  only; the rest is documented, not wired.
- **`o2jam-shell`** (now: `ScreenGame`/`load_scene`/`ScreenFlow`, 7 main + 9 side
  scenes, `.bnd` staging, winit keymap, sample-clock Playing, trace hooks
  `screen_name`/`state_hash`/fixed clock). Composition FAILs stay in
  `docs/parity.md` until placement is pinned.
- **`recomp2jam`** (now: `connect`/`play`/`shots`/`online` + `session.rs` drains +
  `trace.rs` `--input-log`/`--frame-dump`; `fixtures/input-trace/` U1 harness).
  Next: windowed online as the default path.
- **`o2jam-dump`** (now: `identify` + hexdump + `run_*` dispatch over tested
  library fns). Later: full `dump-ojm`/`dump-ojn` printers.

## Key decisions

1. **Sample clock owns song time (approved deviation, ADR-0001).** The 7.55
   client paces from `timeGetTime`; rhythm parity needs sample-accurate audio
   time. The loop interpolates render positions from the audio clock and holds
   wall-equivalence inside the 6/18/25 ms windows.
2. **Parsers are total and bounded.** All file parsing is fallible,
   length-checked, and fuzzable (`cargo fuzz` later). No `unwrap` on
   untrusted bytes.
3. **Boost-isms become ownership.** `shared_ptr` webs become explicit
   owners + `Arc` only where threads share. Threads: `std::thread` now,
   `tokio` only if networking/loading needs it.
4. **CDX-isms become data.** Sprites/scenes become plain data + a wgpu
   renderer later — never Win32 calls inside game logic.
5. **Geometry is data-driven (ADR-0003).** Placements come from parsed
   archives + control codes; no new hardcoded rects.
6. **Live output beats archive bytes on conflict (ADR-0002).** The file stays
   recorded as contradicted, never silently applied.
7. **Risky statics need replay (ADR-0004).** Dossier alone never promotes an
   inert scan or exclusion rule to spec.

## Milestones

1. File parsers verified against installed 7.55 data, skip-clean without
   `install-root/` (hashes in `docs/progress.md`).
2. Null-backend autoplay of a chart reaches the expected score deterministically;
   real-window U1 trace (`fixtures/input-trace/`) replays green.
3. Windowed online client (login → channel → room → waiting → playing → result)
   runs on real input with an OS-boundary trace + smoke note; latency budget measured.
