# Dialog string sources (7.55)

Where every visible UI string lives, and how far each source
has traveled: extracted, decoded, drawn, or missing.

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`;
`Interface1.opi` 75,100,702 B; `Playing1.opi` 221,737,026 B;
`avatar.opa` 144,761,749 B.

## 1. Sprite-baked labels — drawn as art, never as text

Most visible wording is pixels inside sprite-55 bodies, not
character data. The archives hold 9231 standard sprite bodies
(1485 Interface `.ojs` + 615 Playing + 7131 avatar) plus 3
strided combo atlases and 5 `.oji` outliers. Each body parses
through `parse_sprite55` (`tag + 0x0555 + count + colorkey`),
and raw frames satisfy `size == w*h*2` (63742 images).

Status: extracted, decoded (RGB555 via `rgb555_to_rgb888`),
drawn as blits. There is no text to localize here; the
renderer copies pixels. Single-image probes prove placement:
`Dialog_Information.ojs` anchor `(241,206)` 318x187 matches
rect 0; `Info_OK.ojs` anchor `(102,150)` 117x26 matches rect 1.

## 2. ControlList entries — decoded, drawn as layout only

`ControlList_Interface.txt` (`Interface1.opi` slot 1595) and
`ControlList_Playing.txt` (`Playing1.opi` slot 632) decode via
`decode_control_text` (CP949) and parse via
`parse_control_list`. The 1295 + 235 `IDC_` lines carry an id
(`IDC_…`), a code (`0X…`), and an optional sprite stem. They
carry no caption literals: wording comes from the linked
sprite or from runtime data. Parser keywords live at file
`0x243f34` (`NUMBER_OF_STATE`, `SET`, `%s%x`, …).

Status: extracted, decoded, drawn as control identity and rect
binding only. Caption text: missing by design.

## 3. Exe string tables — extracted, partly drawn, partly dead

Read-only sweeps of the exe `.rdata` locate these blocks
(file offset = VA − `0x400000`):

| Block | Offset | Status |
|---|---|---|
| Grade strings (`ALL COMBO`, `COOL 95%`, `MISS 10`, …) | file `0x247362`–`0x2476dd` | Extracted; zero `.text` refs (full-image LE32 scan) — missing from draw path |
| `JAM` strings | file `0x24745e`/`0x24762b`/`0x2476b3` | Extracted; same dead verdict as grades |
| `LIFE 100%` | file `0x2474df` | Extracted; bar art drawn, string not drawn |
| Level bands (`초급 1–8` …) + `"X급 : LEVEL %s"` | file `0x24578e…`, dossier `ev_68760cac…` | Extracted; band filter drawn, format partially drawn |
| Key-name table (65 VA pointers to CP949 names) | file `0x2454d8`–`0x2455dc` | Decoded; filter + banner path pinned (`strncmp` at file `0x448c2`, converter `FUN_00406100`) — drawn |
| Search banners (`"%s 검색 결과입니다."`, `"%s -> %s ..."`) | VAs `0x645a50`/`0x645a64`/`0x645a80` | Extracted; STEP1/2/3 banner writers pinned — drawn |
| Option keys (`MODE`, `NoteTail`, `LongnoteMode`, …) | file `0x245224`–`0x2452a8` | Extracted; read via `GetPrivateProfileIntA` — drawn as settings, not as captions |
| Sprite patterns (`%s.ojs`, `%s.oja`, `%s%d.ojs`) | file `0x24b0ec` | Extracted; `_`-prefix resolution rule — drawn as lookup |
| Channel/planet script bridge (`My_channel`, `CTuser_id[%d]`) | file `0x24df70`–`0x24e024` | Extracted; web bridge — drawn |
| Fonts (`Trebuchet MS`/`Arial`) | song-select flow `FUN_004e47a0`, file `0x8e47a0` | Extracted; setup call present — drawn |
| Room fill (`"Lv.%d:%s"`) | `FUN_00491af0` | Extracted — drawn |
| Create-room default titles (8 CP949 phrases) | file `0x244b50`–`0x244bec`, pointers `0x244c08`, pick site `0x436a00` | Extracted; prefilled on dialog open + drawn (`§3a`) |
| Message-box / error-caption table | — | Missing: no caption, `MessageBox`, or `IDS_` sweep has been pinned |

### 3a. Create-room default titles — pinned, prefilled

A CREATE ROOM dialog never opens with an empty title. Version pin:
`OTwo.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`; method = Python
3.14.10 byte scans, GNU objdump 2.47 slices, radare2 6.2.2 call
xrefs, all over the read-only copy (file offset = VA − `0x400000`).

Eight null-terminated CP949 titles sit back to back at file
`0x244b50`–`0x244c03` (18–21 bytes each); the `u32` pointer table
behind them is file `0x244c08`–`0x244c27` (VA `0x644c08`), and the
whole image holds exactly one reference to it, file `0x36a60`:

| file offset | CP949 bytes | text |
|---|---|---|
| `0x244b50` | 20 | `니들이 리듬을 알아?!` |
| `0x244b68` | 21 | `자나깨나 두들겨보세!!` |
| `0x244b80` | 18 | `신나게 두들겨보자!` |
| `0x244b94` | 19 | `키보드는 부수지마!!` |
| `0x244ba8` | 20 | `따라올테면 따라와봐!` |
| `0x244bc0` | 19 | `짜릿한 콤보의 쾌감!` |
| `0x244bd4` | 21 | `신나는 음악을 즐겨봐!` |
| `0x244bec` | 20 | `손가락이 부서지도록!` |

The pick helper at VA `0x436a00` (159 B) seeds the C runtime
generator with the tick clock modulo 1000, takes one value mod 8
(MSVC signed-`%` idiom at VA `0x436a4b`), copies the entry into a
50-byte stack buffer, writes it into the dialog's title edit
(this+`0x240`), then resets that edit's selection to `[0, len]` —
the default opens fully selected, so the first typed char replaces
it. `CDCreateRoom` calls the helper on dialog open/show (VA
`0x43554f`, `0x43581f`) and on the re-show path (VA `0x435c80`),
each guarded on the edit existing.

No player name is involved: the helper reads no session, nickname,
or config global, and none of the eight entries carries a `%s`.
The dialog's accepted title lives at this+`0x185` and the OK
handler (VA `0x436ac0`) refuses an empty or all-space title with
`방 제목을 입력하세요.` (file `0x244f04`) and a forbidden word with
file `0x244f1c`. The owner report's "incorporating the player
name" is contradicted by the pin; the exe is the oracle.

Corroboration, not evidence: `docs/wine-boot.md` §Attempt 12 (live
Wine capture, 2026-11-08) saw two of these eight strings in the
title field; its transcription `신나는 음악을 즐겨라!` is wrong —
the pinned bytes at `0x244bd4` read `신나는 음악을 즐겨봐!`. The
caption at file `0x249be4` caps a title at `영문 21자, 한글 10자`
= 21 CP949 bytes, and every entry fits. Server side adds nothing:
Memoryer `CreateRoomRequest.Title` (order 0) goes straight into
`RoomService.CreateRoom` (`~/mozart-src`, behavior reference).

Drawn: `create_room_sprite_names`-staged title edits paint the
form text at the parsed rect, so the prefill is visible
(`crates/o2jam-shell/src/screens/side.rs` `DEFAULT_ROOM_TITLES`,
`default_room_title`; prefill at `open_create_room_dialog`).
Known divergence, not a pin: our wire is UTF-8
(`docs/claims.md` D4), so a 20-char Korean default reaches a
Memoryer server as 7 Hangul chars under the 21-byte title cap,
where the exe's CP949 title fits whole.

## 4. Word-list text — decoded, not drawn

`abusedata.ojs` and `abuse_NickNamedata.ojs` (`Interface1.opi`,
2 of 1587 files) are CP949 word lists decoded by
`decode_control_text`. Strict UTF-8 and EUC-KR reject them;
CP949 decodes every body.

Status: extracted, decoded. They feed nickname/abuse filters,
not visible labels.

## 5. Dynamic runtime text — decoded, placeholder-drawn

Names arriving per session, not per build:

- Song title (64 B CP949 at OJN header +108; `OJNList.dat`
  record +112), artist (+172 / +176), label, `o2ma###.ojm`
  filename. Status: decoded (`parse_ojn`, `parse_ojn_list`);
  select-music screen draws synthetic `M{music_id}` /
  `NORMAL` stamps instead of real titles — placeholder-drawn.
