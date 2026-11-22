# STATE_MYROOM screen (7.55) — evidence log

Avatar room (`CStateMyroom`, `0x6A`; `CItemManager`, `CPlayerInfo`).
Read-only walks below; offsets are file offsets unless marked VA.
Version pin for all EXE rows: `AngelO2Jam.exe` 5,029,888 B, SHA-256
`83a04541…72c823b`.

## Block walk (2026-11-09, `ControlList_Interface.txt`; codes re-verified 2026-11-10 from the shipped dump)

`STATE_MYROOM`, decl `0X6A`, `BOUND "State_Myroom.bnd"`, 112
parsed entries (header number 106; 2 `SET` groups). Full code
table (file order, `0X` prefix as shipped):

| Codes | Entries | Art |
|---|---|---|
| `0X15000000` | `IDC_STATE_MYROOM` (self) | — |
| `0X15100001` | `IDC_BACK_SMYROOM_1` | `State_Myroom.ojs` |
| `0X15200002..04` | BACK / GOSHOP / ITEMSELL(vacate) | matching `.ojs` |
| `0X15200005..0F` | 11 equip-slot buttons (instrument…shoes) | — (selectors, art-less) |
| `0X15400010..11` | O2MONEY/CHARGEMONEY texts | — (mapping OPEN) |
| `0X15700012` | AVATAR image | staged figure |
| `0X15600013..22` | 16 slot images with Korean slot comments (악기/헤어/소품/장갑/목걸이/상의/하의/안경/귀걸이/의상소품/신발/날개/악기소품/펫/헤어소품/코스튬) | — (icons OPEN) |
| `0X15600023..2C` | 10 bag items | — (names paint as text) |
| `0X1520002D..32` | bag pager LEFT/RIGHT + 4 page-number digits | `Shop_Btn_Bag*.ojs`, `MShop_PageNumber.ojs` |
| `0X15500033` | `MYBAGSELECT` cursor | `Myroom_MybagSelect.oji` |
| `0X15203034` | INVENTORY (`Myroom_Inventory.ojs`, family `0x03`) | equip-apply control (ours-by-design) |
| `0X15500035..36` | slotcover/userinfo | matching `.ojs` |
| `0X15400037..44` | nickname/level/bonus/exp/nextup/winrate/rank/guild + money texts | identity fields paint, mapping OPEN |
| `0X15600045..46` | MYBAG_POS / GIFTBOX_POS | — (anchors OPEN) |
| `0X15500047` | GIFTBOX_BACK | `Myroom_GiftBox.ojs` |
| `0X15200048..49` | gift pager LEFT/RIGHT | `Myroom_Btn_Gift*.ojs` |
| `0X1501C04A..4E` | 5 gift-box EX cells (family `0x01`) | — (names paint as text) |
| `0X1540004F` | TEXT_GIFT_NUM | count caption (format ours) |
| `0X15500050` | GIFT_TYPE image | `Myroom_Gift_TypeImage.ojs` |
| `0X15400051..52` | GIFT_ITEMNAME / GIFT_SENDER | resolved captions |
| `0X15500053` | GIFT_MOVE | `Myroom_Btn_GiftMove.ojs`, claim control (ours-by-design) |
| `0X15500054..57` | gift page numbers | `Myroom_GiftPageNum.ojs` |
| `0X15200058` | WEBAVATAR (`Myroom_Btn_Avatar.ojs`) | — (P0) |
| `0X15400059..5B` | O2CASH/MUSICCASH/ITEMCASH | name-identity captions |
| `0X1540005C..65` | SKILLCOUNT1..10 | — (source OPEN) |
| `0X15400066..67` | PENALTYCOUNT/LEVEL | identity captions |
| `0X15500068..69` | NOTICE_TEXT/NOTICEBOARD | — (OPEN, like waiting NOTICE) |
| `0X1550006A` | PCROOMMARK | `MyRoom_PCRoomMark.ojs` |
| `0X1520006B..6D` | CONFERENCE / BAGSORT / MESSENGER | — (P0) |
| `0X1550006E..6F` | MESSENGER_ALRAM(+POPUP) | — (P0) |

## Bind file (corrected 2026-11-10 — the absent line is retired)

`State_Myroom.bnd` is PRESENT in `Interface1.opi` (slot 1340,
kind 1, magic `FFFFFFFF`, 111 rects, `6 + 16×111` = 1782 B;
the stem hits twice image-wide, like `State_Lanking.bnd`).
The old "declared but absent" line was wrong. The block paints
the bound path: art at ordinal-paired rects (claims A1/A27
family), never anchor origins. The avatar portrait resolves
through the `.bnd` like every other bound scene.

