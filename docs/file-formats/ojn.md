# OJN (note chart) — loader located, format partially understood

## The two generations (verified)

- **Old format** (`Tutorial.ojn`, 118738 bytes): header starts
  `B8 22 00 00 6F 6A 6E 00` (`"ojn\0"` magic at offset 4). This is the
  community-documented layout (~300-byte header + note blocks).
  Verified by walking: the `0x571d60` event layout
  (`i32 measure` + `u16 kind` + `u16 count` + payload) parses 43 clean
  events from offset 300, landing exactly on an embedded JPEG
  (`FFD8FFE0…JFIF`) at offset 11784 — the album art. Header offsets
  288/292/296 all read `11784` (image offset per level); offset 284
  reads `300` (header size); offset 64 reads `43` = the walked event
  count. Other header fields: id@0 (`8888`), `"ojn"`@4, BPM
  (`100.0f`)@16, per-level-looking counts at 28/40/52
  (`51/38/102`) — mapping TBD.
- **New format** (`o2ma*.ojn`, 13 files, 179–1037 KB): header starts
  `"new\x06…"`, high-entropy afterwards, NO `"ojn"` anywhere (except one
  incidental hit in `o2ma770.ojn` at 435160). Byte-period analysis shows a
  6-byte structural rhythm — consistent with either a repeating-key XOR or
  6-byte note records, TBD.

## Loader in `AngelO2Jam.exe` (file offsets, v7.55 sha256 `83a04541…`)

Evidence: `ev_66e97a17…` (loader dossier), `ev_68d920c2…` (dispatch).

- Old-format loader `FUN_004dc390` (file offset `0x9c390`): builds
  `MUSIC\<id>.ojn` + `MUSIC\TMP_<id>.ojn` (strings at file offsets
  `0x24abd0`/`0x24abc0`), stages a TMP copy, reads 300 bytes and
  `__stricmp`s header+4 against `"ojn"`. Equal → malloc, full read,
  `DeleteFileA(TMP)` (IAT `0x63f264` = `KERNEL32!DeleteFileA`),
  parse via `FUN_0041e830`. Not equal (the 13 `"new"` files) → close,
  return, **no parse**. So the new generation loads elsewhere.
- Dispatch `FUN_004e47a0` (file offset `0x8e47a0`) is the song-select
  flow (Trebuchet MS/Arial font setup, song vector stride `0x1b4`): it
  calls the old loader per song id and falls back to a
  `"MusicLoading.ojs"` overlay when the load returns 0. No new-format
  branch here either.
- Per-difficulty loader near file offset `0x170000`: maps type bytes
  `2/3/4` → difficulties `1/2/3`, builds `MUSIC\%s` (string at
  `0x24dc38`) + id, appends extension (`strcat`, `0x5f2800`), then calls
  the real chart parser `FUN_00571d60(path, mask, type, level, …)`.
  Ghidra dossier for `0x571d60` in progress — that function (plus the
  `0x1731b0` validator with its BPM float math) is where the `"new"`
  decode must live, if it exists.

## OJNList.dat (song index — VERIFIED layout)

`Image/OJNList.dat` (215848 bytes): 300-byte records. Record 0 is the
header (`562`, `100`, `"ojn"`); records 1+ are per-song entries with
title, artist, level tag and `o2ma###.ojm` filename (e.g. `<Bach Alive` /
`s.Doma` / `o2ma100.ojm`, ` on the fire[3M]`, `Out of Control` /
`RAVEN`, `NoteFactory`). Cross-check: `o2ma757` → record 437,
`o2ma770` → 450, `o2ma860` → 537. File holds 719 slots + 148-byte tail,
so the `562` count's exact semantics (playable filter? region subset?) is
TBD — next probe. Per-record numeric fields (BPM? levels? ids) still need
a field map; use record 437 (song 757, whose `.ojn` we also have) as the
Rosetta stone. First decode of record 437 (offsets in-record):
`@4 id=757`, `@8 "ojn"`, `@20 f32 BPM=124.0`, `@24 `0x50003``
(level flags?), `@32…` note counts per difficulty
(`3594/3679/3700`, then `312/401/505`, `69/69/…`), `@112` Korean
title + `" (O2 Ver.)[3K]"`, `@176–193` Korean artist + `"Penny)"`,
`@208 "NoteFactory"`, `@240 "o2ma757.ojm"`, tail has `129/129/129`
(per-difficulty levels? TBD), `300` (= record size) and two unknown
u32s. Cross-song check: song 770 reads BPM `168.0`, song 860 `103.0` —
the BPM field hypothesis holds.

## Chart parser `FUN_00571d60` (file offset `0x171d60`) — REA dossier `ev_c0325c85…`