- Room, player, chat, whisper, password, and level/EXP
  (`MYLEVEL`, `EXP`, `NEXTUPEXP`, `MYRANK`) fields. These are
  `IDC_TEXT_*` controls bound to `.bnd` rects; the shell maps
  them to wire values and draws from rect corners with
  micro-font metrics. Status: extracted from packets;
  layout-drawn, typography missing.
- Wheel buckets (GOLD/BLACK/BLUE/RED/SILVER) and STEP1 bands
  ride `song+0x194` → `entry+0x4` into the select dialog.
  Status: decoded, drawn as filter lists.

## 6. Catalog text — decoded, partly drawn

`Itemdata_Korea.dat` (slot 7148, offset 142934156, 740945 B)
and `SetInfodata.ojs` (slot 7043, offset 136063397, 32463 B)
hold EUC-KR/CP949 names, descriptions, and ring effect lines
(`11A080xx` mirror/random/hidden/sudden/dark catalog in
`docs/gameplay.md`). Parsed by `parse_avatar_item_table` /
`parse_set_info_table`.

Status: extracted, decoded. Shop view shows name/description;
`item_type`, params, flags, and per-slot semantics stay
undrawn.

## Coverage order

Most-drawn first: sprite-baked art → ControlList ids →
runtime names → catalog text → exe banners/filters →
word lists (never visible) → dead grade/JAM strings →
unmapped error/message captions (missing).
