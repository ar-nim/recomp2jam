# Waiting caption sources (7.55) — MAPNAME/BPM/SKILLCOUNT pinned, INSTNAME/NOTICE open

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
| Map name | `IDC_TEXT_SWAIT_MAPNAME` | `0x16400018` |
| Instrument name | `IDC_TEXT_SWAIT_INSTNAME` | `0x16400019` |
| BPM | `IDC_TEXT_SWAIT_BPM` | `0x16400069` |
| Skill counts | `IDC_TEXT_SWAIT_1/2/3SKILLCOUNT` | `0x1640005E..60` |
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
5. Global `0x7789d8`: the BPM path matches it against song-record
   ids, while `stage-bga.md` labels it channel id. Its ownership
   stays open.

## Shell rule

All five captions stay unpainted until a live source lands in the
shell (BPM can use the catalog BPM of the selected song, MAPNAME the
stage table, SKILLCOUNT the item-vector counts). Those follow-ups
belong to shell code and stay untouched here.
