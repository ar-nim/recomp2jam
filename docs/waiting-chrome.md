# Waiting chrome — static pins for the readout/seat/chat family

Version: O2Jam 7.55 `OTwo.exe`, 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
VA = file offset + `0x400000`. Tool: REA MCP on target
`target-83a045418b62.exe`, Ghidra 12.2 (2026-11-22). No disassembly is
copied here; behavior is described with file offsets and evidence IDs.
This note records only static pins; where an event is not observed in a
qualified capture, it stays OPEN with a named next step.

## Effect toggles (#771) — PINNED

- Global `0x7789e0` is the effect-state flag. Its ten static references
  are at files `0x9514b` (init), `0xb7e4e`/`0xb7e5a` (toggle writes),
  `0xba90d`–`0xba93d` (Waiting setup), `0xe7925`/`0xee6d5`/`0xf2a95`
  (Playing renderers). Evidence: `ev_7e3694f55f27a71569527aa79f8afb1fcc4858c3168d19d9b26a6dec145cb998`
  (address enumeration) plus the per-instruction reads of the same
  target. The init at file `0x914b` writes 1, so BOTTOM is the default.
- `FUN_004b7c60` arms both toggles as one group: it calls the toggle
  mapper with base `0x16201020`, count 2 and the pressed code, then
  writes 0 when the code is TOP (`0x16201020`, file `0xb7e4e`) and 1
  when it is BOTTOM (`0x16201021`, file `0xb7e5a`).
- `FUN_0048c810` (evidence `ev_c16da3e9754039c8ed1c179553f5c789b6813c6ad9ef78efea4bf4c32361d6b6`)
  walks the code run and calls its frame setter with 1 on the pressed
  control and 0 on the other — an exclusive pair, never two lit frames.
- `FUN_004ba6c0` (evidence `ev_ca7b37d824de6c51990857a3cdeddbdc1e68404271fb8040dc2e9fca1a4dd8ab`)
  sets `CHAT_ALL` active and selects `0x16201020 + flag` active at
  Waiting entry (files `0xba929`–`0xba94b`). Its unsigned bound checks
  are statically unreachable (Ghidra warns on the removed blocks), so
  only the writers above can name a valid control.
- The three Playing renderers read the flag into `flag + 1` as a note
  effect layer (files `0xe792b`, `0xee6db`, `0xf2a95`); with the high
  word of the room word set they derive the layer from RNG instead.
  The shell pins the numeric layer (`WaitingEffect::note_layer`) but
  does not claim a Playing pixel path: the exact sprite frame that
  layer selects stays OPEN (next step: one matched note-effect capture
  per toggle state).

## Shell rule

`WaitingEffect` in `screens/paint/waiting.rs` carries the two codes,
the default (BOTTOM), and the layer pin. `ScreenGame::press_waiting_effect`
commits only staged `0x16201020/21` rects from the live click chain;
`paint_waiting_effect_toggle` paints record 1 on the selected control
and record 0 on the other. No decorative layer is invented, and the
Playing consumer stays OPEN as above.
