# OPI/OPA archives — VERIFIED

Image/script archives (`Image/*.opi`, `Image/avatar.opa`). Flat
container: data blobs first, trailing entry table at EOF.

Version pinned to 7.55 client files (sizes + SHA-256):

| File | Size | SHA-256 |
|---|---|---|
| `Interface1.opi` | 75,100,702 | `d6654119…bdd50584` |
| `Playing1.opi` | 221,737,026 | `8c1c7566…aed9eed40` |
| `avatar.opa` | 144,761,749 | `ced3ed1e…2953b16` |

(Analysis method: static structure walk over the installed files
with Python 3.14 — offsets and counts below, no disassembly.)

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
| +136 | u32 | stored size: the body is always the first `size` bytes |
| +140 | u32 | on-disk stride (`aux_size`): gap to the next blob's offset |
| +144 | 8 B | zero pad |

`aux_size` semantics (open question 1, resolved): for ~99.5% of
slots, `aux_size == next_offset − data_offset`, i.e. the reserved
stride; bytes in `[off+size, off+aux)` are zero padding. The last
slot's stride runs to the data-region end (start of the entry
table, e.g. `ControlList_Interface.txt`: `off 74438614 + aux
419496 == table start 74858110`). A handful of stale slots
disagree with the gap (notably several `avatar.opa` bodies whose
`aux_size` overshoots the next offset), plus the known 32 B overlap
in `Interface1.opi` (`State_MShop.bnd`/`SMI_790.ojs`); readers must
slice by `size`, never by `aux_size`. Free (kind-0) slots are
deleted-entry reservations: nonzero `offset`/`aux_size` with no
body (open question 2, resolved — e.g. `Interface1.opi` slot 672
reserves `[31694688, 31777792)` between two named blobs).

## Verified inventories (real 7.55 files, counts only)

- `Interface1.opi` (75,100,702 B): 1596 slots → 1587 files
  (1531 `.ojs`, 49 `.bnd`, 5 `.oji`, 1 `.ojt`, 1 `.txt`), 9 free.
- `Playing1.opi` (221,737,026 B): 633 slots → 627 files
  (338 `.ojt`, 275 `.ojs`, 5 `.oji`, 4 `.bnd`, 3 `.oja`,
  1 `.dat`, 1 `.txt`), 6 free.
- `avatar.opa` (144,761,749 B): 7149 slots → 7134 files, 15 free.

## Inner formats

### Text bodies — CP949, not UTF-8/EUC-KR

`.ojs` word lists (`abusedata.ojs`, `abuse_NickNamedata.ojs`) and
`ControlList_*.txt` are Korean-locale text: UTF-8 decoding fails on
the first high byte; strict EUC-KR rejects bytes the client uses
(e.g. `ControlList_Interface.txt` offset 36053); CP949 (the
MSVC2005 Korean codepage) decodes every text body without error.
`o2jam-file::decode_control_text` implements this via
`encoding_rs::EUC_KR` (WHATWG euc-kr covers CP949).

### Sprite-55 bodies (`.ojs`/`.ojt`/`.oji`) — VERIFIED

The dominant body (9234/9234 observed): `tag + 0x0555 + count +
colorkey`, then `count` 20-byte image records and one
concatenated pixel blob.

Header (8 B): `u16` tag (observed 0, 1; 45 tag-0 bodies),
`u16` magic `0x0555`, `u16` image count, `u16` colorkey
(`0x0000`: 3444 bodies, `0x7D5F`: 5787 bodies).

Record (20 B): `u16 x`, `u16 y` (placement anchor), `u16 w`,
`u16 h`, `u32 data_offset` (from blob start), `u32 size`,
`u32` zero. Verified on every body: offsets chain exactly
(`data_offset[i+1] == data_offset[i] + size[i]`) and
`8 + 20*count + Σsize == len`.

Pixels: `size == w*h*2` means raw little-endian RGB565
(63742 images; `Sprite55::is_raw_rgb565` + `rgb565_to_rgb888`).
Colorkey regions (`0x7D5F` runs) read as flat transparent art;
e.g. `Shop_ItemGem.ojs` (9x12, 216 B) and `OptionKey_Checkbox.ojs`
(16x16 at `(51, 400)`, 512 B) account byte-exact.

Outlier classes (same envelope, non-raw blobs, parsed but not
pixel-decoded): `.oji` list bars with per-image sub-directories
(`SelectAlbum_Bar.oji`: 15 records + 20 B sub-records),
`RLE`-style compressed frames (`Shop_MybagSelect.oji`: 2014 B
stored for 86x69), prefixed frames (`Lanking_NoticeBoard.oji`:
800x31 + 188 B header), and the three strided combo atlases
(`Note_Combo4/5/6.ojt`: 6 records of 64x64/8192 B, blob holds 4
consecutive RGB565 glow frames per record — 128 + 6·32768 =
196736 B exact; `parse_sprite55_strided(_, 4)`, digit bands
rendered to confirm).

