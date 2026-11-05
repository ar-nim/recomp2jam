# OJN (note chart) — loader located, format partially understood

## The two generations (verified)

- **Old format** (`Tutorial.ojn`, 118738 bytes): header starts
  `B8 22 00 00 6F 6A 6E 00` (`"ojn\0"` magic at offset 4). This is the
  community-documented layout (~300-byte header + note blocks).
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
Rosetta stone.

## Chart parser `FUN_00571d60` (file offset `0x171d60`) — REA dossier `ev_c0325c85…`

Signature `(song_id, ?, level 0..2)`. Flow, in own words:

- Reads the whole file (`FUN_00412890`), gates on the validator
  (`FUN_005718b0` — the `"ojn"` memcmp + BPM float check). Invalid →
  free, no parse.
- Per-difficulty slice: data offset from table `+0x814[level]`, event
  count from `+0x738[level]`, song BPM from `+0x708`.
- Event stream is float-anchored: each event = `float beat_time` +
  `u16 kind` + `u16 count` + `count` payload floats; pointer advances
  in 4-byte units.
  - `kind == 0`: BPM change (payload float, `0.0` means `1.0`); sets the
    measure tick length.
  - `kind == 1`: BPM list for tempo changes inside the slice.
  - `kind >= 2`: note events; lane index is `kind - 9` (7 lanes),
    payload entries are 32-bit values with bit 26 = long-note hold and
    low 12 bits = key-sound sample id (matches the OJM sample-id
    ranges `0–12` and `980–991`). Max lane tracked at `+0x838`.
- Timing: 192 ticks per beat (`0xC0`); milliseconds via `240000 / BPM`
  accumulators, float-rounded per event. Parsed notes land in the
  `+0x42c` / `+0x504` object arrays; holds in a 1000-byte flag table.

OPEN: the file bytes on disk are high-entropy but the parser reads
plaintext floats — so a decrypt step sits in the reader
(`FUN_00412890`, dossier pending) or the validator. That function is
the last unknown in the OJN chain.

## Open questions (in priority order)

1. New-format decrypt: reader `0x412890` or validator? (dossier in
   flight; crib with the float/`u16` layout above.)
   old-format field positions; keystream search at the loader branch.)
2. Note event encoding + difficulty slots per file.
3. Long-note + simultaneous-press encoding.
