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
| +140 | u32 | reserved size: the entry's allocation when packed |
| +144 | 8 B | zero pad |

`reserved_size` semantics (open question 1, resolved, corroborated
by the independent CXO2 spec): the patcher can shrink `size` for a
replacement payload while `reserved_size` retains the original
allocation, so slack is stale data, not padding — read exactly
`size` bytes. table order is
already offset order (0 inversions in all three archives), and for
~99.9% of slots `reserved_size == next_slot_offset − data_offset`, i.e.
the reserved stride tiling `[16, table_start)` exactly. The last
slot's stride runs to the data-region end (start of the entry
table, e.g. `ControlList_Interface.txt`: `off 74438614 + aux
419496 == table start 74858110`). Bytes in `[off+size, off+reserved)`
are stale slack, NOT zero padding (verified nonzero in 53 + 17 + 59
bodies across the three archives); only the `Playing1.opi` tail
(`[221621059, 221640810)`, 19751 B) is zero-filled. Readers must
slice by `size`, never by `reserved_size`. Table-order mismatches (stride
overruns, never holes): 2 in `Interface1.opi` (the known 32 B
overlap `State_MShop.bnd`/`SMI_790.ojs`: `63271746 + 2934 = 63274680
> 63273928`; plus 16 B `FREE slot 1460`/`Messenger_Alram_PopUp.ojs`),
0 in `Playing1.opi`, 6 in `avatar.opa` (1–10 KB overshoots, e.g.
`X203FB13LA002.ojs` `+24688` vs `+14838` gap). Free (kind-0) slots
are deleted-entry reservations: `size` 0 with nonzero
`offset`/`reserved_size` whose bytes are stale (verified nonzero in every
free reservation — e.g. `Interface1.opi` slot 672 reserves
`[31694688, 31777792)`, 83104 B, 83080 B nonzero).

## Verified inventories (real 7.55 files, counts only)

- `Interface1.opi` (75,100,702 B): 1596 slots → 1587 files
  (1531 `.ojs`, 49 `.bnd`, 5 `.oji`, 1 `.ojt`, 1 `.txt`), 9 free.
- `Playing1.opi` (221,737,026 B): 633 slots → 627 files
  (338 `.ojt`, 275 `.ojs`, 5 `.oji`, 4 `.bnd`, 3 `.oja`,
  1 `.dat`, 1 `.txt`), 6 free.
- `avatar.opa` (144,761,749 B): 7149 slots → 7133 files
  (7132 `.ojs`, 1 `.dat`), 16 free.

(`o2jam-dump opi` prints every slot: `parse_opi_slots` keeps table
order with offsets, free reservations shown as `free reservation`.)

### Body classes per archive (magic/structure, counts only)

Cross-checked with `o2jam-file` parsers (`parse_sprite55` +
`parse_sprite55_strided(_, 4)`, `parse_bnd`, `parse_oja`,
`is_jpeg_body`): every named body lands in exactly one class.

| Class | `Interface1.opi` | `Playing1.opi` | `avatar.opa` | Total |
|---|---|---|---|---|
| sprite-55 standard (`55 05` envelope, exact `8+20·n+Σsize`) | 1485 | 615 | 7131 | 9231 |
| sprite-55 strided ×4 (`Note_Combo4/5/6.ojt`) | 0 | 3 | 0 | 3 |
| `.bnd` sections (`FFFFFFFF` + exact `6+16·n`) | 49 | 4 | 0 | 53 |
| `.oja` animation envelopes (`5A 55`, 6/10/6 frames) | 0 | 3 | 0 | 3 |
| JPEG album art (`FF D8 … FF D9`, `ATI*`/`ati*` `.ojs`) | 47 | 0 | 0 | 47 |
| CP949 word-list text (`abusedata.ojs`, `abuse_NickNamedata.ojs`) | 2 | 0 | 0 | 2 |
| CP949 control lists (`ControlList_*.txt`) | 1 | 1 | 0 | 2 |
| small tables (`album`/`AlbumList`/`SMI_790`, `JamTable.dat`, `SetInfodata`/`Itemdata_Korea`) | 3 | 1 | 2 | 6 |
| **Files** | **1587** | **627** | **7133** | **9347** |
| free slots (kind 0, `size` 0, stale reservation) | 9 | 6 | 16 | 31 |
| **Slots** | **1596** | **633** | **7149** | **9378** |

