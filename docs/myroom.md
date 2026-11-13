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
  `myroom_vacate_queues_no_wire_send`.
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
  fallback ours-by-design). Control names pinned from the shipped
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
- Amber stand-in kept: highlight rendering still OPEN (no rig pins
  the real frames).

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
   MGAME/O2JAM money, RANK, GUILD, FREEINFO1..4, SKILLCOUNT1..10,
   NOTICE content). WINRATION value source pinned (wins/battles),
   format ours-by-design; NEXTUPEXP paints experience per the room
   precedent (single-capture pin). Next step: rig capture of the
   userinfo plate at known stats naming each remaining rect's source
   and format.
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
8. Highlight rendering: the amber cursor is an OPEN stand-in.
   Next step: rig capture of the selected-slot/cell frames.
