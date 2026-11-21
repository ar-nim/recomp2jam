# Waiting caption sources (7.55) — MAPNAME/BPM/SKILLCOUNT/LEVEL pinned, INSTNAME/NOTICE open

Version pinned to 7.55 (`AngelO2Jam.exe` 5,029,888 B, sha256
`83a04541…72c823b`). File offsets (VA = file + `0x400000`). Method:
full-image LE32 scan per control code, then objdump behavior reads
(sweep 2026-11-07, offset-only scan, no new dossier — the precedent
`channel-fill.md` states for offset-only findings). No disassembly
reproduced.

## The captions

The `STATE_WAITING` block (`Interface1.opi`) carries five captions the
shell leaves unpainted (`MAPNAME`/`INSTNAME` per the
`waiting_music_caption` note in `paint.rs`, skill counts per
`paint_waiting_skills`; BPM and NOTICE have no painter at all):

| Caption | Control | Code |
|---|---|---|
| Song name | `IDC_TEXT_SWAIT_MUSICNAME` | `0x16400017` |
| Map name | `IDC_TEXT_SWAIT_MAPNAME` | `0x16400018` |
| Instrument name | `IDC_TEXT_SWAIT_INSTNAME` | `0x16400019` |
| BPM | `IDC_TEXT_SWAIT_BPM` | `0x16400069` |
| Skill counts | `IDC_TEXT_SWAIT_1/2/3SKILLCOUNT` | `0x1640005E..60` |
| Difficulty badge | `IDC_IMAGE_SWAIT_LEVEL` | `0x1650001B` |
| Notice | `IDC_IMAGE_SWAIT_NOTICE_TEXT` / `NOTICEBOARD` | `0x16500062/63` |

## Verdicts

- **MAPNAME — PINNED** (name table plus room word). Map-cycle path:
  lookup through `0x48bcf0` (file `0xb3e02`), store through `0x50f520`
  (file `0xb4067`). The string is the centered entry of the rdata
  table at file `0x249990` (`None`, `한가한대전장의오후`,
  `패자의역습`, `원하지않은결말`) indexed by the low word of room word
  `0x7789dc` (0 maps to 3; files `0xb3cc6`–`0xb3ce3`; the word is
  stored at file `0xb3b46` and confirmed to the server with 4002 per
  `stage-bga.md`). Applier `0x4ad350` path (lookup file `0xad6cf`,
  store file `0xad6fa`): the high-word branch centers `Random` (rdata
  file `0x2498a4`); the normal branch centers the table at file
  `0x2496b0` (`None`, `NoteFactory`, `NynusRoom`) indexed by
  `0x7789dc % 2` (0 maps to 2; files `0xad593`–`0xad5b0`) — the reason
  stays OPEN (OPEN 3). The packet path writes style only (helper
  `0x4b9ab0`, table-3 slot 9 of the handler tables pinned under
  `ev_aba36a3a…` per `channel-fill.md`; file `0xb9b72`, `0x50f6f0`
  color `0x191919`).
- **BPM — PINNED** (catalog column). Lookup through `0x48bcf0` (file
  `0xbdfd9`). No song stores the empty string through `0x50f520`
  (file `0xbe000`). Otherwise the song-record vector (base `0x779528`,
  stride `0x1b4`, files `0xbe0b6`–`0xbe0e4`) is scanned for the record
  whose id matches global `0x7789d8`, and the float at record `+0x10`
  (the `OJNList.dat` row `+20` BPM column per `file-formats/ojn.md`)
  formats with `%0.f` (rdata file `0x24a1d4`) and stores through
  `0x50f520` (file `0xbe1d5`; font setup `0x50f6a0` file `0xbe1c9`).
  Mismatch default `-0-` (file `0x24a1d0`) through `0x50f520` (file
  `0xbe1e4`).
- **SKILLCOUNT — PINNED** (inventory word). Three button handlers
  cycle dialog selection words (`1SKILLLEFT/RIGHT` `0x16200050/51`
  adjust `+0x35c`; `2SKILL` `0x16200052/53` adjust `+0x360`; `3SKILL`
  `0x16200054/55` adjust `+0x364`; files `0xbb7ac` / `0xbbb7c` /
  `0xbbf6c` regions). Each slot reads its owned-item vector
  (`0x77960c` / `0x77961c` / `0x77962c`, stride `0xC`; files `0xbb875`
  / `0xbbb49` / `0xbbf36`), stages art from the id at `+4`
  (`0x52e730` / `0x52e0d0`, frame 2 through `0x42f5a0`), and writes
  the count word at `+8` (`movzwl`, e.g. file `0xbba00`) as decimal
  (`0x62c109`, space-padded to width 3, slots 2–3 add a `999` check;
  files `0xbba6a`–`0xbbad5`) through `0x50f520` (files `0xbbb14` /
  `0xbbf0a` / `0xbc2fa`; lookups `0xbbaf8` / `0xbbeee` / `0xbc2de`).
  Style-only rect pass in the same table-3 helper (`0x4b9ab0`, loop
  files `0xb9af0`–`0xb9b6c`).
