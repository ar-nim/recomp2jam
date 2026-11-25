# Interaction census — method and Room section

Purpose: a standing, per-state gap inventory that makes interaction parity
enumerable before playtest. Every interactive control gets one row with an
exe behavior pin and a verification status; no row is left without either a
certifying guard or the next evidence step. Room is filled in here; other
states point forward to their own sections.

Version pin: O2Jam 7.55 (`OTwo.exe` == `AngelO2Jam.exe`), 5,029,888 bytes,
SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4d2e9dd1def72c823b`.
Parent: [#916](https://github.com/ar-nim/recomp2jam/issues/916). Tier
definitions, red-proof, and the live caller-chain rule:
[`docs/live-path-reachability.md`](live-path-reachability.md).

## Method

Run this procedure for each `STATE_*` block. Keep the raw game data local
(`install-root/`, `extracted/`); the census stores control ids, codes, and
rect ordinals, never bulk archive bytes.

1. **Walk the `BOUND` section.** Parse every `IDC_*` entry in the block,
   including `SET` members, and emit one row per control id. Do not sort by
   file order or assume a positional pairing: the exe pairs by
   `rect ordinal = (code & 0xFFF) - 1`, bounds-checks against the `.bnd`
   count, and gives a miss the all-zero rect (pairing step VA `0x548640`,
   dossier `ev_3a869bb7…`; see [`docs/bnd-semantics.md`](bnd-semantics.md)).
   A zero-low-word container entry misses by design and is listed only for
   completeness.
2. **Expand `SET` groups per populated parent.** A member is template-local;
   match the parent by `code & 0xFF000` against the group's declared second
   parameter, then clone the member for each populated parent at
   `parent origin + member rect + anchor`. Skip parents past the live
   row/seat count. In Room, `SET 0x04 0x00021000` parents on
   `IDC_SCROLL_SROOM_CHAT` (`0x13021016`); `SET 0x11 0x00012000` parents on
   the six `IDC_BUTTON_SROOM_EXBUTTON*` rows (`0x1301201B..20`). The staging
   implementation is `avatar::expand_set_clones`; group-level exe behavior
   beyond the per-entry rule remains OPEN in `docs/bnd-semantics.md` OPEN 3.
3. **Classify by code family.** `0x132…` = button, `0x132010…` = tab/filter,
   `0x1302…` = scrollbar, `0x133…` = edit, `0x138…` = list; `0x130120…` row
   panels are interactive templates. `0x134…` text, `0x135…` static image,
   and `0x131…`/`0x136…`/`0x137…` art/avatar entries are paint-only. Add
   keyboard paths, hover, and navigation edges as extra rows even when no
   `.bnd` rect exists for them.
4. **Pin exe behavior, not staged art.** For each row record what press,
   hover, key, or navigation actually does, from statics/REA or a rig
   capture. Where a guard and a playtest disagree, keep both and mark the
   contradiction OPEN for the exe to arbitrate: `#536`'s local
   ALL/NORMAL/LIVE filter pin versus the `[W]`/`[A]` report in `#290`, and
   `#88`'s Result BACK-to-Room pin versus `#294`.
5. **Set the status with one of two exact forms.**
   - `VERIFIED: <guard name> (Tier N)` — the guard has a live chain from a
     user entry to the guarded symbol, and its assertion covers the exe pin.
     A Tier 0 guard or a settled screenshot alone never verifies a
     player-visible row.
   - `OPEN: <next evidence step>` — name the REA/rig/capture or live guard
     still needed. Seam guards may be cited as context but never replace the
     step. Paint-only statics get a Tier 3 live-art pin as their step.
6. **Gate the inventory.** A future CI gate must fail when an interactive
   row has neither form, or when a cited guard name does not exist in
   `crates/`. Guard names in this document are cited without line numbers so
   code moves do not silently drift the census. The in-code source of many
   next steps is `ROOM_CONTROLS` (`HANDLER_STEP`, `CHAT_TAB_STEP`) in
   `crates/o2jam-shell/src/screens/game/mod.rs`.

## Room

