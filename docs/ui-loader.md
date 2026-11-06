# UI loader chain — ControlList text → blocks → `.bnd` → controls (7.55) — VERIFIED

How the 7.55 client loads `ControlList_*.txt`, parses blocks, binds
`.bnd` rect tables, and creates controls. Closes the loader/applier
address gap left open in `docs/opi-pattern.md` (OPEN 4) and
`docs/bnd-semantics.md` (OPEN 1). No disassembly or decompiled code is
reproduced; every step is described in own words with file offsets
(VA = file + `0x400000`) plus evidence ids from the local dossier
`extracted/rea-ui-loader-dossier.json` (format `rea-dossier-bundle-v1`,
dossier `rea-ui-loader-controllist`, 9 records, all `observed`).

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`
(both `install-root/` and the analyzed `extracted/` copy verified
byte-identical). Method: read-only headless Ghidra 12.1.2 project
(`extracted/ghidra-4015-work/proj4015.gpr`, import 2026-11-06) opened
read-only by a standalone decompiler-dump reader
(`extracted/ghidra-4015-scripts/DumpControlList.java`; scratch in
git-ignored `extracted/ghidra-controllist*/`), plus read-only python
byte scans and CP949 text extraction (`ControlList_Interface.txt`,
slot 1595, 1,979 lines) over the installed `Interface1.opi`.

## The chain

1. **Archive init** `0x475990` — opens `Image\Avatar.opa`,
   `Image\Interface1.opi`, `Image\Playing1.opi` in that order and
   registers handles into archive slots 0/1/2 (slot guard `< 3`,
   `0x479840`; select/release bracket `0x4795A0`/`0x479750`, 57/56
   callers, current slot stored at `+0xB20`). Then calls the loader
   (`ev_d75a596c…`).
2. **Load entry** `0x475A80` — parses two ControlList texts into one
   global parent rooted at `0x762B44`: states block array at
   `+0xF80` (0x1068 B = 15 × 0x118), dialogs array behind it at
   `+0x1FE8` (0x1D88 B = 27 × 0x118); second text buffer at
   `+0x1958` (`ev_d75a596c…`).
3. **Block-list parser** `0x410F40` — line reader plus `sscanf`
   against the keyword block at file `0x243F34`; `NUMBER_OF_STATE`
   → states output, `NUMBER_OF_DIALOG` → dialogs output; arrays
   zeroed at the two fixed sizes above. Missing text falls into an
   archive pick by `stricmp` against the three literal names plus an
   error dialog (`ev_38eab6b2…`).
4. **Block reader** `0x411310` — header line `%s%x%x` gives the
   declared entry count (`+0x08`) and declared SET count (`+0x10`)
   of a 0x118-stride block record; allocates the entry array
   (count × 0x11c → `+0x0C`) and SET pointer array (count × 4 →
   `+0x14`). Reads the `BOUND "stem.bnd"` line (`%s%s`; stem with
   quotes → `+0x18`, 256 B) and the block-id line (`%s%x` →
   `+0x04`), then hands members to the entry reader. A `}` line is
   consumed at the end (`ev_f4662376…`). Verified against the live
   text: 38 `BOUND` stems, `STATE_CHANNEL` header `0x27 0X01`.
5. **Entry + SET readers** `0x4115F0` / `0x411890` — member line
   `%s%x%s`: code u32 → entry `+0x00`, optional string → `+0x04`
   (0x11c stride). `SET` lines do not consume entry slots; each
   allocates a 0x118 set object (ctor `0x50E070`) with member count
   (`+0x08`), match key (`+0x04`), child entry array (`+0x0C`),
   parsed by recursion. Trailing parameter-less SET runs fill the
   remaining declared slots (`ev_2fdbec57…`).
6. **Bind loaders** — `0x548450` (out-table, 2 callers: the avatar
   appliers) and `0x547DB0` (into-record, 5 callers incl. playing
   screen and state screens). Both: archive lookup (assert anchor
   `.\Input\InputManager.cpp`, lines `0x273`/`0x250`), fetch, read
   u32 magic; on `0xFFFFFFFF` read u16 count, alloc count × 0x10,
   bulk-read. Into-record variant stores rect table at `+0x0C` and
   count at `+0x10` of the current record of a screen vector
   (`+0x40` vector, `+0x4C` index). Literal stems only for
   `Avatar_Info.bnd`/`Map_Avatar.bnd` (file `0x24AB9C`/`0x24ABB6`)
   and `State_Playing_network.bnd` (file `0x24B080`); all other
   stems flow from block `+0x18` (`ev_3eb885cd…`).
7. **Rect pairing** `0x548640` — **rect ordinal = `(entry.code &
   0xFFF) − 1`**; bounds-checked against the u16 count; miss →
   all-zero rect. Copies 4 dwords (x1, y1, x2, y2, 0x10 stride).
   Write-back helper `0x5487B0` persists parent-offset-adjusted
   rects. This pins the mechanism behind `docs/bnd-semantics.md`
   step pairing: the file-order rule on 38 blocks is the consequence
   of sequential ordinals (`ev_3a869bb7…`).
8. **Control creation** — factory `0x48B2D0` dispatches on
   `code & 0xFF0000` to 16 ctor arms (allocations 0x60…0x20c);
   wrapper `0x48A620` creates the control, fetches its rect from the
   bind table when block `+0x18` is non-empty, matches the SET group
   by `code & 0xFF000` against set `+0x04`, recurses for SET
   children, stores via vector push `0x48CA20`. The playing screen
   also feeds stack-built synthetic 0x11c entries (e.g. code
   `0x1B200001`, sprite `Playing_Exit.ojs`) into the same factory —
   the runtime-resolution half of the `_`-prefix rule
   (`ev_1be2d784…`).
9. **Appliers** — state open `0x48A060`: block =
   `0x762B44+0xF80+(n−1)×0x118` (n 1-based), archive slot 1 bracket,
   bind if stem non-empty, one control per entry with
   `code & 0xFF0000 ≠ 0`. Avatar dialogs `0x4DA630`/`0x4E4EE0`:
   archive slot 2, load `Map_Avatar.bnd`/`Avatar_Info.bnd`, zip rect
   tables against entries 0–7 / 8–15 filtered by the same mask and
   an 8-member presence table, write control `+0x1E4`/`+0x1E8`;
   `0x4DA630` subtracts the dialog origin at apply time — dialog
   rects are screen-absolute, client offsets are computed here
   (resolves `docs/bnd-semantics.md` OPEN 2 for the avatar dialog).
   Playing `0x4F1140`; `State_Playing_network` path `0x4E5D90`
   (both virtual-dispatched, no direct callers) (`ev_9e526b62…`).
10. **Archive open helper** `0x4795E0` — named member → 0x104-B file
    object; failure asserts with `.\Interface\State.cpp` line `0x39A`
    (string cluster file `0x247958`–`0x2479A4`). Lookup `0x4797D0`
    (12 callers) / fetch `0x479790` (10 callers) are the archive API
    under every step above (`ev_3f37301b…`).

## Record layouts (parser-consumed)

| Record | Stride | Fields |
|---|---|---|
| block | 0x118 | `+0x04` block code, `+0x08` entry count, `+0x0C` entries ptr, `+0x10` SET count, `+0x14` SET ptr array, `+0x18` bind stem (256 B, quotes kept) |
| entry | 0x11c | `+0x00` code u32, `+0x04` string |
| set | 0x118 | `+0x04` match key, `+0x08` member count, `+0x0C` children ptr |
| rect | 0x10 | x1, y1, x2, y2 |

Note: the into-record bind loader reuses block `+0x0C`/`+0x10` of the
screen-held record for the rect table, and header SET declarations are
allocated eagerly at parse (settling `docs/opi-pattern.md`'s
"trailing parameter-less `SET` runs" question on the client side:
they fill the declared `+0x10` slots).

## Dossier

`rea-ui-loader-controllist` (headless Ghidra function dossier for the
parser entry and the full chain), record ids:

- `ev_38eab6b2…` parse-list-entry `0x410F40`
- `ev_f4662376…` parse-block-entry `0x411310`
- `ev_2fdbec57…` parse-entry-set `0x4115F0`/`0x411890`
- `ev_d75a596c…` loader-entry `0x475A80` (+ archive init `0x475990`)
- `ev_3eb885cd…` bind-loader `0x548450`/`0x547DB0`
- `ev_3a869bb7…` rect-pairing `0x548640`/`0x5487B0`
- `ev_1be2d784…` control-factory `0x48B2D0`/`0x48A620`/`0x48CA20`
- `ev_9e526b62…` screen-appliers `0x48A060`/`0x4DA630`/`0x4E4EE0`/`0x4F1140`
- `ev_3f37301b…` archive-open-helper `0x4795E0`

## OPEN

1. Factory dispatch arm ↔ `CC*` RTTI class-name join: ctor VAs are
   pinned, but the class names from the RTTI cluster at file
   `0x27E705`–`0x27E7C8` are not joined to arms yet. The
   `CControlList` type descriptor (name file `0x27E7C8`, TD base VA
   `0x67E7C0`) is referenced by two locator structures near VA
   `0x668370`/`0x668394`, but no code reference to either locator was
   found by byte scan, and the Ghidra RTTI analyzer produced no
   symbols in this project — vftable/constructor identity open.
2. Dialog-open applier `0x4ED650` not yet read (309-line body); the
   generic dialog path for the 27-dialog array (`+0x1FE8`) beyond the
   avatar dialog is unpinned.
3. Dialog-origin rule: subtraction observed only in `0x4DA630`
   (avatar dialog); whether every `Dialog_*.bnd` screen applies it
   the same way awaits the `0x4ED650` trace.
4. Entry `+0x04` string consumer at create time (first half of
   `0x48A620` not fully traced); sprite resolution rules stay with
   `docs/opi-pattern.md` step 9.
5. Paint order (topmost-last) — unchanged from `docs/opi-pattern.md`
   OPEN 1; nothing in this chain observes z-order beyond background
   entry ordering.
6. `0x4E5D90` (408-line playing/network body) has no direct callers;
   dispatch context (virtual table slot) not traced.