Sprite-55 split of the 9231 standard bodies: tag 1 / tag 0 =
1450/35 (Interface), 608/10 with 3 more tag-1 strided (Playing),
7131/0 (avatar); colorkey `0x0000` / `0x7D5F` = 1142/343,
547/71, 1758/5373. Free-slot reservations (offset + reserved, table
order): Interface slot 672 (+83104), 1169 (+1596), 1192/1193
(+7780 each), 1351 (+1686), 1460 (+1750), 1502 (+1720), 1503
(+1554068, largest), 1508 (+422); Playing 427 (+150), 429 (+630),
445 (+68806), 446 (+758), 455 (+774), 468 (+22974); avatar 16 tiny
reservations (228–368 B: slots 24, 26, 30, 988, 1238, 1544, 1546,
1548, 1760, 1762, 1989, 2485, 4359, 4523, 4525, 4527).

### Data-region coverage (offset-sorted walk, table order == offset order)

First blob at offset 16 in all three archives; strides tile
`[16, table_start)` with no holes — every gap is the stale slack of
a body with `reserved_size > size` (Interface 51 gaps + 2 overlaps =
53 bodies; Playing 16 gaps + 0 overlaps; avatar 53 gaps + 0
overlaps), and every overlap is a documented stride overrun, never
an underrun. Named bytes vs region: Interface 71745062 +
free 1659906 = 73404968 of 74858094 (1453126 B slack + 329024 B
zero-free tail before the table); Playing 220063661 + 94092 =
220157753 of 221640794 (1483041 B slack + 19751 B zero tail);
avatar 143245748 + 4956 = 143250704 of 143675085 (424381 B slack,
no tail — `Itemdata_Korea.dat` ends exactly at the table).

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

Pixels: `size == w*h*2` means raw little-endian RGB555
(63742 images; `Sprite55::is_raw_rgb555` + `rgb555_to_rgb888`).
CXO2 documents the same 16-bit format for magic `0x0555` (plus a
custom-logic variant for `0x555A`, matching our `.oja` codec TBD).
Note for the renderer: non-power-of-two widths leave uneven
bytes on every other scanline.
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
consecutive RGB555 glow frames per record — 128 + 6·32768 =
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
`(u32 x1, u32 y1, u32 x2, u32 y2)` corner rects in absolute
coordinates;
`6 + 16*count == len` holds on all 53 real sections (49 + 4).
`parse_bnd` enforces magic + exact size.

### `ControlList_*.txt` — role mapped, block↔rect relation verified

UI state/dialog definitions, not scripts: `NUMBER_OF_STATE`
(`0x0F` = 15; file holds 16 `STATE_` headers of which
`PLAYING`/`TUTORIAL`/`DRUM_PLAYING`/`EXIT` are empty `0x00`
stubs) and `NUMBER_OF_DIALOG` (`0x1B` = 27, exact). (CXO2
lists `NUMBER_OF_STATES`/`NUMBER_OF_DIALOGS` — the 7.55 file
spells them without the trailing S; the parser matches the file.) Each block is
`NAME 0xN [0xM] { BOUND "stem.bnd" IDC… [SET … { IDC… }] }`;
entries are `IDC_NAME 0XCODE ["sprite.ojs"]` with `//` comments.
The game stores the code as a single integer (CXO2 notes it does
not split it); its second byte names the object type for the
renderer: `00` state, `01` toggle/radio/checkbox, `02` scrollbar,
`10` image/sprite, `20` button, `30` input box, `50`
interactable/clickable, `60` SET, `70` avatar.
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
size/reserved_size`; nameless slots skipped; bounds-checked),
`parse_opi_slots` (every slot in table order incl. free reservations,
stride-end checked), `is_jpeg_body` (`FF D8 … FF D9`, 47/47) +
`decode_control_text`, `parse_sprite55` (+ `parse_sprite55_strided`,
`rgb555_to_rgb888`), `parse_bnd`, `parse_oja`,
`parse_jam_thresholds`, `parse_control_list` — each with synthetic
unit tests; `o2jam-dump opi` for local listing. Still TBD: pixel
codecs for the `.oji`/compressed outlier frames, `.oja`
frame-data words, `album`/`AlbumList`/`SMI`/`SetInfo`/`Itemdata`
strides, and the Playing rect subset.

## Open questions

1. ~~`reserved_size` semantics when it exceeds `size`.~~ → on-disk
   stride (gap to next table slot); slack bytes are stale, not zero.
2. ~~Type-0 free slots with nonzero offset/size2.~~ → deleted-entry
   reservations (`size` 0, stale bytes, never bodies).
3. The 32 B overlap: encoder sloppiness or intentional sharing?
4. ~~Inner formats: `.ojs`/`.ojt`/`.oji`/`.oja` text vs binary,
   `.bnd` structure, `JamData.dat`/`Itemdata_Korea.dat` tables.~~ →
   mapped above; remaining TBD items listed under Implementation.