Signature `(song_id, ?, level 0..2)`. Flow, in own words:

- Reads the whole file (`FUN_00412890`), gates on the validator
  (`FUN_005718b0` — the `"ojn"` memcmp + BPM float check). Invalid →
  free, no parse.
- Per-difficulty slice: data offset from table `+0x814[level]`, event
  count from `+0x738[level]`, song BPM from `+0x708`.
- Event stream is measure-anchored: each event = `i32 measure_tick` +
  `u16 kind` + `u16 count` + `count` payload `u32`s; pointer advances
  in 4-byte units. (The head word is an integer measure index, not a
  float beat: Tutorial.ojn ticks read 0/12/17/… as ints, new-format
  ticks run 1..~100 per slice. Kinds up to 30 observed.)
  - `kind == 0`: meter event (single `f32`, e.g. `0.75` = 3/4 time on
    o2ma769 at measure 21 — TENTATIVE, one float per song observed).
  - `kind == 1`: BPM list for tempo changes inside the slice — single
    `f32` = new BPM from that measure (o2ma765: `290.0` at 62 then
    `145.0` at 64; o2ma770: `100.0`/`75.0` at 70), or one `f32` per
    subdivision with `0.0` = keep (o2ma769 at 21: `67.0` then `134.0`
    at 16th 4). `999.0` terminators on o2ma770 (measures 71-72) TBD.
  - `kinds 2-8`: playable lane notes; lane index is `kind - 2`
    (7 lanes). Songs use either lanes {0, 2, 3} (10 songs) or all
    seven (o2ma860/864 and, per-level, neither 861/865 — same `.ojm`,
    3-lane charts). Max lane tracked at `+0x838`.
  - `kinds 9+`: autoplay/BGM channels, same value encoding (e.g.
    o2ma757 kind 19: BGM samples `981-992`, one per 4 measures;
    o2ma860 kind 23: `988-993`; dense 16th backing on kind 11).
  - Values: `0` = empty slot; else `(flags << 16) | (sample_id + 1)`.
    `value - 1` lands in the song `.ojm` sample-id set for every lane
    note on 12/13 new-format charts (one dangling ref on o2ma765:
    value 35, id 34 absent from the `.ojm`) and for autoplay notes
    modulo a few dangling high refs (`999`/`1000`-class on
    768/769/770/860/861/864/865, `0`/`1` on 770 whose `.ojm` starts
    at id 3). Counts: kinds 2-8 nonzero values per level equal the
    header counts at +40/+44/+48 on 13/13 charts × 3 levels (39/39),
    and all-nonzero (excl. kinds 0/1) equal +28/+32/+36 (39/39) — so
    +40-group = playable notes, +28-group = playable + autoplay.
    Observed subdivision counts are 1/16/32/192 (192 = full
    resolution, e.g. Tutorial BGM channel).
  - Flags (high word) semantics UNVERIFIED: observed `0x00`,
    `0xC0-0xF0`, `0x70/0xB0`, `0x200/0x300`, `0x400` (Tutorial BGM
    only). Bit 25 (`0x2000000`) is the leading hold-pattern
    candidate (paired `0x200`→`0x300` same-sample notes), but flagged
    spans routinely overlap the next same-lane note, so duration
    encoding is ruled out and pairing is unproven.
- Timing: 192 ticks per beat (`0xC0`); milliseconds via `240000 / BPM`
  accumulators, float-rounded per event. Parsed notes land in the
  `+0x42c` / `+0x504` object arrays (stride `0xC`); scalar payloads
  are staged in a stack array at the `8 + count * 4` walk.
- Hold marking in-exe (objdump survey, dossier TBD): the parser tests
  each staged payload word with `and eax, 0x4000000` (file offsets
  `0x172a4f` and, in the post-parse fix-up `FUN_00572f50` at file
  offset `0x172f50`, `0x173061`), masks `and edx, 0xFFF` (file offset
  `0x172a64`), and sets byte `id` in a 1000-byte stack flag table
  (store at file offset `0x172a6a`). Shipped note payloads never set
  bit 26 (only Tutorial-BGM `0x0400000N` words and kind-0/1 `f32`
  bit patterns do), so this path is live code whose on-disk trigger
  is essentially absent — hold decoding stays UNVERIFIED.

