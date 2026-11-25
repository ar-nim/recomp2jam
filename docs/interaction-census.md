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
| `settled_solo_result_back_sends_4021_and_lands_in_waiting_after_4022` | `4021 → 4022 → Waiting` from a settled panel (Tier 2), corrected for `#294` T3 from the exe's state-5 install; `live_click_result_back_sends_4021_and_installs_waiting_from_boot` repeats it through the boot input path (Tier 3) | Result BACK, ord 4 VERIFIED below; the earlier Room capture is a mid-animation click and stays OPEN as capture attribution |

2026-11-25 playtest bugs (`#915`, now split as below) mapped onto census
rows:

| Bug | Class | Census row |
|---|---|---|
| `#290` T1 `[W]`/`[A]` toggle | C1 | ord 2 and ord 3 (`SHOW_ALLMODE`, `SHOW_NORMALMODE`), both OPEN; ord 4 shares the filter guard |
| `#314` T2 Options keys inert / hover blanks | C2 + C4/C3 | ord 10 (`SHOW_…OPTION`), OPEN; Options key-tab rows: state section pending (#920) |
| `#294` T3 Result BACK target | C2 | Result state section, ord 4 VERIFIED (Tier 2 + Tier 3); Room nav-edge pin stays OPEN |
| `#294` T4 Result background source | C6 | Result state section, ord 3 VERIFIED (Tier 3 data-varied paint guard) |
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
| 0 | `IDC_BACK_SRESULT_1` (`0x17100001`) | static art | Exe Result fill fetches `State_Result.ojs` unconditionally behind the verdict and rows (`ev_531d1ded…`, `ev_7b5ddc48…`); the staged body rides the real `STATE_RESULT` scene in the Tier 3 guard, but its own pixels are not sampled. | OPEN: sample the `0x17100001` body's ink in a Tier 3 Result frame; statics source itself is pinned |
| 1 | `IDC_IMAGE_SRESULT_RANKTAPFIELD` (`0x17600002`) | static art | Rank-table panel at the paired rect; row-0 rank digits are GDI anchors, not rects. | OPEN: statics pin plus Tier 3 live-art pin |
| 2 | `IDC_IMAGE_SRESULT_RESULTTAPFIELD` (`0x17600003`) | static art | Score/judgement panel at the paired rect; digit anchors are exe-pinned GDI (`docs/cresult-anchors.md`). | OPEN: statics pin plus Tier 3 live-art pin |
| 3 | `IDC_IMAGE_SRESULT_RESULTTAP` (`0x17500004`) | static art | Exe left-tab selector mounts `Result_EachResult.ojs`; no chart JPEG in the Result path (`ev_30a0db95…`, `ev_60cf3595…`). `result_music_pane_uses_staged_opi_not_the_walked_ojn_cover` walks two different OJN covers and asserts identical staged-pane ink. | VERIFIED: Tier 3 data-varied paint guard (`result_music_pane_uses_staged_opi_not_the_walked_ojn_cover`) + Tier 3 boot leg (`live_click_result_back_sends_4021_and_installs_waiting_from_boot`) |
| 4 | `IDC_BUTTON_SRESULT_BACK` (`0x17200005`) | button | Exe posts empty 4021 and the scripted 4022 installs state id 5 = Waiting, not room state id 3 (`ev_70fafb64…`, `ev_d6a8ea7f…`). `settled_solo_result_back_sends_4021_and_lands_in_waiting_after_4022` pins the wire pair and target (Tier 2); boot leg repeats it (Tier 3). | VERIFIED: Tier 2 + Tier 3 guards above; the earlier Room frame was a mid-animation click — a settled exe wire+frame capture with timestamps stays OPEN as attribution evidence |
| 5 | `IDC_BUTTON_SRESULT_RETRY` (`0x17202006`) | button | RETRY hands back to Waiting and posts empty StartGame 4010 (#88). | OPEN: live retry click + 4010 leg; seam staged_retry_click_returns_to_waiting (Tier 0) |
| 6 | `IDC_IMAGE_SRESULT_RANKINGFIELD` (`0x17500007`) | static art | Win/lose banner frame selected by the result rule (`docs/result-variants.md`). | OPEN: statics pin plus Tier 3 live-art pin |

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Result Confirm / Back key mirror | Confirm still mirrors RETRY; Back is an ours-by-design local Room shortcut (#294 T3), not the exe button's Waiting target. | OPEN: exe keyboard-Back pin or owner decision to align it with the button; seam result_keys_mirror_the_staged_buttons (Tier 0) |
| Event-ticket web call on solo Result entry | HTTP GET to `ticket_add.php?USER_INDEX_ID=…` (file `0x24a2d0`); not a gateway frame. | OPEN: live request-line capture and value-source pin |

## MyRoom

Source: the `BOUND "State_Myroom.bnd"` block of the pinned
`ControlList_Interface.txt` (lines 436–583; **117 raw IDC entries =
112 live + 5 commented-out MY BAG scroll-bar tokens** / 111 rects),
with panel-face and STATUS flip pins in `docs/myroom.md`. Ordinal is
`(code & 0xFFF) - 1`. The five commented tokens are enumerated below.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_STATE_MYROOM` (`0x15000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_SMYROOM_1` (`0x15100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_SMYROOM_BACK` (`0x15200002`) | button | Returns to the lobby side; #178 T8 disputes side-destination pins generally. | OPEN: live Back + destination leg; seam item_shop_go_myroom_leaves_without_an_exit_send (Tier 0) |
| 2 | `IDC_BUTTON_SMYROOM_GOSHOP` (`0x15200003`) | button | Goes to Item Shop side; send shape unpinned (P0 in `docs/myroom.md`). | OPEN: REA dispatch plus Tier 3 click/destination leg |
| 3 | `IDC_BUTTON_SMYROOM_ITEMSELL` (`0x15200004`) | button | Vacates the worn slot locally; send shape OPEN (#532). | OPEN: REA vacate opcode trace plus Tier 3 click (#532) |
| 4 | `IDC_BUTTON_SMYROOM_INSTRUMENT` (`0x15200005`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 5 | `IDC_BUTTON_SMYROOM_HAIR` (`0x15200006`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 6 | `IDC_BUTTON_SMYROOM_ARTICLE` (`0x15200007`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 7 | `IDC_BUTTON_SMYROOM_GLOVES` (`0x15200008`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 8 | `IDC_BUTTON_SMYROOM_NECKLACE` (`0x15200009`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 9 | `IDC_BUTTON_SMYROOM_JACKET` (`0x1520000A`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 10 | `IDC_BUTTON_SMYROOM_PANTS` (`0x1520000B`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 11 | `IDC_BUTTON_SMYROOM_GLASSES` (`0x1520000C`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 12 | `IDC_BUTTON_SMYROOM_EARRING` (`0x1520000D`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 13 | `IDC_BUTTON_SMYROOM_CLOTHINGART` (`0x1520000E`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 14 | `IDC_BUTTON_SMYROOM_SHOES` (`0x1520000F`) | button | Equip-slot selector; 5004 slot derivation is exe-pinned, slot-selection semantics stay OPEN (#530). | OPEN: rig slot-filter capture (#530) + live equip leg; seam myroom_record_slot_byte_selects_the_well_not_the_equipment_ordinal (Tier 0) |
| 15 | `IDC_TEXT_SMYROOM_O2MONEY` (`0x15400010`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 16 | `IDC_TEXT_SMYROOM_CHARGEMONEY` (`0x15400011`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 17 | `IDC_IMAGE_SMYROOM_AVATAR` (`0x15700012`) | static art | Worn figure; body 30 + face 35/36 when no body item is worn (#243). | OPEN: statics pin plus Tier 3 live-art pin |
| 18 | `IDC_IMAGE_SMYROOM_INSTRUMENT` (`0x15600013`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 19 | `IDC_IMAGE_SMYROOM_HAIR` (`0x15600014`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 20 | `IDC_IMAGE_SMYROOM_ARTICLE` (`0x15600015`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 21 | `IDC_IMAGE_SMYROOM_GLOVES` (`0x15600016`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 22 | `IDC_IMAGE_SMYROOM_NECKLACE` (`0x15600017`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 23 | `IDC_IMAGE_SMYROOM_JACKET` (`0x15600018`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 24 | `IDC_IMAGE_SMYROOM_PANTS` (`0x15600019`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 25 | `IDC_IMAGE_SMYROOM_GLASSES` (`0x1560001A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 26 | `IDC_IMAGE_SMYROOM_EARRING` (`0x1560001B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 27 | `IDC_IMAGE_SMYROOM_CLOTHINGART` (`0x1560001C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 28 | `IDC_IMAGE_SMYROOM_SHOES` (`0x1560001D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 29 | `IDC_IMAGE_SMYROOM_WING` (`0x1560001E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 30 | `IDC_IMAGE_SMYROOM_INSTARTICLE` (`0x1560001F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 31 | `IDC_IMAGE_SMYROOM_PET` (`0x15600020`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 32 | `IDC_IMAGE_SMYROOM_ARTICLE_F` (`0x15600021`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 33 | `IDC_IMAGE_SMYROOM_CUSTUME` (`0x15600022`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 34 | `IDC_IMAGE_SMYROOM_MYBAGITEM1` (`0x15600023`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 35 | `IDC_IMAGE_SMYROOM_MYBAGITEM2` (`0x15600024`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 36 | `IDC_IMAGE_SMYROOM_MYBAGITEM3` (`0x15600025`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 37 | `IDC_IMAGE_SMYROOM_MYBAGITEM4` (`0x15600026`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 38 | `IDC_IMAGE_SMYROOM_MYBAGITEM5` (`0x15600027`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 39 | `IDC_IMAGE_SMYROOM_MYBAGITEM6` (`0x15600028`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 40 | `IDC_IMAGE_SMYROOM_MYBAGITEM7` (`0x15600029`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 41 | `IDC_IMAGE_SMYROOM_MYBAGITEM8` (`0x1560002A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 42 | `IDC_IMAGE_SMYROOM_MYBAGITEM9` (`0x1560002B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 43 | `IDC_IMAGE_SMYROOM_MYBAGITEM10` (`0x1560002C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 44 | `IDC_BUTTON_SMYROOM_BAGLEFT` (`0x1520002D`) | pager | Pages the owned-inventory face back; ten cells per page. | OPEN: live bag-page leg; seam myroom_hand_session_walks_slots_bag_and_gifts (Tier 0) |
| 45 | `IDC_BUTTON_SMYROOM_BAGRIGHT` (`0x1520002E`) | pager | Pages the owned-inventory face forward; ten cells per page. | OPEN: live bag-page leg; seam myroom_hand_session_walks_slots_bag_and_gifts (Tier 0) |
| 46 | `IDC_IMAGE_SMYROOM_LPAGECNT_10` (`0x1550002F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 47 | `IDC_IMAGE_SMYROOM_LPAGECNT_1` (`0x15500030`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 48 | `IDC_IMAGE_SMYROOM_RPAGECNT_10` (`0x15500031`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 49 | `IDC_IMAGE_SMYROOM_RPAGECNT_1` (`0x15500032`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 50 | `IDC_IMAGE_SMYROOM_MYBAGSELECT` (`0x15500033`) | static art | Selection frame on the bag face; exe pins no ink at this rect (#369). | OPEN: statics pin plus Tier 3 live-art pin |
| 51 | `IDC_BUTTON_SMYROOM_INVENTORY` (`0x15203034`) | button | At (177,318) flips the panel face: baked STATUS label ↔ Inventory button + status plate (#369, #180 T5). | OPEN: live click both directions (#180 T5); seam myroom_my_bag_tab_press_leaves_the_gift_panel_open (Tier 0) |
| 52 | `IDC_IMAGE_SMYROOM_SLOTCOVER` (`0x15500035`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 53 | `IDC_IMAGE_SMYROOM_USERINFO` (`0x15500036`) | static art | Status plate on the toggled face; default face keeps the baked STATUS label (#369). | OPEN: statics pin plus Tier 3 live-art pin |
| 54 | `IDC_TEXT_SMYROOM_NICKNAME` (`0x15400037`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 55 | `IDC_TEXT_SMYROOM_LEVEL` (`0x15400038`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 56 | `IDC_TEXT_SMYROOM_BONUSPOINT` (`0x15400039`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 57 | `IDC_TEXT_SMYROOM_EXP` (`0x1540003A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 58 | `IDC_TEXT_SMYROOM_NEXTUPEXP` (`0x1540003B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 59 | `IDC_TEXT_SMYROOM_WINRATION` (`0x1540003C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 60 | `IDC_TEXT_SMYROOM_RANK` (`0x1540003D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 61 | `IDC_TEXT_SMYROOM_GUILD` (`0x1540003E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 62 | `IDC_TEXT_SMYROOM_MGAME_MONEY` (`0x1540003F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 63 | `IDC_TEXT_SMYROOM_O2JAM_MONEY` (`0x15400040`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 64 | `IDC_TEXT_SMYROOM_FREEINFO1` (`0x15400041`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 65 | `IDC_TEXT_SMYROOM_FREEINFO2` (`0x15400042`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 66 | `IDC_TEXT_SMYROOM_FREEINFO3` (`0x15400043`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 67 | `IDC_TEXT_SMYROOM_FREEINFO4` (`0x15400044`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 68 | `IDC_IMAGE_SMYROOM_MYBAG_POS` (`0x15600045`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 69 | `IDC_IMAGE_SMYROOM_GIFTBOX_POS` (`0x15600046`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 70 | `IDC_IMAGE_SMYROOM_GIFTBOX_BACK` (`0x15500047`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 71 | `IDC_BUTTON_SMYROOM_GIFT_LEFT` (`0x15200048`) | pager | Pages the gift box back; count caption comes from the 5017/5018 reply. | OPEN: live gift-page leg; seam myroom_hand_session_walks_slots_bag_and_gifts (Tier 0) |
| 72 | `IDC_BUTTON_SMYROOM_GIFT_RIGHT` (`0x15200049`) | pager | Pages the gift box forward; count caption from the wire. | OPEN: live gift-page leg; seam myroom_hand_session_walks_slots_bag_and_gifts (Tier 0) |
| 73 | `IDC_BUTTON_SMYROOM_GIFTBOX_EX1` (`0x1501C04A`) | row panel (SET parent) | Gift cell 1; gift-row chrome hidden on the empty MY BAG face (#878). | OPEN: qualified gift-row capture with a populated 5018 reply |
| 74 | `IDC_BUTTON_SMYROOM_GIFTBOX_EX2` (`0x1501C04B`) | row panel (SET parent) | Gift cell 2; gift-row chrome hidden on the empty MY BAG face (#878). | OPEN: qualified gift-row capture with a populated 5018 reply |
| 75 | `IDC_BUTTON_SMYROOM_GIFTBOX_EX3` (`0x1501C04C`) | row panel (SET parent) | Gift cell 3; gift-row chrome hidden on the empty MY BAG face (#878). | OPEN: qualified gift-row capture with a populated 5018 reply |
| 76 | `IDC_BUTTON_SMYROOM_GIFTBOX_EX4` (`0x1501C04D`) | row panel (SET parent) | Gift cell 4; gift-row chrome hidden on the empty MY BAG face (#878). | OPEN: qualified gift-row capture with a populated 5018 reply |
| 77 | `IDC_BUTTON_SMYROOM_GIFTBOX_EX5` (`0x1501C04E`) | row panel (SET parent) | Gift cell 5; gift-row chrome hidden on the empty MY BAG face (#878). | OPEN: qualified gift-row capture with a populated 5018 reply |
| 78 | `IDC_TEXT_GIFT_NUM` (`0x1540004F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 79 | `IDC_IMAGE_GIFT_TYPE` (`0x15500050`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 80 | `IDC_TEXT_GIFT_ITEMNAME` (`0x15400051`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 81 | `IDC_TEXT_GIFT_SENDER` (`0x15400052`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 82 | `IDC_IMAGE_GIFT_MOVE` (`0x15500053`) | static art | ClaimGift 5023 on the picked gift row; verdict byte ignored like the exe (#422). | OPEN: qualified gift-claim click + gateway replay (#422) |
| 83 | `IDC_IMAGE_SMYROOM_GIFT_CUR_P01` (`0x15500054`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 84 | `IDC_IMAGE_SMYROOM_GIFT_CUR_P10` (`0x15500055`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 85 | `IDC_IMAGE_SMYROOM_GIFT_TOT_P01` (`0x15500056`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 86 | `IDC_IMAGE_SMYROOM_GIFT_TOT_P10` (`0x15500057`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 87 | `IDC_BUTTON_SMYROOM_WEBABATAR` (`0x15200058`) | button | Web avatar chrome; P0, sends nothing (`docs/myroom.md`). | OPEN: REA dispatch plus Tier 3 click (P0 silence check) |
| 88 | `IDC_TEXT_SMYROOM_O2CASH` (`0x15400059`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 89 | `IDC_TEXT_SMYROOM_MUSICCASH` (`0x1540005A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 90 | `IDC_TEXT_SMYROOM_ITEMCASH` (`0x1540005B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 91 | `IDC_TEXT_SMYROOM_SKILLCOUNT1` (`0x1540005C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 92 | `IDC_TEXT_SMYROOM_SKILLCOUNT2` (`0x1540005D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 93 | `IDC_TEXT_SMYROOM_SKILLCOUNT3` (`0x1540005E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 94 | `IDC_TEXT_SMYROOM_SKILLCOUNT4` (`0x1540005F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 95 | `IDC_TEXT_SMYROOM_SKILLCOUNT5` (`0x15400060`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 96 | `IDC_TEXT_SMYROOM_SKILLCOUNT6` (`0x15400061`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 97 | `IDC_TEXT_SMYROOM_SKILLCOUNT7` (`0x15400062`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 98 | `IDC_TEXT_SMYROOM_SKILLCOUNT8` (`0x15400063`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 99 | `IDC_TEXT_SMYROOM_SKILLCOUNT9` (`0x15400064`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 100 | `IDC_TEXT_SMYROOM_SKILLCOUNT10` (`0x15400065`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 101 | `IDC_TEXT_SMYROOM_PENALTYCOUNT` (`0x15400066`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 102 | `IDC_TEXT_SMYROOM_PENALTYLEVEL` (`0x15400067`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 103 | `IDC_IMAGE_SMYROOM_NOTICE_TEXT` (`0x15500068`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 104 | `IDC_IMAGE_SMYROOM_NOTICEBOARD` (`0x15500069`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 105 | `IDC_IMAGE_SMYROOM_PCROOMMARK` (`0x1550006A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 106 | `IDC_BUTTON_SMYROOM_CONFERENCE` (`0x1520006B`) | button | Conference chrome; P0, sends nothing. | OPEN: REA dispatch plus Tier 3 click (P0 silence check) |
| 107 | `IDC_BUTTON_SMYROOM_BAGSORT` (`0x1520006C`) | button | Bag-sort chrome; P0, sends nothing. | OPEN: REA dispatch plus Tier 3 click (P0 silence check) |
| 108 | `IDC_BUTTON_SMYROOM_MESSENGER` (`0x1520006D`) | button | Messenger chrome on the MyRoom board; another liveness host stays N-HOST (#648/#650/#652). | OPEN: REA host routing pin then Tier 3 click leg |
| 109 | `IDC_IMAGE_SMYROOM_MESSENGER_ALRAM` (`0x1550006E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 110 | `IDC_IMAGE_SMYROOM_MESSENGER_ALRAM1` (`0x1550006F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |

Commented-out MY BAG scroll-bar family (raw IDC tokens at lines 498, 503–506; counted by the raw block walk, but authored with a `//` prefix). Each ordinal collides with a live entry above; do not resolve the collision silently. The exe does not have a pinned consumer for the commented codes.


| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| 43 | `IDC_SCROLL_SMYROOM_MYBAG` (`0x1502102C`) | scrollbar | Authored comment `O2_SBS_HORZ 5 15`; ordinal 43 is already held by `MYBAGITEM10`. Exe consumption unpinned. | OPEN: REA statics pin of the commented scrollbar code plus Tier 3 drag/click leg; #180 |
| 44 | `IDC_BUTTON_SMYROOM_SMYBAGLEFT` (`0x1520002D`) | scrollbar (SET child) | Authored comment `Myroom_MybagScroll_Left.ojs`; ordinal 44 is already held by `BAGLEFT`. | OPEN: REA statics pin of the commented arrow code plus Tier 3 click; #180 |
| 45 | `IDC_BUTTON_SMYROOM_SMYBAGRIGHT` (`0x1520002E`) | scrollbar (SET child) | Authored comment `Myroom_MybagScroll_Right.ojs`; ordinal 45 is already held by `BAGRIGHT`. | OPEN: REA statics pin of the commented arrow code plus Tier 3 click; #180 |
| 46 | `IDC_BUTTON_SMYROOM_SMYBAGBAR` (`0x1520002F`) | scrollbar (SET child) | Authored comment `Myroom_MybagScroll_Bar.ojs`; ordinal 46 is already held by `LPAGECNT_10`. | OPEN: REA statics pin of the commented thumb code plus Tier 3 drag; #180 |
| 47 | `IDC_BUTTON_SMYROOM_SMYBAGRANGE` (`0x15200030`) | scrollbar (SET child) | Authored comment with no sprite; ordinal 47 is already held by `LPAGECNT_1`. | OPEN: REA statics pin of the commented track code plus Tier 3 click; #180 |

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Pointer hover over MyRoom chrome | Rest desaturated / hover frame 0 / active frame 1 pattern applies, but per-control frames are not captured here. | OPEN: off/on/off pixel trio per control; seam `myroom_chrome_sweep_presses_every_staged_control` (Tier 0) |
| Inventory ↔ gift-panel round trip | Present-box tab must return to MY BAG (#180 T6); existing guard pins the gift panel staying open. | OPEN: live two-way tab leg (#180); seam myroom_my_bag_tab_press_leaves_the_gift_panel_open (Tier 0) |

## Item Shop

Source: the `BOUND "State_ItemShop.bnd"` block of the pinned
`ControlList_Interface.txt` (lines 179–431; **192 raw IDC entries =
187 live + 5 commented-out MY BAG scroll-bar tokens** / 186 rects);
BUYITEM and browse-grid pins are in `docs/shop.md` and
`docs/parity/shop.md`. Ordinal is `(code & 0xFFF) - 1`. The five
commented tokens are enumerated below.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_STATE_SHOP` (`0x14000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_SSHOP_1` (`0x14100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_SSHOP_BACK` (`0x14200002`) | button | BACK leaves the shop; no 5007 body pinned for this press (#399 wire arm OPEN). | OPEN: qualified BACK tap + wire leg (#399); seam item_shop_back_sends_the_empty_5007_exit (Tier 0) |
| 2 | `IDC_BUTTON_SSHOP_GOMYROOM` (`0x14200003`) | button | GO MYROOM leaves the shop with no exit send; #178 T8 says the target is Room, not MyRoom — contradiction OPEN. | OPEN: REA target pin + Tier 3 destination leg (#178 T8); seam item_shop_go_myroom_leaves_without_an_exit_send (Tier 0) |
| 3 | `IDC_BUTTON_SSHOP_CHARGEMONEY` (`0x14200004`) | button | Charge plate / O2CASH-shortfall host for the BUYITEM MESSAGE sheet (#867). | OPEN: live charge-view capture with game TCP and HTTP taps (#867) |
| 4 | `IDC_BUTTON_SSHOP_ITEMVACATE` (`0x14200005`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 5 | `IDC_BUTTON_SSHOP_GIRL` (`0x14202006`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 6 | `IDC_BUTTON_SSHOP_BOY` (`0x14202007`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 7 | `IDC_BUTTON_SSHOP_PLANET_UP` (`0x14200008`) | button | Theme chrome; no live destination pinned. | OPEN: REA dispatch plus Tier 3 click; no handler pinned |
| 8 | `IDC_BUTTON_SSHOP_PLANET_DOWN` (`0x14200009`) | button | Theme chrome; no live destination pinned. | OPEN: REA dispatch plus Tier 3 click; no handler pinned |
| 9 | `IDC_BUTTON_SSHOP_SET` (`0x1420100A`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA category dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 10 | `IDC_BUTTON_SSHOP_FASHION` (`0x1420100B`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA category dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 11 | `IDC_BUTTON_SSHOP_ACCESSORY` (`0x1420100C`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA category dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 12 | `IDC_BUTTON_SSHOP_BEAUTY` (`0x1420100D`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA category dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 13 | `IDC_BUTTON_SSHOP_INSTRUMENT` (`0x1420100E`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA category dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 14 | `IDC_IMAGE_SSHOP_RESERVE1` (`0x1450000F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 15 | `IDC_IMAGE_SSHOP_RESERVE2` (`0x14500010`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 16 | `IDC_IMAGE_SSHOP_RESERVE3` (`0x14500011`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 17 | `IDC_BUTTON_SSHOP_SUBMENU1` (`0x14208012`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA submenu dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 18 | `IDC_BUTTON_SSHOP_SUBMENU2` (`0x14208013`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA submenu dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 19 | `IDC_BUTTON_SSHOP_SUBMENU3` (`0x14208014`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA submenu dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 20 | `IDC_BUTTON_SSHOP_SUBMENU4` (`0x14208015`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA submenu dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 21 | `IDC_BUTTON_SSHOP_SUBMENU5` (`0x14208016`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA submenu dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 22 | `IDC_BUTTON_SSHOP_SUBMENU6` (`0x14208017`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA submenu dispatch plus Tier 3 filtered-grid leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 23 | `IDC_BUTTON_SSHOP_ITEMBUY1` (`0x14200018`) | button | Item row 1 buy cell: checks the cart and opens the SHOPPING BAG face (#867). | OPEN: live row-click + cart leg; seam shop_row_toggle_checks_the_cart_and_buy_posts_the_native_sheet (Tier 0) |
| 24 | `IDC_BUTTON_SSHOP_ITEMBUY2` (`0x14200019`) | button | Item row 2 buy cell: checks the cart and opens the SHOPPING BAG face (#867). | OPEN: live row-click + cart leg; seam shop_row_toggle_checks_the_cart_and_buy_posts_the_native_sheet (Tier 0) |
| 25 | `IDC_BUTTON_SSHOP_ITEMBUY3` (`0x1420001A`) | button | Item row 3 buy cell: checks the cart and opens the SHOPPING BAG face (#867). | OPEN: live row-click + cart leg; seam shop_row_toggle_checks_the_cart_and_buy_posts_the_native_sheet (Tier 0) |
| 26 | `IDC_BUTTON_SSHOP_ITEMBUY4` (`0x1420001B`) | button | Item row 4 buy cell: checks the cart and opens the SHOPPING BAG face (#867). | OPEN: live row-click + cart leg; seam shop_row_toggle_checks_the_cart_and_buy_posts_the_native_sheet (Tier 0) |
| 27 | `IDC_BUTTON_SSHOP_ITEMBUY5` (`0x1420001C`) | button | Item row 5 buy cell: checks the cart and opens the SHOPPING BAG face (#867). | OPEN: live row-click + cart leg; seam shop_row_toggle_checks_the_cart_and_buy_posts_the_native_sheet (Tier 0) |
| 28 | `IDC_BUTTON_SSHOP_ITEMBUY6` (`0x1420001D`) | button | Item row 6 buy cell: checks the cart and opens the SHOPPING BAG face (#867). | OPEN: live row-click + cart leg; seam shop_row_toggle_checks_the_cart_and_buy_posts_the_native_sheet (Tier 0) |
| 29 | `IDC_BUTTON_SSHOP_ITEMBUY7` (`0x1420001E`) | button | Item row 7 buy cell: checks the cart and opens the SHOPPING BAG face (#867). | OPEN: live row-click + cart leg; seam shop_row_toggle_checks_the_cart_and_buy_posts_the_native_sheet (Tier 0) |
| 30 | `IDC_BUTTON_SSHOP_ITEMBUY8` (`0x1420001F`) | button | Item row 8 buy cell: checks the cart and opens the SHOPPING BAG face (#867). | OPEN: live row-click + cart leg; seam shop_row_toggle_checks_the_cart_and_buy_posts_the_native_sheet (Tier 0) |
| 31 | `IDC_BUTTON_SSHOP_ITEMGIFT1` (`0x14200020`) | button | Item row 1 gift cell; send shape unpinned. | OPEN: REA gift dispatch plus Tier 3 click; seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 32 | `IDC_BUTTON_SSHOP_ITEMGIFT2` (`0x14200021`) | button | Item row 2 gift cell; send shape unpinned. | OPEN: REA gift dispatch plus Tier 3 click; seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 33 | `IDC_BUTTON_SSHOP_ITEMGIFT3` (`0x14200022`) | button | Item row 3 gift cell; send shape unpinned. | OPEN: REA gift dispatch plus Tier 3 click; seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 34 | `IDC_BUTTON_SSHOP_ITEMGIFT4` (`0x14200023`) | button | Item row 4 gift cell; send shape unpinned. | OPEN: REA gift dispatch plus Tier 3 click; seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 35 | `IDC_BUTTON_SSHOP_ITEMGIFT5` (`0x14200024`) | button | Item row 5 gift cell; send shape unpinned. | OPEN: REA gift dispatch plus Tier 3 click; seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 36 | `IDC_BUTTON_SSHOP_ITEMGIFT6` (`0x14200025`) | button | Item row 6 gift cell; send shape unpinned. | OPEN: REA gift dispatch plus Tier 3 click; seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 37 | `IDC_BUTTON_SSHOP_ITEMGIFT7` (`0x14200026`) | button | Item row 7 gift cell; send shape unpinned. | OPEN: REA gift dispatch plus Tier 3 click; seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 38 | `IDC_BUTTON_SSHOP_ITEMGIFT8` (`0x14200027`) | button | Item row 8 gift cell; send shape unpinned. | OPEN: REA gift dispatch plus Tier 3 click; seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 39 | `IDC_TEXT_SSHOP_GEMMONEY` (`0x14400028`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 40 | `IDC_TEXT_SSHOP_MGASH` (`0x14400029`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 41 | `IDC_TEXT_SSHOP_NICKNAME` (`0x1440002A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 42 | `IDC_IMAGE_SSHOP_PLANET` (`0x1450002B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 43 | `IDC_TEXT_SSHOP_ITEMNAME1` (`0x1440002C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 44 | `IDC_TEXT_SSHOP_ITEMNAME2` (`0x1440002D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 45 | `IDC_TEXT_SSHOP_ITEMNAME3` (`0x1440002E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 46 | `IDC_TEXT_SSHOP_ITEMNAME4` (`0x1440002F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 47 | `IDC_TEXT_SSHOP_ITEMNAME5` (`0x14400030`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 48 | `IDC_TEXT_SSHOP_ITEMNAME6` (`0x14400031`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 49 | `IDC_TEXT_SSHOP_ITEMNAME7` (`0x14400032`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 50 | `IDC_TEXT_SSHOP_ITEMNAME8` (`0x14400033`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 51 | `IDC_IMAGE_SSHOP_AVATAR` (`0x14700034`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 52 | `IDC_IMAGE_SSHOP_ITEMSHOW1` (`0x14600035`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 53 | `IDC_IMAGE_SSHOP_ITEMSHOW2` (`0x14600036`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 54 | `IDC_IMAGE_SSHOP_ITEMSHOW3` (`0x14600037`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 55 | `IDC_IMAGE_SSHOP_ITEMSHOW4` (`0x14600038`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 56 | `IDC_IMAGE_SSHOP_ITEMSHOW5` (`0x14600039`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 57 | `IDC_IMAGE_SSHOP_ITEMSHOW6` (`0x1460003A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 58 | `IDC_IMAGE_SSHOP_ITEMSHOW7` (`0x1460003B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 59 | `IDC_IMAGE_SSHOP_ITEMSHOW8` (`0x1460003C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 60 | `IDC_IMAGE_SSHOP_MYBAGITEM1` (`0x1460003D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 61 | `IDC_IMAGE_SSHOP_MYBAGITEM2` (`0x1460003E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 62 | `IDC_IMAGE_SSHOP_MYBAGITEM3` (`0x1460003F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 63 | `IDC_IMAGE_SSHOP_MYBAGITEM4` (`0x14600040`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 64 | `IDC_IMAGE_SSHOP_MYBAGITEM5` (`0x14600041`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 65 | `IDC_IMAGE_SSHOP_MYBAGITEM6` (`0x14600042`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 66 | `IDC_IMAGE_SSHOP_MYBAGITEM7` (`0x14600043`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 67 | `IDC_IMAGE_SSHOP_MYBAGITEM8` (`0x14600044`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 68 | `IDC_IMAGE_SSHOP_MYBAGITEM9` (`0x14600045`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 69 | `IDC_IMAGE_SSHOP_MYBAGITEM10` (`0x14600046`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 70 | `IDC_SCROLL_SSHOP_ITEM` (`0x14022047`) | scrollbar | Item-grid scrollbar; #178 T7 reports browse scroll dead. | OPEN: Tier 3 drag/click browse leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 71 | `IDC_BUTTON_SSHOP_SITEMLEFT` (`0x14200048`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: Tier 3 scroll leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 72 | `IDC_BUTTON_SSHOP_SITEMRIGHT` (`0x14200049`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: Tier 3 scroll leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 73 | `IDC_BUTTON_SSHOP_SITEMBAR` (`0x1420004A`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: Tier 3 scroll leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 74 | `IDC_BUTTON_SSHOP_SITEMRANGE` (`0x1420004B`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: Tier 3 scroll leg (#178 T7); seam shop_unwired_browse_chrome_lands_nowhere (Tier 0) |
| 75 | `IDC_BUTTON_SSHOP_BAGLEFT` (`0x1420004C`) | pager | Pages the staged owned-inventory face back (ten cells per page). | OPEN: live bag-page leg; seam shop_bag_pager_turns_over_staged_owned_inventory (Tier 0) |
| 76 | `IDC_BUTTON_SSHOP_BAGRIGHT` (`0x1420004D`) | pager | Pages the staged owned-inventory face forward. | OPEN: live bag-page leg; seam shop_bag_pager_turns_over_staged_owned_inventory (Tier 0) |
| 77 | `IDC_IMAGE_SSHOP_LPAGECNT_10` (`0x1450004E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 78 | `IDC_IMAGE_SSHOP_LPAGECNT_1` (`0x1450004F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 79 | `IDC_IMAGE_SSHOP_RPAGECNT_10` (`0x14500050`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 80 | `IDC_IMAGE_SSHOP_RPAGECNT_1` (`0x14500051`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 81 | `IDC_IMAGE_SSHOP_MYBAGSELECT` (`0x14500052`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 82 | `IDC_IMAGE_SSHOP_SHOPMASTER` (`0x14600053`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 83 | `IDC_TEXT_SSHOP_ITEMWON1` (`0x14400054`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 84 | `IDC_TEXT_SSHOP_ITEMWON2` (`0x14400055`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 85 | `IDC_TEXT_SSHOP_ITEMWON3` (`0x14400056`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 86 | `IDC_TEXT_SSHOP_ITEMWON4` (`0x14400057`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 87 | `IDC_TEXT_SSHOP_ITEMWON5` (`0x14400058`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 88 | `IDC_TEXT_SSHOP_ITEMWON6` (`0x14400059`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 89 | `IDC_TEXT_SSHOP_ITEMWON7` (`0x1440005A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 90 | `IDC_TEXT_SSHOP_ITEMWON8` (`0x1440005B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 91 | `IDC_TEXT_SSHOP_ITEMGEM1` (`0x1440005C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 92 | `IDC_TEXT_SSHOP_ITEMGEM2` (`0x1440005D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 93 | `IDC_TEXT_SSHOP_ITEMGEM3` (`0x1440005E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 94 | `IDC_TEXT_SSHOP_ITEMGEM4` (`0x1440005F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 95 | `IDC_TEXT_SSHOP_ITEMGEM5` (`0x14400060`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 96 | `IDC_TEXT_SSHOP_ITEMGEM6` (`0x14400061`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 97 | `IDC_TEXT_SSHOP_ITEMGEM7` (`0x14400062`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 98 | `IDC_TEXT_SSHOP_ITEMGEM8` (`0x14400063`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 99 | `IDC_BUTTON_SSHOP_ITEMPRE1` (`0x14200064`) | button | Item row 1 preview: selects/clears preview, queues nothing (#178). | OPEN: live preview click + frame leg; seam shop_preview_selects_row_and_back_clears (Tier 0) |
| 100 | `IDC_BUTTON_SSHOP_ITEMPRE2` (`0x14200065`) | button | Item row 2 preview: selects/clears preview, queues nothing (#178). | OPEN: live preview click + frame leg; seam shop_preview_selects_row_and_back_clears (Tier 0) |
| 101 | `IDC_BUTTON_SSHOP_ITEMPRE3` (`0x14200066`) | button | Item row 3 preview: selects/clears preview, queues nothing (#178). | OPEN: live preview click + frame leg; seam shop_preview_selects_row_and_back_clears (Tier 0) |
| 102 | `IDC_BUTTON_SSHOP_ITEMPRE4` (`0x14200067`) | button | Item row 4 preview: selects/clears preview, queues nothing (#178). | OPEN: live preview click + frame leg; seam shop_preview_selects_row_and_back_clears (Tier 0) |
| 103 | `IDC_BUTTON_SSHOP_ITEMPRE5` (`0x14200068`) | button | Item row 5 preview: selects/clears preview, queues nothing (#178). | OPEN: live preview click + frame leg; seam shop_preview_selects_row_and_back_clears (Tier 0) |
| 104 | `IDC_BUTTON_SSHOP_ITEMPRE6` (`0x14200069`) | button | Item row 6 preview: selects/clears preview, queues nothing (#178). | OPEN: live preview click + frame leg; seam shop_preview_selects_row_and_back_clears (Tier 0) |
| 105 | `IDC_BUTTON_SSHOP_ITEMPRE7` (`0x1420006A`) | button | Item row 7 preview: selects/clears preview, queues nothing (#178). | OPEN: live preview click + frame leg; seam shop_preview_selects_row_and_back_clears (Tier 0) |
| 106 | `IDC_BUTTON_SSHOP_ITEMPRE8` (`0x1420006B`) | button | Item row 8 preview: selects/clears preview, queues nothing (#178). | OPEN: live preview click + frame leg; seam shop_preview_selects_row_and_back_clears (Tier 0) |
| 107 | `IDC_BUTTON_SSHOP_ITEMBACK` (`0x1420006C`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 108 | `IDC_IMAGE_SSHOP_MYBACK_POS` (`0x1460006D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 109 | `IDC_IMAGE_SSHOP_MYCART_POS` (`0x1460006E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 110 | `IDC_IMAGE_SSHOP_MYCART_BACK` (`0x1450006F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 111 | `IDC_BUTTON_SSHOP_BUYITEM` (`0x14200070`) | button | Cart BUYITEM posts the native type-2 MESSAGE sheet (Dialog_Question1) and sends no buy opcode (#867). | VERIFIED: live_boot_cart_row_buy_paints_the_native_message_sheet (Tier 3) — boot → row click → BUYITEM → sheet; wire stays OPEN |
| 112 | `IDC_BUTTON_SSHOP_GIFTITEM` (`0x14200071`) | button | Gift chrome; P0 silence in `docs/myroom.md`, shop-side send unpinned. | OPEN: REA dispatch plus Tier 3 click; gift wire unpinned (#178) |
| 113 | `IDC_BUTTON_SSHOP_MYCART_LEFT` (`0x14200072`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 114 | `IDC_BUTTON_SSHOP_MYCART_RIGHT` (`0x14200073`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 115 | `IDC_TEXT_SSHOP_CART_CHECKGEM` (`0x14400074`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 116 | `IDC_TEXT_SSHOP_CART_CHECKMONEY` (`0x14400075`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 117 | `IDC_BUTTON_SSHOP_CART_EX1` (`0x1401B076`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 118 | `IDC_BUTTON_SSHOP_CART_EX2` (`0x1401B077`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 119 | `IDC_BUTTON_SSHOP_CART_EX3` (`0x1401B078`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 120 | `IDC_BUTTON_SSHOP_CART_EX4` (`0x1401B079`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 121 | `IDC_BUTTON_SSHOP_CART_EX5` (`0x1401B07A`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 122 | `IDC_TEXT_CART_NUM` (`0x1440007B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 123 | `IDC_IMAGE_CART_TYPE` (`0x1450007C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 124 | `IDC_TEXT_CART_ITEMNAME` (`0x1440007D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 125 | `IDC_TEXT_CART_PRICE` (`0x1440007E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 126 | `IDC_IMAGE_CART_DELETE` (`0x1450007F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 127 | `IDC_IMAGE_SSHOP_CART_CUR_P01` (`0x14500080`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 128 | `IDC_IMAGE_SSHOP_CART_CUR_P10` (`0x14500081`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 129 | `IDC_IMAGE_SSHOP_CART_TOT_P01` (`0x14500082`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 130 | `IDC_IMAGE_SSHOP_CART_TOT_P10` (`0x14500083`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 131 | `IDC_IMAGE_SSHOP_ITEMTOOLTIP` (`0x14500084`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 132 | `IDC_TEXT_SSHOP_ITEMCOMMENT` (`0x14400085`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 133 | `IDC_IMAGE_SSHOP_LUCKYBAG` (`0x14600086`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 134 | `IDC_IMAGE_SSHOP_SETBACK` (`0x14500087`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 135 | `IDC_BUTTON_SSHOP_SETITEM_EX1` (`0x1401D088`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 136 | `IDC_BUTTON_SSHOP_SETITEM_EX2` (`0x1401D089`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 137 | `IDC_BUTTON_SSHOP_SETITEM_EX3` (`0x1401D08A`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 138 | `IDC_BUTTON_SSHOP_SETITEM_EX4` (`0x1401D08B`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 139 | `IDC_IMAGE_SSHOP_SET_BACK` (`0x1450008C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 140 | `IDC_IMAGE_SSHOP_SET_AVATAR` (`0x1470008D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 141 | `IDC_TEXT_SSHOP_SET_TITLE` (`0x1440008E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 142 | `IDC_TEXT_SSHOP_SET_TYPE1` (`0x1440008F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 143 | `IDC_TEXT_SSHOP_SET_TYPE2` (`0x14400090`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 144 | `IDC_TEXT_SSHOP_SET_TYPE3` (`0x14400091`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 145 | `IDC_TEXT_SSHOP_SET_TYPE4` (`0x14400092`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 146 | `IDC_TEXT_SSHOP_SET_TYPE5` (`0x14400093`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 147 | `IDC_TEXT_SSHOP_SET_PRICE` (`0x14400094`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 148 | `IDC_IMAGE_SSHOP_SET_BTN_BUY` (`0x14500095`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 149 | `IDC_IMAGE_SSHOP_SET_BTN_WAER` (`0x14500096`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 150 | `IDC_IMAGE_SSHOP_SET_RESERVE` (`0x14500097`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 151 | `IDC_IMAGE_SSHOP_SET_RESERVE2` (`0x14500098`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 152 | `IDC_BUTTON_SSHOP_EXTMENU_ON` (`0x14200099`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 153 | `IDC_IMAGE_SSHOP_EXTMENU_BACK` (`0x1450009A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 154 | `IDC_BUTTON_SSHOP_EXTMENU_ALL` (`0x1420109B`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 155 | `IDC_BUTTON_SSHOP_EXTMENU_NORMAL` (`0x1420109C`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 156 | `IDC_BUTTON_SSHOP_EXTMENU_AQUA` (`0x1420109D`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 157 | `IDC_BUTTON_SSHOP_EXTMENU_ELETEN` (`0x1420109E`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 158 | `IDC_BUTTON_SSHOP_EXTMENU_GRAFFITI` (`0x1420109F`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 159 | `IDC_BUTTON_SSHOP_EXTMENU_BIKINI` (`0x142010A0`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 160 | `IDC_BUTTON_SSHOP_EXTMENU_CRUSH` (`0x142010A1`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 161 | `IDC_BUTTON_SSHOP_EXTMENU_WONDERLAND` (`0x142010A2`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 162 | `IDC_BUTTON_SSHOP_EXTMENU_MEGANUT` (`0x142010A3`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 163 | `IDC_BUTTON_SSHOP_EXTMENU_CRYSTAL` (`0x142010A4`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 164 | `IDC_BUTTON_SSHOP_EXTMENU_DRACONIC` (`0x142010A5`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 165 | `IDC_BUTTON_SSHOP_EXTMENU_EVENT` (`0x142010A6`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA chrome dispatch plus Tier 3 click (#178 T7); seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| 166 | `IDC_TEXT_SSHOP_SKILLCOUNT1` (`0x144010A7`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 167 | `IDC_TEXT_SSHOP_SKILLCOUNT2` (`0x144010A8`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 168 | `IDC_TEXT_SSHOP_SKILLCOUNT3` (`0x144010A9`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 169 | `IDC_TEXT_SSHOP_SKILLCOUNT4` (`0x144010AA`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 170 | `IDC_TEXT_SSHOP_SKILLCOUNT5` (`0x144010AB`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 171 | `IDC_TEXT_SSHOP_SKILLCOUNT6` (`0x144010AC`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 172 | `IDC_TEXT_SSHOP_SKILLCOUNT7` (`0x144010AD`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 173 | `IDC_TEXT_SSHOP_SKILLCOUNT8` (`0x144010AE`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 174 | `IDC_TEXT_SSHOP_SKILLCOUNT9` (`0x144010AF`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 175 | `IDC_TEXT_SSHOP_SKILLCOUNT10` (`0x144010B0`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 176 | `IDC_IMAGE_SSHOP_ITEMTOOLTIP2` (`0x145000B1`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 177 | `IDC_TEXT_SSHOP_ITEMCOMMENT2` (`0x144000B2`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 178 | `IDC_IMAGE_SSHOP_NOTICE_TEXT` (`0x145000B3`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 179 | `IDC_IMAGE_SSHOP_NOTICEBOARD` (`0x145000B4`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 180 | `IDC_IMAGE_SSHOP_PCROOMMARK` (`0x145000B5`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 181 | `IDC_BUTTON_SSHOP_CONFERENCE` (`0x142000B6`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dispatch plus Tier 3 click; web/P0 chrome sends nothing |
| 182 | `IDC_BUTTON_SSHOP_BAGSORT` (`0x142000B7`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dispatch plus Tier 3 click; web/P0 chrome sends nothing |
| 183 | `IDC_BUTTON_SSHOP_MESSENGER` (`0x142000B8`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dispatch plus Tier 3 click; web/P0 chrome sends nothing |
| 184 | `IDC_IMAGE_SSHOP_MESSENGER_ALRAM` (`0x145000B9`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 185 | `IDC_IMAGE_SSHOP_MESSENGER_ALRAM1` (`0x145000BA`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |

Commented-out MY BAG scroll-bar family (raw IDC tokens at lines 271, 285–288; counted by the raw block walk, but authored with a `//` prefix). Each ordinal collides with a live entry above; do not resolve the collision silently. The exe does not have a pinned consumer for the commented codes.


| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| 71 | `IDC_SCROLL_SSHOP_MYBAG` (`0x14023048`) | scrollbar | Authored comment `O2_SBS_HORZ 5 15`; ordinal 71 is already held by `SITEMLEFT`. Exe consumption unpinned. | OPEN: REA statics pin of the commented scrollbar code plus Tier 3 drag/click leg; #178 T7 |
| 76 | `IDC_BUTTON_SSHOP_SMYBAGLEFT` (`0x1420004D`) | scrollbar (SET child) | Authored comment `Shop_MybagScroll_Left.ojs`; ordinal 76 is already held by `BAGRIGHT`. | OPEN: REA statics pin of the commented arrow code plus Tier 3 click; #178 T7 |
| 77 | `IDC_BUTTON_SSHOP_SMYBAGRIGHT` (`0x1420004E`) | scrollbar (SET child) | Authored comment `Shop_MybagScroll_Right.ojs`; ordinal 77 is already held by `LPAGECNT_10`. | OPEN: REA statics pin of the commented arrow code plus Tier 3 click; #178 T7 |
| 78 | `IDC_BUTTON_SSHOP_SMYBAGBAR` (`0x1420004F`) | scrollbar (SET child) | Authored comment `Shop_MybagScroll_Bar.ojs`; ordinal 78 is already held by `LPAGECNT_1`. | OPEN: REA statics pin of the commented thumb code plus Tier 3 drag; #178 T7 |
| 79 | `IDC_BUTTON_SSHOP_SMYBAGRANGE` (`0x14200050`) | scrollbar (SET child) | Authored comment with no sprite; ordinal 79 is already held by `RPAGECNT_10`. | OPEN: REA statics pin of the commented track code plus Tier 3 click; #178 T7 |

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Shop browse chrome hover / clicks | `#178` T7 reports scrollbar, grid and tabs dead live; seam sweep asserts staged presses only. | OPEN: SendInput sweep on the live shop; seam shop_chrome_sweep_presses_every_staged_control (Tier 0) |
| BUYITEM purchase wire | OK/CANCEL choice is local; no qualified OK tap has pinned a 5000-series send (#867). | OPEN: qualified OK tap with game TCP and HTTP(S) taps plus gateway answer |

## Music Shop

Source: the `BOUND "State_MShop.bnd"` block of the pinned
`ControlList_Interface.txt` (lines 810–1007; 139 entries / 138 rects);
row/cart pins in `docs/musicshop.md` and `docs/parity/musicshop.md`.
Ordinal is `(code & 0xFFF) - 1`.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_STATE_MSHOP` (`0x1A000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_MSHP_1` (`0x1A100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_IMAGE_MSHOP_FRONT_BG` (`0x1A500002`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 2 | `IDC_BUTTON_MSHOP_OPENVIEW` (`0x1A200003`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 3 | `IDC_IMAGE_MSHOP_LEFTBOXLINE_LEFT` (`0x1A500004`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 4 | `IDC_IMAGE_MSHOP_LEFTBOXLINE_RIGHT` (`0x1A500005`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 5 | `IDC_IMAGE_MSHOP_LEFTBOXLINE_TOP` (`0x1A500006`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 6 | `IDC_IMAGE_MSHOP_LEFTBOXLINE_BOTTOM` (`0x1A500007`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 7 | `IDC_BUTTON_MSHOP_EXIT` (`0x1A200008`) | button | Leaves the music-shop side locally; no 5007 body pinned (#399). | OPEN: qualified BACK tap + wire leg (#399); seam music_shop_hand_session_sorts_pages_and_buys (Tier 0) |
| 8 | `IDC_BUTTON_MSHOP_CASHFILLUP` (`0x1A200009`) | button | Cash-charge view; #867 keeps it separate from the BUYITEM MESSAGE sheet. | OPEN: live charge-view capture with TCP + HTTP taps (#867) |
| 9 | `IDC_BUTTON_MSHOP_MSALL_LEFT` (`0x1A20000A`) | pager | Song-list pager back; pairings/directions ASSUMED (`docs/parity/musicshop.md`). | OPEN: live pager + direction leg; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 10 | `IDC_BUTTON_MSHOP_MSALL_RIGHT` (`0x1A20000B`) | pager | Song-list pager forward; pairings/directions ASSUMED. | OPEN: live pager + direction leg; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 11 | `IDC_BUTTON_MSHOP_MYALL_LEFT` (`0x1A20000C`) | pager | Owned-list pager back; pairings/directions ASSUMED. | OPEN: live pager + direction leg; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 12 | `IDC_BUTTON_MSHOP_MYALL_RIGHT` (`0x1A20000D`) | pager | Owned-list pager forward; pairings/directions ASSUMED. | OPEN: live pager + direction leg; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 13 | `IDC_BUTTON_MSHOP_DOWN_START` (`0x1A20000E`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 14 | `IDC_BUTTON_MSHOP_DOWN_CANCEL` (`0x1A20000F`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 15 | `IDC_BUTTON_MSHOP_MS_NEW` (`0x1A201010`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 16 | `IDC_BUTTON_MSHOP_MS_GOLD` (`0x1A201011`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 17 | `IDC_BUTTON_MSHOP_MS_BLACK` (`0x1A201012`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 18 | `IDC_BUTTON_MSHOP_MS_BLUE` (`0x1A201013`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 19 | `IDC_BUTTON_MSHOP_MS_RED` (`0x1A201014`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 20 | `IDC_BUTTON_MSHOP_MS_SILVER` (`0x1A201015`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 21 | `IDC_BUTTON_MSHOP_MY_TITLE` (`0x1A200016`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live sort-direction leg; seam music_shop_my_title_sort_toggles_direction_on_the_second_press (Tier 0) |
| 22 | `IDC_BUTTON_MSHOP_MY_PLAYTIME` (`0x1A200017`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live sort-direction leg; seam music_shop_my_title_sort_toggles_direction_on_the_second_press (Tier 0) |
| 23 | `IDC_BUTTON_MSHOP_MY_DOWNSTATUS` (`0x1A200018`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live sort-direction leg; seam music_shop_my_title_sort_toggles_direction_on_the_second_press (Tier 0) |
| 24 | `IDC_TEXT_MSHOP_MGAME_MONEY` (`0x1A400019`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 25 | `IDC_TEXT_MSHOP_O2JAM_MONEY` (`0x1A40001A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 26 | `IDC_TEXT_MSHOP_DN_CURRENTMUSIC` (`0x1A40001B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 27 | `IDC_TEXT_MSHOP_DN_SPEED` (`0x1A40001C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 28 | `IDC_TEXT_MSHOP_DN_LEFTTIME` (`0x1A40001D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 29 | `IDC_TEXT_MSHOP_DN_TOTALSTATUS` (`0x1A40001E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 30 | `IDC_TEXT_MSHOP_DN_TOTALLEFTTIME` (`0x1A40001F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 31 | `IDC_TEXT_MSHOP_DN_TOTAL_SIZE` (`0x1A400020`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 32 | `IDC_TEXT_MSHOP_DN_STATUS` (`0x1A400021`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 33 | `IDC_IMAGE_MSHOP_DN_CUR_PROGRESS` (`0x1A500022`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 34 | `IDC_IMAGE_MSHOP_DN_TOTAL_PROGRESS` (`0x1A500023`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 35 | `IDC_IMAGE_MSHOP_DN_PROG_ANI` (`0x1A500024`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 36 | `IDC_IMAGE_MSHOP_MS_ALL_LPAGECNT_10` (`0x1A500025`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 37 | `IDC_IMAGE_MSHOP_MS_ALL_LPAGECNT_1` (`0x1A500026`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 38 | `IDC_IMAGE_MSHOP_MS_ALL_RPAGECNT_10` (`0x1A500027`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 39 | `IDC_IMAGE_MSHOP_MS_ALL_RPAGECNT_1` (`0x1A500028`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 40 | `IDC_IMAGE_MSHOP_MY_ALL_LPAGECNT_10` (`0x1A500029`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 41 | `IDC_IMAGE_MSHOP_MY_ALL_LPAGECNT_1` (`0x1A50002A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 42 | `IDC_IMAGE_MSHOP_MY_ALL_RPAGECNT_10` (`0x1A50002B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 43 | `IDC_IMAGE_MSHOP_MY_ALL_RPAGECNT_1` (`0x1A50002C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 44 | `IDC_BUTTON_MS_EX_1` (`0x1A01802D`) | row panel (SET parent) | Song row 1 hit area; stages the cart row, no buy wire on the pick. | OPEN: live row-click + cart leg; seam music_shop_row_pick_feedback_stages_cart_without_a_buy_wire (Tier 0) |
| 45 | `IDC_BUTTON_MS_EX_2` (`0x1A01802E`) | row panel (SET parent) | Song row 2 hit area; stages the cart row, no buy wire on the pick. | OPEN: live row-click + cart leg; seam music_shop_row_pick_feedback_stages_cart_without_a_buy_wire (Tier 0) |
| 46 | `IDC_BUTTON_MS_EX_3` (`0x1A01802F`) | row panel (SET parent) | Song row 3 hit area; stages the cart row, no buy wire on the pick. | OPEN: live row-click + cart leg; seam music_shop_row_pick_feedback_stages_cart_without_a_buy_wire (Tier 0) |
| 47 | `IDC_BUTTON_MS_EX_4` (`0x1A018030`) | row panel (SET parent) | Song row 4 hit area; stages the cart row, no buy wire on the pick. | OPEN: live row-click + cart leg; seam music_shop_row_pick_feedback_stages_cart_without_a_buy_wire (Tier 0) |
| 48 | `IDC_BUTTON_MS_EX_5` (`0x1A018031`) | row panel (SET parent) | Song row 5 hit area; stages the cart row, no buy wire on the pick. | OPEN: live row-click + cart leg; seam music_shop_row_pick_feedback_stages_cart_without_a_buy_wire (Tier 0) |
| 49 | `IDC_IMAGE_MMS_BACK` (`0x1A500032`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 50 | `IDC_IMAGE_MMS_IMAGE` (`0x1A500033`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 51 | `IDC_IMAGE_MMS_NEW` (`0x1A500034`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 52 | `IDC_IMAGE_MMS_COLOR` (`0x1A500035`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 53 | `IDC_TEXT_MMS_TITLE` (`0x1A400036`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 54 | `IDC_TEXT_MMS_ISBUY` (`0x1A400037`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 55 | `IDC_TEXT_MMS_ARTIST` (`0x1A400038`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 56 | `IDC_TEXT_MMS_EDITOR` (`0x1A400039`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 57 | `IDC_TEXT_MMS_GENRE` (`0x1A40003A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 58 | `IDC_TEXT_MMS_BPM` (`0x1A40003B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 59 | `IDC_TEXT_MMS_DIFF` (`0x1A40003C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 60 | `IDC_IMAGE_MMS_GEM` (`0x1A50003D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 61 | `IDC_TEXT_MMS_RATE_GEMPRICE` (`0x1A40003E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 62 | `IDC_IMAGE_MMS_GEMARROW` (`0x1A50003F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 63 | `IDC_TEXT_MMS_GEM_PRICE` (`0x1A400040`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 64 | `IDC_IMAGE_MMS_GEM_BUY` (`0x1A200041`) | button (row clone) | Per-row buy control: the `0x200` family is a button despite the authored `IMAGE_` name; row-local clone queue 5010. | OPEN: live row-buy + gateway leg; seam music_shop_buy_buttons_tile_per_row_slot (Tier 0) |
| 65 | `IDC_IMAGE_MMS_WON` (`0x1A500042`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 66 | `IDC_TEXT_MMS_RATE_WONPRICE` (`0x1A400043`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 67 | `IDC_IMAGE_MMS_WONARROW` (`0x1A500044`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 68 | `IDC_TEXT_MMS_WON_PRICE` (`0x1A400045`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 69 | `IDC_IMAGE_MMS_WON_BUY` (`0x1A200046`) | button (row clone) | Per-row buy control: the `0x200` family is a button despite the authored `IMAGE_` name; row-local clone queue 5010. | OPEN: live row-buy + gateway leg; seam music_shop_buy_buttons_tile_per_row_slot (Tier 0) |
| 70 | `IDC_BUTTON_MY_EX1` (`0x1A019047`) | row panel (SET parent) | Owned row 1 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 71 | `IDC_BUTTON_MY_EX2` (`0x1A019048`) | row panel (SET parent) | Owned row 2 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 72 | `IDC_BUTTON_MY_EX3` (`0x1A019049`) | row panel (SET parent) | Owned row 3 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 73 | `IDC_BUTTON_MY_EX4` (`0x1A01904A`) | row panel (SET parent) | Owned row 4 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 74 | `IDC_BUTTON_MY_EX5` (`0x1A01904B`) | row panel (SET parent) | Owned row 5 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 75 | `IDC_BUTTON_MY_EX6` (`0x1A01904C`) | row panel (SET parent) | Owned row 6 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 76 | `IDC_BUTTON_MY_EX7` (`0x1A01904D`) | row panel (SET parent) | Owned row 7 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 77 | `IDC_BUTTON_MY_EX8` (`0x1A01904E`) | row panel (SET parent) | Owned row 8 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 78 | `IDC_BUTTON_MY_EX9` (`0x1A01904F`) | row panel (SET parent) | Owned row 9 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 79 | `IDC_BUTTON_MY_EX10` (`0x1A019050`) | row panel (SET parent) | Owned row 10 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 80 | `IDC_BUTTON_MY_EX11` (`0x1A019051`) | row panel (SET parent) | Owned row 11 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 81 | `IDC_BUTTON_MY_EX12` (`0x1A019052`) | row panel (SET parent) | Owned row 12 hit area; CHECK/COLOR/TITLE/TIME/DOWNSTATUS clones expand per row. | OPEN: live owned-row click leg; seam music_shop_my_title_sort_orders_owned_rows_and_clears_marks (Tier 0) |
| 82 | `IDC_IMAGE_MMY_CHECK` (`0x1A500053`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 83 | `IDC_IMAGE_MMY_COLOR` (`0x1A500054`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 84 | `IDC_TEXT_MMY_TITLE` (`0x1A400055`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 85 | `IDC_TEXT_MMY_TIME` (`0x1A400056`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 86 | `IDC_IMAGE_MMY_DOWNSTATUS` (`0x1A500057`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 87 | `IDC_IMAGE_MSHOP_MYCART_POS` (`0x1A600058`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 88 | `IDC_IMAGE_MSHOP_DOWN_POS` (`0x1A600059`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 89 | `IDC_IMAGE_NSHOP_MYCART_BACK` (`0x1A50005A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 90 | `IDC_TEXT_MSHOP_CART_GEMMONEY` (`0x1A50005B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 91 | `IDC_TEXT_MSHOP_CART_MCASH` (`0x1A50005C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 92 | `IDC_BUTTON_MSHOP_BUYITEM` (`0x1A20005D`) | button | Cart buy queues the pinned empty SyncMusicPurchase 5010; wire ride ASSUMED behind the stub seam. | OPEN: gateway replay of 5010 + destination; seam music_shop_row_pick_stages_the_cart_and_cart_buy_queues_5010 (Tier 0) |
| 93 | `IDC_BUTTON_MSHOP_MYCART_LEFT` (`0x1A20005E`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 94 | `IDC_BUTTON_MSHOP_MYCART_RIGHT` (`0x1A20005F`) | pager | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 95 | `IDC_BUTTON_MSHOP_CART_EX1` (`0x1A01B060`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 96 | `IDC_BUTTON_MSHOP_CART_EX2` (`0x1A01B061`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 97 | `IDC_BUTTON_MSHOP_CART_EX3` (`0x1A01B062`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 98 | `IDC_BUTTON_MSHOP_CART_EX4` (`0x1A01B063`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 99 | `IDC_BUTTON_MSHOP_CART_EX5` (`0x1A01B064`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 100 | `IDC_TEXT_MSHOP_CART_NUM` (`0x1A400065`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 101 | `IDC_IMAGE_MSHOP_CART_TYPE` (`0x1A500066`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 102 | `IDC_TEXT_MSHOP_CART_ITEMNAME` (`0x1A400067`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 103 | `IDC_TEXT_MSHOP_CART_PRICE` (`0x1A400068`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 104 | `IDC_IMAGE_MSHOP_CART_DELETE` (`0x1A500069`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 105 | `IDC_IMAGE_MSHOP_CART_CUR_P10` (`0x1A50006A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 106 | `IDC_IMAGE_MSHOP_CART_CUR_P1` (`0x1A50006B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 107 | `IDC_IMAGE_MSHOP_CART_TOT_P10` (`0x1A50006C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 108 | `IDC_IMAGE_MSHOP_CART_TOT_P1` (`0x1A50006D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 109 | `IDC_IMAGE_MSHOP_LIGHTBOARD` (`0x1A50006E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 110 | `IDC_IMAGE_MSHOP_TOOLTIP` (`0x1A50006F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 111 | `IDC_TEXT_MSHOP_TOOLTIP` (`0x1A400070`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 112 | `IDC_TEXT_MSHOP_NOTBUY_TOTAL` (`0x1A400071`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 113 | `IDC_TEXT_MSHOP_BUY_TOTAL` (`0x1A400072`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 114 | `IDC_BUTTON_MSHOP_GIFTITEM` (`0x1A200073`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 115 | `IDC_BUTTON_MSHOP_SHOWALL` (`0x1A502074`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 116 | `IDC_BUTTON_MSHOP_SHOWNOTBUY` (`0x1A502075`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 117 | `IDC_IMAGE_MSHOP_TABDOWN` (`0x1A600076`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 118 | `IDC_IMAGE_MSHOP_TABDELETE` (`0x1A600077`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 119 | `IDC_IMAGE_MSHOP_TABDELETE_BACK` (`0x1A500078`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 120 | `IDC_IMAGE_MSHOP_MUSICDELETE_BASE` (`0x1A500079`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 121 | `IDC_BUTTON_MSHOP_MUSICDELETE` (`0x1A20007A`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 122 | `IDC_EDIT_MSHOP_SEARCH` (`0x1A30007B`) | edit / key-path | Search draft; no live search handler pinned. | OPEN: live typing + result capture (`docs/parity/musicshop.md` search OPEN) |
| 123 | `IDC_BUTTON_MSHOP_SEARCH` (`0x1A20007C`) | button | Search press; no live search result pinned. | OPEN: REA search dispatch plus Tier 3 click/result leg |
| 124 | `IDC_BUTTON_MSHOP_TOP100` (`0x1A20007D`) | button | Opens the TOP100 arm; eight staged rects answer presses, standings unpinned (#179). | OPEN: live standings capture; seam music_shop_top100_arm_stages_eight_rects_and_answer_each_press (Tier 0) |
| 125 | `IDC_BUTTON_MSHOP_MS_3KEY` (`0x1A20007E`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA sort/filter dispatch plus Tier 3 press; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 126 | `IDC_BUTTON_MSHOP_MS_7KEY` (`0x1A20007F`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA sort/filter dispatch plus Tier 3 press; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 127 | `IDC_BUTTON_MSHOP_MS_TITLE` (`0x1A200080`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA sort/filter dispatch plus Tier 3 press; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 128 | `IDC_BUTTON_MSHOP_MS_DIFF` (`0x1A200081`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA sort/filter dispatch plus Tier 3 press; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 129 | `IDC_BUTTON_MSHOP_MS_BPM` (`0x1A200082`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA sort/filter dispatch plus Tier 3 press; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 130 | `IDC_BUTTON_MSHOP_MS_PRICE` (`0x1A200083`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA sort/filter dispatch plus Tier 3 press; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| 131 | `IDC_BUTTON_MSHOP_SELECTALL` (`0x1A200084`) | button | Select-all chrome; no exe selection rule pinned. | OPEN: REA dispatch plus Tier 3 press + row-selection frame leg |
| 132 | `IDC_IMAGE_MSHOP_NOTICE_TEXT` (`0x1A500085`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 133 | `IDC_IMAGE_MSHOP_NOTICEBOARD` (`0x1A500086`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 134 | `IDC_BUTTON_MSHOP_CONFERENCE` (`0x1A200087`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 135 | `IDC_BUTTON_MSHOP_MESSENGER` (`0x1A200088`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA click dispatch plus Tier 3 click from boot; rig capture if no handler |
| 136 | `IDC_IMAGE_MSHOP_MESSENGER_ALRAM` (`0x1A500089`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 137 | `IDC_IMAGE_MSHOP_MESSENGER_ALRAM1` (`0x1A50008A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |

Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Music-shop sort direction and pager pairing | Directions/pairings ASSUMED in `docs/parity/musicshop.md`. | OPEN: SendInput sweep + gateway replay; seam music_shop_sort_and_pager_answer_their_staged_buttons (Tier 0) |
| TOP100 standings contents | Eight chrome rects answer presses; standings unpinned (#179). | OPEN: capture with a 5011 rank reply; seam music_shop_top100_arm_stages_eight_rects_and_answer_each_press (Tier 0) |

## Options

The ControlList defines three option sheets (`DIALOG_OPTION_KEY`,
`DIALOG_OPTION_SOUND`, `DIALOG_OPTION_GAME`); there is no combined
`STATE_OPTION` block. Each sheet is censused below.

### Options / Key sheet

Source: `BOUND "Dialog_Option_*.bnd"` in the pinned `ControlList_Interface.txt`
(lines 1242–1277; 26 entries / 25 rects). Ordinals are `(code & 0xFFF) - 1`.
Sheet order and tab pins: `docs/options.md`; #314 reports inert keys/hover
from Room and Planet, so key rows stay OPEN even where a seam guard passes.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_OPTION_KEY` (`0x25000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DOPTION_1` (`0x25100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_IMAGE_DOPTION_KEY_BACK` (`0x25500002`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 2 | `IDC_BUTTON_DOPTION_3KEY` (`0x25201003`) | tab / toggle | Key tab 3K selector; switches the staged-code family (#18). | OPEN: live tab press from Planet and Room (#314); seam options_key_tab_rebind_answers_on_the_new_lane (Tier 0) |
| 3 | `IDC_BUTTON_DOPTION_7KEY` (`0x25201004`) | tab / toggle | Key tab 7K selector; switches the staged-code family (#18). | OPEN: live tab press from Planet and Room (#314); seam options_key_tab_rebind_answers_on_the_new_lane (Tier 0) |
| 4 | `IDC_BUTTON_DOPTION_KEYTEST` (`0x2520A005`) | tab / toggle | Key-test checkbox; checked state paints the key-down bodies (#845). | OPEN: checkbox press capture (#845); seam options_sheets_paint_rest_hover_and_active_frames (Tier 0) |
| 5 | `IDC_IMAGE_DOPTION_3KEYSELECT` (`0x25500006`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 6 | `IDC_IMAGE_DOPTION_KEYSELECT` (`0x25500007`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 7 | `IDC_BUTTON_DOPTION_3KEY_1` (`0x25209008`) | tab / toggle | 3K lane 1 rebind cell; mapped press writes the 3K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 8 | `IDC_BUTTON_DOPTION_3KEY_2` (`0x25209009`) | tab / toggle | 3K lane 2 rebind cell; mapped press writes the 3K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 9 | `IDC_BUTTON_DOPTION_3KEY_3` (`0x2520900A`) | tab / toggle | 3K lane 3 rebind cell; mapped press writes the 3K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 10 | `IDC_BUTTON_DOPTION_3KEY_4` (`0x2520900B`) | tab / toggle | 3K lane 4 rebind cell; mapped press writes the 3K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 11 | `IDC_BUTTON_DOPTION_KEY1` (`0x2520900C`) | tab / toggle | 7K lane 1 rebind cell; mapped press writes the 7K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 12 | `IDC_BUTTON_DOPTION_KEY2` (`0x2520900D`) | tab / toggle | 7K lane 2 rebind cell; mapped press writes the 7K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 13 | `IDC_BUTTON_DOPTION_KEY3` (`0x2520900E`) | tab / toggle | 7K lane 3 rebind cell; mapped press writes the 7K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 14 | `IDC_BUTTON_DOPTION_KEY4` (`0x2520900F`) | tab / toggle | 7K lane 4 rebind cell; mapped press writes the 7K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 15 | `IDC_BUTTON_DOPTION_KEY5` (`0x25209010`) | tab / toggle | 7K lane 5 rebind cell; mapped press writes the 7K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 16 | `IDC_BUTTON_DOPTION_KEY6` (`0x25209011`) | tab / toggle | 7K lane 6 rebind cell; mapped press writes the 7K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 17 | `IDC_BUTTON_DOPTION_KEY7` (`0x25209012`) | tab / toggle | 7K lane 7 rebind cell; mapped press writes the 7K bind table (#18). | OPEN: live raw-key capture from Planet/Room (#314); seam options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects (Tier 0) |
| 18 | `IDC_IMAGE_DOPTION_3KEYDOWN` (`0x25500013`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 19 | `IDC_IMAGE_DOPTION_KEYDOWN` (`0x25500014`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 20 | `IDC_BUTTON_DOPTION_DEFAULT` (`0x25200015`) | button | Restores the tab defaults; SAVE rejects zero/duplicate binds (#18). | OPEN: live press + ini leg; seam options_default_restores_the_exe_defaults_per_tab (Tier 0) |
| 21 | `IDC_BUTTON_DOPTION_SAVE` (`0x25200016`) | button | Commits option.ini beside the install root (#18). | OPEN: live press + ini leg; seam options_save_commits_beside_the_install_root (Tier 0) |
| 22 | `IDC_BUTTON_DOPTION_EXIT` (`0x25200017`) | button | Closes without saving; Escape also closes (#18). | OPEN: live press + dialog-ownership leg (#314) |
| 23 | `IDC_BUTTON_DOPTION_SOUNDSET` (`0x25600018`) | button | Switches to the Sound tab at the staged rect (#854). | OPEN: live tab press from both entry paths (#314); seam live_boot_driven_tab_clicks_repaint_sound_and_game_sheets (Tier 3) |
| 24 | `IDC_BUTTON_DOPTION_GAMESET` (`0x25600019`) | button | Switches to the Game tab at the staged rect (#854). | OPEN: live tab press from both entry paths (#314); seam live_boot_driven_tab_clicks_repaint_sound_and_game_sheets (Tier 3) |

### Options / Sound sheet

Source: `BOUND "Dialog_Option_*.bnd"` in the pinned `ControlList_Interface.txt`
(lines 1281–1308; 23 entries / 22 rects). Ordinals are `(code & 0xFFF) - 1`.
Sheet order and tab pins: `docs/options.md`; #314 reports inert keys/hover
from Room and Planet, so key rows stay OPEN even where a seam guard passes.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_OPTION_SOUND` (`0x26000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DOPTIONSOUND` (`0x26100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_DOPTIONSOUND_BGDOWN` (`0x26200002`) | button | Volume step ±4 in 0..255; first pair moves GameBGVolume and BGVolume together (#712). | OPEN: live slider click + ini leg (#314); seam options_sound_tab_steps_move_volumes_with_exe_bounds (Tier 0) |
| 2 | `IDC_BUTTON_DOPTIONSOUND_BGUP` (`0x26200003`) | button | Volume step ±4 in 0..255; first pair moves GameBGVolume and BGVolume together (#712). | OPEN: live slider click + ini leg (#314); seam options_sound_tab_steps_move_volumes_with_exe_bounds (Tier 0) |
| 3 | `IDC_BUTTON_DOPTIONSOUND_GAMEALLDOWN` (`0x26200004`) | button | Volume step ±4 in 0..255; first pair moves GameBGVolume and BGVolume together (#712). | OPEN: live slider click + ini leg (#314); seam options_sound_tab_steps_move_volumes_with_exe_bounds (Tier 0) |
| 4 | `IDC_BUTTON_DOPTIONSOUND_GAMEALLUP` (`0x26200005`) | button | Volume step ±4 in 0..255; first pair moves GameBGVolume and BGVolume together (#712). | OPEN: live slider click + ini leg (#314); seam options_sound_tab_steps_move_volumes_with_exe_bounds (Tier 0) |
| 5 | `IDC_BUTTON_DOPTIONSOUND_GAMEMUSICDOWN` (`0x26200006`) | button | Volume step ±4 in 0..255; first pair moves GameBGVolume and BGVolume together (#712). | OPEN: live slider click + ini leg (#314); seam options_sound_tab_steps_move_volumes_with_exe_bounds (Tier 0) |
| 6 | `IDC_BUTTON_DOPTIONSOUND_GAMEMUSICUP` (`0x26200007`) | button | Volume step ±4 in 0..255; first pair moves GameBGVolume and BGVolume together (#712). | OPEN: live slider click + ini leg (#314); seam options_sound_tab_steps_move_volumes_with_exe_bounds (Tier 0) |
| 7 | `IDC_BUTTON_DOPTIONSOUND_GAMEKEYDOWN` (`0x26200008`) | button | Volume step ±4 in 0..255; first pair moves GameBGVolume and BGVolume together (#712). | OPEN: live slider click + ini leg (#314); seam options_sound_tab_steps_move_volumes_with_exe_bounds (Tier 0) |
| 8 | `IDC_BUTTON_DOPTIONSOUND_GAMEKEYUP` (`0x26200009`) | button | Volume step ±4 in 0..255; first pair moves GameBGVolume and BGVolume together (#712). | OPEN: live slider click + ini leg (#314); seam options_sound_tab_steps_move_volumes_with_exe_bounds (Tier 0) |
| 9 | `IDC_IMAGE_DOPTIONSOUND_BGEDGE` (`0x2650000A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 10 | `IDC_IMAGE_DOPTIONSOUND_GAMEALLEDGE` (`0x2650000B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 11 | `IDC_IMAGE_DOPTIONSOUND_GAMEMUSICEDGE` (`0x2650000C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 12 | `IDC_IMAGE_DOPTIONSOUND_GAMEKEYEDGE` (`0x2650000D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 13 | `IDC_IMAGE_DOPTIONSOUND_KEYSET` (`0x2660000E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 14 | `IDC_IMAGE_DOPTIONSOUND_GAMESET` (`0x2660000F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 15 | `IDC_BUTTON_DOPTIONSOUND_DEFAULT` (`0x26200010`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live press + ini leg (#314); seam options_default_restores_the_exe_defaults_per_tab (Tier 0) |
| 16 | `IDC_BUTTON_DOPTIONSOUND_SAVE` (`0x26200011`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live press + ini leg (#314); seam options_default_restores_the_exe_defaults_per_tab (Tier 0) |
| 17 | `IDC_BUTTON_DOPTIONSOUND_EXIT` (`0x26200012`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live press + ini leg (#314); seam options_default_restores_the_exe_defaults_per_tab (Tier 0) |
| 18 | `IDC_IMAGE_DOPTIONSOUND_BGBAR` (`0x26500013`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 19 | `IDC_IMAGE_DOPTIONSOUND_GAMEALLBAR` (`0x26500014`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 20 | `IDC_IMAGE_DOPTIONSOUND_GAMEMUSICBAR` (`0x26500015`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 21 | `IDC_IMAGE_DOPTIONSOUND_GAMEKEYBAR` (`0x26500016`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |

### Options / Game sheet

Source: `BOUND "Dialog_Option_*.bnd"` in the pinned `ControlList_Interface.txt`
(lines 1611–1639; 24 entries / 23 rects). Ordinals are `(code & 0xFFF) - 1`.
Sheet order and tab pins: `docs/options.md`; #314 reports inert keys/hover
from Room and Planet, so key rows stay OPEN even where a seam guard passes.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_OPTION_GAME` (`0x2F000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DOPTION_GAME` (`0x2F100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_DOPTIONGAME_EFFECTHIGH` (`0x2F209002`) | tab / toggle | Effects High → in-memory 2 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 2 | `IDC_BUTTON_DOPTIONGAME_EFFECTNORMAL` (`0x2F209003`) | tab / toggle | Effects Normal → in-memory 1 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 3 | `IDC_BUTTON_DOPTIONGAME_EFFECTLOW` (`0x2F209004`) | tab / toggle | Effects Low → in-memory 0 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 4 | `IDC_BUTTON_DOPTIONGAME_BLENDHALF_YES` (`0x2F209005`) | tab / toggle | EQ Yes → 1 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 5 | `IDC_BUTTON_DOPTIONGAME_BLENDHALF_NO` (`0x2F209006`) | tab / toggle | EQ No → 0 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 6 | `IDC_BUTTON_DOPTIONGAME_NOTETYPE1` (`0x2F209007`) | tab / toggle | NoteType1 → 0 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 7 | `IDC_BUTTON_DOPTIONGAME_NOTETYPE2` (`0x2F209008`) | tab / toggle | NoteType2 → 1 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 8 | `IDC_BUTTON_DOPTIONGAME_NOTEGUIDELONG` (`0x2F209009`) | tab / toggle | Guide Long → 3 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 9 | `IDC_BUTTON_DOPTIONGAME_NOTEGUIDEMIDDLE` (`0x2F20900A`) | tab / toggle | Guide Middle → 2 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 10 | `IDC_BUTTON_DOPTIONGAME_NOTEGUIDESHORT` (`0x2F20900B`) | tab / toggle | Guide Short → 1 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 11 | `IDC_BUTTON_DOPTIONGAME_NOTEGUIDENOUSE` (`0x2F20900C`) | tab / toggle | Guide NoUse → 0 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 12 | `IDC_BUTTON_DOPTIONGAME_DEFAULT` (`0x2F20000D`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 13 | `IDC_BUTTON_DOPTIONGAME_SAVE` (`0x2F20000E`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 14 | `IDC_BUTTON_DOPTIONGAME_EXIT` (`0x2F20000F`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 15 | `IDC_BUTTON_DOPTIONGAME_KEYSET` (`0x2F600010`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live tab press from both entry paths (#314); seam live_boot_driven_tab_clicks_repaint_sound_and_game_sheets (Tier 3) |
| 16 | `IDC_BUTTON_DOPTIONGAME_SOUNDSET` (`0x2F600011`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: live tab press from both entry paths (#314); seam live_boot_driven_tab_clicks_repaint_sound_and_game_sheets (Tier 3) |
| 17 | `IDC_IMAGE_DOPTIONGAME_EFFECTUNCHECK1` (`0x2F500012`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 18 | `IDC_IMAGE_DOPTIONGAME_EFFECTUNCHECK2` (`0x2F500013`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 19 | `IDC_BUTTON_DOPTIONGAME_MODEFULL` (`0x2F209014`) | tab / toggle | ModeFull → 1 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 20 | `IDC_BUTTON_DOPTIONGAME_MODEWIN` (`0x2F209015`) | tab / toggle | ModeWin → 2 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 21 | `IDC_BUTTON_DOPTIONGAME_LONGNOTEMODE_USED` (`0x2F209016`) | tab / toggle | Longnote Used → 1 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |
| 22 | `IDC_BUTTON_DOPTIONGAME_LONGNOTEMODE_NONE` (`0x2F209017`) | tab / toggle | Longnote None → 0 (`docs/options.md`). | OPEN: live press + ini leg (#314); seam options_game_tab_buttons_map_to_the_pinned_ini_values (Tier 0) |


Input edges with no matching ControlList entry:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Options raw-key capture and focus | Mapped press writes the active bind table; unmapped keys refused; keys leak to chat/background in Room and Planet (#314). | OPEN: live raw-key leg from both entry paths (#314); seam options_key_tab_rebind_answers_on_the_new_lane (Tier 0) |
| Options hover over key/slider cells | Hover blanks the key area in Room/Planet (#314 C4/C3); sheet rest/hover/active frames are seam-pinned only. | OPEN: off/on/off pixel trio from both entry paths; seam options_sheets_paint_rest_hover_and_active_frames (Tier 0) |

## Dialogs

The pinned ControlList defines **27 dialog-family blocks / 489 IDC
entries** (`DIALOG_*` plus `DAILOG_SELECTMUSIC`). Three option sheets
are censused under `## Options`; the other 24 blocks are listed here.
No family is scoped out. Each entry is one row; `Dialog_*.bnd` rects
pair by `(code & 0xFFF) - 1` (`docs/bnd-semantics.md`).

### create room / DIALOG_CREATEROOM

Source: `BOUND "Dialog_CreateRoom.bnd"` (lines 1222–1238; 10 entries / 9 rects).

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_CROOM` (`0x24000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DCROOM_1` (`0x24100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_DCROOM_CREATE` (`0x24200002`) | button | Commit 2004 → Waiting as master; typed title and mode fields ride the staged form (`#53`, `#269`). | OPEN: live CREATE press + 2004 gateway leg (#53/#269); seam waiting_create_room_commit_stages_room_over_scripted_gateway (Tier 2) |
| 2 | `IDC_BUTTON_DCROOM_CANCEL` (`0x24200003`) | button | Cancels the sheet and returns to the lobby board. | OPEN: live CANCEL click + sheet-dismissal leg |
| 3 | `IDC_BUTTON_DCROOM_NORMAL_MODE` (`0x2420B004`) | tab / toggle | NORMAL-mode selector; baked disabled faces on a fresh open (#53). | OPEN: mode toggle capture; seam create_room_toggle_click_paints_the_selected_face_under_the_cursor (Tier 0) |
| 4 | `IDC_BUTTON_DCROOM_LIVEBATTLE_MODE` (`0x2420B005`) | tab / toggle | LIVE BATTLE-mode selector; same disabled-face rule. | OPEN: mode toggle capture; seam create_room_toggle_click_paints_the_selected_face_under_the_cursor (Tier 0) |
| 5 | `IDC_BUTTON_DCROOM_3KEY_MODE` (`0x2420B006`) | tab / toggle | 3-key mode selector; live toggle face under the cursor (#53). | OPEN: mode toggle capture; seam create_room_toggle_click_paints_the_selected_face_under_the_cursor (Tier 0) |
| 6 | `IDC_BUTTON_DCROOM_7KEY_MODE` (`0x2420B007`) | tab / toggle | 7-key mode selector; live toggle face under the cursor (#53). | OPEN: mode toggle capture; seam create_room_toggle_click_paints_the_selected_face_under_the_cursor (Tier 0) |
| 7 | `IDC_EDIT_DCROOM_TITLE` (`0x24300008`) | edit / key-path | Room-title draft; Commit 2004 carries the typed string. | OPEN: live typing + 2004 body leg; seam create_room_commit_stages_the_typed_title (Tier 0) |
| 8 | `IDC_EDIT_DCROOM_PASSWORD` (`0x24300009`) | edit / key-path | Password draft; Commit 2004 carries the typed string. | OPEN: live typing + 2004 body leg; seam create_room_form (Tier 2) |

### password / DIALOG_INPUTPASSWORD

Source: `BOUND "Dialog_Password.bnd"` (lines 1494–1503; 6 entries / 5 rects).

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_PASSWORD` (`0x28000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DPASSWORD_1` (`0x28100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_TEXT_PASSWORDINFO` (`0x28400002`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 2 | `IDC_EDIT_DPASSWORD` (`0x28300003`) | edit / key-path | Password draft; join body carries it. | OPEN: live typing + 3002 password-body leg |
| 3 | `IDC_BUTTON_DPASSWORD_OK` (`0x28200004`) | button | Join Commit 3002 with the typed password → Waiting (#269). | OPEN: live typed Commit + 3002 gateway leg (#269); seam password_join_commit_stages_room_over_scripted_gateway (Tier 2) |
| 4 | `IDC_BUTTON_DPASSWORD_CANCEL` (`0x28200005`) | button | Dismisses the password sheet without a join. | OPEN: live CANCEL click + sheet-dismissal leg |

### shop confirm / DIALOG_INFORMATION

Source: block `DIALOG_INFORMATION` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_INFO` (`0x21000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DINFO` (`0x21100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_DINFO_OK` (`0x21200002`) | button | One-OK information sheet: dismisses and releases held rows (#717 / relay-down pin). | VERIFIED: live_click_dialog_ok_releases_the_held_channel_rows (Tier 3) — boot → OK click → rows released |
| 2 | `IDC_TEXT_DINFO_MESSAGE` (`0x21400003`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |

### shop confirm / DIALOG_QUESTION1

Source: block `DIALOG_QUESTION1` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_QUESTION1` (`0x22000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DQUESTION1` (`0x22100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_DQUESTION1_OK` (`0x22200002`) | button | BUYITEM MESSAGE sheet OK: local commit, no purchase opcode (#867). | VERIFIED: live_boot_cart_row_buy_paints_the_native_message_sheet (Tier 3) — boot → BUYITEM → OK commit, no buy wire |
| 2 | `IDC_BUTTON_DQUESTION1_CANCEL` (`0x22200003`) | button | BUYITEM MESSAGE sheet CANCEL: dismiss, no purchase opcode (#867). | VERIFIED: live_boot_cart_row_buy_paints_the_native_message_sheet (Tier 3) — boot → BUYITEM → CANCEL dismiss, no buy wire |
| 3 | `IDC_TEXT_DQUESTION1_MESSAGE` (`0x22400004`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |

### shop confirm / DIALOG_QUESTION2

Source: block `DIALOG_QUESTION2` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_QUESTION2` (`0x23000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DQUESTION2` (`0x23100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_DQUESTION2_YES` (`0x23200002`) | button | YES arm of the two-choice sheet; no live shop mount pinned. | OPEN: pin a live exe mount of Dialog_Question2, then a Tier 3 YES click leg |
| 2 | `IDC_BUTTON_DQUESTION2_NO` (`0x23200003`) | button | NO arm of the two-choice sheet; no live shop mount pinned. | OPEN: pin a live exe mount of Dialog_Question2, then a Tier 3 NO click leg |
| 3 | `IDC_TEXT_DQUESTION2_MESSAGE` (`0x23400004`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |

### shop confirm / DIALOG_SHOPPINGBAG_ADD

Source: block `DIALOG_SHOPPINGBAG_ADD` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_SHOPPINGBAG_ADD` (`0x82000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_SHOPPINGBAG_ADD` (`0x82100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_SHOPPINGBAG_ADD_GEM` (`0x82200002`) | button | Gem-price purchase choice; sheet parked with no exe mount or add opcode (#316 / #685). | OPEN: one qualified cart-add click + add/reply opcode capture (#316 story 11, #685) |
| 2 | `IDC_BUTTON_SHOPPINGBAG_ADD_CASH` (`0x82200003`) | button | Cash-price purchase choice; same parked-sheet rule. | OPEN: one qualified cart-add click + add/reply opcode capture (#316 story 11, #685) |
| 3 | `IDC_BUTTON_SHOPPINGBAG_ADD_CANCEL` (`0x82200004`) | button | Cancels the parked add sheet; no live exe frame. | OPEN: one qualified cart-add dismissal capture (#316 story 11) |
| 4 | `IDC_TEXT_SHOPPINGBAG_ADD_ITEMNAME` (`0x82400005`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 5 | `IDC_TEXT_SHOPPINGBAG_ADD_GEMPRICE` (`0x82400006`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 6 | `IDC_TEXT_SHOPPINGBAG_ADD_CASHPRICE` (`0x82400007`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |

### messenger / liveness / DIALOG_MESSENGER_MAIN

Source: block `DIALOG_MESSENGER_MAIN` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_MESSENGER_MAIN` (`0x86000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_MESSENGER_MAIN` (`0x86100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_MESSENGER_MAIN_CLOSE` (`0x86200002`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 2 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND1` (`0x86041003`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 3 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND2` (`0x86041004`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 4 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND3` (`0x86041005`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 5 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND4` (`0x86041006`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 6 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND5` (`0x86041007`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 7 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND6` (`0x86041008`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 8 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND7` (`0x86041009`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 9 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND8` (`0x8604100A`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 10 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND9` (`0x8604100B`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 11 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND10` (`0x8604100C`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 12 | `IDC_BUTTON2_MESSENGER_FRIEND_EXTEND11` (`0x8604100D`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 13 | `IDC_IMAGE_MESSENGER_MAIN_CHECK` (`0x8650000E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 14 | `IDC_TEXT_MESSENGER_MAIN_GROUPNAME` (`0x8640000F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 15 | `IDC_IMAGE_MESSENGER_MAIN_FLISTONOFF` (`0x86500010`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 16 | `IDC_TEXT_MESSENGER_MAIN_FRIENDNAME` (`0x86400011`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 17 | `IDC_BUTTON2_MESSENGER_NOTE_EXTEND1` (`0x86042012`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 18 | `IDC_BUTTON2_MESSENGER_NOTE_EXTEND2` (`0x86042013`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 19 | `IDC_BUTTON2_MESSENGER_NOTE_EXTEND3` (`0x86042014`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 20 | `IDC_BUTTON2_MESSENGER_NOTE_EXTEND4` (`0x86042015`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 21 | `IDC_BUTTON2_MESSENGER_NOTE_EXTEND5` (`0x86042016`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 22 | `IDC_BUTTON2_MESSENGER_NOTE_EXTEND6` (`0x86042017`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 23 | `IDC_BUTTON2_MESSENGER_NOTE_EXTEND7` (`0x86042018`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 24 | `IDC_BUTTON2_MESSENGER_NOTE_EXTEND8` (`0x86042019`) | row panel (SET parent) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 25 | `IDC_TEXT_MESSENGER_MAIN_SENDUSERNAME` (`0x8640001A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 26 | `IDC_TEXT_MESSENGER_MAIN_RECVMESAGE` (`0x8640001B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 27 | `IDC_IMAGE_MESSENGER_MAIN_NOTEDEL` (`0x8650001C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 28 | `IDC_IMAGE_MESSENGER_MAIN_RECVNOTE` (`0x8660001D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 29 | `IDC_IMAGE_MESSENGER_MAIN_SAVENOTE` (`0x8660001E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 30 | `IDC_TEXT_MESSENGER_MAIN_RECVNOTE` (`0x8640001F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 31 | `IDC_TEXT_MESSENGER_MAIN_SAVENOTE` (`0x86400020`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 32 | `IDC_BUTTON_MESSENGER_MAIN_NOTEPAGELEFT` (`0x86200021`) | button (static digit) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 33 | `IDC_BUTTON_MESSENGER_MAIN_NOTEPAGERIGHT` (`0x86200022`) | button (static digit) | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 34 | `IDC_TEXT_MESSENGER_MAIN_NOTEPAGENUM` (`0x86400023`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 35 | `IDC_BUTTON_MESSENGER_MAIN_NOTESEND` (`0x86200024`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 36 | `IDC_BUTTON_MESSENGER_MAIN_FRIENDLIST_TAP` (`0x86201025`) | tab / toggle | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 37 | `IDC_BUTTON_MESSENGER_MAIN_NOTE_TAP` (`0x86201026`) | tab / toggle | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 38 | `IDC_BUTTON_MESSENGER_MAIN_CHAT1_TAP` (`0x8620B027`) | tab / toggle | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 39 | `IDC_BUTTON_MESSENGER_MAIN_CHAT2_TAP` (`0x8620B028`) | tab / toggle | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 40 | `IDC_BUTTON_MESSENGER_MAIN_CHAT3_TAP` (`0x8620B029`) | tab / toggle | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 41 | `IDC_BUTTON_MESSENGER_MAIN_CHAT4_TAP` (`0x8620B02A`) | tab / toggle | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 42 | `IDC_IMAGE_MESSENGER_MAIN_NOTE_ALARM` (`0x8650002B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 43 | `IDC_IMAGE_MESSENGER_MAIN_CHAT1_ALARM` (`0x8650002C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 44 | `IDC_IMAGE_MESSENGER_MAIN_CHAT2_ALARM` (`0x8650002D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 45 | `IDC_IMAGE_MESSENGER_MAIN_CHAT3_ALARM` (`0x8650002E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 46 | `IDC_IMAGE_MESSENGER_MAIN_CHAT4_ALARM` (`0x8650002F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 47 | `IDC_TEXT_MESSENGER_MAIN_CHAT1_TAP` (`0x86400030`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 48 | `IDC_TEXT_MESSENGER_MAIN_CHAT2_TAP` (`0x86400031`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 49 | `IDC_TEXT_MESSENGER_MAIN_CHAT3_TAP` (`0x86400032`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 50 | `IDC_TEXT_MESSENGER_MAIN_CHAT4_TAP` (`0x86400033`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 51 | `IDC_BUTTON_MESSENGER_MAIN_CHAT_TAPUP` (`0x86200034`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 52 | `IDC_BUTTON_MESSENGER_MAIN_CHAT_TAPDOWN` (`0x86200035`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 53 | `IDC_IMAGE_MESSENGER_MAIN_TAPUP_ALARM` (`0x86500036`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 54 | `IDC_IMAGE_MESSENGER_MAIN_TAPDOWN_ALARM` (`0x86500037`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 55 | `IDC_BUTTON_MESSENGER_MAIN_FRIEND_ADD` (`0x86200038`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 56 | `IDC_BUTTON_MESSENGER_MAIN_GROUP_ADD` (`0x86200039`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 57 | `IDC_IMAGE_MESSENGER_MAIN_O2JAMFRINED_TAP` (`0x8660003A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 58 | `IDC_IMAGE_MESSENGER_MAIN_OTHER_TAP` (`0x8660003B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 59 | `IDC_IMAGE_MESSENGER_MAIN_OTHER_TAP_BACK` (`0x8650003C`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 60 | `IDC_TEXT_MESSENGER_MAIN_O2JAMFRINED_TAP` (`0x8640003D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 61 | `IDC_TEXT_MESSENGER_MAIN_OTHER_TAP` (`0x8640003E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 62 | `IDC_EDIT_MESSENGER_MAIN_CHAT` (`0x8630003F`) | edit / key-path | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 63 | `IDC_LIST_MESSENGER_MAIN_CHAT` (`0x86800040`) | list | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 64 | `IDC_SCROLL_MESSENGER_MAIN_FRIEND` (`0x86021041`) | scrollbar | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 65 | `IDC_BUTTON_MESSENGER_MAIN_SCROLLUP` (`0x86200042`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 66 | `IDC_BUTTON_MESSENGER_MAIN_SCROLLDOWN` (`0x86200043`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 67 | `IDC_BUTTON_MESSENGER_MAIN_SCROLLBAR` (`0x86200044`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 68 | `IDC_BUTTON_MESSENGER_MAIN_SCROLLRANGE` (`0x86200045`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 69 | `IDC_SCROLL_MESSENGER_MAIN_CHAT` (`0x86022046`) | scrollbar | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 70 | `IDC_BUTTON_MESSENGER_MAIN_CHAT_SCROLLUP` (`0x86200047`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 71 | `IDC_BUTTON_MESSENGER_MAIN_CHAT_SCROLLDOWN` (`0x86200048`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 72 | `IDC_BUTTON_MESSENGER_MAIN_CHAT_SCROLLBAR` (`0x86200049`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 73 | `IDC_BUTTON_MESSENGER_MAIN_CHAT_SCROLLRANGE` (`0x8620004A`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 74 | `IDC_IMAGE_MESSENGER_MAIN_CHARROOM_ICON` (`0x8650004B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 75 | `IDC_TEXT_MESSENGER_MAIN_CHATROOM_USERNAME` (`0x8640004C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 76 | `IDC_BUTTON_MESSENGER_MAIN_CHAT_CLOSE` (`0x8620004D`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |

### messenger / liveness / DIALOG_MESSENGER_FRIENDADD

Source: block `DIALOG_MESSENGER_FRIENDADD` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_MESSENGER_FRIENDADD` (`0x87000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_MESSENGER_FRIENDADD` (`0x87100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_MESSENGER_FRIENDADD_OK` (`0x87200002`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 2 | `IDC_BUTTON_MESSENGER_FRIENDADD_CANCEL` (`0x87200003`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 3 | `IDC_EDIT_MESSENGER_FRIENDADD_NICKNAME` (`0x87300004`) | edit / key-path | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 4 | `IDC_BUTTON_MESSENGER_FRIENDADD_SEARCH` (`0x87200005`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 5 | `IDC_TEXT_MESSENGER_FRIENDADD_RESULT` (`0x87400006`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |

### messenger / liveness / DIALOG_MESSENGER_QUESTION

Source: block `DIALOG_MESSENGER_QUESTION` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_MESSENGER_QUESTION` (`0x88000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_MESSENGER_QUESTION` (`0x88100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_MESSENGER_QUESTION_OK` (`0x88200002`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 2 | `IDC_BUTTON_MESSENGER_QUESTION_CANCEL` (`0x88200003`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 3 | `IDC_TEXT_MESSENGER_QUESTION_TITLE` (`0x88400004`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 4 | `IDC_TEXT_MESSENGER_QUESTION_OK` (`0x88400005`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 5 | `IDC_TEXT_MESSENGER_QUESTION_CANCEL` (`0x88400006`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 6 | `IDC_TEXT_MESSENGER_QUESTION_MESSAGE` (`0x88400007`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |

### messenger / liveness / DIALOG_MESSENGER_NOTE

Source: block `DIALOG_MESSENGER_NOTE` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_MESSENGER_NOTE` (`0x89000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_MESSENGER_NOTE` (`0x89100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_MESSENGER_NOTE_CLOSE` (`0x89200002`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 2 | `IDC_BUTTON_MESSENGER_NOTE_SEARCH` (`0x89200003`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 3 | `IDC_EDIT_MESSENGER_NOTE_SERACH` (`0x89300004`) | edit / key-path | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 4 | `IDC_EDIT_MESSENGER_NOTE_MESSAGE` (`0x89300005`) | edit / key-path | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 5 | `IDC_TEXT_MESSENGER_NOTE_RESULT` (`0x89400006`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 6 | `IDC_BUTTON_MESSENGER_NOTE_SEND` (`0x89200007`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 7 | `IDC_BUTTON_MESSENGER_NOTE_CANCEL` (`0x89200008`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 8 | `IDC_BUTTON_MESSENGER_NOTE_REPLY` (`0x89200009`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 9 | `IDC_BUTTON_MESSENGER_NOTE_SAVE` (`0x8920000A`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 10 | `IDC_TEXT_MESSENGER_NOTE_SENDUSER` (`0x8940000B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 11 | `IDC_TEXT_MESSENGER_NOTE_SENDTIME` (`0x8940000C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 12 | `IDC_TEXT_MESSENGER_NOTE_CHECKMESSAGE` (`0x8940000D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |

### messenger / liveness / DIALOG_MESSENGER_POPUPMENU

Source: block `DIALOG_MESSENGER_POPUPMENU` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_MESSENGER_POPUPMENU` (`0x8A000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_MESSENGER_POPUPMENU` (`0x8A100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_MESSENGER_POPUPMENU01` (`0x8A200002`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 2 | `IDC_BUTTON_MESSENGER_POPUPMENU02` (`0x8A200003`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 3 | `IDC_BUTTON_MESSENGER_POPUPMENU03` (`0x8A200004`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 4 | `IDC_BUTTON_MESSENGER_POPUPMENU04` (`0x8A200005`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 5 | `IDC_BUTTON_MESSENGER_POPUPMENU05` (`0x8A200006`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |

### messenger / liveness / DIALOG_MESSENGER_INFORMATION

Source: block `DIALOG_MESSENGER_INFORMATION` in the pinned ControlList; see the family note above.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_MESSENGER_INFORMATION` (`0x8B000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_MESSENGER_INFORMATION` (`0x8B100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_MESSENGER_INFORMATION_OK` (`0x8B200002`) | button | Messenger sheet control; only the Room one-OK liveness host is pinned (wave-2 `w2-messenger-exe`, #859). | OPEN: REA host routing pin for this sheet then Tier 3 click/key leg (#648/#650/#652) |
| 2 | `IDC_TEXT_MESSENGER_INFORMATION_TITLE` (`0x8B400003`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 3 | `IDC_TEXT_MESSENGER_INFORMATION_OK` (`0x8B400004`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 4 | `IDC_TEXT_MESSENGER_INFORMATION_MESSAGE` (`0x8B400005`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |


### DIALOG_WHISPER

Source: `Dialog_Whisper.bnd` block in the pinned ControlList (lines 1507–1516; 6 entries / 5 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_WHISPER` (`0x29000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_WHISPER` (`0x29100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_TEXT_WHISPER` (`0x29400002`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 2 | `IDC_EDIT_WHISPER` (`0x29300003`) | edit / key-path | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 3 | `IDC_BUTTON_WHISPER_OK` (`0x29200004`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 4 | `IDC_BUTTON_WHISPER_CANCEL` (`0x29200005`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |


### DIALOG_CHANGETITLE

Source: `Dialog_ChangeTitle.bnd` block in the pinned ControlList (lines 1520–1529; 6 entries / 5 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_CHANGETITLE` (`0x2A000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_CHANGETITLE` (`0x2A100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_TEXT_CHANGETITLE` (`0x2A400002`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 2 | `IDC_EDIT_CHANGETITLE` (`0x2A300003`) | edit / key-path | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 3 | `IDC_BUTTON_CHANGETITLE_OK` (`0x2A200004`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 4 | `IDC_BUTTON_CHANGETITLE_CANCEL` (`0x2A200005`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |


### DIALOG_EMOTION

Source: `Dialog_EmotionHelp.bnd` block in the pinned ControlList (lines 1533–1546; 10 entries / 9 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_EMOTION` (`0x2B000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_EMOTION` (`0x2B100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_EMOTION_CLOSE` (`0x2B200002`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 2 | `IDC_BUTTON_EMOTION_LEFT` (`0x2B200003`) | pager | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 3 | `IDC_BUTTON_EMOTION_RIGHT` (`0x2B200004`) | pager | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 4 | `IDC_IMAGE_EMOTION_PAGE01` (`0x2B500005`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 5 | `IDC_IMAGE_EMOTION_PAGE10` (`0x2B500006`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 6 | `IDC_IMAGE_EMOTION_TPAGE01` (`0x2B500007`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 7 | `IDC_IMAGE_EMOTION_TPAGE10` (`0x2B500008`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 8 | `IDC_IMAGE_EMOTION_CONTENTS` (`0x2B500009`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |


### DIALOG_GIFTVIEW

Source: `Dialog_GiftView.bnd` block in the pinned ControlList (lines 1550–1573; 20 entries / 19 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_GIFTVIEW` (`0x2C000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_GIFTVIEW` (`0x2C100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_GIFT_OK` (`0x2C200002`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 2 | `IDC_BUTTON_GIFT_GOGIFTBOX` (`0x2C200003`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 3 | `IDC_BUTTON_GIFT_LEFT` (`0x2C200004`) | pager | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 4 | `IDC_BUTTON_GIFT_RIGHT` (`0x2C200005`) | pager | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 5 | `IDC_TEXT_GIFT_SEND_NICKNAME` (`0x2C400006`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 6 | `IDC_TEXT_GIFT_SEND_DATETIME` (`0x2C400007`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 7 | `IDC_TEXT_GIFT_SEND_TITLE1` (`0x2C400008`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 8 | `IDC_TEXT_GIFT_SEND_TITLE2` (`0x2C400009`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 9 | `IDC_TEXT_GIFT_CONTENTS1` (`0x2C40000A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 10 | `IDC_TEXT_GIFT_CONTENTS2` (`0x2C40000B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 11 | `IDC_TEXT_GIFT_CONTENTS3` (`0x2C40000C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 12 | `IDC_TEXT_GIFT_CONTENTS4` (`0x2C40000D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 13 | `IDC_TEXT_GIFT_CONTENTS5` (`0x2C40000E`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 14 | `IDC_TEXT_GIFT_CONTENTS6` (`0x2C40000F`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 15 | `IDC_IMAGE_GIFT_PAGE01` (`0x2C500010`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 16 | `IDC_IMAGE_GIFT_PAGE10` (`0x2C500011`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 17 | `IDC_IMAGE_GIFT_TPAGE01` (`0x2C500012`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 18 | `IDC_IMAGE_GIFT_TPAGE10` (`0x2C500013`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |


### DIALOG_SKILLIINFO

Source: `Dialog_SkillInfo.bnd` block in the pinned ControlList (lines 1577–1592; 12 entries / 11 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_SKILLINFO` (`0x2D000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_SKILLINFO` (`0x2D100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_SKILLINFO_OK` (`0x2D200002`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 2 | `IDC_TEXT_SKILLINFO_ITEMNAME` (`0x2D400003`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 3 | `IDC_TEXT_SKILLINFO_COUNT` (`0x2D400004`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 4 | `IDC_TEXT_SKILLINFO_SKILLNAME` (`0x2D400005`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 5 | `IDC_IMAGE_SKILLINFO_SKILLIMAGE` (`0x2D500006`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 6 | `IDC_TEXT_SKILLINFO_DESC1` (`0x2D400007`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 7 | `IDC_TEXT_SKILLINFO_DESC2` (`0x2D400008`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 8 | `IDC_TEXT_SKILLINFO_DESC3` (`0x2D400009`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 9 | `IDC_TEXT_SKILLINFO_DESC4` (`0x2D40000A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 10 | `IDC_TEXT_SKILLINFO_DESC5` (`0x2D40000B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |


### DIALOG_CHANGENICKNAME

Source: `Dialog_ChangeNickName.bnd` block in the pinned ControlList (lines 1597–1607; 7 entries / 6 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_CHANGENICKNAME` (`0x2E000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_CHANGENICKNAME` (`0x2E100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_EDIT_CHANGENICKNAME` (`0x2E300002`) | edit / key-path | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 2 | `IDC_TEXT_CHANGENICKNAME` (`0x2E400003`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 3 | `IDC_BUTTON_SEARCHNICKNAME` (`0x2E200004`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 4 | `IDC_BUTTON_CHANGENICKNAME_OK` (`0x2E200005`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 5 | `IDC_BUTTON_CHANGENICKNAME_CANCEL` (`0x2E200006`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |


### DIALOG_MUSICINFO

Source: `Dialog_MusicInfo.bnd` block in the pinned ControlList (lines 1643–1669; 23 entries / 22 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_MUSICINFO` (`0x81000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DIALOG_MUSICINFO` (`0x81100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_MUSICINFO_CANCEL` (`0x81200002`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 2 | `IDC_IMAGE_MUSICINFO_MUSICIMAGE` (`0x81500003`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 3 | `IDC_TEXT_MUSICINFO_TITLE` (`0x81400004`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 4 | `IDC_TEXT_MUSICINFO_ARTIST` (`0x81400005`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 5 | `IDC_TEXT_MUSICINFO_EDITOR` (`0x81400006`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 6 | `IDC_TEXT_MUSICINFO_GANRE` (`0x81400007`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 7 | `IDC_TEXT_MUSICINFO_BPM` (`0x81400008`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 8 | `IDC_TEXT_MUSICINFO_LABEL` (`0x81400009`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 9 | `IDC_TEXT_MUSICINFO_NOTECNT` (`0x8140000A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 10 | `IDC_TEXT_MUSICINFO_LAVEL_E` (`0x8140000B`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 11 | `IDC_TEXT_MUSICINFO_LAVEL_N` (`0x8140000C`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 12 | `IDC_TEXT_MUSICINFO_LAVEL_H` (`0x8140000D`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 13 | `IDC_IMAGE_MUSICINFO_RANKIMAGE` (`0x8150000E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 14 | `IDC_IMAGE_MUSICINFO_CLEAR` (`0x8150000F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 15 | `IDC_IMAGE_MUSICINFO_ALLCOM` (`0x81500010`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 16 | `IDC_IMAGE_MUSICINFO_ALLCOOL` (`0x81500011`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 17 | `IDC_TEXT_MUSICINFO_MAXSCORE` (`0x81400012`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 18 | `IDC_TEXT_MUSICINFO_DATE` (`0x81400013`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 19 | `IDC_BUTTON_MUSICINFO_QUESTION` (`0x81200014`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |
| 20 | `IDC_IMAGE_MUSICINFO_GUIDIMG` (`0x81500015`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 21 | `IDC_TEXT_MUSICINFO_RANKRESULT` (`0x81400016`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |


### DIALOG_EVENT

Source: `Dialog_Event.bnd` block in the pinned ControlList (lines 1701–1709; 4 entries / 3 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_EVENT` (`0x84000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_EVENT` (`0x84100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_IMAGE_EVENT_WEB` (`0x84500002`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 2 | `IDC_BUTTON_EVENT_CLOSE` (`0x84200003`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |


### DIALOG_CONFERENCE

Source: `Dialog_Conference.bnd` block in the pinned ControlList (lines 1713–1720; 4 entries / 3 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_CONFERENCE` (`0x85000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_CONFERENCE` (`0x85100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_IMAGE_CONFERENCE_WEB` (`0x85500002`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 2 | `IDC_BUTTON_CONFERENCE_CLOSE` (`0x85200003`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#316) |


### DIALOG_CHARGECASH

Source: `Dialog_ChargeCash.bnd` block in the pinned ControlList (lines 1689–1697; 4 entries / 3 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_CHARGECASH` (`0x83000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_CHARGECASH` (`0x83100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_IMAGE_CHARGECASH_WEB` (`0x83500002`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 2 | `IDC_BUTTON_CHARGECASH_CLOSE` (`0x83200003`) | button | Sheet input; exe wiring untraced in the pinned docs. | OPEN: REA state handler pin plus Tier 3 click/key leg (#867) |


### DIALOG_CHANNEL_INFO

Source: `Dialog_ChannelInfo.bnd` block in the pinned ControlList (lines 1932–1975; 28 entries / 27 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_CHANNEL_INFO` (`0x8C000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_CHANNEL_INFO` (`0x8C100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_CHANNEL_INFO_CLOSE` (`0x8C200002`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 2 | `IDC_BUTTON_CHANNEL_INFO_EXTEND1` (`0x8C01A003`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 3 | `IDC_BUTTON_CHANNEL_INFO_EXTEND2` (`0x8C01A004`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 4 | `IDC_BUTTON_CHANNEL_INFO_EXTEND3` (`0x8C01A005`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 5 | `IDC_BUTTON_CHANNEL_INFO_EXTEND4` (`0x8C01A006`) | row panel (SET parent) | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 6 | `IDC_IMAGE_CH_BACK` (`0x8C500007`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 7 | `IDC_IMAGE_CH10` (`0x8C500008`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 8 | `IDC_IMAGE_CH01` (`0x8C500009`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 9 | `IDC_IMAGE_CH_CONNECT_COUNT` (`0x8C50000A`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 10 | `IDC_IMAGE_CH_ENTER` (`0x8C50000B`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 11 | `IDC_SCROLL_CHANNEL_INFO` (`0x8C02100C`) | scrollbar | Scroll control at the paired rect; dispatch unpinned | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 12 | `IDC_BUTTON_CHANNEL_SCROLLUP` (`0x8C20000D`) | button | Channel-select bar arrow/thumb/track; arrows step the viewport, highlight untouched (#55). | OPEN: Tier 3 arrow/thumb leg plus REA dispatch pin (#55) |
| 13 | `IDC_BUTTON_CHANNEL_SCROLLDOWN` (`0x8C20000E`) | button | Channel-select bar arrow/thumb/track; arrows step the viewport, highlight untouched (#55). | OPEN: Tier 3 arrow/thumb leg plus REA dispatch pin (#55) |
| 14 | `IDC_BUTTON_CHANNEL_SCROLLBAR` (`0x8C20000F`) | button | Channel-select bar arrow/thumb/track; arrows step the viewport, highlight untouched (#55). | OPEN: Tier 3 arrow/thumb leg plus REA dispatch pin (#55) |
| 15 | `IDC_BUTTON_CHANNEL_SCROLLRANGE` (`0x8C200010`) | button | Channel-select bar arrow/thumb/track; arrows step the viewport, highlight untouched (#55). | OPEN: Tier 3 arrow/thumb leg plus REA dispatch pin (#55) |
| 16 | `IDC_BUTTON_CHANNEL_INFO_HARD` (`0x8C201011`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 17 | `IDC_BUTTON_CHANNEL_INFO_NORMAL` (`0x8C201012`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 18 | `IDC_BUTTON_CHANNEL_INFO_EASY` (`0x8C201013`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 19 | `IDC_BUTTON_CHANNEL_INFO_FREE` (`0x8C201014`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 20 | `IDC_BUTTON_CHANNEL_INFO_DRUM` (`0x8C201015`) | tab / toggle | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |
| 21 | `IDC_TEXT_CHANNEL_INFO_HARD` (`0x8C400016`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 22 | `IDC_TEXT_CHANNEL_INFO_NORMAL` (`0x8C400017`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 23 | `IDC_TEXT_CHANNEL_INFO_EASY` (`0x8C400018`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 24 | `IDC_TEXT_CHANNEL_INFO_FREE` (`0x8C400019`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 25 | `IDC_TEXT_CHANNEL_INFO_DRUM` (`0x8C40001A`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 26 | `IDC_BUTTON_CHANNEL_INFO_EXIT` (`0x8C20001B`) | button | Press behavior unpinned beyond the ControlList entry and paired rect | OPEN: REA dialog-chrome dispatch pin plus Tier 3 click (#26/#55) |


### DAILOG_SELECTMUSIC

Source: `Dialog_SelectMusic.bnd` block in the pinned ControlList (lines 1312–1490; 134 entries / 133 rects). Exe wiring for this family is unpinned; every row carries a named next step.

| Ord | Control id (code) | Kind | Exe behavior pin | Status |
|---:|---|---|---|---|
| -1 | `IDC_DIALOG_SELECTMUSIC` (`0x27000000`) | container | Block self; zero low word, all-zero rect on miss; never placed | OPEN: statics pin of the container code; no input row |
| 0 | `IDC_BACK_DSMUSIC_1` (`0x27100001`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 1 | `IDC_BUTTON_DSMUSIC_OK` (`0x27200002`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 2 | `IDC_BUTTON_DSMUSIC_CANCEL` (`0x27200003`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 3 | `IDC_BUTTON_DSMUSIC_EXTEND1` (`0x27017004`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 4 | `IDC_BUTTON_DSMUSIC_EXTEND2` (`0x27017005`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 5 | `IDC_BUTTON_DSMUSIC_EXTEND3` (`0x27017006`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 6 | `IDC_BUTTON_DSMUSIC_EXTEND4` (`0x27017007`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 7 | `IDC_BUTTON_DSMUSIC_EXTEND5` (`0x27017008`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 8 | `IDC_BUTTON_DSMUSIC_EXTEND6` (`0x27017009`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 9 | `IDC_BUTTON_DSMUSIC_EXTEND7` (`0x2701700A`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 10 | `IDC_BUTTON_DSMUSIC_EXTEND8` (`0x2701700B`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 11 | `IDC_BUTTON_DSMUSIC_EXTEND9` (`0x2701700C`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 12 | `IDC_BUTTON_DSMUSIC_EXTEND10` (`0x2701700D`) | button | Song-select row cell; master row click commits the room song and queues 4000 (#15, `#272`); row note/grade icons stay unpainted. | OPEN: live row-click + 4000 gateway leg (#272/#276); seam click_staged_select_row_commits_song_and_queues_4000 (Tier 0) |
| 13 | `IDC_IMAGE_DSMUSIC_CHECK` (`0x2750000E`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 14 | `IDC_IMAGE_DSMUSIC_REMOVE` (`0x2750000F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 15 | `IDC_IMAGE_DSMUSIC_COLOR` (`0x27500010`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 16 | `IDC_IMAGE_DSMUSIC_IMG_DIFF` (`0x27500011`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 17 | `IDC_TEXT_DSMUSIC_TITLE` (`0x27400012`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 18 | `IDC_TEXT_DSMUSIC_ARTIST` (`0x27400013`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 19 | `IDC_TEXT_DSMUSIC_TXT_DIFF` (`0x27400014`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 20 | `IDC_IMAGE_DSMUSIC_ISDOWN` (`0x27500015`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 21 | `IDC_IMAGE_DSMUSIC_HEAR` (`0x27500016`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 22 | `IDC_IMAGE_DSMUSIC_INFO` (`0x27500017`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 23 | `IDC_IMAGE_DSMUSIC_RANK` (`0x27500018`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 24 | `IDC_IMAGE_DSMUSIC_SELECT_BG` (`0x27500019`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 25 | `IDC_BUTTON_DSMUSIC_MUISCSELECT` (`0x2720001A`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 26 | `IDC_BUTTON_DSMUSIC_RANDOMSELECT` (`0x2720001B`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 27 | `IDC_BUTTON_DSMUSIC_STEP1_ALL` (`0x2720101C`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 28 | `IDC_BUTTON_DSMUSIC_STEP1_EASY` (`0x2720101D`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 29 | `IDC_BUTTON_DSMUSIC_STEP1_INTERMEDIATE` (`0x2720101E`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 30 | `IDC_BUTTON_DSMUSIC_STEP1_HIGH` (`0x2720101F`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 31 | `IDC_BUTTON_DSMUSIC_STEP1_HIGHEST` (`0x27201020`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 32 | `IDC_BUTTON_DSMUSIC_STEP2_ALL` (`0x2720B021`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 33 | `IDC_BUTTON_DSMUSIC_STEP2_GOLD` (`0x2720B022`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 34 | `IDC_BUTTON_DSMUSIC_STEP2_BLACK` (`0x2720B023`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 35 | `IDC_BUTTON_DSMUSIC_STEP2_BLUE` (`0x2720B024`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 36 | `IDC_BUTTON_DSMUSIC_STEP2_RED` (`0x2720B025`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 37 | `IDC_BUTTON_DSMUSIC_STEP2_SILVER` (`0x2720B026`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 38 | `IDC_BUTTON_DSMUSIC_STEP3_ALL` (`0x2720B027`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 39 | `IDC_BUTTON_DSMUSIC_STEP3_BALLAD` (`0x2720B028`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 40 | `IDC_BUTTON_DSMUSIC_STEP3_ROCK` (`0x2720B029`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 41 | `IDC_BUTTON_DSMUSIC_STEP3_DANCE` (`0x2720B02A`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 42 | `IDC_BUTTON_DSMUSIC_STEP3_HIPHOP_SOUL` (`0x2720B02B`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 43 | `IDC_BUTTON_DSMUSIC_STEP3_JAZZ_FUNK` (`0x2720B02C`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 44 | `IDC_BUTTON_DSMUSIC_STEP3_CLASSIC` (`0x2720B02D`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 45 | `IDC_BUTTON_DSMUSIC_STEP3_ETC` (`0x2720B02E`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 46 | `IDC_IMAGE_DSMUSIC_TABRESULT` (`0x2760002F`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 47 | `IDC_IMAGE_DSMUSIC_TABPLAYLIST` (`0x27600030`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 48 | `IDC_IMAGE_DSMUSIC_TABPLAYLIST_BACK` (`0x27500031`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 49 | `IDC_IMAGE_DSMUSIC_TABPLAYLIST_ANI` (`0x27500032`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 50 | `IDC_TEXT_DSMUSIC_TABRESULT` (`0x27400033`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 51 | `IDC_TEXT_DSMUSIC_TABPLAYLIST` (`0x27400034`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 52 | `IDC_BUTTON_DSMUSIC_SERACH_SORT_NEW` (`0x27200035`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 53 | `IDC_BUTTON_DSMUSIC_POPULAR` (`0x27200036`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 54 | `IDC_IMAGE_DSMUSIC_CHECKALL_BG` (`0x27500037`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 55 | `IDC_BUTTON_DSMUSIC_CHECKALL` (`0x27201038`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 56 | `IDC_IMAGE_DSMUSIC_REMOVALL_BG` (`0x27500039`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 57 | `IDC_BUTTON_DSMUSIC_REMOVEALL` (`0x2720003A`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 58 | `IDC_BUTTON_DSMUSIC_COLOR` (`0x2720003B`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 59 | `IDC_BUTTON_DSMUSIC_DIFF` (`0x2720003C`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 60 | `IDC_BUTTON_DSMUSIC_TITLE` (`0x2720003D`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 61 | `IDC_BUTTON_DSMUSIC_ARTIST` (`0x2720003E`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 62 | `IDC_BUTTON_DSMUSIC_DIFF_LEVEL` (`0x2720003F`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 63 | `IDC_BUTTON_DSMUSIC_ISDOWN` (`0x27200040`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 64 | `IDC_BUTTON_DSMUSIC_RANK` (`0x27200041`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 65 | `IDC_TEXT_DSMUSIC_SERACHRESHLT` (`0x27400042`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 66 | `IDC_IMAGE_DSMUSIC_UPDOWN_BG` (`0x27500043`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 67 | `IDC_BUTTON_DSMUSIC_UP` (`0x27200044`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 68 | `IDC_BUTTON_DSMUSIC_DOWN` (`0x27200045`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 69 | `IDC_BUTTON_DSMUSIC_LEFT` (`0x27200046`) | pager | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 70 | `IDC_BUTTON_DSMUSIC_RIGHT` (`0x27200047`) | pager | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 71 | `IDC_TEXT_DSMUSIC_PAGENUM` (`0x27400048`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 72 | `IDC_BUTTON_DSMUSIC_SEARCH_TITLE` (`0x27201049`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 73 | `IDC_BUTTON_DSMUSIC_SEARCH_ARTIST` (`0x2720104A`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 74 | `IDC_EDIT_DSMUSIC_SEARCH` (`0x2730004B`) | edit / key-path | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 75 | `IDC_BUTTON_DSMUSIC_SERACH` (`0x2720004C`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 76 | `IDC_IMAGE_DSMUSIC_RANDOM1_BG` (`0x2750004D`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 77 | `IDC_BUTTON_DSMUSIC_RANDOM1_ALL` (`0x2720004E`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 78 | `IDC_BUTTON_DSMUSIC_RANDOM1_EASY` (`0x2720104F`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 79 | `IDC_BUTTON_DSMUSIC_RANDOM1_INTERMEDIATE` (`0x27201050`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 80 | `IDC_BUTTON_DSMUSIC_RANDOM1_HIGH` (`0x27201051`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 81 | `IDC_BUTTON_DSMUSIC_RANDOM1_HIGHEST` (`0x27201052`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 82 | `IDC_BUTTON_DSMUSIC_LABELQUESTION` (`0x27200053`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 83 | `IDC_IMAGE_DSMUSIC_LABELGUIDIMG` (`0x27500054`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 84 | `IDC_BUTTON_DSMUSIC_SEARCHQUESTION` (`0x27200055`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 85 | `IDC_IMAGE_DSMUSIC_SEARCHGUIDIMG` (`0x27500056`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 86 | `IDC_IMAGE_DSMUSIC_LISTGUIDIMG` (`0x27500057`) | static art | Static figure/icon at the paired rect; frame source unpinned here | OPEN: statics pin plus Tier 3 live-art pin |
| 87 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX0` (`0x27200058`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 88 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX1` (`0x27200059`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 89 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX2` (`0x2720005A`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 90 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX3` (`0x2720005B`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 91 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX4` (`0x2720005C`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 92 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX5` (`0x2720005D`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 93 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX6` (`0x2720005E`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 94 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX7` (`0x2720005F`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 95 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX8` (`0x27200060`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 96 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX9` (`0x27200061`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 97 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX10` (`0x27200062`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 98 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX11` (`0x27200063`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 99 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX12` (`0x27200064`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 100 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX13` (`0x27200065`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 101 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX14` (`0x27200066`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 102 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX15` (`0x27200067`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 103 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX16` (`0x27200068`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 104 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX17` (`0x27200069`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 105 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX18` (`0x2720006A`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 106 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX19` (`0x2720006B`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 107 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX20` (`0x2720006C`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 108 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX21` (`0x2720006D`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 109 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX22` (`0x2720006E`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 110 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX23` (`0x2720006F`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 111 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX24` (`0x27200070`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 112 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX25` (`0x27200071`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 113 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX26` (`0x27200072`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 114 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX27` (`0x27200073`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 115 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX28` (`0x27200074`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 116 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX29` (`0x27200075`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 117 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX30` (`0x27200076`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 118 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX31` (`0x27200077`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 119 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX32` (`0x27200078`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 120 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX33` (`0x27200079`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 121 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX34` (`0x2720007A`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 122 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX35` (`0x2720007B`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 123 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX36` (`0x2720007C`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 124 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX37` (`0x2720007D`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 125 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX38` (`0x2720007E`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 126 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX39` (`0x2720007F`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 127 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX40` (`0x27200080`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 128 | `IDC_BUTTON_DSMUSIC_SERACH_INDEX41` (`0x27200081`) | button | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 129 | `IDC_BUTTON_DSMUSIC_SERACH_NEW` (`0x2720B082`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 130 | `IDC_BUTTON_DSMUSIC_SERACH_SPECIAL1` (`0x2720B083`) | tab / toggle | Song-select sheet chrome; filters/pager/search/OK clicks fall through in the current spec (#15); exe handler unpinned. | OPEN: REA chrome handler pin plus Tier 3 click/frame leg (#15); seam select_row_hit_maps_the_search_table_bands (Tier 0) |
| 131 | `IDC_TEXT_DSMUSIC_SERACH_NEW` (`0x27400084`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |
| 132 | `IDC_TEXT_DSMUSIC_SERACH_SPECIAL1` (`0x27400085`) | static text | Static readout at the paired rect; value source unpinned here | OPEN: statics pin plus Tier 3 live-art text pin |


Input edges with no separate ControlList row:

| Edge | Exe behavior pin | Status |
|---|---|---|
| Dialog OK / Esc ownership | One-OK sheets dismiss on OK; Escape closes options; key leak to chat/background is OPEN (#314). | OPEN: live dialog-ownership leg from Room and Planet; seam dialog_owns_input (Tier 0) |
| Room messenger liveness press | Room `IDC_BUTTON_SROOM_MESSENGER` mounts a one-OK sheet with no wire frame (wave-2 pin). | VERIFIED: live_room_messenger_press_mounts_the_one_ok_liveness_sheet_from_boot (Tier 1 live input leg) |
