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

## Seat state icons (#769) — PINNED refresh slots

The 7.55 `ControlList_Interface.txt` (sha256
`d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`,
`Interface1.opi` slot 1595) authors `SET 0x0C 0x00015000` with twelve
children: ReadyWindow, AvatarInfo (text), Avatar (image), BossMark,
ReadyText, WaitingImage, NoMusic, NoGem, PCRoom, Downloading,
DownBar, Playing (codes `0x1650002C..37`, with codes `0x2D`/`0x2E`
owned by the text/avatar children). `FUN_0050dd50` indexes that exact
child vector (evidence `ev_e6184e5bd8d9b8f1e0629447d86f4567aef7ba254c99efdd92af29c179ae90aa`),
so the indices in the frame applier `FUN_0052a2b0` (evidence
`ev_ee564ef6b796c08bc09c43e2f62b9a133759ed7650078043904d83e9ebea22a1`)
map to authored bodies.

- Refresh slot 7 (`Wait_NoGemImage.ojs`) paints frame 1 when the seat's
  `+0x217c` music-state byte is 2 (NoAccess). `FUN_004b2e30` sets the
  backing word through `FUN_0052ac60` (dossier
  `ev_87cb9191aafbd93164a29eb66ff55dae787e92aa7bd7fc5a1d17532db954b6bb`).
- Slot 8 (`Wait_PCRoomMark.ojs`) paints frame 1 on a nonzero `+0x212c`
  free-pass word (same refresh, `FUN_0052ac90`; field pin
  `docs/relay.md` §states).
- Slot 11 (`Wait_Playing.ojs`) paints frame 1 when `+0x2130`
  playing-state equals 1 (`FUN_0052acf0` writes the word, the applier
  tests `== 1`).
- Slots 9/10 (Downloading and its bar) share the word set from
  `+0x217c == 4`, the existing matched-ab-339 pin. Slot 3 (BossMark)
  is the #303 master emblem pin.

The shell stages NoGem and PC-room bodies per seat from those two wire
words; `Wait_ReadyWindow`, `Wait_BossMark`, `Wait_ReadyText`,
`Wait_NoMusicImage`, Downloading/bar and PLAYING keep their earlier
sources. A two-client capture isolating each icon byte stays OPEN.

## Chat tabs and messenger lamp (#770) — PINNED buttons, OPEN alert source

- `FUN_004b7c60` handles the four `CHAT_ALL/FRIEND/GUILD/WHISPER`
  buttons (`0x1620100E..11`) as one four-control group through
  `FUN_0048c810` (base `0x1620100e`, count 4): exactly one tab lights.
  Every arm writes zero to dialog `+0x298`, and the WHISPER arm zeroes
  a 256-byte buffer at `+0x18c` and posts the literal at `0x647d70`
  (instructions `0xb7d5d`-`0xb7e29`). The shell models the first two
  words as a tab switch plus history/draft clears; the literal is a
  presentation detail, not a wire source.
- The two messenger lamps `0x1650006B/6C` are written together by
  `FUN_004c0690` (evidence
  `ev_21ff56de7431c8573df8bfc6011e0873031bd986430c5b4dce71425ea753bfe8`
  for its callers): a non-zero argument starts an 800 ms animation on
  each idle lamp, zero resets both to their rest frame. Its direct
  callers are one Waiting dispatcher case — MESSENGER `0x1620006a`
  calls it with 0 (file `0xb81e0`) — and six messenger-dialog callbacks
  at files `0x97df0`/`0x980b0`/`0x98870`/`0x99130`/`0x99780`/`0x9a280`.
  No live notification reaches the shell, so the ON edge is a named
  seam; the OFF edge is wired to the staged MESSENGER press.
- Which wire rows belong to which tab is still OPEN: no row-filter use
  of the `+0x298` word was resolved statically. Next step: one tapped
  tab-switch capture with two differently addressed chat rows.

## WAITING stamp, change-title and emotion (#774) — PINNED data and arms

- `ControlList_Interface.txt` authors `IDC_IMAGE_SWAIT_WAITTEXTIMG
  0x16500067` (`Wait_Btn_WaittextIMG.ojs`), `IDC_IMAGE_SWAIT_WAITIMG_ANI
  0x16500068` (`Wait_Btn_WaitIMG_Ani.ojs`),
  `IDC_BUTTON_SWAIT_CHANGE_TITLE 0x16200047` (`Wait_ChangeTitle.ojs`)
  and `IDC_BUTTON_SWAIT_EMOTION 0x16200048` (`Wait_Btn_Emotion.ojs`).
  The earlier scan missed the two button codes because they are
  compared as computed dispatcher cases, not pushed as immediates.
- Waiting setup `FUN_004ba6c0` shows the wait-text image (setter
  `FUN_0042f870(1,0)` paints its first record) and starts the animation
  control (`FUN_0042f5a0(3)`, then `FUN_0042f400(0,0x4b,0,0)`) when a
  seat other than the local one carries playing-state 1 (files
  `0xba7b7`-`0xba89f`). Waiting update `FUN_004b9dc0` clears the text and
  stops the animation every 5000 ms while `0x778a54 == 0` (files
  `0xba60e`-`0xba68a`, dossier `ev_87cb91...`). The shell pins the arm
  and the 5-second window, not the animation's individual frames.
- `FUN_004b2540` case `0x16200047` posts `(4<<16)|8` when the local
  master byte is set (file `0xb26ac`); `FUN_004b81e0` case `0x16200048`
  posts `(0xb<<16)|0x48` for every seat. `FUN_00478c90` composes the
  word and posts it through `FUN_0047edf0` (dossier
  `ev_63d154f8e8c9f6d017c1d16e69eb3b4fb973946af8988956945420e10838e44e`).
  `Dialog_ChangeTitle.bnd` and `Dialog_EmotionHelp.bnd` are unmounted
  (`docs/bnd-consumption.md` E bucket), so the shell records the pinned
  request and does not invent a sheet. A dwell capture of the animation
  stays OPEN.

## Shell rule

`WaitingEffect` in `screens/paint/waiting.rs` carries the two codes,
the default (BOTTOM), and the layer pin. `ChatState` carries the selected tab and the lamp pulse; tab codes
come only from staged rects, and the MESSENGER press resets the lamps.
The tab painter lights one authored record and clears the history (and
the whisper draft) on the same click. No row is invented or hidden by
a tab until the filter source is pinned.

`ScreenGame::press_waiting_effect`
commits only staged `0x16201020/21` rects from the live click chain;
`paint_waiting_effect_toggle` paints record 1 on the selected control
and record 0 on the other. No decorative layer is invented, and the
Playing consumer stays OPEN as above.