Source: the `BOUND "State_Room.bnd"` block of the pinned
`ControlList_Interface.txt` (archive entry slot 1595, body 90,472 B,
SHA-256 `5d627fe33a767fa38536fd554d6b605c253bbf01d0823f37190f8f304a0dc4af`)
and `State_Room.bnd` (75 rects, 1,206 B). The walk finds **76 entries / 75
rects**: **37 interactive rows** and **39 static/container rows** below, plus
**3 non-`.bnd` input edges**. Every control id is listed once; ordinal is
`(code & 0xFFF) - 1`, with `-1` for the block self.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_STATE_ROOM` (`0x13000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | `OPEN: statics pin of the container code; no input row` |
| 0 | `IDC_BACK_SROOM_1` (`0x13100001`) | static art | 800x600 backdrop | `OPEN: Tier 3 live-art backdrop pin; seam every_bound_entry_paints_at_its_placement_origin (Tier 0)` |
| 1 | `IDC_BUTTON_SROOM_BACK` (`0x13200002`) | nav-edge | Chrome returns to Channel; menu Back is ours-by-design (no exe scene key handler); leave opcode unpinned (`#38`, `docs/back-navigation-735.md`) | `OPEN: Tier 3 live click plus gateway opcode; seam click_staged_room_back_returns_to_channel (Tier 0)` |
| 2 | `IDC_BUTTON_SROOM_SHOW_ALLMODE` (`0x13201003`) | tab | Local 2003 row filter only; peer tap carried no 2002 (W6FB-PEER-02, `#536`); `[W]` toggle reported dead (`#290`) | `OPEN: live click and both toggle edges; seam room_filter_tabs_keep_only_their_family_of_rows (Tier 0)` |
| 3 | `IDC_BUTTON_SROOM_SHOW_NORMALMODE` (`0x13201004`) | tab | Local row filter; `[A]` toggle reported dead (`#290`) | `OPEN: live click and both toggle edges; seam room_filter_tabs_keep_only_their_family_of_rows (Tier 0)` |
| 4 | `IDC_BUTTON_SROOM_SHOW_BATTLEMODE` (`0x13201005`) | tab | Local row filter (W6FB-PEER-02, `#536`) | `OPEN: live click and both toggle edges; seam room_filter_tabs_keep_only_their_family_of_rows (Tier 0)` |
| 5 | `IDC_BUTTON_SROOM_QUICKJOIN` (`0x13200006`) | button | Join first open room; wire rule unpinned (`#32`) | `OPEN: REA click dispatch plus Tier 3 click; seam staged_room_control_without_a_target_records_a_labelled_noop (Tier 0, HANDLER_STEP)` |
| 6 | `IDC_BUTTON_SROOM_CREATE` (`0x13200007`) | nav-edge | Opens create sheet; Commit 2004 → Waiting as master (`#53`, `#269`) | `OPEN: live Room CREATE press; seams live_staged_create_sheet_commits_a_title (Tier 3) and waiting_create_room_commit_stages_room_over_scripted_gateway (Tier 2)` |
| 7 | `IDC_BUTTON_SROOM_ITEMSHOP` (`0x13200008`) | nav-edge | Enters Item Shop side (`#32`); `#178` T7 reports its browse chrome dead | `OPEN: live click plus destination guard; seam staged_room_chrome_buttons_reach_their_side_views (Tier 0)` |
| 8 | `IDC_BUTTON_SROOM_MYROOM` (`0x13200009`) | nav-edge | Enters MyRoom side (`#32`); `#178` T8 target disputed | `OPEN: live click plus destination guard; seam staged_room_chrome_buttons_reach_their_side_views (Tier 0)` |
| 9 | `IDC_BUTTON_SROOM_COUPON` (`0x1320000A`) | button | Web/promo chrome; label is not behavior (`#32`, `#169`) | `OPEN: REA click dispatch plus Tier 3 click; seam staged_room_control_without_a_target_records_a_labelled_noop (Tier 0)` |
| 10 | `IDC_BUTTON_SROOM_OPTION` (`0x1320000B`) | button | Opens Options over Room (`#31`); `#314` reports inert keys/hover there | `OPEN: live click plus key-capture leg; seam click_staged_room_option_opens_options_over_room (Tier 0)` |
| 11 | `IDC_BUTTON_SROOM_LEFT` (`0x1320000C`) | pager | Room-list pager; untraced after `#55` | `OPEN: REA pager dispatch plus Tier 3 click/viewport; HANDLER_STEP` |
| 12 | `IDC_BUTTON_SROOM_RIGHT` (`0x1320000D`) | pager | Room-list pager; untraced after `#55` | `OPEN: REA pager dispatch plus Tier 3 click/viewport; HANDLER_STEP` |
| 13 | `IDC_BUTTON_SROOM_CHAT_ALL` (`0x1320100E`) | chat tab | Chat filter untraced | `OPEN: rig click plus filtered-panel frame; CHAT_TAB_STEP` |
| 14 | `IDC_BUTTON_SROOM_CHAT_FRIEND` (`0x1320100F`) | chat tab | Chat filter untraced | `OPEN: rig click plus filtered-panel frame; CHAT_TAB_STEP` |
| 15 | `IDC_BUTTON_SROOM_CHAT_GUILD` (`0x13201010`) | chat tab | Chat filter untraced | `OPEN: rig click plus filtered-panel frame; CHAT_TAB_STEP` |
| 16 | `IDC_BUTTON_SROOM_CHAT_WHISPER` (`0x13201011`) | chat tab | Chat filter untraced | `OPEN: rig click plus filtered-panel frame; CHAT_TAB_STEP` |
| 17 | `IDC_TEXT_SROOM_NICKNAME` (`0x13400012`) | static text | Nickname readout on the identity panel | `OPEN: Tier 3 live-art text pin; seam room_user_panel_paints_character_values_into_staged_wells (Tier 0)` |
| 18 | `IDC_IMAGE_SROOM_AVATA` (`0x13700013`) | static art | Worn-figure slot | `OPEN: Tier 3 live-art figure pin; seam room_user_panel_paints_the_worn_figure_in_its_slot (Tier 0)` |
| 19 | `IDC_LIST_SROOM_CHAT` (`0x13800014`) | list | Chat rows inside the staged list rect | `OPEN: Tier 3 live key/2012-row leg; seam room_chat_rows_paint_inside_the_staged_list_rect (Tier 0)` |
| 20 | `IDC_EDIT_SROOM_CHAT` (`0x13300015`) | edit / key-path | One 2012 send per Enter (`#37`) | `OPEN: live Enter plus wire leg; seam typing_on_the_room_scene_fills_the_room_chat_draft (Tier 0)` |
| 21 | `IDC_SCROLL_SROOM_CHAT` (`0x13021016`) | scrollbar | Chat scroll; behavior untraced; parent of `SET 0x00021000` | `OPEN: REA scroll dispatch plus Tier 3 drag/click` |
| 22 | `IDC_BUTTON_SROOM_SCROLLUP` (`0x13200017`) | button (`SET` child) | Scroll-up arrow; untraced | `OPEN: REA arrow dispatch plus Tier 3 click/viewport` |
| 23 | `IDC_BUTTON_SROOM_SCROLLDOWN` (`0x13200018`) | button (`SET` child) | Scroll-down arrow; untraced | `OPEN: REA arrow dispatch plus Tier 3 click/viewport` |
| 24 | `IDC_BUTTON_SROOM_SCROLLBAR` (`0x13200019`) | button (`SET` child) | Scroll thumb; untraced | `OPEN: REA thumb dispatch plus Tier 3 drag/viewport` |
| 25 | `IDC_BUTTON_SROOM_SCROLLRANGE` (`0x1320001A`) | button (`SET` child) | Scroll track/hit range; untraced | `OPEN: REA track dispatch plus Tier 3 click/viewport` |
| 26 | `IDC_BUTTON_SROOM_EXBUTTON1` (`0x1301201B`) | row panel (`SET` parent) | Row 1: entry face enters Waiting, body highlights (`#56`); children clone per populated row (`#35`) | `OPEN: Tier 3 live row-click target; seams click_selects_staged_room_panel and room_number_clones_paint_their_digit_frames_in_place (Tier 0)` |
| 27 | `IDC_BUTTON_SROOM_EXBUTTON2` (`0x1301201C`) | row panel (`SET` parent) | Row 2: same entry-face/body split and clone rule | `OPEN: Tier 3 live row-click target; seams click_selects_staged_room_panel and room_number_clones_paint_their_digit_frames_in_place (Tier 0)` |
| 28 | `IDC_BUTTON_SROOM_EXBUTTON3` (`0x1301201D`) | row panel (`SET` parent) | Row 3: same entry-face/body split and clone rule | `OPEN: Tier 3 live row-click target; seams click_selects_staged_room_panel and room_number_clones_paint_their_digit_frames_in_place (Tier 0)` |
| 29 | `IDC_BUTTON_SROOM_EXBUTTON5` (`0x1301201E`) | row panel (`SET` parent) | Row 4 (authored `5`): same entry-face/body split and clone rule | `OPEN: Tier 3 live row-click target; seams click_selects_staged_room_panel and room_number_clones_paint_their_digit_frames_in_place (Tier 0)` |
| 30 | `IDC_BUTTON_SROOM_EXBUTTON4` (`0x1301201F`) | row panel (`SET` parent) | Row 5 (authored `4`): same entry-face/body split and clone rule | `OPEN: Tier 3 live row-click target; seams click_selects_staged_room_panel and room_number_clones_paint_their_digit_frames_in_place (Tier 0)` |
| 31 | `IDC_BUTTON_SROOM_EXBUTTON6` (`0x13012020`) | row panel (`SET` parent) | Row 6: same entry-face/body split and clone rule | `OPEN: Tier 3 live row-click target; seams click_selects_staged_room_panel and room_number_clones_paint_their_digit_frames_in_place (Tier 0)` |
| 32 | `IDC_TEXT_SROOM_ROOMNAME` (`0x13400021`) | static text (`SET` child) | Room title on a populated row | `OPEN: Tier 3 live-art pin; seam room_title_anchors_to_the_roomname_rect (Tier 0)` |
| 33 | `IDC_TEXT_SROOM_ROOM_USER` (`0x13400022`) | static text (`SET` child) | Seat count on a populated row | `OPEN: Tier 3 live-art pin; seam room_rows_split_number_title_and_count_to_pinned_offsets (Tier 0)` |
| 34 | `IDC_TEXT_SROOM_MUSICTITLE` (`0x13400023`) | static text (`SET` child) | Music title on a populated row | `OPEN: Tier 3 live-art pin; seam room_details_anchor_to_bnd_rects (Tier 0)` |
| 35 | `IDC_TEXT_SROOM_MUSICTITLE2` (`0x13400024`) | static text (`SET` child) | Second music-title line | `OPEN: Tier 3 live-art pin; seam room_details_anchor_to_bnd_rects (Tier 0)` |
| 36 | `IDC_IMAGE_SROOM_NEWIMG` (`0x13500025`) | static icon (`SET` child) | New-music mark | `OPEN: Tier 3 live-art pin; seam room_icons_paint_their_frames_at_the_rect_origins (Tier 0)` |
| 37 | `IDC_IMAGE_SROOM_NOTHAVE` (`0x13500026`) | static icon (`SET` child) | No-song mark | `OPEN: Tier 3 live-art pin; seam room_icons_paint_their_frames_at_the_rect_origins (Tier 0)` |
| 38 | `IDC_IMAGE_SROOM_SPEED` (`0x13500027`) | static icon (`SET` child) | Speed badge | `OPEN: Tier 3 live-art pin; seam room_icons_paint_their_frames_at_the_rect_origins (Tier 0)` |
| 39 | `IDC_IMAGE_SROOM_MODE` (`0x13500028`) | static icon (`SET` child) | Mode badge | `OPEN: Tier 3 live-art pin; seam room_icons_paint_their_frames_at_the_rect_origins (Tier 0)` |
| 40 | `IDC_IMAGE_SROOM_OM` (`0x13500029`) | static icon (`SET` child) | OM badge | `OPEN: Tier 3 live-art pin; seam room_icons_paint_their_frames_at_the_rect_origins (Tier 0)` |
| 41 | `IDC_IMAGE_SROOM_LEVEL` (`0x1350002A`) | static icon (`SET` child) | Level-limit badge | `OPEN: Tier 3 live-art pin; seam room_icons_paint_their_frames_at_the_rect_origins (Tier 0)` |
| 42 | `IDC_TEXT_SROOM_LEVEL_RANGE` (`0x1340002B`) | static text (`SET` child) | Level range | `OPEN: Tier 3 live-art pin; seam room_details_anchor_to_bnd_rects (Tier 0)` |
| 43 | `IDC_IMAGE_SROOM_PASSWORD` (`0x1350002C`) | static icon (`SET` child) | Lock badge | `OPEN: Tier 3 live-art pin; seam unlocked_room_row_hides_the_padlock_but_locked_room_row_shows_it (Tier 0)` |
| 44 | `IDC_IMAGE_SROOM_STATE` (`0x1350002D`) | static icon (`SET` child) | Room state badge | `OPEN: Tier 3 live-art pin; seam room_state_icon_frame_tracks_the_state_byte (Tier 0)` |
| 45 | `IDC_IMAGE_SROOM_ROOMNO1` (`0x1350002E`) | static digit (`SET` child) | Ones digit of room number | `OPEN: Tier 3 live-art pin; seam room_number_clones_paint_their_digit_frames_in_place (Tier 0)` |
| 46 | `IDC_IMAGE_SROOM_ROOMNO10` (`0x1350002F`) | static digit (`SET` child) | Tens digit of room number | `OPEN: Tier 3 live-art pin; seam room_number_clones_paint_their_digit_frames_in_place (Tier 0)` |
| 47 | `IDC_IMAGE_SROOM_ROOMNO100` (`0x13500030`) | static digit (`SET` child) | Hundreds digit of room number | `OPEN: Tier 3 live-art pin; seam room_number_clones_paint_their_digit_frames_in_place (Tier 0)` |
| 48 | `IDC_IMAGE_SROOM_SKILLPREMIUM` (`0x13500031`) | static icon (`SET` child) | Skill-premium badge | `OPEN: Tier 3 live-art pin; seam room_icons_paint_their_frames_at_the_rect_origins (Tier 0)` |
| 49 | `IDC_BUTTON_SROOM_FIRSTSTEP` (`0x13200032`) | nav-edge | Tutorial entry (static `S-PLAY-B3`, `#34`) | `OPEN: Tier 3 live click; seams staged_firststep_button_enters_the_tutorial_flow and only_the_pinned_firststep_dispatch_can_enter_the_tutorial (Tier 0)` |
| 50 | `IDC_IMAGE_SROOM_NEWMUSIC` (`0x13500033`) | static art | New-music flash | `OPEN: Tier 3 live-art pin; seam every_bound_entry_paints_at_its_placement_origin (Tier 0)` |
| 51 | `IDC_IMAGE_SROOM_GIFTALERT` (`0x13600034`) | static art | Gift alert (`SET`-coded image, no group in this block) | `OPEN: statics pin plus Tier 3 live-art frame` |
| 52 | `IDC_IMAGE_SROOM_GIFTALERT_ANI` (`0x13500035`) | static art | Gift alert animation | `OPEN: statics pin plus Tier 3 live-art frame` |
| 53 | `IDC_TEXT_SROOM_MYRANK` (`0x13400036`) | static text | Rank readout | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 54 | `IDC_BUTTON_SROOM_MUSICSHOP` (`0x13200037`) | nav-edge | Enters Music Shop side (`#32`) | `OPEN: live click plus destination guard; seam staged_room_chrome_buttons_reach_their_side_views (Tier 0)` |
| 55 | `IDC_BUTTON_SROOM_MYCAPSULE` (`0x13200038`) | button | Web/promo or hidden chrome; label is not behavior (`#32`, `#169`) | `OPEN: REA click dispatch plus Tier 3 click; seam staged_room_control_without_a_target_records_a_labelled_noop (Tier 0)` |
| 56 | `IDC_BUTTON_SROOM_EVENT` (`0x13200039`) | button | Web/promo or hidden chrome | `OPEN: REA click dispatch plus Tier 3 click; seam staged_room_control_without_a_target_records_a_labelled_noop (Tier 0)` |
| 57 | `IDC_BUTTON_SROOM_MYBAND` (`0x1320003A`) | button | Exe hides on room setup; the Lanking route in `ROOM_CONTROLS` is held out of dispatch | `OPEN: REA visibility/unhide condition plus Tier 3 click` |
| 58 | `IDC_IMAGE_SROOM_NOTICE_TEXT` (`0x1350003B`) | static art | Notice line | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 59 | `IDC_IMAGE_SROOM_PCROOMMARK` (`0x1350003C`) | static art | PC-room mark | `OPEN: statics pin plus Tier 3 live-art pin` |
| 60 | `IDC_BUTTON_SROOM_CONFERENCE` (`0x1320003D`) | button | Web/promo or hidden chrome | `OPEN: REA click dispatch plus Tier 3 click; seam staged_room_control_without_a_target_records_a_labelled_noop (Tier 0)` |
| 61 | `IDC_TEXT_SROOM_MYLEVEL` (`0x1340003E`) | static text | Level readout | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 62 | `IDC_TEXT_SROOM_EXP` (`0x1340003F`) | static text | Experience readout | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 63 | `IDC_TEXT_SROOM_WINLOSE` (`0x13400040`) | static text | Win/lose readout | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 64 | `IDC_TEXT_SROOM_BONUSPOINT` (`0x13400041`) | static text | Bonus point readout | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 65 | `IDC_TEXT_SROOM_NEXTUPEXP` (`0x13400042`) | static text | Next-up experience readout | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 66 | `IDC_TEXT_SROOM_PENALTYCOUNT` (`0x13400043`) | static text | Penalty count readout | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 67 | `IDC_TEXT_SROOM_PENALTYLEVEL` (`0x13400044`) | static text | Penalty level readout | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 68 | `IDC_TEXT_SROOM_GUILD` (`0x13400045`) | static text | Guild readout | `OPEN: statics pin plus Tier 3 live-art text pin` |
| 69 | `IDC_TEXT_SROOM_DESC` (`0x13400046`) | static text | Description well | `OPEN: Tier 3 live-art pin; seam room_user_panel_leaves_the_desc_well_empty_without_a_web_source (Tier 0)` |
| 70 | `IDC_IMAGE_SROOM_USERLIST_WEB` (`0x13500047`) | static art | Web user-list art | `OPEN: statics pin plus Tier 3 live-art pin` |
| 71 | `IDC_BUTTON_SROOM_MESSENGER` (`0x13200048`) | button | One-OK liveness sheet, no wire frame (wave-2 pin `w2-messenger-exe`, `#859`) | `VERIFIED: live_room_messenger_press_mounts_the_one_ok_liveness_sheet_from_boot (Tier 1 live input leg)` |
| 72 | `IDC_IMAGE_SROOM_MESSENGER_ALRAM` (`0x13500049`) | static art | Messenger alert icon at the messenger button rect | `OPEN: statics pin plus Tier 3 live-art pin` |
| 73 | `IDC_IMAGE_SROOM_MESSENGER_ALRAM1` (`0x1350004A`) | static art | Messenger alert popup | `OPEN: statics pin plus Tier 3 live-art pin` |
| 74 | `IDC_BUTTON_SROOM_WATINGVIEW` (`0x1320C04B`) | button | Web/promo or hidden chrome; the `0xC0` nibble rides in the low word | `OPEN: REA click dispatch plus Tier 3 click; seam staged_room_control_without_a_target_records_a_labelled_noop (Tier 0)` |

