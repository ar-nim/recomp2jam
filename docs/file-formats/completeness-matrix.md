# File-format completeness matrix (7.55)

Version pin for every row: `AngelO2Jam.exe`/`OTwo.exe` 5,029,888 B,
SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`;
`Interface1.opi` 75,100,702 B (`d6654119…bdd50584`), `Playing1.opi`
221,737,026 B (`8c1c7566…aed9eed40`), `avatar.opa` 144,761,749 B
(`ced3ed1e…2953b16`), `OJNList.dat` 215,848 B (`3be8ca00…`). Tools:
Python 3.14 byte walks, GNU objdump 2.47, Ghidra 12.x REA evidence where
named. File offset = VA − `0x400000`. No original bytes or decompiled text
are reproduced.

This matrix is the field-level index into the pinned sources. **PINNED**
means the byte position and its meaning have evidence; **PARTIAL** means the
bytes/walk are pinned but a semantic label is not — the next step is named;
**OPEN** means neither is pinned. Evidence IDs `Cn` are rows in
`docs/claims/40-file-formats.md`; `ev_…` are REA evidence IDs in the
referenced note. Every field listed below has one of those three statuses.
Sibling docs stay authoritative for the argument.

## 1. OPI / OPA container

Source: `docs/file-formats/opi.md` Layout/Entry, claims C1–C4.

| Field | Bytes | Status | Evidence / next step |
|---|---|---|---|
| version | `+0` u32 (2 OPI, 1 OPA) | PINNED | three-archive walk; C1 |
| entry count | `+4` u32 | PINNED | reader count at archive `+0x110`, `imul 0x98` VA `0x40aa05`; C1/C4 |
| zero word pair | `+8` u64 zero | PINNED | three-archive walk; C1 |
| data blobs | `+16 .. EOF−152·count` | PINNED | first blob at 16, strides tile the region; C2 |
| entry table | EOF − `152·count` | PINNED | SEEK_END negative seek VA `0x40a9fa`–`0x40aa14`; C1 |
| entry kind | row `+0` u32 (1 file, 0 free) | PINNED | name→slot find accepts only `== 1` VA `0x40aa67`; C3 |
| entry name | row `+4` 128 B NUL-padded ASCII | PINNED | case-fold compare VA `0x40aa58`; C1 |
| data offset | row `+132` u32 | PINNED | stride tiling, 0 inversions; C1/C2 |
| stored size | row `+136` u32 | PINNED | read exactly `size`; slack is stale; C2 |
| reserved size | row `+140` u32 | PINNED | on-disk stride to next slot; C2 (open question 1 closed) |
| zero pad | row `+144` 8 B | PINNED | three-archive walk |
| free-slot reservation | kind 0, size 0, nonzero offset/reserved | PINNED | stale bytes verified in all 31 reservations; C3 |
| 32 B overlap / tail slack | two overlaps, zero tails | PARTIAL | documented anomaly (Interface `State_MShop.bnd`/`SMI_790.ojs`; Playing tail); encoder cause OPEN — next: patch-builder evidence |

## 2. Sprite-55 and inner bodies

Source: `docs/file-formats/opi.md` Inner formats, claims C5–C10,
`docs/codec-coverage.md`.

| Field | Bytes | Status | Evidence / next step |
|---|---|---|---|
| sprite tag | header `+0` u16 (0/1) | PINNED | 9234 bodies; reader never tests it; C5 |
| sprite magic | `+2` `0x0555` / `0x555A` | PINNED | compares VA `0x424c96`/`0x424caf`; C5/C7 |
| image count | `+4` u16 | PINNED | reader field `+0x11c`; C5 |
| colorkey | `+6` u16 (`0x0000`/`0x7D5F`) | PINNED | reader field `+0x11e`; pixel-accounted colours; C5 |
| image x/y anchor | record `+0/+2` u16 | PINNED | 9234 bodies, anchor == `.bnd` rect origin; C5 + `docs/opi-lessons.md` |
| image w/h | record `+4/+6` u16 | PINNED | `size == w·h·2` on 63,742 images; C5 |
| image data offset | record `+8` u32 | PINNED | offsets chain exactly; C5 |
| image size | record `+12` u32 | PINNED | `8 + 20n + Σsize == len`; C5 |
| record zero word | record `+16` u32 | PINNED | strict walk on every body; C5 |
| pixel blob | raw LE RGB555 | PINNED | `rgb555_to_rgb888`; non-power-of-two widths documented; C5 |
| `.oji` row groups | `[u16 width][u16 x][u16 y]` + width pixels | PINNED | `parse_sprite55_prefixed`, 36/36 images; C9 |
| `.oji` terminator | trailing u16 zero | PINNED | strict walk; C9 |
| `.ojt` strided combo atlas | 4 frames per 64×64 record | PARTIAL | exact 196,736 B accounting, three atlases; exe consumes flat (C8) — meaning of the extra frames OPEN; next: gameplay consumer trace |
| `.oja` header tag/magic/count | `tag`, `0x555A`, count | PINNED | 3/3 envelopes; C7 |
| `.oja` frame x/y/w/h | frame `+0..+6` i16/u16 | PINNED | click-effect geometry shrinks 144→39 px; C7 |
| `.oja` frame words a/b/c | three u32 | OPEN | header verified, stored bodies are compacted row groups; fixed-plane exe read cannot consume them — next: locate the compacted-frame reader（`0x425715` context） |
| `.oja` byte plane | `w·h` byte alpha map | PARTIAL | alpha role pinned (`c == Σ span widths`), exe fixed-size read refuted; C7 |
| CP949 text bodies | NUL/terminated Korean text | PINNED | UTF-8/EUC-KR rejected, CP949 accepts; converter VA `0x406360`; C10 |
| JPEG art bodies | `FF D8 … FF D9` | PINNED | 47/47 `is_jpeg_body`; C item codec table |

### Other small tables inside OPI/OPA

Source: `docs/file-formats/opi.md` Other tables; claim C31.

| Table / field | Bytes | Status | Evidence / next step |
|---|---|---|---|
| `JamTable.dat` thresholds | 54 × u16, 25..4800 | PARTIAL | `parse_jam_thresholds` walks 108 B exactly; no `JamTable` string in the exe, consumer OPEN — next: find a numeric consumer of the threshold list |
| `album.ojs` count / reserved / first id | `+0/+4/+8` u32 = 38/2/2001 | PARTIAL | walk pinned, stride TBD — next: solve record stride over 38 rows |
| `AlbumList.ojs` count / first id | `+0/+4` u32 = 17/1001 | PARTIAL | walk pinned, stride TBD — next: same stride walk |
| `SMI_790.ojs` lyric words | u16 stream starting 625/569/665 | OPEN | envelope only; next: identify the scroll/lyric consumer in the Playing state |
| 47 JPEG `.ojs` bodies | `FF D8 … FF D9` | PINNED | 47/47 `is_jpeg_body` |

## 3. BND bind sections

Source: `docs/bnd-semantics.md`, claim C6, dossier `ev_3a869bb7…`.

| Field / rule | Bytes | Status | Evidence / next step |
|---|---|---|---|
| magic | `+0` u32 `0xFFFFFFFF` | PINNED | test VA `0x548532`; 53/53 sections; C6 |
| rect count | `+4` u16 | PINNED | read VA `0x548562`, `count×0x10` alloc; C6 |
| rect corners | `count × (u32 x1,y1,x2,y2)` | PINNED | `6+16n == len` on all 53; C6 |
| coordinate space | absolute 800×600, x2/y2 exclusive | PINNED | state back rects span `[0,800]×[0,600]`; `docs/bnd-semantics.md` |
| rect↔entry pairing | ordinal = `(code & 0xFFF)−1` | PINNED | VA `0x548640`, ev `3a869bb7…`; overrides the file-order guess |
| pairing miss | all-zero rect | PINNED | bounds check at VA `0x548640` |
| parent offset | child rect += parent origin | PINNED (#114) | attach helper VA `0x50dcc0`, `OffsetRect`; writeback helper VA `0x5487b0` for scrollbars |
| unbound legacy sections | names with no `BOUND` | PINNED | literal-string survey; `docs/bnd-semantics.md` |
| SET-group rect interplay | group-level behavior | OPEN | per-entry ordinal verified; next: trace one `SET` group's runtime rect use |
| keyword block / RTTI join | `0x643f34` keywords, `?AVCControlList@@` | OPEN | pairing pinned; next: REA dossier for the keyword block and class-name join |
| comment block `0x24bdb9` | nature | OPEN | next: xref scan of the four state names |

## 4. ControlList

Source: `docs/file-formats/opi.md` ControlList inventory,
`docs/opi-pattern.md`, claims C10/C16/C19–C21.

| Field / rule | Encoding | Status | Evidence / next step |
|---|---|---|---|
| state/dialog counts | `NUMBER_OF_STATE` `0x0F`, `NUMBER_OF_DIALOG` `0x1B` | PINNED | 16 headers vs 15 declared, 27 dialogs exact; docs opi.md |
| block name | `NAME 0xN [0xM] {` | PINNED | 16+27 headers parsed without dropping a line |
| declaration count | first hex word | PINNED | equals top-level entries minus self on 39/39 full blocks |
| extra header word | second hex word on 10 blocks | PARTIAL | preserved as `extra`; semantics OPEN — next: xref of one block's second word |
| BOUND stem | quoted `.bnd` stem; empty line for `STATE_CHANNEL` | PINNED | 38 bound Interface names + Playing resolve to archive entries |
| entries | `IDC_NAME 0XCODE ["sprite.ojs"]` | PINNED | 806 Interface state + 489 dialog + 235 Playing entries walked |
| code type byte | second byte `00/01/02/10/20/30/50/60/70` | PINNED | renderer object-type split; docs opi.md |
| SET groups | `SET … { IDC… }` | PINNED shape | 22 groups; first param == member count 22/22; group-level rect behavior OPEN (§3) |
| comments / style strings | trailing `//`, `O2_SBS_VERT\t4\t\t100` | PINNED | parser ignores comments, keeps style verbatim; 6 non-file strings |
| quoted sprite references | 568 Interface + 64 Playing unique names | PINNED | 562+53 resolve; 6+11 non-resolving strings catalogued |
| underscore-prefixed refs | `_Playing_BG.ojs` etc. | PARTIAL | near-match names catalogued; resolution rule OPEN — next: stopped-state read of the sprite lookup for one `_` stem |
| code ordinal ↔ rect | `(code&0xFFF)−1` | PINNED | ev `3a869bb7…`; `docs/opi-lessons.md` |
| CP949 text encoding | all header text | PINNED | C10 |

## 5. Item tables

Source: `docs/file-formats/opi.md` Other tables,
`crates/o2jam-file/src/tables/shop.rs`, claims C32/C33, `docs/equip.md`.

### SetInfodata.ojs (32,463 B)

| Field | Bytes | Status | Evidence / next step |
|---|---|---|---|
| declared count | `+0` u32 = 210 | PINNED | walk + C32; parser stops at footer |
| reserved zeros | `+4 .. +80` (77 B) | PINNED | rows begin at 81; 209 rows land exact |
| set id | row `+0` u32, 1-based | PINNED | 1..209 sequential; C32 |
| group word | row `+4` u32 | PARTIAL | bytes pinned; semantics TBD — next: join one set to its shop UI card |
| item ids | row `+8..+19` 5 u32 | PARTIAL | positions pinned, 0 = no item; item-table join OPEN |
| stat words | row `+20..+59` 10 u32 | PARTIAL | positions pinned; stat meanings OPEN |
| pad byte | row `+60` u8 | PINNED | row accounting lands exact |
| set name | `u32` length + EUC-KR bytes | PINNED | `read_table_text`; C32 |
| description | `u32` length + EUC-KR bytes | PINNED | `read_table_text`; C32 |
| promo footer | trailing 260 B | OPEN | different shape; kept as `trailer_len` — next: byte walk of one footer against a tagged promo sheet |

### Itemdata_Korea.dat (740,945 B)

| Field | Bytes | Status | Evidence / next step |
|---|---|---|---|
| count-plus-one / zeros | `+0` u32 + zeros; rows at 84 | PINNED | C32; walk lands exactly at EOF |
| item id | row `+0` u32, 1..1541 | PINNED | strict sequence check; C32 |
| item type | row `+4` u32 | PINNED | type 5 → slot 11, types 15..18 → set-info pose branch; `docs/equip.md` |
| params | row `+8` u32, `+17/+21/+25` u32 | PARTIAL | byte positions pinned; `+22`/`+26` reconstructed as experience/jam percent (`ev_ee6fcc7…fb`); remaining params TBD |
| flag bytes | row `+12`, `+29` u8 | PARTIAL | positions pinned; flag meanings TBD |
| price | row `+13` u32 | PINNED | shop price ink; docs shop.md #833 |
| name / description | `u32` length + EUC-KR, twice | PINNED | strict decode; C32 |
| sprite slots | 42 × `(u8 flag, u32 len, filename)` | PINNED | walk exact on 1541 rows; stems resolve to `avatar.opa` `.ojs` bodies |
| slot layout | 2 icons + 4 blocks × (5 male + 5 female) | PINNED | 1541-row survey; even male, odd female; C33 |
| pose variants | block slots = idle/guitar/bass/keyboard/drum | PINNED | #222/#222d; selector VA `0x52e730`, `docs/equip.md` |
| worn-body rule | one body per part block in kit pose | PINNED | `pick_wear_sprites`, C33 |

## 6. OJN chart

Source: `docs/file-formats/ojn.md`, claims C11–C13/C34/C35.

### 300-byte header (old; new is the decrypted same layout)

| Off | Field | Status | Evidence / next step |
|---|---|---|---|
| 0 | song id u32 | PINNED | validator mask `0x0fffffff` VA `0x571921`; C11 |
| 4 | `"ojn\0"` magic | PINNED | case-fold compare VA `0x5718f9`; C11 |
| 8 | container version f32 | PINNED | drives OJM descriptor; C25 |
| 12 | genre u32 | PARTIAL | bytes pinned; bitmask transfer to `song+0x15c` unpinned — C36 OPEN; next: find the writer |
| 16 | BPM f32 | PINNED | validator float check; C11 |
| 20 | level flags (lo Easy, hi Normal) | PINNED | `u16[difficulty]` read VA `0x44268`, C34/C35 |
| 24 | Hard level u32 | PINNED | same read slot 2; C35 |
| 28/32/36 | total notes ×3 | PARTIAL | count equality 39/39; semantics TBD (`+28` group differs from list) |
| 40/44/48 | playable notes ×3 | PINNED | cross-check vs note walk and list +44/+48/+52 |
| 52/56/60 | last measure ×3 | PINNED | bound ≤300 VA `0x57193b`; C13 |
| 64/68/72 | event counts ×3 | PINNED | `+0x738[difficulty]` from stored header `+0x40`; C13 |
| 76 | key field u32 | PARTIAL | bytes pinned; hi song id / lo 27 old, 29 new; exact use OPEN |
| 80..92 | pad 4×u32 zero | PINNED | all real files |
| 96 | flag u32 (`0x01000000` on 770) | OPEN | no consumer pinned; next: sweep for the immediate |
| 100 | const u32 (19256 on new songs) | OPEN | no `.text` immediate, validator/parser never read it |
| 104 | tune f32 | PINNED | 0.0..0.03 on shipped files; parser staging |
| 108/172/204 | title/artist/label 64/32/32 CP949 | PINNED | byte-exact decode, C10 |
| 236 | container filename 32 B | PINNED | names the `.ojm` |
| 268 | unknown u32 (106954 Tutorial) | OPEN | next: cross-file walk for a constant pattern |
| 272/276/280 | durations ×3 | PARTIAL | seconds-shaped, no consumer pinned |
| 284 | first slice start u32 | PINNED | 300 on all files |
| 288/292/296 | slice ends ×3 | PINNED | walk lands on JPEG at final end; C37 |

### New-format decrypt and event stream

| Field / rule | Status | Evidence / next step |
|---|---|---|
| byte-reversed whole file | PINNED | `FUN_00412180`, ev `011eab73…`, C12 |
| period byte from `rev[len-4]` | PINNED | C12 |
| keys k1/k2/k3 at `rev[len-7..-5]` | PINNED | C12 |
| modular XOR (period, half-period, else) | PINNED | round-trip test; C12 |
| event `i32` measure tick | PINNED | measure-anchored walk, `ev_c0325c85…` |
| event `u16` kind / `u16` count | PINNED | strict `8+4·count` framing |
| kinds 0 (meter) / 1 (BPM) | PARTIAL | float semantics from one/two songs; `999.0` terminator OPEN — next: BPM-tick capture |
| kinds 2..8 lane notes | PINNED | lane = kind−2; count cross-check 39/39 |
| kinds 9+ BGM channels | PINNED | sample ids match `.ojm` sets modulo dangling high refs |
| note value `(flags<<16)|(id+1)` | PINNED | 12/13 id sets match; dangling ids documented |
| note flags high word | OPEN | bit 25 hold candidate; bit-26 path never fires on shipped data — next: `+0x42c`/`+0x504` consumer trace |
| hold bit 26 | PARTIAL | exe test at `0x172a4f`/`0x173061`, no shipped trigger; `docs/ojn-bit26.md` |
| timing 192 ticks/beat | PINNED | `240000/BPM` accumulators |
| embedded JPEG | PINNED | ends last slice 14/14; C37 |

## 7. OJM audio container

Source: `docs/file-formats/ojm.md`, claims C23–C30.

| Field | Bytes | Status | Evidence / next step |
|---|---|---|---|
| M30 magic | `+0` `"M30\0"` | PINNED | 13 files; C23 |
| M30 version/flags word | `+4` `0x00030000` | PARTIAL | constant in all 13; role OPEN |
| M30 unknown word | `+8` zero | OPEN | next: compare a patched M30 body if found |
| M30 sample count | `+12` u32 | PINNED | matches 25/25/714/… entry walks |
| M30 first-entry offset | `+16` 28 | PINNED | walk discipline |
| M30 entry name | 32 B | PINNED | sample names pinned |
| M30 entry size / codec / flag | u32/u32/u32 | PINNED | codec halves `0x0005`/`0x0200`, flag 1; `ref` word is the sample id |
| M30 ref / reserved / pcm bytes | u16/u16/u32 | PINNED shape | `reserved` reads zero; `pcm_bytes?` ≈ decoded size — semantic label PARTIAL, next: compare decoder buffer size |
| OMC/OJM magic | `+0` `M30`/`OMC`/`OJM` | PINNED | sniffer VA `0x54e7c0`; C23 |
| OMC/OJM flags word | `+4` | PINNED | descriptor gate at `+0x110`; C25 |
| OMC/OJM `20` word | `+8` | PINNED | all 21 files |
| OMC/OJM first-entry offset | `+12` | PINNED | fully encrypted files point to EOF |
| OMC/OJM 36-byte entry | name[32] + u32 size | PINNED | all-zero slots skipped; C23 |
| new-variant 56-byte entry | name[32], format, channels, rate, bitrate, align, bits, reserved, chunk size | PINNED | rate×channels×2 on every shipped entry; mode-2 PCM decode |
| descriptor bits `|1`,`|2`,`|4` | version thresholds 0x20006/8/5 | PINNED | C25/C27 |
| descriptor bits `|8`,`|0x10` | mirror / XOR | PARTIAL | no writers in 7.55, vestigial; C27 |
| interleave tables | u16 permutations at file `0x27f1d0`/`0x27f3d0` | PINNED | C26 |
| transform chain | NOT / mirror S-box / u32-XOR `"nami"` | PINNED | C27 |
| mode-2/3 decode | PCM16 / raw OGG | PINNED | entry counts 40+1, 30+12, etc. |
| fetch ordinal → UI bank slot | entry buffer `+0x2c` → bank base + ordinal | PINNED (#718/#721) | file `0x27f728` bases `[0,0x28,0x2f,0x2f,0x2f,3,3]`; `ev_a9fb8aef…` |
| song-cache id-less slot | same correction for the `0x173625` loader | OPEN | next: re-read the loader with the ordinal write in hand |
| DialogBGM odd stripes | pairwise byte swap in shipped file | PARTIAL | 3 exe leads exhausted; encoder-side anomaly, not an exe transform; C30 |

## 8. Song list (`OJNList.dat`)

Source: `docs/file-formats/ojn.md` OJNList section,
`docs/ojnlist-trailer.md`, `docs/xtd-table.md`, claims C14–C21/C39/C40.

### 562 × 300-byte records `[0,168600)`

Record bytes are the 300-byte chart header shifted by +4 (row 0 carries the
global count at `+0`, song 100's id at `+4`). Each row below cites the
pinned header field; fields without a consumer are marked accordingly.

| Record off | Field | Status | Evidence / next step |
|---|---|---|---|
| +0 | leading count (row 0 only; song id thereafter for other rows) | PINNED | 562 records; C14 |
| +4 | song id | PINNED | C14 |
| +8 | `"ojn"` magic | PINNED | C11/C14 |
| +12 | version f32 | PINNED | 2.9 on shipped songs |
| +16 | genre u32 | PARTIAL | C36 (transfer OPEN) |
| +20 | BPM f32 | PINNED | 757/770/860 cross-checks |
| +24 | level flags | PINNED | C34/C35 |
| +28 | Hard level | PINNED | C35 |
| +32/+36/+40 | total notes ×3 | PARTIAL | cross-check vs disk differs from playable group; labels TBD |
| +44/+48/+52 | playable notes ×3 | PINNED | validator cross-check VA `0x171bfe` |
| +56/+60/+64 | last measures ×3 | PINNED | C13 |
| +68/+72/+76 | event counts ×3 | PINNED | C13 |
| +80 | key field | PARTIAL | as chart +76 |
| +84..+96 | pad | PINNED | zero |
| +100 | flag | OPEN | as chart +96 |
| +104 | const | OPEN | as chart +100 |
| +108 | tune f32 | PINNED | as chart +104 |
| +112/+176/+208 | title/artist/label CP949 | PINNED | 757 row decoded; C10 |
| +240 | `.ojm` filename | PINNED | `o2ma###.ojm` |
| +272 | unknown | OPEN | as chart +268 |
| +276/+280/+284 | durations ×3 | PARTIAL | as chart +272..280 |
| +288 | slice start | PINNED | 300 |
| +292/+296/+300 | slice ends ×3 | PINNED | C37 |
| tail levels | per-difficulty levels | PARTIAL | `129/129/129` observed; meaning TBD — next: compare with UI band text |

### Middle region `[168600,195616)` and trailer

| Field | Bytes | Status | Evidence / next step |
|---|---|---|---|
| header magic/count | `0xF834`, 204 | PINNED | part 1 exact 8 B |
| table 1 rows | 204 × `(id,0,group,0)` | PARTIAL shape | stored to `song+0x144/148/14c`, never read — C36-era write-only result; purpose OPEN |
| table 2 header/rows | count 562 + `(id, listing,0,param_b,0)` | PARTIAL | `listing` consumer-pinned (`+0x16c`); `param_b` write-only, OPEN |
| table 3 header/rows | count 407 + `(id,param_a,param_b,0x77d2e4b7)` | PARTIAL | `param_a` gate pinned; `param_b`+anchor OPEN |
| table 4 rows | 120 × 3 words | PARTIAL | stored to `+0x184/188/18c`, never read; purpose OPEN |
| table 5 rows | `(id, category 1..=5)` | PINNED | wheel bucket → `entry+0x4`; C16 |
| footer 10 words | `[2,173,50,0,166,50,0,0,562,887]` | PINNED | counts drive the vestigial trailer loops; C40 |
| patch index pairs | 561 × `(marker,id)` | PINNED shape | reader count 0 → dead data; C40 |
| patch header | marker/count/date/tag | PINNED shape | 28 B; C40 |
| `;HD` records | magic/const/patch id/date/tag ×561 + 8 B truncated slot | PINNED shape | dead to 7.55 client; C40 |
| key-name entries | 65 × u32 VA pointers to CP949 names | PINNED | C18/C39 |
| STEP2 banner row | 7 names vs 6 buttons | PARTIAL | off-by-one pinned, interpretation OPEN (C20) |

## 9. Coverage boundaries

- A field absent from this matrix is not claimed as covered: add it here
  with an evidence ID or an OPEN next step before coding a consumer.
- Contradictions stay visible: the `.bnd` file-order reading (now refuted),
  the `.oja` fixed-plane reading (refuted by compacted bodies), the `xTd`
  code reading (refuted by the pointer decode), and the patched-song reader
  (dead data) remain named in the sibling docs.
- Claim IDs are stable; evidence IDs depend on the pinned SHA-256 above.
  An unpinned field can be closed only by a new byte walk or stopped-state
  read recorded in the relevant sibling doc, then linked here.