### `.oja` animation envelopes — header verified, codec TBD

`tag + 0x555A + count`, then `count` 20-byte frame records
(`i16 x/y`, `u16 w/h`, three raw `u32` words `a/b/c`).
`Note_Click.oja` (87875 B): 6 frames shrinking 144→39 px around
(−72,−72)→(−19,−20) — coherent click-effect geometry; the
`a`/`b`/`c` words and frame-data codec are TBD.
`parse_oja` checks the envelope only (3/3 real files).

### `.bnd` bind sections — VERIFIED

`u32` magic `0xFFFFFFFF`, `u16` count, then `count` 16-byte
`(u32 x1, u32 y1, u32 x2, u32 y2)` corner rects in 800x600 space;
`6 + 16*count == len` holds on all 53 real sections (49 + 4).
`parse_bnd` enforces magic + exact size.

### `ControlList_*.txt` — role mapped, block↔rect relation verified

UI state/dialog definitions, not scripts: `NUMBER_OF_STATE`
(`0x0F` = 15; file holds 16 `STATE_` headers of which
`PLAYING`/`TUTORIAL`/`DRUM_PLAYING`/`EXIT` are empty `0x00`
stubs) and `NUMBER_OF_DIALOG` (`0x1B` = 27, exact). Each block is
`NAME 0xN [0xM] { BOUND "stem.bnd" IDC… [SET … { IDC… }] }`;
entries are `IDC_NAME 0XCODE ["sprite.ojs"]` with `//` comments.
The header comment reads (CP949): "sets control structure and
links used images".

Rect mapping: `.bnd` record `i` lays out block entry `i + 1`
(entry 0 is the block itself) — verified on all 38 Interface
blocks with a matching `.bnd` (`entries − 1 == rects`), e.g.
`DIALOG_EVENT` (4 entries → 3 rects: frame `(30,3,612,595)`,
back `(19,55,564,561)`, close button `(505,2,553,38)`).
`ControlList_Playing.txt` (`STATE_PLAYING`, 230 loaded entries —
the file itself comments "loads up to here" at entry 230 of 235)
names only 17 rects, so its rect↔entry subset is TBD.
`parse_control_list` keeps file order across `SET` groups
(`all_entries()`); the one `DAILOG_SELECTMUSIC` typo header
parses by prefix.

### Other tables (envelopes only)

- `JamTable.dat` (108 B): 54 `u16` combo thresholds
  `25..4800` (`parse_jam_thresholds`).
- `album.ojs` (6084 B): `u32` count 38, `u32` 2, first id 2001,
  then CP949-titled album records (stride TBD).
- `AlbumList.ojs` (2724 B): `u32` count 17, first id 1001 (stride TBD).
- `SMI_790.ojs` (12828 B): `u16` lyric/scroll table (starts
  625, 569, 665, …; semantics TBD).
- `avatar.opa` `SetInfodata.ojs` (32463 B): `u32` count 210 +
  sparse table (TBD); `Itemdata_Korea.dat` (740945 B, starts
  `06 06 …`): shop-item database (TBD).
- 47 `.ojs` bodies are raw JPEGs (`FF D8 … FF D9` verified on
  all); 5 `.oji` bodies share the sprite-55 envelope.

## Implementation

o2jam-file: `parse_opi` (entries with `name/kind/data_offset/
size/aux_size`; nameless slots skipped; bounds-checked) +
`decode_control_text`, `parse_sprite55` (+ `parse_sprite55_strided`,
`rgb565_to_rgb888`), `parse_bnd`, `parse_oja`,
`parse_jam_thresholds`, `parse_control_list` — each with synthetic
unit tests; `o2jam-dump opi` for local listing. Still TBD: pixel
codecs for the `.oji`/compressed outlier frames, `.oja`
frame-data words, `album`/`AlbumList`/`SMI`/`SetInfo`/`Itemdata`
strides, and the Playing rect subset.

## Open questions

1. ~~`aux_size` semantics when it exceeds `size`.~~ → on-disk
   stride (gap to next blob); stale exceptions documented above.
2. ~~Type-0 free slots with nonzero offset/size2.~~ → deleted-entry
   reservations, no bodies.
3. The 32 B overlap: encoder sloppiness or intentional sharing?
4. ~~Inner formats: `.ojs`/`.ojt`/`.oji`/`.oja` text vs binary,
   `.bnd` structure, `JamData.dat`/`Itemdata_Korea.dat` tables.~~ →
   mapped above; remaining TBD items listed under Implementation.