## Data side (owned systems)

- Item catalog + names: `Itemdata_Korea.dat` parsed (1541 rows).
- Equip loadout: `SetInfodata.ojs` parsed.
- Avatar figures: staged bodies resolve (tally strip proves it).
- Inventory/stats/gifts: server-fed (`CharacterInfo` 2001 —
  equipment, inventory, item/music gifts, cash wallets); staged
  through `set_myroom_character` until a live gateway exists.

## Ticket #369 (default upper-right face)

Exe b8 (`~/scratch/side-scenes/b8-cash/manifest.json`, manifest
`f9d4a695…3f8b`, exe `83a04541…c823b`,
`artifacts/screenshots/myroom.png` `1743d36d…fae0`) opens the panel with
an `Inventory` badge at x≈427..476, y≈22..46 and the slot grid beneath;
our base shot `ff5f949f…459ddc` opened on `STATUS` with user-info fields.

Pin from the shipped sheet and the staged `.bnd` (111 rects, ordinal
`(code & 0xFFF)-1`):

| Code | Control / body | `.bnd` rect | Sprite anchor |
|---|---|---|---|
| `0x15203034` | INVENTORY button `Myroom_Inventory.ojs` | (147,307)-(207,330) | (147,307), 60x23 |
| `0x15500035` | SLOTCOVER `Myroom_SlotCover.ojs` | (240,46)-(734,358) | (240,46), 494x312 |
| `0x15500036` | USERINFO `Myroom_Status.ojs` | (392,22)-(776,379) | (392,22), 384x357 |

The background `State_Myroom.ojs` already carries the cyan `STATUS`
button at the INVENTORY rect. In staged file order 0x34 → 0x35 → 0x36,
so our old painter covered that baked label with the INVENTORY body and
then covered the slot grid with the status plate. The exe's default face
keeps only the background label + slotcover; the other face shows the
status plate + INVENTORY body. Which gesture switches faces is not in the
qualified capture: OPEN, next step = a pinned slot/button press capture
under the same manifest. No wire behavior is bound to the face.

Landed: `MyRoomPanel` defaults to `Inventory`; `paint_side` and
`paint_myroom` filter staged placements through `myroom_panel_kept`
(never literal geometry), and identity captions ink only on the status
face. `show_myroom_panel` is the documented seam while the gesture is
OPEN. Guards:
`myroom_entry_paints_inventory_panel_not_status_plate` (live chain),
`myroom_status_face_restores_the_plate_and_inventory_button` (seam),
and the live gate assertion in
`live_boot_myroom_data_path_paints_on_channel_login` (install-root skip).
Matched-state pixel hash against the exe pack stays OPEN without a live
boot; the synthetic guard pins paint order, not a 1:1 screenshot hash.

## Selection frame body (ticket #305, 2026-11-19)

Version pin as above (`83a04541…c823b`, 5,029,888 B); archive
`Interface1.opi` `d6654119…bdd50584`, 75,100,702 B. Local read-only
archive parse (no bytes added to the repo):

| Entry | Slot / file offset / size | Body | Evidence |
|---|---|---|---|
| `State_Myroom.ojs` | 1137 / 42,946,754 / 960,028 B | 1 raw 800x600 frame, anchor (0,0) | default Inventory face, baked STATUS badge (427,22)-(476,46), slot grid |
| `Myroom_Inventory.ojs` | 306 / 22,023,032 / 2,788 B | 1 frame, anchor (147,307), 60x23 | INVENTORY body on the STATUS-toggled face (`0x15203034`, bnd ordinal `0x34`) |
| `Myroom_Status.ojs` | 312 / 22,037,148 / 274,204 B | 1 frame, anchor (392,22), 384x357 | STATUS plate on that face (`0x15500036`, ordinal `0x36`) |
| `Myroom_SlotCover.ojs` | 1584 / 73,857,906 / 308,284 B | 1 frame, anchor (240,46), 494x312 | slot grid on both faces (`0x15500035`, ordinal `0x35`) |
| `Myroom_MybagSelect.oji` | 1305 / 58,501,780 / 2,042 B | 1 prefixed frame, 86x69, 134 row spans | static blue selection frame (`0x15500033`, ordinal `0x33`, bnd rect (0,0,86,69)) |
| `Window_Cursor.ojs` | 1303 / 58,448,076 / 1,848 B | 2 frames, 18x25 | orange pointer, not panel art |