- **INSTNAME — OPEN** (OPEN 1). Full-image LE32 scan returns 0 hits
  for `0x16400019`: no code looks the control up, so any caption must
  arrive through a cached pointer or array iteration, both untraced.
  `SetInstrument` 4006 has no vendored shape in the repo (opcode only
  in `net-protocol.md`).
- **NOTICE — OPEN** (OPEN 2; control path pinned, content not).
  `NOTICE_TEXT` is imageless, so it paints nothing. Lookup file
  `0xbc9b8`; font and color setup through `0x50dc60` (file `0xbc9e6`,
  Korean face file `0x24407c`); content call `0x50b320` over dialog
  `+0x46c` (file `0xbca0f`). `NOTICEBOARD` lookup file `0xbca2c`,
  stored to dialog `+0x4f4`, frame 2 through `0x42f5a0` (files
  `0xbca52` / `0xbca5d`).

- **MUSICNAME - PINNED** (control plus rect, ticket #274). Local
  static data: `Interface1.opi` sha256
  `d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`,
  `ControlList_Interface.txt` entry slot 1595 (data offset `0x46fd7d6`,
  90472 B) names `IDC_TEXT_SWAIT_MUSICNAME 0X16400017` with no sprite -
  a TEXT control whose glyphs the shell fills. Code ordinal
  `(0x17 - 1) = 22` pairs it to `State_Waiting.bnd` (archive slot 1375,
  offset 62684300, 1862 B) rect 22 `(573,15)-(773,38)` = top-right of
  the 800x600 canvas. Shell source order: the master's staged pick
  first (it takes effect at once; the room mirror moves only on the
  server echo), else the joined room's song; the catalog supplies the
  title, else the bare `M{id}`. The shell-invented readout panel that
  used to ink title/difficulty/speed above the chat rows is gone - the
  difficulty half is the `LEVEL` badge below (ticket #275). Whether the
  exe writes the pick into
  its own room struct before the echo is OPEN (next step: a capture
  with a provenance manifest under #240); rect and caption control are
  static pins either way.
- **Caption plate and bearing - PINNED** (owner re-check 2026-11-18, #274
  second leg). The shell used `paint_room_detail_line` for `MUSICNAME`
  and `SPEED_NUM`, which filled `(x-4,y-2,text_w+8,9)` with near-black
  scrim. The 7.55 Wine capture `~/scratch/ab-wait.png` (800x600 game
  area) shows both captions bare on their staged plates: `MUSICNAME`
  rect `(573,15)` carries `디젤오 (O2Ver.1)[3K]` ink from `(579,22)`,
  and the `SPEED_NUM` chip shows `X1` with no dark rectangle. The
  shell now inks bare glyphs at that reference bearing `(6,7)` from
  the `.bnd` rect origin, scaled with the frame; the bearing is a GDI
  cell/presentation pin, not geometry. The speed *value* source is
  still OPEN (waiting speed cannot be changed yet), so only its plate
  is fixed.

- **LEVEL badge - PINNED** (control, rect, frame formula; ticket #275).
  Local static data: `ControlList_Interface.txt` names
  `IDC_IMAGE_SWAIT_LEVEL 0X1650001B` with sprite `Wait_Level.ojs`
  (no text), so the family letters and the speed suffix are the body's
  own pixels and the painter picks one frame. Code ordinal
  `(0x1B - 1) = 26` pairs it to `State_Waiting.bnd` rect 26
  `(218,12)-(273,40)` (55x28, the badge's own size), immediately left
  of `IDC_TEXT_SWAIT_ROOMNAME` rect 21 `(274,15)-(466,38)`.
  `Wait_Level.ojs` (Interface1.opi) carries 60 frames, every anchor
  `(0,0)`: five families of 12 speed cells, EX 0-11 (green), NX 12-23
  (orange-yellow), HX 24-35 (red), then the AX 36-47 and MX 48-59
  families; the cells run `0.5`, `1`, `1.5`, ..., `5`, `6`, `R` — the
  `GameSpeed` code order 0..11. Frame: the exe computes
  `12*difficulty + speed + 1` (difficulty byte `0x7789e7`, speed byte
  `0x7789e9`, `imul ecx,0xc` file `0x4b33f7`, `lea
  eax,[ecx+edx*1+0x1]` file `0x4b3405`; the same arithmetic repeats in
  the speed-button arms at files `0x4b7fe6`-`0x4b7ffb` and
  `0x4b80d1`-`0x4b80e6`) and stages it through setter VA `0x42f870`,
  which looks the image up one below its argument — so the painted
  frame is `12*difficulty + speed`. With no song the Waiting setup
  resets the control (file `0x4ba88d`, setter arg 0 = no image), so the
  badge paints nothing then. Shell source order matches the `MUSICNAME`
  caption: the master's staged pick (its difficulty slot plus the
  surviving speed), else the joined room's echo; the badge paints no
  rest stack (the exe's image control shows one frame). The speed half
  rides the same `JoinedRoom.speed` the `SPEED_NUM` caption paints, so
  both name one local speed; a per-member speed distinct from the room
  echo is not staged yet.
  Speed selector (#756, PINNED): `FUN_004b7c60` (file `0xb7c60`, EV
  `ev_75d1bc10…`) handles `IDC_BUTTON_SWAIT_SPEED_LEFT/RIGHT`
  (`0x16200064/65`): step the speed 0..=11, mirror it into the room-speed
  byte (`0x7789e9`), and repaint this badge frame through setter VA
  `0x42f870`; the handler sends no wire frame. `ScreenGame::step_waiting_speed`
  mirrors the visible speed into the joined room and a staged pick, so the
  badge and `SPEED_NUM` repaint from the same byte. Guards:
  `speed_button_steps_the_mirrored_speed_and_repaints_the_badge`,
  `speed_buttons_clamp_the_mirror_at_both_ends`. A captured click leg with
  every C2S frame and a before/after badge pair stays OPEN (the ticket's
  next step); the static pin does not assert exe local-only wire behavior.
  Two arithmetic branches stay OPEN and never paint in the shell:
  (a) an EX chart whose music-record EX level word (`+0x14`) exceeds 3
  shifts the family by `0x30` into the MX block (files
  `0x4b3372`-`0x4b33b7`) — the shell stages no per-chart level, so the
  next evidence step is a per-chart level source; (b) a selected-song
  dword with a set top nibble (`0x7789d8 & 0xf0000000`) always paints
  the AX family at `speed + 0x25` regardless of the difficulty byte
  (files `0x4b33d4`-`0x4b33e9`) — which writer sets that nibble is
  untraced (OPEN 5 states the global, not the flag).
  Guards: `staged_pick_badge_paints_the_level_frame_the_pick_names` and
  `room_without_a_song_paints_no_level_badge` (Tier 0 seam, synthetic
  cells), `live_waiting_badge_paints_the_bound_level_frame` (real
  archives, skips cleanly without game data).

## OPEN

1. `INSTNAME` source: zero code immediates; join one writer or record
   the negative. `SetInstrument` 4006 layout unpinned.
2. NOTICE content: what fills dialog `+0x46c` behind the `0x50b320`
   call (file `0xbca0f`).
3. MAPNAME applier semantics: why the normal branch of `0x4ad350`
   centers a `0x2496b0` entry (`None` / `NoteFactory` / `NynusRoom`)
   indexed by `0x7789dc % 2`.
4. SKILLCOUNT upstream: the server fill of the owned-item vectors
   (`0x77960c` / `0x77961c` / `0x77962c`).
5. Global `0x7789d8` — RESOLVED as the selected-song id (low 28 bits;
   top-nibble flag overlay). Writer sites: select-time song-id store file
   `0x3f379`, init default `0x22b8` (8888, "none") file `0x95137`,
   sentinel compares files `0xd8806`/`0x1719c0`, and high-half OR-compose file
   `0x15caa8`. Readers: the BPM full-dword record-id compare (files
   `0xbe0b6`–`0xbe0e4`, compare file `0xbe0d2`) and the play loader masked
   scan (`& 0x0FFFFFFF` file `0x1719d6`, scan files `0x171a8c`–`0x171ab8`).
   The old `stage-bga.md` channel-id label was wrong: the picker compares at
   files `0xbb324`/`0xbb330` are song ids 654/655, both present in the
   562-record `OJNList.dat` walk.

## Shell rule

All five captions stay unpainted until a live source lands in the
shell (BPM can use the catalog BPM of the selected song, MAPNAME the
stage table, SKILLCOUNT the item-vector counts). Those follow-ups
belong to shell code and stay untouched here.