Validator `FUN_005718b0` (file offset `0x1718b0`) — dossier `ev_66e60d16…`:
returns 0 = valid, 1 = invalid. Requires size ≥ 300, `__stricmp(buf+4,
"ojn") == 0`, `u32(buf+0) == (song_id & 0xfffffff)`, per-level bound
`< 301`, a float check (`ROUND(x*10 - 0.9) < 27`), then a song-vector
scan (stride `0x1b4`) matching id + name + level field. (Fast path when
global id == `0x22b8` = 8888 = Tutorial's song id.) Pure validation +
table lookup — **no transform**. So the `"new"` decrypt, if any, is
between file read and parse: prime suspect the per-difficulty
pre-pass `FUN_00570f90` (called right before the parser in the
`0x170xxx` loader; dossier in flight).

## New-format decrypt — CRACKED (`FUN_00412180`, dossier `ev_011eab73…`)

The parser's read helper decrypts in place before returning the
buffer (old-format files pass through when byte 0 is not `n`):

1. Whole file is stored byte-reversed (`out[len-1-i] = in[i]`).
2. From the reversed tail: `period = rev[len-4]` (byte), keys
   `k1 = rev[len-7]`, `k2 = rev[len-6]`, `k3 = rev[len-5]`.
3. For each byte `i`: `i % period == 0` → XOR `k1`;
   `i % period == period/2` → XOR `k2`; else XOR `k3`.

Verified: `o2ma757.ojn` decrypts to `"ojn\0"` magic at offset 4,
song id `757`, BPM `124.0f`, and a 580-event walk from offset 300
lands exactly on an embedded JPEG at 40020 (Photoshop blob to
46530). o2jam-file implements this as `decrypt_ojn_new_format`
with an encoder-accurate round-trip test; `parse_ojn` decrypts and
validates the magic (note-table parsing next).

## Per-level segmentation — VERIFIED (all 14 `.ojn` files)

The decrypted header (300 bytes) locates three concatenated level
slices (offsets are file bytes):

- Event counts: `u32` at +64/+68/+72.
- Slice end offsets: `u32` at +288/+292/+296 (first slice starts at
  the `u32` at +284, which reads 300 on every file).
- Each slice head is `u32` tick + `u16` kind + `u16` count, then
  `count` payload `u32`s (`8 + count * 4` framing); walking
  `event_count` events lands exactly on the slice end.
- The last slice ends exactly on the first embedded JPEG (`end2 ==
  jpeg0`) on every file; empty trailing slices (`start == end`,
  count 0) cover Tutorial.ojn levels 1–2.

Verified end-to-end (walk lands exactly, counts match) on all 13
new-format files plus Tutorial.ojn:

| Song | Counts +64/+68/+72 | Ends +288/+292/+296 |
|---|---|---|
| 757 | 208/180/192 | 15036/27036/40020 |
| 758 | 168/171/176 | 21008/41932/63216 |
| 763 | 739/724/717 | 84356/152112/218984 |
| 765 | 579/582/552 | 41868/83652/123276 |
| 766 | 195/205/196 | 22732/45180/66732 |
| 767 | 197/175/161 | 13824/25764/36696 |
| 768 | 284/264/217 | 54092/119948/178324 |
| 769 | 310/295/289 | 36196/71012/105456 |
| 770 | 350/358/363 | 197256/392928/589244 |
| 860 | 321/375/426 | 22752/48792/78924 |
| 861 | 315/326/373 | 22320/45792/72168 |
| 864 | 396/459/462 | 28392/61020/93924 |
| 865 | 393/378/394 | 28176/54912/82800 |
| Tutorial | 43/0/0 | 11784/11784/11784 |

OJNList cross-check (13/13): decrypted file bytes +40/+44/+48 equal
the song's `OJNList.dat` record bytes +44/+48/+52 (list records carry
a leading field, shifting them 4 bytes; e.g. 757 reads 312/401/505
on both sides). The +28/+32/+36 group differs from the list +32/+36/+40
group (semantics TBD — not event counts). o2jam-file implements this
as `parse_ojn_levels` with walk-exact validation and synthetic-
fixture tests.

## Open questions (in priority order)

1. Per-note flag (high word) semantics: hold vs expression/volume.
   Bit 25 is the hold-pattern candidate; the in-exe bit-26 path
   (`0x172a4f`/`0x173061`) never fires on shipped payloads.
   Needs the `+0x42c`/`+0x504` object consumer + gameplay hold logic.
2. Judgement function: timing windows, scoring, combo — grade strings
   (`ALL COMBO`, `COOL 95%`, `MISS 10`, … at file offsets
   `0x247363`/`0x2473db`/`0x24765b`) are unreferenced by code (likely
   driven by `.ojs` result-screen scripts); `LongnoteMode` refs
   (`0x3a71c`, `0x1a4744`, …) are `option.ini` display settings, not
   judgement. Candidate region: the play-state update near `0x1a4xxx`
   plus `timeGetTime` (IAT `0x63f4f8`, 28 call sites) consumers.
   Windows and scoring stay UNVERIFIED in `o2jam-game`.
3. Kind-0 meter semantics (single `0.75` float) and kind-1 `999.0`
   terminators — one song each so far.