`State_Myroom.bnd` is slot 1340, offset 61,442,280, 1,782 B, 111 rects;
ordinal `(code & 0xFFF)-1` reproduces the four rects named in the #369
table above.

The live-4 gesture pack (`~/scratch/live-4/myroom-gesture/exe`,
`myroom.png` `5ddb5454…1663`, `myroom-status-toggle.png` `b095a915…aeb9c6`,
`myroom-gesture.png` `c838e437…c7bfe3`, exe `83a04541…c823b`) pins both
panel faces and the click that switches them (input log: (177,318) on the
INVENTORY control). The orange arrows in those frames are **not** a
per-state frame: decoding `Window_Cursor.ojs` frame 0 matches its pixels
at (517,16) in `myroom.png` and (177,318) in both later frames with mean
per-channel error 0.68 — exactly the two logged click points. The bnd
`0x33` rect shows no `.oji` ink in either capture (0/604 opaque pixels
match), so no exe frame pins the selection body to a pressed cell. That
mapping stays ours-by-design; the next evidence step is a qualified
exe capture with a pressed slot/bag/gift cell.

Owner visual confirm is supplied by the cited `story-myroom-select/exe3`
pack: bag-cell picks at (171,451) / (224,423)-(297,480) center the blue
frame on the staged `MYBAGITEM1/2` rects (diff bboxes
(129,417)-(215,486) / (218,417)-(304,486)). The earlier `exe2` pack
(`c693cf07…998affdf`, empty tester bag) pins the negative arm: a press
on a blank cell moves only the OS cursor and paints no frame. The
`25c5997e…` slot-after-bag frame keeps the bag frame in place.

Landed: `paint_myroom_data` stops painting `paint_cursor` amber on the
MyRoom path. It reads the staged `MYBAGSELECT` sprite and its decoded
86x69 frame body, then centers that body over each selected staged cell
rect (bag or gift) whose staged row carries a name. Geometry comes from
staged `.bnd` rects and the sprite record only. Guards:
`myroom_inventory_face_paints_the_staged_selection_frame_without_amber`,
`myroom_status_face_paints_the_staged_selection_frame_without_amber`,
`myroom_slot_and_gift_selections_paint_the_staged_frame_without_amber`,
and `myroom_empty_bag_cell_paints_no_selection_frame` (red before the
populated-row filter: blank cell painted blue 255; green after), plus
the live gate assertion in
`live_boot_myroom_data_path_paints_on_channel_login` (install-root skip).

## Equipment wells (ticket #424, 2026-11-20)

The 16 slot images `0X15600013..22` are bound rects in `State_Myroom.bnd`;
the first eleven Korean comments (악기/헤어/소품/장갑/목걸이/상의/하의/안경/
귀걸이/의상소품/신발) name the same parts, in the same order, as the eleven
slot-selector buttons (`docs/myroom.md` block walk). The 2001 character body
carries seventeen equipment words in `ItemType` order (Instrument 0 …
Costume 16, `docs/net-protocol.md`), so well ordinal `0..11` pairs with the
equipment array at the same index. `Itemdata_Korea.dat` sprite slots 0–1 are
shop icons and are never worn (`docs/equip.md`); the exe's hash-pinned MyRoom
frame shows the jacket, pants and shoes icons in their wells.

Landed: `paint_myroom_slot_icons` inks each styled well from the 2001
equipment word's staged shop portrait at the paired `.bnd` rect. Missing
word, row, portrait, or rect paints nothing — no fallback geometry. The
same `shop_item_portraits` map the Item Shop stages from `set_avatar_source`
serves the wells, so equip/vacate restages update both figure and well.
Guard: `myroom_worn_item_inks_its_staged_equipment_well` with a synthetic
portrait and jacket slot 5; no ink before the painter was wired.

### Well join contradiction (ticket #794, 2026-11-22)

The ordinal join above (`ee8aa26`) assumes the equipment array ordinal
equals the well number. State entry `FUN_004a3730` (VA `0x4a3730`, file
`0xa3730`, EV `ev_004adf63036fbbdbfa97bf4a5f5cae816abb3c74e1179576408f93df669735e9`)
instead scans 29 equipment words and stores each in a 17-entry worn array
indexed by that item record's byte `+0x18` when it is below 17; the 16 wells
read that array. The exe is the behavior oracle, so the join now resolves
`(well, item id)` from the record byte, with last writer winning on a
duplicate well. The older ordinal join was only correct for sequential
record bytes and is kept here as the documented contradiction, not as a
silent fallback. OPEN next step: a live non-sequential account, capture the
wells and pair them with the 2001 body. Guard:
`myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal`.

