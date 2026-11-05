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

`Image/OJNList.dat` (215848 bytes, sha256 `3be8ca00…`) starts
`3202 0000 6400 0000 6F6A6E00` — plausibly 562 entries + `"ojn\0"`.
Hypothesis: per-song metadata (BPM? level? id mapping `o2ma###`).
Next: split into 384-byte records (`215848/562 ≈ 384.2`) and cross-check
song 757's entry against its `.ojn`.

## Open questions (in priority order)

1. New-format header: XOR keystream or 6-byte records? (crib against the
   old-format field positions; keystream search at the loader branch.)
2. Note event encoding + difficulty slots per file.
3. Long-note + simultaneous-press encoding.