Input edges with no `.bnd` entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Room `BACK`/Esc key path | No exe scene key handler routes Esc to back; ours-by-design after `#735` statics | `OPEN: live key leg; seam room_back_game (Tier 0)` |
| Edit focus plus Enter key path | One 2012 body per Enter (`#37`) | `OPEN: live Enter plus wire leg; seam typing_on_the_room_scene_fills_the_room_chat_draft (Tier 0)` |
| Pointer hover over Room chrome | Rest desaturated, hover frame 0, active frame 1 (`docs/hover.md`, `#33`) | `OPEN: Tier 1 off/on/off pixel trio; seam hovered_room_button_repaints_on_an_empty_room_list (Tier 0)` |

## Certifying-guard map

These guards currently certify a defect or a partial story. They do not
close their rows; each points at the OPEN row or the state section still to
be written (#920).

| Guard | What it asserts | Row it touches |
|---|---|---|
| `shop_unwired_browse_chrome_lands_nowhere` | Shop browse chrome lands nowhere — exactly `#178` T7 | Item Shop rows, state section pending (#920); entry edge ord 7 above stays OPEN |
| `myroom_my_bag_tab_press_leaves_the_gift_panel_open` | MY BAG press leaves the gift panel open, citing exe3 frame `8515a7e5…` — exactly `#180` T6 | MyRoom tab, state section pending (#920) |
| `item_shop_go_myroom_leaves_without_an_exit_send` | Shop closes with no 5007 send; never asserts the destination state — `#178` T8 | Shop GO MYROOM, state section pending (#920); Room MYROOM entry edge ord 8 stays OPEN |
| `settled_solo_result_back_click_posts_4021_and_takes_the_4022_answer` | `4021 → 4022 → Room` from a settled panel, contradicted by `#294` T3 | Result BACK, state section pending (#920); Room entry target depends on which state wins |

2026-11-25 playtest bugs (`#915`, now split as below) mapped onto census
rows:

| Bug | Class | Census row |
|---|---|---|
| `#290` T1 `[W]`/`[A]` toggle | C1 | ord 2 and ord 3 (`SHOW_ALLMODE`, `SHOW_NORMALMODE`), both OPEN; ord 4 shares the filter guard |
| `#314` T2 Options keys inert / hover blanks | C2 + C4/C3 | ord 10 (`SHOW_…OPTION`), OPEN; Options key-tab rows: state section pending (#920) |
| `#294` T3 Result BACK target | C2 | Result state section pending (#920); Room nav-edge pin stays OPEN |
| `#294` T4 Result background source | C6 | Result state section pending (#920) |
| `#180` T5 MyRoom STATUS dead | C1 | MyRoom state section pending (#920) |
| `#180` T6 MyRoom present-box tab one-way | C2 | MyRoom state section pending (#920); certifying guard above |
| `#178` T7 Shop scrollbar/grid/tabs dead | C1 | Item Shop state section pending (#920); Room ITEMSHOP edge ord 7 stays OPEN |
| `#178` T8 Shop GO MYROOM target | C2 | Item Shop state section pending (#920); certifying guard above |

The two patterns to avoid are already visible here: a labelled no-op guard
that records the press but pins no target (C1), and a guard that asserts the
packet or local state but not the exe-pinned destination (C2). Both stay
OPEN until a live chain reaches the asserted outcome.

## Planet

Source: the `BOUND "State_Planet.bnd"` block of the pinned
`ControlList_Interface.txt` (lines 785–806; 18 entries / 17 rects),
with rects and hover behavior pinned in `docs/planet.md` and
`docs/hover.md`. Ordinal is `(code & 0xFFF) - 1`.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_STATE_PLANET` (`0x19000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_SPLANET_1` (`0x19100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_SPLANET_HARD` (`0x19201002`) | tab / toggle | HARD emblem; opens the Channel dialog and records the hard difficulty pick (`docs/planet.md`). | OPEN: live difficulty-pick + Channel-dialog leg; seam lit_emblem_click_opens_the_channel_dialog_and_records_the_pick (Tier 0) |
| 2 | `IDC_BUTTON_SPLANET_NORMAL` (`0x19201003`) | tab / toggle | NORMAL emblem; opens the Channel dialog and records the normal difficulty pick (`docs/planet.md`). | OPEN: live difficulty-pick + Channel-dialog leg; seam lit_emblem_click_opens_the_channel_dialog_and_records_the_pick (Tier 0) |
| 3 | `IDC_BUTTON_SPLANET_EASY` (`0x19201004`) | tab / toggle | EASY emblem; opens the Channel dialog and records the easy difficulty pick (live-confirmed 2026-11-09, `~/scratch/planet-verify/`). | OPEN: live difficulty-pick + Channel-dialog leg; seam lit_emblem_click_opens_the_channel_dialog_and_records_the_pick (Tier 0) |
| 4 | `IDC_BUTTON_SPLANET_FREE` (`0x19201005`) | tab / toggle | FREE tutorial emblem; commented undecided, no wired backend (`docs/planet.md`). | OPEN: REA trace of the FREE backend (none in `docs/planet.md`) plus rig click |
| 5 | `IDC_BUTTON_SPLANET_DRUM` (`0x19201006`) | tab / toggle | DRUM quest emblem; no wired backend (`docs/planet.md`, gameplay level-up note). | OPEN: REA trace of the DRUM backend (none in `docs/planet.md`) plus rig click |
| 6 | `IDC_TEXT_SPLANET_HARD` (`0x19400007`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 7 | `IDC_TEXT_SPLANET_NORMAL` (`0x19400008`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 8 | `IDC_TEXT_SPLANET_EASY` (`0x19400009`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 9 | `IDC_TEXT_SPLANET_FREE` (`0x1940000A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 10 | `IDC_TEXT_SPLANET_DRUM` (`0x1940000B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 11 | `IDC_IMAGE_SPLANET_PROMOTION` (`0x1950000C`) | static art | Promo banner art; carousel index changes its frame (`docs/planet.md`). | OPEN: statics pin plus Tier 3 live-art pin |
| 12 | `IDC_BUTTON_SPLANET_PROMOTION_LEFT` (`0x1920000D`) | pager | Promo carousel LEFT: wrap-step back one banner on its pinned rect (promo #129). | OPEN: Tier 3 carousel-step leg; seam planet_promo_cells_answer_clicks_at_their_staged_rects (Tier 0) |
| 13 | `IDC_BUTTON_SPLANET_PROMOTION_RIGHT` (`0x1920000E`) | pager | Promo carousel RIGHT: wrap-step forward one banner on its pinned rect (promo #129). | OPEN: Tier 3 carousel-step leg; seam planet_promo_cells_answer_clicks_at_their_staged_rects (Tier 0) |
| 14 | `IDC_BUTTON_SPLANET_PROMOTION_PLAYSTOP` (`0x1920C00F`) | tab / toggle | Promo PLAYSTOP: toggles the 3000 ms rotation; rest face follows the spin flag (#711). | OPEN: qualified hold-face capture (#711 residual); seam promo_stop_click_swaps_the_playstop_face_both_ways (Tier 0) |
| 15 | `IDC_BUTTON_SPLANET_EXIT` (`0x19200010`) | button | Power disc: opens the quit-confirm sheet; OK quits, CANCEL closes the sheet. | OPEN: live boot click + quit-confirm leg; seam power_disc_opens_quit_confirm_and_ok_quits (Tier 0) |
| 16 | `IDC_BUTTON_SPLANET_OPTION` (`0x19200011`) | button | Gear disc: opens the Options dialog over Planet; Escape closes it (#18). | OPEN: live boot click from Planet plus #314 key-capture leg; seam gear_disc_opens_options_and_escape_closes_it (Tier 0) |

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Planet pointer hover over emblems / discs / promo cells | Lit emblems reveal their own plate band; gear/exit/promo arrows restore frame 0; PLAYSTOP alone swaps frames; QUEST/FREE/DRUM stay exe-static (`docs/hover.md`). | OPEN: dwell ≥2 s region-locked capture (#116); seam hovered_planet_action_tracks_pointer_motion (Tier 0) |
| Pointer release outside a pressed Planet control | Reverts to REST with no action; release-inside fires on UP (`docs/hover.md`, #115). | OPEN: one qualified release-outside capture then Tier 3 pointer leg |

## Channel

Source: the bare `BOUND` line in `STATE_CHANNEL` of the pinned
`ControlList_Interface.txt` (lines 12–74; 50 entries / 49 ordinal slots,
no `.bnd` stem — all rects miss to zero by the statics pin in
`docs/channel-fill.md`). Ordinal is `(code & 0xFFF) - 1`.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_STATE_CHANNEL` (`0x12000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_SCHANNEL_1` (`0x12100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_SCHANNEL_BACK` (`0x12200002`) | button | Returns to Planet; menu-edge Back is ours-by-design (no exe scene key handler), opcode unpinned (#38, `docs/back-navigation-735.md`). | OPEN: live click plus gateway opcode; seam channel_back_closes_to_the_planet_board (Tier 0) |
| 2 | `IDC_BUTTON_SCHANNEL_RANK` (`0x12200003`) | button | Ranking chrome; destination unpinned (no live entry in `docs/parity/channel.md`). | OPEN: REA click dispatch plus Tier 3 click; seam channel_click_maps_through_viewport (Tier 0) |
| 3 | `IDC_BUTTON_SCHANNEL_CONTESTRANK` (`0x12200004`) | button | Contest-ranking chrome; destination unpinned. | OPEN: REA click dispatch plus Tier 3 click; seam channel_click_maps_through_viewport (Tier 0) |
| 4 | `IDC_BUTTON_SCHANNEL_LEFT` (`0x12200005`) | pager | Scroll-column arrow steps the list viewport top; highlight untouched (#55). | OPEN: Tier 3 arrow-click viewport leg; seam scroll_up_arrow_at_top_clamps_the_highlight (Tier 0) |
| 5 | `IDC_BUTTON_SCHANNEL_RIGHT` (`0x12200006`) | pager | Scroll-column arrow steps the viewport the other way; highlight untouched (#55). | OPEN: Tier 3 arrow-click viewport leg; seam scroll_down_arrow_steps_the_channel_highlight (Tier 0) |
| 6 | `IDC_BUTTON_SCHANNEL_EXBUTTON1` (`0x12011007`) | row panel (SET parent) | Channel row 1: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 7 | `IDC_BUTTON_SCHANNEL_EXBUTTON2` (`0x12011008`) | row panel (SET parent) | Channel row 2: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 8 | `IDC_BUTTON_SCHANNEL_EXBUTTON3` (`0x12011009`) | row panel (SET parent) | Channel row 3: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 9 | `IDC_BUTTON_SCHANNEL_EXBUTTON4` (`0x1201100A`) | row panel (SET parent) | Channel row 4: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 10 | `IDC_BUTTON_SCHANNEL_EXBUTTON5` (`0x1201100B`) | row panel (SET parent) | Channel row 5: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 11 | `IDC_BUTTON_SCHANNEL_EXBUTTON6` (`0x1201100C`) | row panel (SET parent) | Channel row 6: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 12 | `IDC_BUTTON_SCHANNEL_EXBUTTON7` (`0x1201100D`) | row panel (SET parent) | Channel row 7: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 13 | `IDC_BUTTON_SCHANNEL_EXBUTTON8` (`0x1201100E`) | row panel (SET parent) | Channel row 8: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 14 | `IDC_BUTTON_SCHANNEL_EXBUTTON9` (`0x1201100F`) | row panel (SET parent) | Channel row 9: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 15 | `IDC_BUTTON_SCHANNEL_EXBUTTON10` (`0x12011010`) | row panel (SET parent) | Channel row 10: entry face enters Room, body only highlights (#56); SET children clone per populated row (#35). | OPEN: Tier 3 entry-face vs body click leg; seam face_click_enters_the_highlighted_channel (Tier 0) |
| 16 | `IDC_IMAGE_SCHANNEL_ANI` (`0x12500011`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 17 | `IDC_IMAGE_SCHANNEL_NUMBER1` (`0x12500012`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 18 | `IDC_IMAGE_SCHANNEL_NUMBER10` (`0x12500013`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 19 | `IDC_IMAGE_SCHANNEL_NUMBER100` (`0x12500014`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 20 | `IDC_IMAGE_SCHANNEL_IN1` (`0x12500015`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 21 | `IDC_IMAGE_SCHANNEL_IN10` (`0x12500016`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 22 | `IDC_IMAGE_SCHANNEL_IN100` (`0x12500017`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 23 | `IDC_IMAGE_SCHANNEL_WHOLE1` (`0x12500018`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 24 | `IDC_IMAGE_SCHANNEL_WHOLE10` (`0x12500019`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 25 | `IDC_IMAGE_SCHANNEL_WHOLE100` (`0x1250001A`) | static art | SET child cloned per channel row; digits paint `channel_id + 1` (#28 proxy pin). | OPEN: statics pin plus Tier 3 live-art pin |
| 26 | `IDC_BUTTON_SCHANNEL_CHOBO` (`0x1220001B`) | button | Beginner family filter; only the category index rule is statics-pinned (`an_out_of_range_channel_family_index_stages_no_category_record`). | OPEN: REA category dispatch plus Tier 3 filtered-row capture |
| 27 | `IDC_BUTTON_SCHANNEL_JUNGSU` (`0x1220001C`) | button | Intermediate family filter; family-to-channel join unpinned. | OPEN: REA category dispatch plus Tier 3 filtered-row capture |
| 28 | `IDC_BUTTON_SCHANNEL_GOSU` (`0x1220001D`) | button | Advanced family filter; family-to-channel join unpinned. | OPEN: REA category dispatch plus Tier 3 filtered-row capture |
| 29 | `IDC_TEXT_SCHANNEL_TITLE1` (`0x1240001E`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 30 | `IDC_TEXT_SCHANNEL_TITLE2` (`0x1240001F`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 31 | `IDC_TEXT_SCHANNEL_TITLE3` (`0x12400020`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 32 | `IDC_TEXT_SCHANNEL_TITLE4` (`0x12400021`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 33 | `IDC_TEXT_SCHANNEL_TITLE5` (`0x12400022`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 34 | `IDC_TEXT_SCHANNEL_TITLE6` (`0x12400023`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 35 | `IDC_TEXT_SCHANNEL_TITLE7` (`0x12400024`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 36 | `IDC_TEXT_SCHANNEL_TITLE8` (`0x12400025`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 37 | `IDC_TEXT_SCHANNEL_TITLE9` (`0x12400026`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 38 | `IDC_TEXT_SCHANNEL_TITLE10` (`0x12400027`) | static text | SET child title cell cloned per channel row; name half of #28 was refuted by the 7.55 capture. | OPEN: statics pin plus Tier 3 live-art text pin |
| 39 | `IDC_IMAGE_SCHANNEL_COUNT1` (`0x12500028`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 40 | `IDC_IMAGE_SCHANNEL_COUNT2` (`0x12500029`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 41 | `IDC_IMAGE_SCHANNEL_COUNT3` (`0x1250002A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 42 | `IDC_IMAGE_SCHANNEL_COUNT4` (`0x1250002B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 43 | `IDC_IMAGE_SCHANNEL_COUNT5` (`0x1250002C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 44 | `IDC_IMAGE_SCHANNEL_COUNT6` (`0x1250002D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 45 | `IDC_IMAGE_SCHANNEL_COUNT7` (`0x1250002E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 46 | `IDC_IMAGE_SCHANNEL_COUNT8` (`0x1250002F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 47 | `IDC_IMAGE_SCHANNEL_COUNT9` (`0x12500030`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 48 | `IDC_IMAGE_SCHANNEL_COUNT10` (`0x12500031`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Channel Up/Down key path | Rig-pinned no-op on populated lists (#189); no VK_UP/VK_DOWN compare in `.text`. | OPEN: confirm-key sweep leg; seam channel_arrow_keys_leave_list_top_and_highlight_untouched (Tier 0) |
| Channel Enter/Confirm key path | Enters the highlighted channel into Room; exe key code unpinned (`docs/parity/channel.md`). | OPEN: SendInput key leg; seam channel_confirm_enters_highlighted_channel (Tier 0) |
| Channel BACK / Esc key path | No exe scene key handler routes Esc to back; menu Back is ours-by-design (#735). | OPEN: live key leg plus gateway opcode; seam `room_back_game` (Tier 0) |
| Pointer hover over scrolled channel rows | Select bar tracks the hovered row through the scrolled viewport (#26/#33). | OPEN: off/on/off pixel trio; seam channel_hover_follows_the_scrolled_viewport (Tier 0) |

## Waiting

Source: the `BOUND "State_Waiting.bnd"` block of the pinned
`ControlList_Interface.txt` (lines 587–753; 117 entries / 116 rects),
with static refresh slots pinned in `docs/waiting-chrome.md` and
team/chat inputs in `docs/parity/waiting.md`. Ordinal is `(code & 0xFFF) - 1`.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_STATE_WAIT` (`0x16000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_SWAIT_1` (`0x16100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_SWAIT_BACK` (`0x16200002`) | button | BACK leaves Waiting for Room with the pinned ExitWaiting 3005 body (#734). | VERIFIED: live_click_waiting_back_sends_the_room_leave_from_boot (Tier 3) — boot → pointer → 3005 → Room |
| 2 | `IDC_BUTTON_SWAIT_MUSELECT` (`0x16200003`) | button | Song-select button; master-only MUSELECT opens the dialog, non-master presses dropped (file `0xb2622`). | OPEN: master/guest live click leg plus REA host dispatch pin (#15) |
| 3 | `IDC_BUTTON_SWAIT_TEAMA` (`0x16201004`) | tab / toggle | Team 1 button: SetRoomTeam 4004 then Ready 4008; one lit frame per press (#771-adjacent toggle rule). | OPEN: live team-click + wire leg; seam click_staged_team_button_queues_4004_then_4008 (Tier 0) |
| 4 | `IDC_BUTTON_SWAIT_TEAMB` (`0x16201005`) | tab / toggle | Team 2 button: SetRoomTeam 4004 then Ready 4008; one lit frame per press (#771-adjacent toggle rule). | OPEN: live team-click + wire leg; seam click_staged_team_button_queues_4004_then_4008 (Tier 0) |
| 5 | `IDC_BUTTON_SWAIT_TEAMC` (`0x16201006`) | tab / toggle | Team 3 button: SetRoomTeam 4004 then Ready 4008; one lit frame per press (#771-adjacent toggle rule). | OPEN: live team-click + wire leg; seam click_staged_team_button_queues_4004_then_4008 (Tier 0) |
| 6 | `IDC_BUTTON_SWAIT_TEAMD` (`0x16201007`) | tab / toggle | Team 4 button: SetRoomTeam 4004 then Ready 4008; one lit frame per press (#771-adjacent toggle rule). | OPEN: live team-click + wire leg; seam click_staged_team_button_queues_4004_then_4008 (Tier 0) |
| 7 | `IDC_BUTTON_SWAIT_TEAME` (`0x16201008`) | tab / toggle | Team 5 button: SetRoomTeam 4004 then Ready 4008; one lit frame per press (#771-adjacent toggle rule). | OPEN: live team-click + wire leg; seam click_staged_team_button_queues_4004_then_4008 (Tier 0) |
| 8 | `IDC_BUTTON_SWAIT_TEAMF` (`0x16201009`) | tab / toggle | Team 6 button: SetRoomTeam 4004 then Ready 4008; one lit frame per press (#771-adjacent toggle rule). | OPEN: live team-click + wire leg; seam click_staged_team_button_queues_4004_then_4008 (Tier 0) |
| 9 | `IDC_BUTTON_SWAIT_TEAMG` (`0x1620100A`) | tab / toggle | Team 7 button: SetRoomTeam 4004 then Ready 4008; one lit frame per press (#771-adjacent toggle rule). | OPEN: live team-click + wire leg; seam click_staged_team_button_queues_4004_then_4008 (Tier 0) |
| 10 | `IDC_BUTTON_SWAIT_TEAMH` (`0x1620100B`) | tab / toggle | Team 8 button: SetRoomTeam 4004 then Ready 4008; one lit frame per press (#771-adjacent toggle rule). | OPEN: live team-click + wire leg; seam click_staged_team_button_queues_4004_then_4008 (Tier 0) |
| 11 | `IDC_BUTTON_SWAIT_MAPLEFT` (`0x1620000C`) | pager | Cycles the room map backward; cycle entry from the room word. | OPEN: live click plus map-name frame leg |
| 12 | `IDC_BUTTON_SWAIT_MAPRIGHT` (`0x1620000D`) | pager | Cycles the room map forward; cycle entry from the room word. | OPEN: live click plus map-name frame leg |
| 13 | `IDC_BUTTON_SWAIT_CHAT_ALL` (`0x1620100E`) | tab / toggle | Chat filter ALL; active at Waiting entry (#771 static pin). | OPEN: filtered-panel frame per tab; seam chat_tab_clicks_switch_the_waiting_feed_and_clear_its_buffers (Tier 0) |
| 14 | `IDC_BUTTON_SWAIT_CHAT_FRIEND` (`0x1620100F`) | tab / toggle | Chat filter FRIEND; exact feed mapping unpinned. | OPEN: filtered-panel frame per tab; seam chat_tab_clicks_switch_the_waiting_feed_and_clear_its_buffers (Tier 0) |
| 15 | `IDC_BUTTON_SWAIT_CHAT_GUILD` (`0x16201010`) | tab / toggle | Chat filter GUILD; exact feed mapping unpinned. | OPEN: filtered-panel frame per tab; seam chat_tab_clicks_switch_the_waiting_feed_and_clear_its_buffers (Tier 0) |
| 16 | `IDC_BUTTON_SWAIT_CHAT_WHISPER` (`0x16201011`) | tab / toggle | Chat filter WHISPER; exact feed mapping unpinned. | OPEN: filtered-panel frame per tab; seam chat_tab_clicks_switch_the_waiting_feed_and_clear_its_buffers (Tier 0) |
| 17 | `IDC_BUTTON_SWAIT_START` (`0x16202012`) | button | Host START: readiness gate then StartGame 4010 into Loading. | VERIFIED: live_click_master_start_then_back_from_boot (Tier 3) — boot → staged rect → 4010 |
| 18 | `IDC_BUTTON_SWAIT_READY` (`0x16202013`) | button | Guest READY: Ready 4008 toggle, stays in Waiting. | VERIFIED: live_click_guest_ready_then_back_from_boot (Tier 3) — boot → staged rect → 4008 |
| 19 | `IDC_EDIT_SWAIT_CHAT` (`0x16300014`) | edit / key-path | Chat draft; one 3012 body per Enter (`docs/parity/waiting.md`). | OPEN: live Enter + wire leg; seam confirm_sends_the_draft_and_stays_waiting (Tier 0) |
| 20 | `IDC_IMAGE_SWAIT_FREEUSER` (`0x16500015`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 21 | `IDC_TEXT_SWAIT_ROOMNAME` (`0x16400016`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 22 | `IDC_TEXT_SWAIT_MUSICNAME` (`0x16400017`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 23 | `IDC_TEXT_SWAIT_MAPNAME` (`0x16400018`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 24 | `IDC_TEXT_SWAIT_INSTNAME` (`0x16400019`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 25 | `IDC_IMAGE_SWAIT_MAP` (`0x1650001A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 26 | `IDC_IMAGE_SWAIT_LEVEL` (`0x1650001B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 27 | `IDC_IMAGE_SWAIT_ALBUMMODE` (`0x1650001C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 28 | `IDC_IMAGE_SWAIT_COVERMUSIC` (`0x1650001D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 29 | `IDC_IMAGE_SWAIT_COVERLEFT` (`0x1650001E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 30 | `IDC_IMAGE_SWAIT_COVERRIGHT` (`0x1650001F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 31 | `IDC_BUTTON_SWAIT_TOPEFFECT` (`0x16201020`) | tab / toggle | TOP effect toggle: writes global `0x7789e0` to 0; exclusive paired frame (#771). | OPEN: paired toggle capture per note-effect layer (#771); seam waiting_top_buttons_sit_on_their_own_block_rects (Tier 0) |
| 32 | `IDC_BUTTON_SWAIT_BOTTOMEFFECT` (`0x16201021`) | tab / toggle | BOTTOM effect toggle: writes global `0x7789e0` to 1; exclusive paired frame (#771, default). | OPEN: paired toggle capture per note-effect layer (#771); seam waiting_top_buttons_sit_on_their_own_block_rects (Tier 0) |
| 33 | `IDC_BUTTON_SWAIT_EXTEND1` (`0x16015022`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 34 | `IDC_BUTTON_SWAIT_EXTEND2` (`0x16015023`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 35 | `IDC_BUTTON_SWAIT_EXTEND3` (`0x16015024`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 36 | `IDC_BUTTON_SWAIT_EXTEND4` (`0x16015025`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 37 | `IDC_BUTTON_SWAIT_EXTEND5` (`0x16015026`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 38 | `IDC_BUTTON_SWAIT_EXTEND6` (`0x16015027`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 39 | `IDC_BUTTON_SWAIT_EXTEND7` (`0x16015028`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 40 | `IDC_BUTTON_SWAIT_EXTEND8` (`0x16015029`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 41 | `IDC_SCROLL_SWAIT_CHAT` (`0x1602102A`) | scrollbar | Chat scrollbar; behavior unpinned beyond its staged rect. | OPEN: REA scroll dispatch plus Tier 3 drag/click from boot |
| 42 | `IDC_LIST_SWAIT_CHAT` (`0x1680002B`) | list | List viewport at the paired rect; row input unpinned | OPEN: rig capture of the press plus a Tier 3 live-input leg |
| 43 | `IDC_IMAGE_SWAIT_READYWINDOW` (`0x1650002C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 44 | `IDC_TEXT_SWAIT_AVATARINFO` (`0x1640002D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 45 | `IDC_IMAGE_SWAIT_AVATAR` (`0x1670002E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 46 | `IDC_IMAGE_SWIAT_BOSSMARK` (`0x1650002F`) | static art | Master emblem: wire creator flag selects the frame (#303); one own-slot write only. | OPEN: statics pin plus Tier 3 live-art pin |
| 47 | `IDC_IMAGE_SWIAT_READYIMAGE` (`0x16500030`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 48 | `IDC_IMAGE_SWAIT_WAITINGIMAGE` (`0x16500031`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 49 | `IDC_IMAGE_SWIAT_NOMUSIMAGE` (`0x16500032`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 50 | `IDC_IMAGE_SWIAT_NOGEMIMAGE` (`0x16500033`) | static art | Seat NoGem frame when the +0x217c byte is 2 (#771). | OPEN: two-client capture isolating the music-state byte (#771) |
| 51 | `IDC_IMAGE_SWIAT_PCROOMMARK` (`0x16500034`) | static art | Seat PC-room frame on a nonzero +0x212c word (#771). | OPEN: two-client capture isolating the free-pass word (#771) |
| 52 | `IDC_IMAGE_SWIAT_DOWNLODING` (`0x16500035`) | static art | Seat DOWNLOADING badge when +0x217c == 4 (`docs/waiting-chrome.md`). | OPEN: matched A/B capture with a rewritten +0x217c byte |
| 53 | `IDC_IMAGE_SWIAT_DOWNBAR` (`0x16500036`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 54 | `IDC_IMAGE_SWIAT_PLAYING` (`0x16500037`) | static art | Seat PLAYING frame when +0x2130 == 1 (#771). | OPEN: two-client capture isolating the playing-state word (#771) |
| 55 | `IDC_BUTTON_SWAIT_SCROLLUP` (`0x16200038`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 56 | `IDC_BUTTON_SWAIT_SCROLLDOWN` (`0x16200039`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 57 | `IDC_BUTTON_SWAIT_SCROLLBAR` (`0x1620003A`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 58 | `IDC_BUTTON_SWAIT_SCROLLRANGE` (`0x1620003B`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 59 | `IDC_TEXT_SWAIT_CHAT` (`0x1640003C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 60 | `IDC_IMAGE_SWAIT_COVERAVATAR1` (`0x1650003D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 61 | `IDC_IMAGE_SWAIT_COVERAVATAR2` (`0x1650003E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 62 | `IDC_IMAGE_SWAIT_COVERAVATAR3` (`0x1650003F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 63 | `IDC_IMAGE_SWAIT_COVERAVATAR4` (`0x16500040`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 64 | `IDC_IMAGE_SWAIT_COVERAVATAR5` (`0x16500041`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 65 | `IDC_IMAGE_SWAIT_COVERAVATAR6` (`0x16500042`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 66 | `IDC_IMAGE_SWAIT_COVERAVATAR7` (`0x16500043`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 67 | `IDC_IMAGE_SWAIT_COVERAVATAR8` (`0x16500044`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 68 | `IDC_IMAGE_SWAIT_EFFECT_1` (`0x16500045`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 69 | `IDC_IMAGE_SWAIT_EFFECT_2` (`0x16500046`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 70 | `IDC_BUTTON_SWAIT_CHANGE_TITLE` (`0x16200047`) | button | Opens the change-title sheet; sends the pinned dialog message. | OPEN: live click plus dialog-frame leg; seam change_title_and_emotion_presses_record_pinned_dialog_messages (Tier 0) |
| 71 | `IDC_BUTTON_SWAIT_EMOTION` (`0x16200048`) | button | Opens the emotion-help sheet; sends the pinned dialog message. | OPEN: live click plus dialog-frame leg; seam change_title_and_emotion_presses_record_pinned_dialog_messages (Tier 0) |
| 72 | `IDC_IMAGE_SWAIT_CHCATEGORY` (`0x16500049`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 73 | `IDC_IMAGE_SWAIT_CHNUM_10` (`0x1650004A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 74 | `IDC_IMAGE_SWAIT_CHNUM_01` (`0x1650004B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 75 | `IDC_IMAGE_SWAIT_RMNUM_100` (`0x1650004C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 76 | `IDC_IMAGE_SWAIT_RMNUM_010` (`0x1650004D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 77 | `IDC_IMAGE_SWAIT_RMNUM_001` (`0x1650004E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 78 | `IDC_IMAGE_SWAIT_RANDMAP` (`0x1650004F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 79 | `IDC_BUTTON_SWAIT_1SKILLLEFT` (`0x16200050`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 80 | `IDC_BUTTON_SWAIT_1SKILLRIGHT` (`0x16200051`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 81 | `IDC_BUTTON_SWAIT_2SKILLLEFT` (`0x16200052`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 82 | `IDC_BUTTON_SWAIT_2SKILLRIGHT` (`0x16200053`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 83 | `IDC_BUTTON_SWAIT_3SKILLLEFT` (`0x16200054`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 84 | `IDC_BUTTON_SWAIT_3SKILLRIGHT` (`0x16200055`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 85 | `IDC_BUTTON_SWAIT_MASTER_SKILLSEL` (`0x16202056`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 86 | `IDC_BUTTON_SWAIT_SKILLSEL` (`0x16202057`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 87 | `IDC_IMAGE_SWAIT_1SKILL` (`0x16500058`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 88 | `IDC_IMAGE_SWAIT_2SKILL` (`0x16500059`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 89 | `IDC_IMAGE_SWAIT_3SKILL` (`0x1650005A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 90 | `IDC_IMAGE_SWAIT_COVERSKILLLEFT` (`0x1650005B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 91 | `IDC_IMAGE_SWAIT_COVERSKILLRIGHT` (`0x1650005C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 92 | `IDC_IMAGE_SWAIT_COVERSKILLSEL` (`0x1650005D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 93 | `IDC_TEXT_SWAIT_1SKILLCOUNT` (`0x1640005E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 94 | `IDC_TEXT_SWAIT_2SKILLCOUNT` (`0x1640005F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 95 | `IDC_TEXT_SWAIT_3SKILLCOUNT` (`0x16400060`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 96 | `IDC_IMAGE_SWAIT_SKILLSHOW` (`0x16500061`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 97 | `IDC_IMAGE_SWAIT_NOTICE_TEXT` (`0x16500062`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 98 | `IDC_IMAGE_SWAIT_NOTICEBOARD` (`0x16500063`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 99 | `IDC_BUTTON_SWAIT_SPEED_LEFT` (`0x16200064`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 100 | `IDC_BUTTON_SWAIT_SPEED_RIGHT` (`0x16200065`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 101 | `IDC_IMAGE_SWAIT_SPEED_NUM` (`0x16500066`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 102 | `IDC_IMAGE_SWAIT_WAITTEXTIMG` (`0x16500067`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 103 | `IDC_IMAGE_SWAIT_WAITIMG_ANI` (`0x16500068`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 104 | `IDC_TEXT_SWAIT_BPM` (`0x16400069`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 105 | `IDC_BUTTON_SWAIT_MESSENGER` (`0x1620006A`) | button | Messenger host entry; host routing matrix partly pinned (`messenger-host-captures.md`). | OPEN: REA host routing pin for this host then Tier 3 click leg |
| 106 | `IDC_IMAGE_SWAIT_MESSENGER_ALRAM` (`0x1650006B`) | static art | Messenger alert lamp; fade/reset behavior pinned in `docs/live-path-reachability.md`. | OPEN: live alert-lamp leg; seam messenger_lamp_alert_fades_and_the_messenger_button_resets_it (Tier 0) |
| 107 | `IDC_IMAGE_SWAIT_MESSENGER_ALRAM1` (`0x1650006C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 108 | `IDC_TEXT_SWAIT_WINNERSCROE` (`0x1640006D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 109 | `IDC_IMAGE_SWAIT_READYBUTTEONINACTIVE` (`0x1650006E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 110 | `IDC_IMAGE_SWAIT_SPEEDBUTTEONINACTIVE` (`0x1650006F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 111 | `IDC_IMAGE_SWAIT_MUSIC_SELECT_ARROW` (`0x16500070`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 112 | `IDC_IMAGE_SWAIT_NORMAL_MAP` (`0x16500071`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 113 | `IDC_IMAGE_SWAIT_P2P_MAP` (`0x16500072`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 114 | `IDC_IMAGE_MUSIC_SELECT_OTHERSHOW` (`0x16500073`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 115 | `IDC_IMAGE_SWAIT_MUSIC_SELECT_POPUP` (`0x16500074`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Waiting chat Enter key path | One 3012 body per Enter; focus stays in the chat draft. | OPEN: live Enter + wire leg; seam confirm_sends_the_draft_and_stays_waiting (Tier 0) |
| Waiting Esc / menu Back | No exe scene key handler for the Room transition (#735); local BACK button carries 3005. | OPEN: live Esc key leg (#735); seam live_click_waiting_back_sends_the_room_leave_from_boot (Tier 3) covers the pointer leg only |

## Loading

Source: no `STATE_LOADING` declaration and no `BOUND` section exist in
the 1,979-line pinned `ControlList_Interface.txt` (all `STATE_*`
headers walked; the declarations are CHANNEL/ROOM/SHOP/MYROOM/WAITING/
RESULT/AVI/PLANET/MUSICSHOP/PLAYING/TUTORIAL/PAYMENT/PAYMENT2/LANKING/
DRUM_PLAYING/EXIT). **The whole state is OPEN**: next step = REA pin of
the `MusicLoading.ojs` overlay mount (bnd-audit 35) plus a Tier 3 live
gate capture; `docs/parity/loading-gate.md` holds the current spec story.
No ControlList row is invented here.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Loading Confirm / gate hold | Confirm holds in Loading until select validation, staging, decrypt, decode and clock arm pass; failures latch (`docs/parity/loading-gate.md`). | OPEN: live Wine validation of the full chain (E2); seam loading_hand_session_holds_then_arms_and_hands_off (Tier 0) |
| Loading BACK / return to Waiting | Return edge named in `docs/parity/loading-gate.md`; no ControlList rect. | OPEN: live input + wire leg; seam loading_back_returns_to_waiting (Tier 0) |
| Loading song-title/artist caption | Caption uses the Trebuchet 27×65 cell (`docs/loading-type.md`); no .bnd names the gate. | OPEN: live gate capture of the Arial cell arm; seam loading_caption_routes_through_the_display_cell (Tier 0) |

## Playing

Source: `STATE_PLAYING 0x00` at line 1011 of the pinned
`ControlList_Interface.txt` has no braces and no `BOUND` section; the
1,979-line file declares no Playing controls. The 7.55 UI is in
`Playing1.opi` (`State_Playing.bnd`, 17 rects bound to EXIT…VOLUME;
`docs/parity/playing.md`), outside this ControlList. **The whole state
is OPEN**: next step = a dedicated Playing ControlList/bind census from
`Playing1.opi` plus a SendInput key-up sweep on the live highway. No
ControlList row is invented here.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Seven-lane key-down (Option.ini KEY_1..7) | Scancode→lane poster `FUN_00544b50` → INPUT_STATE; key-down judges the head (polarity pin in `docs/gameplay.md`). | OPEN: REA dispatch pin per lane plus SendInput sweep; key-up dispatch still untraced |
| Seven-lane key-up | Second judge at `+0x864[lane]`; release rates the hold tail (Cool/Good gate at files `0x176c42`/`0x176d78`). | OPEN: SendInput key-up sweep plus REA arm completion; seam `song_clock_equivalence_replay_stays_inside_the_pinned_windows` (Tier 0) |
| EXIT control (Playing1.opi bound run) | Tally-strip EXIT art and rect staged; pressed destination unpinned in this ControlList. | OPEN: statics pin of the Playing1.opi block plus a live click leg |
| VOLUME readout (Playing1.opi bound run) | Staged volume readout; paint pinned in `docs/parity/playing.md`. | OPEN: statics pin plus Tier 3 live-art pin |

## Result

Source: the `BOUND "State_Result.bnd"` block of the pinned
`ControlList_Interface.txt` (lines 757–771; 8 entries / 7 rects);
digit cells stay GDI anchors, not rects (`docs/cresult-anchors.md`).
Ordinal is `(code & 0xFFF) - 1`.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_STATE_RESULT` (`0x17000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_SRESULT_1` (`0x17100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_IMAGE_SRESULT_RANKTAPFIELD` (`0x17600002`) | static art | Rank-table panel at the paired rect; row-0 rank digits are GDI anchors, not rects. | OPEN: statics pin plus Tier 3 live-art pin |
| 2 | `IDC_IMAGE_SRESULT_RESULTTAPFIELD` (`0x17600003`) | static art | Score/judgement panel at the paired rect; digit anchors are exe-pinned GDI (`docs/cresult-anchors.md`). | OPEN: statics pin plus Tier 3 live-art pin |
| 3 | `IDC_IMAGE_SRESULT_RESULTTAP` (`0x17500004`) | static art | Result tab plate; frame source from the walked Result panel. | OPEN: statics pin plus Tier 3 live-art pin |
| 4 | `IDC_BUTTON_SRESULT_BACK` (`0x17200005`) | button | BACK: settled_solo_result_back_click_posts_4021_and_takes_the_4022_answer (Tier 2) asserts 4021 → 4022 → Room; #294 reports Waiting — contradiction left for the exe. | OPEN: REA handler pin to arbitrate Room vs Waiting (#294), then Tier 3 nav leg |
| 5 | `IDC_BUTTON_SRESULT_RETRY` (`0x17202006`) | button | RETRY hands back to Waiting and posts empty StartGame 4010 (#88). | OPEN: live retry click + 4010 leg; seam staged_retry_click_returns_to_waiting (Tier 0) |
| 6 | `IDC_IMAGE_SRESULT_RANKINGFIELD` (`0x17500007`) | static art | Win/lose banner frame selected by the result rule (`docs/result-variants.md`). | OPEN: statics pin plus Tier 3 live-art pin |

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Result Confirm / Back key mirror | Confirm/Back mirror the staged buttons; wire shapes pinned by #87/#88. | OPEN: live key + target leg (#294); seam result_keys_mirror_the_staged_buttons (Tier 0) |
| Event-ticket web call on solo Result entry | HTTP GET to `ticket_add.php?USER_INDEX_ID=…` (file `0x24a2d0`); not a gateway frame. | OPEN: live request-line capture and value-source pin |
