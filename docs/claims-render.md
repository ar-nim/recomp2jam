# Render-claim verification — exe consumer sites (7.55) — VERIFIED

Verdicts for the five render rules in `docs/opi-pattern.md` /
`docs/bnd-semantics.md`, each pinned to the exe site that consumes it.
Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`;
VA = file + `0x400000`. Method: read-only headless Ghidra 12.1.2
(`extracted/ghidra-4015-work/proj4015.gpr`) opened read-only by a
standalone reader, plus read-only python byte scans and live archive
parses. No disassembly or decompiled code reproduced. Dossier ids from
`extracted/rea-ui-loader-dossier.json`; offsets marked **new** are
unfiled evidence pending a dossier record.

## Verdicts

1. **Sprite anchor x/y is the blit destination — REFUTED as stated.**
   The anchor is a per-frame offset *added to* the draw position
   (**new** `0x41DF80`: draw x/y plus s16 at frame record +0/+2, frame
   stride `0x14`, table at DXSurface `+0x120`, current frame `+0x134`).
   Destination = control position + anchor. The anchor==rect probes
   hold because those entries set bit 28 (`0x21100001`,
   `0x21200002`, `0x1B200001`) and the control then draws from origin
   (**new** `0x4F60C0`: `code & 0x50000000 ≠ 0` → position `(0,0)`).
2. **`.bnd` rect positions the blit, it does not clip — CONFIRMED
   (position half).** Wrapper fetches the rect and stores it
   (`ev_1be2d784…`, `ev_3a869bb7…`; **new** `0x50DC90` memcpy to
   `+0x3C`); CCButton init feeds x1/y1 to the sprite position setter
   (**new** `0x4F60C0` → `0x42FC70`, stored `+0x4`/`+0x18`);
   scrollbars store all four (**new** `0x50E320`, `+0x68`–`+0x74`);
   one class centers the sprite on the rect center (**new**
   `0x50B060`). Clipping is viewport bounds (`+0x4E`–`+0x51`,
   **new** `0x42B5E0`), never the `.bnd` rect.
3. **Draw order is entry file order — CONFIRMED.** Applier walks
   entries 0..n−1 (`ev_9e526b62…`), push appends (`ev_1be2d784…`);
   paint loops ascend the child array index 0..count−1 calling child
   vtable `+0x24` (**new** `0x50DEA0`, `0x50DDF0`). Topmost-last is
   now pinned at the consumer (closes `docs/opi-pattern.md` OPEN 1
   at static level).
4. **Colorkey 0x0000/0x7D5F is the transparency test — CORRECTED.**
   The test is a DirectDraw source colorkey taken from the file's u16
   key field: `DXSurface::Load` (**new** `0x4249F0`; asserts
   `.\Display\DXSurface.cpp`) reads magic `0x0555`/`0x555A` at
   `+0x11A`, count `+0x11C`, key `+0x11E`; on 16bpp it re-encodes
   555→565 (`(k & 0x1F) | ((k & 0x7FE0) << 1)`: `0x7D5F` → `0xFA9F`,
   `0x0000` stays 0). Key ≠ 0 installs `SetColorKey` with low==high
   and key-type arg 8, SRCBLT (**new** `0x422F60` → `0x42BA00`); blit
   flag word `0x11` (**new** `0x42B450`). The literal pair
   `0x7D5F`/`0xFA9F` picked by display depth (== 15 → `0x7D5F`,
   **new** `0x50BE81`, `0x50CB81`, `0x5137E1`, `0x51420A`,
   `0x51429A`; depth reader `0x415CC0`, manager `+0x50`) is a
   clear/fill color in unrelated UI classes, not the `.ojs` test.
5. **Zero-low-word entry codes take no rect — CONFIRMED with two
   corrections.** The mask is the low **12** bits, not 16
   (`ev_3a869bb7…`, `0x548640`: ordinal = `(code & 0xFFF) − 1`);
   a zero ordinal underflows and fails the u16 bound, so the entry
   receives an **all-zero rect** `(0,0,0,0)` (stored via `0x50DC90`)
   rather than being skipped; it consumes no table slot. Block-self
   entry 0 is the observed case.

## OPEN

1. DD blit flag decode: the `0x11` flag word at `0x42B450` and the
   COM-slot mapping of the surface call (`vtable +0x1C`) are unpinned.
2. The loop that paints the screen's own pushed child vector
   (`0x48A060`/`0x48A5A0` → push) — the generic child-array paint is
   pinned, the screen-level consumer is not.
3. Whether any caller feeds `DXSurface::Draw`'s optional clip param
   (`0x42B5E0` last arg).
4. Arm ↔ RTTI class-name join for all 16 factory arms (`docs/ui-loader.md`
   OPEN 1); this pass confirmed named vftables for `CCButton`,
   `CCImageBox2`, `CCListBox`, `CCTextBox` only.
5. **New** offsets in this note are not yet filed as REA dossier
   records — file a dossier before implementing.
