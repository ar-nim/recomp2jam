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

- `MUSIC\%sojn` → copied to `MUSIC\TMP_%sojn`, 0x12c (300) bytes read,
  first-300-bytes scanned for `"ojn"` (`"ojn\0"` string at file offset
  `0x24abbc`; validator at file offset `0x1731b0` also memcmps 300 bytes
  against `"ojn"`, then does float timing math — BPM conversion against
  constants at `0x24ce00`/`0x24a4e8`).
- Branch at file offset `0x9c2d7` (`test/strstr` result): `"ojn"`-absent
  headers (the 13 new files) take the main load path
  (copy → `DeleteFileA` on the TMP copy — IAT slot `0x63f264` =
  `KERNEL32!DeleteFileA`, hint 131 — → internal parse at `0x41e830`);
  `"ojn"`-present headers take the other branch. So this loader handles
  **both** generations; start RE at file offset `0x9c390` (function entry,
  `ret $0x10`, thiscall) and the validator at `0x1731b0`.
- No external decryptor: imports are the classic CDX-era surface —
  `DDRAW!DirectDrawCreateEx`, `DINPUT`, `DSOUND`, `WINMM`, `WS2_32`,
  `WININET`, `IMM32`, `SHLWAPI` — so the new-format transform (if any),
  if it exists, is inline in the exe, near the loader above.

## OJNList.dat (plaintext index?)

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

## Open questions (in priority order)

1. New-format header: XOR keystream or 6-byte records? (crib against the
   old-format field positions; keystream search at the loader branch.)
2. Note event encoding + difficulty slots per file.
3. Long-note + simultaneous-press encoding.
