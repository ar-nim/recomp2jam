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

Outlier classes (same envelope, non-raw blobs): `.oji` bodies
store one 20-byte record per image plus a run of prefixed row
groups — `[u16 width][u16 x][u16 y]` + `width` RGB555 pixels
plus one zero terminator word (`SelectAlbum_Bar.oji`: 15
records, `Lanking_NoticeBoard.oji`: 800x31 + 188 B header); `y`
walks rows in order and may repeat, so outline-only bodies
(`Shop_MybagSelect.oji`: 2014 B stored for 86x69 — row edges
kept, interiors colour-key) map through the same strict walk
`parse_sprite55_prefixed` (codec gaps ③+④; all 36 images in the
10 real `.oji` bodies decode — `docs/codec-coverage.md`). The
former "`RLE`-style compressed frames" guess for those bodies is
retired: nothing is compressed, the row groups just repeat the
`y` word. The three strided combo atlases
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

### `ControlList_*.txt` — full inventory VERIFIED

UI state/dialog definitions, not scripts: `NUMBER_OF_STATE`
(`0x0F` = 15; file holds 16 `STATE_` headers of which
`PLAYING`/`TUTORIAL`/`DRUM_PLAYING`/`EXIT` are empty `0x00`
stubs) and `NUMBER_OF_DIALOG` (`0x1B` = 27, exact — the
`DAILOG_SELECTMUSIC` typo header is one of the 27). (CXO2
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

Counts below are live `IDC_` lines per block (full-line `//…`
comments, including two commented-out scrollbar rows, excluded;
entry 0 is the block itself). `SETs` lists file-actual `SET`
groups as `members…`; the parser folds post-`SET` top-level runs
into trailing parameter-less sets, so `ControlBlock::sets` is
longer — `all_entries()` still yields file order. Verified rules:
header `decl` == top-level (non-`SET`) entries excluding self
(39/39 full blocks; stubs declare `0x00`); a `SET` group's first
param == its member count (22/22 groups); `.bnd` record `i` lays
out block entry `i + 1` (`entries − 1 == rects` on all 38 bound
Interface blocks, e.g. `DIALOG_EVENT` 4 entries → 3 rects).
`parse_control_list` parses both real files without dropping a
line; the typo header parses by prefix, the empty `BOUND` line
(`STATE_CHANNEL`) yields no bind file, scrollbar style strings
(`"O2_SBS_VERT\t4\t\t100"` shape) ride in `sprite` verbatim,
and trailing `//` comments on entry lines are ignored.

Interface states (12 full + 4 stubs; 806 entries; 22 `SET` groups):

| Block | decl | entries | SETs (members) | BOUND file | rects |
|---|---|---|---|---|---|
| `STATE_CHANNEL` | `0x27` (39) | 50 | 1 (10) | — (empty line) | — |
| `STATE_ROOM` | `0x36` (54) | 76 | 2 (4, 17) | `State_Room.bnd` | 75 |
| `STATE_SHOP` | `0xA4` (164) | 187 | 3 (4, 5, 13) | `State_ItemShop.bnd` | 186 |
| `STATE_MYROOM` | `0x6A` (106) | 112 | 1 (5) | `State_Myroom.bnd` | 111 |
| `STATE_WAITING` | `0x63` (99) | 117 | 3 (12, 4, 1) | `State_Waiting.bnd` | 116 |
| `STATE_RESULT` | `0x07` (7) | 8 | 0 | `State_Result.bnd` | 7 |
| `STATE_AVI` | `0x01` (1) | 2 | 0 | `State_Avi.bnd` | 1 |
| `STATE_PLANET` | `0x11` (17) | 18 | 0 | `State_Planet.bnd` | 17 |
| `STATE_MUSICSHOP` | `0x6B` (107) | 139 | 3 (21, 5, 5) | `State_MShop.bnd` | 138 |
| `STATE_PLAYING` | `0x00` | 0 | — | — | — (stub) |
| `STATE_TUTORIAL` | `0x00` | 0 | — | — | — (stub) |
| `STATE_PAYMENT` | `0x09` (9) | 10 | 0 | `State_Payment.bnd` | 9 |
| `STATE_PAYMENT2` | `0x0C` (12) | 13 | 0 | `State_Payment2.bnd` | 12 |
| `STATE_LANKING` | `0x37` (55) | 74 | 2 (11, 7) | `State_Lanking.bnd` | 73 |
| `STATE_DRUM_PLAYING` | `0x00` | 0 | — | — | — (stub) |
| `STATE_EXIT` | `0x00` | 0 | — | — | — (stub) |

Interface dialogs (27/27 declared, typo included; 489 entries):

| Block | decl | entries | SETs (members) | BOUND file | rects |
|---|---|---|---|---|---|
| `DIALOG_INFORMATION` | `0x03` | 4 | 0 | `Dialog_Information.bnd` | 3 |
| `DIALOG_QUESTION1` | `0x04` | 5 | 0 | `Dialog_Question1.bnd` | 4 |
| `DIALOG_QUESTION2` | `0x04` | 5 | 0 | `Dialog_Question2.bnd` | 4 |
| `DIALOG_CREATEROOM` | `0x09` | 10 | 0 | `Dialog_CreateRoom.bnd` | 9 |
| `DIALOG_OPTION_KEY` | `0x19` (25) | 26 | 0 | `Dialog_Option_Key.bnd` | 25 |
| `DIALOG_OPTION_SOUND` | `0x16` (22) | 23 | 0 | `Dialog_Option_Sound.bnd` | 22 |
| `DAILOG_SELECTMUSIC` (typo) | `0x7A` (122) | 134 | 1 (11) | `Dialog_SelectMusic.bnd` | 133 |
| `DIALOG_INPUTPASSWORD` | `0x05` | 6 | 0 | `Dialog_Password.bnd` | 5 |
| `DIALOG_WHISPER` | `0x05` | 6 | 0 | `Dialog_Whisper.bnd` | 5 |
| `DIALOG_CHANGETITLE` | `0x05` | 6 | 0 | `Dialog_ChangeTitle.bnd` | 5 |
| `DIALOG_EMOTION` | `0x09` | 10 | 0 | `Dialog_EmotionHelp.bnd` | 9 |
| `DIALOG_GIFTVIEW` | `0x13` (19) | 20 | 0 | `Dialog_GiftView.bnd` | 19 |
| `DIALOG_SKILLIINFO` | `0x0B` (11) | 12 | 0 | `Dialog_SkillInfo.bnd` | 11 |
| `DIALOG_CHANGENICKNAME` | `0x06` | 7 | 0 | `Dialog_ChangeNickName.bnd` | 6 |
| `DIALOG_OPTION_GAME` | `0x17` (23) | 24 | 0 | `Dialog_Option_Game.bnd` | 23 |
| `DIALOG_MUSICINFO` | `0x16` (22) | 23 | 0 | `Dialog_MusicInfo.bnd` | 22 |
| `DIALOG_SHOPPINGBAG_ADD` | `0x07` | 8 | 0 | `Dialog_Shopping_Add.bnd` | 7 |
| `DIALOG_CHARGECASH` | `0x03` | 4 | 0 | `Dialog_ChargeCash.bnd` | 3 |
| `DIALOG_EVENT` | `0x03` | 4 | 0 | `Dialog_Event.bnd` | 3 |
| `DIALOG_CONFERENCE` | `0x03` | 4 | 0 | `Dialog_Conference.bnd` | 3 |
| `DIALOG_MESSENGER_MAIN` | `0x3E` (62) | 78 | 4 (4, 3, 4, 4) | `Dialog_Messenger_Main.bnd` | 77 |
| `DIALOG_MESSENGER_FRIENDADD` | `0x06` | 7 | 0 | `Dialog_Messenger_FriendADD.bnd` | 6 |
| `DIALOG_MESSENGER_QUESTION` | `0x07` | 8 | 0 | `Dialog_Messenger_Question.bnd` | 7 |
| `DIALOG_MESSENGER_NOTE` | `0x0D` (13) | 14 | 0 | `Dialog_Messenger_Note.bnd` | 13 |
| `DIALOG_MESSENGER_POPUPMENU` | `0x06` | 7 | 0 | `Dialog_Messenger_PopUpMenu.bnd` | 6 |
| `DIALOG_MESSENGER_INFORMATION` | `0x05` | 6 | 0 | `Dialog_Messenger_Information.bnd` | 5 |
| `DIALOG_CHANNEL_INFO` | `0x12` (18) | 28 | 2 (5, 4) | `Dialog_ChannelInfo.bnd` | 27 |

Playing (`NUMBER_OF_STATE 0x01`, 1 header): `STATE_PLAYING`
decl `0xE6` (230), 235 entries, 0 `SET` groups,
`State_Playing.bnd` (17 rects — rect↔entry subset TBD). The file
itself comments (CP949) "loads up to here" at entry 230, so
decl 230 is the loaded count including self and the last 5
entries ride unloaded.

Declared-vs-actual: dialogs exact (27 == 27, typo counted);
states off by one (15 declared, 16 headers). The 4 empty-`0x00`
stubs explain the shape: 12 full states plus bodiless
`PLAYING`/`TUTORIAL` (mid-file) and `DRUM_PLAYING`/`EXIT`
(appended after `STATE_LANKING`, just before
`NUMBER_OF_DIALOG`) — the count likely went stale when a stub
was appended, or it excludes the `EXIT` pseudo-state.
`STATE_CHANNEL` is the only full block without a bind file (bare
`BOUND` line); all other 38 bound names resolve to a `.bnd`
entry in the same archive, as does `State_Playing.bnd`.
Second header numbers (`0X01`…`0X04` on 10 blocks) are preserved
as `extra`; semantics TBD.

Sprite references: Interface names 568 unique quoted strings —
562 resolve to an archive entry (case-insensitive; 3 differ only
by case) and 6 are scrollbar style params (`O2_SBS_VERT`/`HORZ`
with numeric args), not files. Playing names 64 unique quoted
sprites — 53 resolve (1 case-only: `Main_level.ojs` vs
`main_level.ojs`) and 11 underscore-prefixed refs have no
archive entry: `_Playing_BG.ojs`, `_Note_Miss.ojt`,
`_Note_Bad.ojt`, `_Note_Good.ojt`, `_Note_Cool.ojt`,
`_Note_Combo.ojt`, `_Note_ComboNum.ojt`, `_Note_LongCombo.ojt`,
`_Note_LongComboNum.ojt`, `_Note_Click.ojt`,
`_Playing_Effect_Jam.ojt`. Near-matches exist for each (bare
name as `.ojs`/`.oja` or numbered `.ojt` siblings; the exact
bare `.ojt` exists only for `Note_LongCombo`/`Note_LongComboNum`),
so the `_` prefix likely marks a runtime resolution rule —
semantics TBD.

### Other tables (envelopes only)

- `JamTable.dat` (108 B): 54 `u16` combo thresholds
  `25..4800` (`parse_jam_thresholds`).
- `album.ojs` (6084 B): `u32` count 38, `u32` 2, first id 2001,
  then CP949-titled album records (stride TBD).
- `AlbumList.ojs` (2724 B): `u32` count 17, first id 1001 (stride TBD).
- `SMI_790.ojs` (12828 B): `u16` lyric/scroll table (starts
  625, 569, 665, …; semantics TBD).
- `avatar.opa` `SetInfodata.ojs` (32463 B) — MAPPED
  (`parse_set_info_table`): `u32` count 210, 77 reserved zero
  bytes, then rows at file offset 81: 17 `u32` (set id 1-based,
  group word, 5 item ids, 10 stat words) + 1 pad byte + two
  EUC-KR length-prefixed strings (set name, description).
  The file holds 209 such rows (ids 1–209, walk lands exact)
  plus a 260-byte promo footer of a different shape (kept as
  `trailer_len`; semantics TBD). Item-id join and stat meanings
  stay OPEN.
- `Itemdata_Korea.dat` (740945 B) — MAPPED
  (`parse_avatar_item_table`): header `u32` rows-plus-one + zeros
  (rows at offset 84), then 1541 rows, ids 1–1541, walk lands
  exact at EOF. Row = 30 fixed bytes (id, type, price, params,
  flags) + EUC-KR name/description + 42 `(flag, filename)` sprite
  slots (stems resolve to `.ojs` bodies in `avatar.opa`; the table
  spells `.oji`). Slot use (1541-row survey): slots 0–1 are icons
  (shop portraits, never worn); even slots carry male variants, odd
  slots female ones (`27M23001`/`27F23001` alternation) — unisex
  items fill one parity only; multi-part items (tops + arms) spread
  torso/parts across their parity in slot order. Filename stems
  encode gender+part (`01F…`/`01M…`, `…T…` torso, `…LA…`/`…RA…`
  arms). Worn-figure rule (`pick_wear_sprites`): wearer parity from
  slot 2 up, fallback to the other parity, slot order kept; exe
  part logic stays OPEN.
- 47 `.ojs` bodies are raw JPEGs (`FF D8 … FF D9` verified on
  all); 5 `.oji` bodies share the sprite-55 envelope.

## Implementation

o2jam-file: `parse_opi` (entries with `name/kind/data_offset/
size/reserved_size`; nameless slots skipped; bounds-checked),
`parse_opi_slots` (every slot in table order incl. free reservations,
stride-end checked), `is_jpeg_body` (`FF D8 … FF D9`, 47/47) +
`decode_control_text`, `parse_sprite55` (+ `parse_sprite55_strided`,
`parse_sprite55_prefixed`, `rgb555_to_rgb888`), `parse_bnd`,
`parse_oja`,
`parse_jam_thresholds`, `parse_control_list` — each with synthetic
unit tests; `o2jam-dump opi` for local listing. Still TBD: the
`.oja` frame-data words, `album`/`AlbumList`/`SMI`/`SetInfo`/
`Itemdata` strides, and the Playing rect subset.

## Open questions

1. ~~`reserved_size` semantics when it exceeds `size`.~~ → on-disk
   stride (gap to next table slot); slack bytes are stale, not zero.
2. ~~Type-0 free slots with nonzero offset/size2.~~ → deleted-entry
   reservations (`size` 0, stale bytes, never bodies).
3. The 32 B overlap: encoder sloppiness or intentional sharing?
4. ~~Inner formats: `.ojs`/`.ojt`/`.oji`/`.oja` text vs binary,
   `.bnd` structure, `JamData.dat`/`Itemdata_Korea.dat` tables.~~ →
   mapped above; remaining TBD items listed under Implementation.
