# OPI/OPA archives — VERIFIED

Image/script archives (`Image/*.opi`, `Image/avatar.opa`). Flat
container: data blobs first, trailing entry table at EOF.

## Layout

| Offset | Size | Meaning |
|---|---|---|
| +0 | u32 | version: 2 (`.opi`), 1 (`.opa`) |
| +4 | u32 | entry slot count |
| +8 | u64 | zero |
| +16 | … | file data blobs, concatenated |
| EOF−`152*count` | `152*count` | entry table |

## Entry (152 B)

| Offset | Size | Meaning |
|---|---|---|
| +0 | u32 | kind: 1 = file, 0 = free slot |
| +4 | 128 B | name, NUL-padded ASCII (`*.ojs`, `*.oji`, `*.ojt`, `*.oja`, `*.bnd`, `*.txt`, `*.dat`) |
| +132 | u32 | data offset from archive start |
| +136 | u32 | stored size |
| +140 | u32 | aux size (meaning TBD) |
| +144 | 8 B | zero pad |

No compression observed (`size == aux_size` for most; structured
`55 05`-magic binary bodies otherwise — never high-entropy).
One 32 B overlap exists in `Interface1.opi`
(`State_MShop.bnd`/`SMI_790.ojs`); entries are consumed as-is.

## Verified inventories (real 7.55 files, counts only)

- `Interface1.opi` (75,100,702 B): 1596 slots → 1587 files
  (1531 `.ojs`, 49 `.bnd`, 5 `.oji`, 1 `.ojt`, 1 `.txt`), 9 free.
- `Playing1.opi` (221,737,026 B): 633 slots → 627 files
  (338 `.ojt`, 275 `.ojs`, 5 `.oji`, 4 `.bnd`, 3 `.oja`,
  1 `.dat`, 1 `.txt`), 6 free.
- `avatar.opa` (144,761,749 B): 7149 slots → 7134 files, 15 free.

## Implementation

o2jam-file: `parse_opi` (entries with `name/kind/data_offset/
size/aux_size`; nameless slots skipped; bounds-checked) +
`o2jam-dump opi` for local listing. Entry data offsets verified
by extracting `ControlList_Interface.txt` (comment header).
Inner script/binary formats (`.ojs` text vs `55 05` bodies,
`.bnd` sections) are the next mapping target.

## Open questions

1. `aux_size` semantics when it exceeds `size`.
2. Type-0 free slots with nonzero offset/size2 (reserved regions?).
3. The 32 B overlap: encoder sloppiness or intentional sharing?
4. Inner formats: `.ojs`/`.ojt`/`.oji`/`.oja` text vs binary,
   `.bnd` structure, `JamData.dat`/`Itemdata_Korea.dat` tables.