## STATUS word/format table (ticket #799, 2026-11-22)

`FUN_004a7a80` (VA `0x4a7a80`, file `0xa7a80`,
EV `ev_21ce4a71b4e5b3ae8f617832f47b721dbd37c96375ae8f73086aad6a49989c9b`)
inks STATUS from these pinned words/formats: nickname `0x15400037` from
`0x77cd05` with a 121-pixel fit loop; level `0x38` and experience `0x3a`
as `" %d"` (VA `0x648140`); next-up `0x3b` as `table[level]` for level < 99
from the 99-entry u32 table at file `0x24776c` (REA read
`ev_b062b9faf5ba4bc445bcda0356993bae5fb098c8959e8d645ccf84f6367a8f7b`),
and nothing past the table; rank `0x3d` as `" %d 등"` (file `0x249474`)
when its web-served word is nonzero (the overnight note transcribed this
as `만`; raw cp949 bytes at file `0x249474` decode to `등` — code-byte pin
wins, but no rank word is staged in our client); penalty count `0x66` as `"%d"`, level
`0x67` as `"Lv %d"`; bonus `0x39` as `"%d PT"`. Money G/W keep their
settled 5029 wallet source (#202).

CONTRADICTION: the older hash-pinned room-panel capture read `Next Up 500`
(level 1) and `Next Up 5001` (level 7), which no index of this table
produces (884, 1819, …, 10404 at index 7). The exe code and table are the
behavior oracle per project rule; the earlier capture is kept visible as
the open question. OPEN next step: pair a known-level exe STATUS frame with
the 2001 word that feeds `0x77cd20` and settle the index/field mapping.
Guards: `myroom_status_next_up_prints_the_level_threshold_not_the_experience`
and `myroom_status_prints_the_exe_number_and_penalty_formats`.

Ticket #810 adds the remaining STATUS words from the same dossier:
nickname fits its staged rect (the exe's 121-pixel fit loop), bonus
prints `"%d PT"` from `0x768b58`, penalty count/level print `"%d"` /
`"Lv %d"` from `0x76fc64/66`, and rank `0x1540003d` prints the exe's
`" %d 등"` for a nonzero web-served word `0x768b78`. Our nickname,
level, exp, next-up, bonus and penalties already ink from the staged
2001 character; rank is staged explicitly via
`ScreenGame::set_myroom_rank_word` because that word's writer is OPEN.
A fresh REA 6.9.0 / Ghidra 12.2 dossier for `FUN_004a7a80` is
`ev_4ba373ad8aab8b9cb9f8adfae1ac6e8860fce4277d9bc5b22368a3b8283218ef`;
a raw read of file `0x249474` confirms `b5 ee` = `등` (the #180 row's
`만` is a transcription error). Guards:
`myroom_status_prints_the_web_served_rank_with_the_exe_format`,
`myroom_status_inks_the_staged_rank_word_in_its_bnd_rect` and
`myroom_status_fits_a_long_nickname_inside_121_pixels`. OPEN: pair a
known-rank exe STATUS frame with the wire field that feeds `0x768b78`;
no such live capture exists, so the staged word is never inferred here.

## Gift pager wrap/clamp (ticket #806, 2026-11-22)

`FUN_004a3ee0` cases `0x15200048/49` (file `0xa3ee0`,
EV `ev_aa8752511eb19d9d34701592a529586e5808751c2b8169e2b5009cdeb3b66c20`)
read the dialog's page/count words at `+0xac/+0xb0`: LEFT wraps from page
0 to the last page, RIGHT clamps at the last page, and both clear the
gift selection. `FUN_004a8dc0` derives the count as gift-vector length / 5
rounded up. The bag pager still clamps both ends — the two pagers differ.
Guards: `myroom_gift_pager_wraps_left_clamps_right_and_clears_selection`
and `myroom_gift_left_click_wraps_to_the_last_staged_page` (staged rects,
6 gifts → 2 pages). No exe account with >5 gifts was captured; that live
pixel leg stays OPEN.

## WEBAVATAR outcome (ticket #804, 2026-11-22)

`FUN_004a3eb0` (VA `0x4a3eb0`, file `0xa3eb0`,
EV `ev_aa64e525e7349e0fde17760e31ce4a599dc3a978f9c8038dfce182c95849ccd9`)
plays sound 13; `FUN_004a4960` (file `0xa4960`,
EV `ev_5539ea3eb8f4e553ca0c3f04002e017d57900f368c999a342e80375bc4047529`)
copies "아바타를 웹에 등록." (file `0x2491e0`) into the UI helper at
`0x513c60` with flag `0xfff9`. Our press cues UI-effect slot `0x0D`
(decimal 13) and posts the pinned copy in a one-OK sheet. Whether the
helper shows a notice or opens a browser, and its live pixels, stay OPEN:
no browser behavior is claimed. Guard:
`myroom_webavatar_posts_the_exe_register_notice_without_a_browser_claim`.

## CONFERENCE / MESSENGER branches (ticket #803, 2026-11-22)

`FUN_004a3ee0` (file `0xa3ee0`, EV `ev_aa875251…`) case `0x1520006b`
opens modal 20; case `0x1520006d` branches on the messenger globals into
the connection-lost string (file `0x247dc0`), the loading string (file
`0x247e08`) or modal 21. Our client stages the branch explicitly (the
globals' writer is OPEN) and shows the pinned one-OK sheet; the two modal
captions are unpinned, so only staged sheet art paints, never invented
copy. Neither press queues wire. Guard:
`myroom_conference_and_messenger_branches_post_their_pinned_sheets`.

## BACK / GOSHOP wire split (ticket #802, 2026-11-22)

`FUN_004a3ee0` (VA `0x4a3ee0`, file `0xa3ee0`,
EV `ev_aa8752511eb19d9d34701592a529586e5808751c2b8169e2b5009cdeb3b66c20`)
case `0x15200002` changes state and calls the send builder with opcode
`0x138f`/5007 (EV `ev_d3647af672ca1747c74f03fd9ce38e9d79e589b278186fd70c5606f7a3b42b94`);
case `0x15200003` (GOSHOP) only changes state. Our BACK now queues the
same empty `ExitShop` as the item-shop BACK; `OnlineSession::flush_exit_shop`
sends 5007 and the 5028 wallet-refresh follow-up. GOSHOP stays local.
The live BACK press/tap remains ticket #426.
Guard: `myroom_back_sends_one_empty_5007_and_goshop_sends_nothing`
(scripted gateway), plus the shell guard
`myroom_back_click_returns_to_the_lobby`.

## NOTICE board path (ticket #801, 2026-11-22)

`FUN_004a9890` (VA `0x4a9890`, file `0xa9890`,
EV `ev_f3b4a8b0b0524f7d3f59ab7f07ca6466c22dca32e557b313529527925e8d26f6`)
inks `NOTICE_TEXT 0x15500068` from the control value with the Korean face
at file `0x24407c`, colour `0xffffff`, and stages `NOTICEBOARD 0x15500069`
frame 2. Our painter selects frame 2 when the staged body carries it;
white ink uses the owner-approved modern font rasterizer (glyph
rasterization is presentation, not gameplay). The control-value writer is
still untraced: `ScreenGame::set_myroom_notice_text` stages content
without inventing a caption. Guards:
`myroom_staged_notice_text_inks_its_bnd_rect` and
`myroom_notice_board_selects_frame_two_when_the_body_has_it`.

## FREEINFO lines and skill rows (ticket #800, 2026-11-22)

`FUN_004a8010` (VA `0x4a8010`, file `0xa8010`,
EV `ev_bf4287e718ef7c085b5513838dc663397805504008bfd6e740c4e1eb4031d3d2`)
writes FREEINFO1 `0x15400041` (first line in colour `0xd2b559`), FREEINFO2
`0x42`, and up to four 28-byte skill rows starting at `0x43` from the pass
word `0x768b34`. Pass 1 is the PC-room label; 2/4/8/0x10/0x40/0x80/0x200/
0x400/0x420 are the all-songs label with a date; 0x20/0x800 are the free-all
songs label; 0x100 adds a second date line. The string bytes are pinned at
files `0x2494a4`–`0x2495ec`.

Our client stages the pass word explicitly (`ScreenGame::set_myroom_pass`);
the wire writer of `0x768b34`, the date and the skill-record source stay
OPEN, so no unverified pass is claimed. Presentation guard:
`myroom_staged_pass_inks_the_pinned_freeinfo_lines`; copy guards in
`screens/side/myroom_freeinfo.rs`. A known-pass exe capture is the next
evidence step.

## PC-room mark (ticket #811, 2026-11-23)

`FUN_004a3730` (VA `0x4a3730`, file `0xa3730`, fresh REA 6.9.0 /
Ghidra 12.2 dossier
`ev_1b8cb1354f11145172bc8bdb441a1e311a2176cd19b4f64a09b67ab1df39c30f`,
exe sha256 `83a04541…72c823b`) stages the avatar at `0x15700012`, then
tests pass word `0x768b34` for exactly `1` or `0x800`. Only those two
words look up PCROOMMARK `0x1550006a` and set its view/frame words to
frame 1; all other words have no else arm, so no mark is staged.

Our painters (`ScreenGame::paint_side` and `paint_myroom`) read the
already-staged `MyRoomPassInfo.word` (#800): the mark is kept only for
`1`/`0x800` and its second frame is selected there; every other word hides
the placement. The staged body carries no invented geometry, and the old
P0 image-only verdict is retired. Guard:
`myroom_pc_room_mark_stages_frame_one_only_for_the_exe_pass_words`
(synthetic two-frame body, no install-root skip).
OPEN: one live PC-room and one non-PC-room frame with the paired pass
word; the wire writer of `0x768b34` remains untraced, so no pass is
inferred from a 2001 field.

## Special-item replies (ticket #799 body, 2026-11-22)

`FUN_004a54e0` (VA `0x4a54e0`, file `0xa54e0`,
EV `ev_45f6c6d01e4fd29e88a8123c77b2088b9bd3f10d84604d3748e9ac2057c9e7b6`)
posts pinned one-OK copy for 5031 (nickname old→new at body `+0xb8`/`+0x1bc`,
error -97), 5033 (new-name confirmation verdict 0, code 1), 5035 (penalty
clear, error -98), 5037 (bag slots `after - 30 -> after`, error -96) and 5039
(bag-sort failure -95). Strings are copied by file offset into
`screens/side/myroom_notice.rs`; the sheet geometry is still
`Dialog_Information.bnd`. The 5031 success and 5035 success also fold into
the staged character (nickname, penalties). The exe's legacy encoding for
the interpolated name bytes stays OPEN; our wire strings are UTF-8.

`OnlineSession::poll_wire -> poll_myroom_replies` drains these opcodes only
while the MyRoom side owns an idle socket; a pending attempt keeps the
socket. Level-2 scripted-gateway guard:
`myroom_penalty_reply_opens_the_pinned_one_ok_sheet_through_scripted_gateway`.
The live known-state reply capture stays OPEN.

## Live boot data path (ticket #199 diagnose leg)

Three sources, three stagings — a blank MyRoom panel traces to
its source, never to "the data":

- Character record: staged live on channel-login boots only
  (`stage_online_identity` → `set_myroom_character`); the
  list-only default boot stages none.
- Bag/gift/info-plate name source: the `Itemdata_Korea.dat` table
  `set_avatar_source` stages beside the shop catalog — every
  archive landing, even item-less (before #199 it never landed:
  the overlay's early return kept every row blank on every boot).
- Portrait figure: the worn stack (`load_worn_figure` from the
  staged archive + table + hero equipment/gender) painted into
  the `0X15700012` AVATAR rect on the paint clock (#193 per-run
  breathe), kept live by the equip restage hook.

## Landed 2026-11-10 (ticket #19)

- Slot selection: 11 buttons dispatch by code ordinal through
  staged `.bnd` rects; selection records, clamps, and paints the
  amber stand-in cursor (highlight rendering OPEN, like planet).
  Guards: `myroom_slot_codes_cover_eleven_buttons_in_order`,
  `myroom_selection_starts_cleared_and_clamps`,
  `myroom_slot_click_selects_through_staged_rect`.
- Equip commit: picked bag cell + INVENTORY queues `EquipItem`
  5004 with the exe's slot derivation (type 5 → 11, else record
  slot byte, < 17; send shape + rule exe-pinned static,
  `docs/equip.md`); staged worn set mirrors. Control choice
  ours-by-design. Guards: `myroom_equip_slot_follows_the_exe_derivation`,
  `myroom_record_slot_reads_file_byte_0x18`,
  `myroom_equip_queues_5004_with_derived_slot`.
- Bag pager: 10 cells/page over owned inventory (zero/unknown
  skip), page turns clamp, names paint into staged MYBAGITEM
  rects. Guards: `myroom_bag_pages_hold_ten_owned_rows`,
  `myroom_bag_pager_turns_over_staged_inventory`.
- Gift flows: entry queues `GetGiftMessageList` 5017 (shape
  exe-pinned, timing ours-by-design); box pager + pick over
  staged item/music gifts (names resolved from parsed data);
  MOVE queues `ClaimGift` 5023 (shape exe-pinned;
  non-attributive). Server proofs ride the take-seam stub —
  ASSUMED. Guards: `myroom_entry_queues_gift_list_fetch`,
  `myroom_gift_views_resolve_names_and_senders`,
  `myroom_gift_claim_queues_5023_for_the_picked_cell`.
- Captions: nickname/level/bonus/exp/penalties/cash wallets by
  name identity + gift num/name/sender paint their TEXT rects;
  hand session trace + smoke note
  (`fixtures/input-trace/myroom-hand-session.json`,
  `docs/smoke/myroom-hand-session.md`; executed through the
  `ScreenGame` seam — no window binding stages sides).
- Entry/exit: `visit_side`/`leave_side` keep passing; BACK
  returns to the opening lobby
  (`myroom_back_click_returns_to_the_lobby`).

## Ticket #200 (2026-11-14) — bag source probe + vacate + slot counts

- Bag source PINNED (live Memoryer 7.2.0 probe, `tester2`): the 2001
  `inventory` run IS the owned-items source — `Bag1..30` in column
  order, zeros included (before fixture: 30 zeros; `Bag31+` gated by
  `BAG_EXT_COUNT`, still 0 on the rig). Setting `Bag1..3` =
  `376, 383, 35` returned `inventory=[376, 383, 35, 0×27]`, so the
  client pages exactly this run in server order, skipping zeros and
  unknown ids (`myroom_owned_rows`). No separate inventory reply and
  no equip-response items exist on this path (5005 is result + tail
  only, `docs/equip.md`). Fixture: tester2 `Bag1..3` hold `376, 383,
  35` (DB backup `~/memoryer-local/O2JAM.db.pre200.bak`); local DB
  rows only, never wire. Guard:
  `myroom_bag_pages_probed_inventory_in_server_order`.
- Vacate now clears the selected worn slot (same-ordinal button →
  equipment cell, ours-by-design) and restages the figure through
  the #193 hook; queues nothing (send shape still OPEN). Guards:
  `myroom_vacate_clears_the_selected_slot_and_restages`,
  `myroom_vacate_queues_no_wire_send`, and the live-channel-login
  assertion in `live_boot_myroom_data_path_paints_on_channel_login`
  (install-root skip) that clicks the bnd-paired ITEMSELL control at
  (25,491)-(110,534) and checks the worn slot clears. The guard was
  red-proved by disabling `apply_myroom_vacate`: the live assertion
  failed with equipment[0] still set.
- Equip restage unchanged (guard annotated, still green).
- Slot-filter counts for the next agent: 11 slot buttons
  (`0X15200005..0F`) vs 16 slot images with Korean slot comments
  (`0X15600013..22`) vs 17 equipment slots — no mapping between the
  three is pinned, so no filter is implemented.

## Ticket #201 (2026-11-14) — info plate + gift claim + P0 verdicts

- Info plate: NEXTUPEXP paints the staged experience (room-panel
  precedent — the 7.55 room capture shows `Exp.` there), WINRATION
  (`0x1540_003C`, exe spelling) paints integer percent from staged
  wins/battles (`0%` with no battles; percent sign and zero-battles
  fallback ours-by-design). The G readout (`MGAME 0x1540_003F`, bnd
  rect (685,421)-(780,434)) now inks the shared wallet's gem word:
  the hash-pinned exe frames `b095a915…` (G 15527) and `929c4817…`
  (G 626) match the live 5029 words `a73c0000` and `72020000` in
  `~/scratch/evidence-shops/uid1-server.jsonl` /
  `uid2-server.jsonl`. Guard:
  `myroom_money_captions_read_the_single_wallet` (red before the
  mapping: `None` vs `Some("15527")`). The W cell (`O2JAM 0x40`)
  inks the other settled 5029 word — the wallet cash slot the exe's
  file `0x163900` writes (`docs/shop-buy.md`); the same guard went red
  `None` vs `Some("22")` before that mapping. Control names pinned from the shipped
  `ControlList_Interface.txt` block dump (O2MONEY `0x10`,
  CHARGEMONEY `0x11`, NEXTUPEXP `0x3B`, WINRATION `0x3C`, RANK
  `0x3D`, GUILD `0x3E`, MGAME `0x3F`, O2JAM `0x40`,
  FREEINFO1..4 `0x41..44`, SKILLCOUNT1..10 `0x5C..65`,
  NOTICE `0x68/69`, PCROOMMARK `0x6A`, WEBAVATAR `0x58`,
  CONFERENCE `0x6B`, BAGSORT `0x6C`, MESSENGER `0x6D`,
  ALARM `0x6E/6F`). Guards:
  `myroom_info_plate_paints_win_rate_from_staged_battles`,
  `myroom_info_plate_paints_next_up_exp_from_staged_experience`,
  `myroom_info_plate_leaves_unpinned_money_rank_guild_skills_notice_blank`.
- Gift claim lands in the bag: `apply_gift_claim_success` removes the
  claimed gift from its staged box run and appends an item gift's
  resource to the staged inventory (music gifts only leave the box);
  the session bridge (`poll_gift_claim`/`drain_gift_claim`) folds a
  successful 5024 verdict through it. Scripted-gateway round trips:
  5017/5018 entry fetch (5018 body exe-ignored, drain-only) and
  5023/5024 claim-into-bag. Guards:
  `myroom_gift_claim_lands_item_in_staged_bag`,
  `myroom_gift_list_fetch_round_trips_through_scripted_gateway`,
  `myroom_gift_claim_round_trips_and_lands_in_bag_through_scripted_gateway`.
- P0 verdicts recorded per control (`myroom_p0_verdict`): WEBAVATAR,
  CONFERENCE, BAGSORT, MESSENGER queue nothing; ALARM/PCROOMMARK are
  image-only; NOTICE stays unpainted like waiting NOTICE. Vacate
  (ITEMSELL) keeps its #200 local-clear + restage with no wire send.
  Guard: `myroom_p0_controls_answer_nothing_and_queue_no_wire`.
- Highlight uses the archive's `Myroom_MybagSelect.oji` body on a
  populated staged cell only; the amber stand-in is gone (see the
  #305 note above).

## OPEN

1. Slot-selection semantics: selection records; what the exe
   filters or targets with it is unpinned — the 11 button ordinals,
   the 16 slot-image comments, and the 17 equipment slots have no
   pinned join. Next step: rig comparison of bag contents per slot
   button (does the bag filter at all, and by which slot set).
2. Apply/MOVE control choice (ours-by-design). Next step: rig
   walk of the myroom apply/claim gestures.
3. Entry gift-list fetch timing (ours-by-design). Next step: rig
   or gateway replay naming the entry fetch.
4. Caption mappings without a pinned source (O2MONEY/CHARGEMONEY,
   RANK, GUILD, FREEINFO1..4, SKILLCOUNT1..10, NOTICE content).
   WINRATION value source pinned (wins/battles), format ours-by-design;
   NEXTUPEXP paints experience per the room precedent (single-capture
   pin); MGAME G and O2JAM W ink the two settled 5029 wallet words
   (`docs/shop-buy.md`, file `0x163900`). Next step: rig capture of
   the userinfo plate at known stats naming each remaining rect's
   source and format.
5. File → record slot-byte join (ASSUMED). Next step: REA trace
   of the `0x52dba0` lookup showing the record layout.
6. Vacate clears the staged worn slot locally and restages but
   queues nothing; GOSHOP/webavatar/sell/bagsort/messenger/
   conference/pcroom/alarm still send nothing (P0 verdicts recorded
   in `myroom_p0_verdict`). The vacate button-ordinal →
   equipment-slot identity and the absent send stay OPEN. Next step:
   rig walk of the vacate gesture naming the slot and any wire send
   (per-control rig or dossier pins before any wire is queued).
7. Gift run order (items-first) and `is_attributive: false` are
   ours-by-design; the claim round trip is scripted-gateway only
   (live Memoryer replay names the server's 5024 verdict and any
   inventory echo). Next step: live gateway replay of the gift box.
8. Highlight rendering: the amber stand-in is gone; the archive frame
   body is staged, but which cell the exe highlights and whether it
   uses `Myroom_MybagSelect.oji` at all is still unpinned. Next step:
   qualified exe capture with a pressed slot/bag/gift cell.
