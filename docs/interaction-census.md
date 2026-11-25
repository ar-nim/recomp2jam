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
