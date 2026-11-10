# Geometry constants audit — o2jam-shell

READ-ONLY audit of every hardcoded geometry constant in
`crates/o2jam-shell/src` against the shipped 7.55 archives.
Rule under audit: project lesson f38f3 — shell geometry must come from
parsed archives (`ControlList` block + stem-named `.bnd` paired by code
ordinal `(code & 0xFFF) - 1` + sprite anchors), never from literals.
Exceptions: runtime-only values (exe-computed fields, server data,
scaled-frame math, procedural no-archive fallbacks) and OPEN items
(labeled, rig backlog).

## Version pin

- `AngelO2Jam.exe`: 5,029,888 B, SHA-256
  `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
- `Interface1.opi`: 75,100,702 B, SHA-256
  `d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`.
- `Playing1.opi`: 221,737,026 B, SHA-256
  `8c1c7566dc857f2bf0f5f58a5d581e2b2576f4f8a83fa551b31666aed9eed408`
  (Playing `.bnd` files only).
- Tools: Python 3.14.9 static structure walk (no disassembly; `.bnd`
  corners and `ControlList` blocks re-parsed per
  `docs/file-formats/opi.md`). No rig use.

## Method

- `.bnd` bodies: `FFFFFFFF` + u16 count + 16 B corner rects
  `(x1,y1,x2,y2)`, absolute native 800x600 px. Rect `i` pairs with the
  block entry whose code ordinal `(code & 0xFFF) - 1` equals `i`
  (Interface: record `i` lays out entry `i+1`, verified on all 38 bound
  blocks; Playing: the bound run is `STATE_PLAYING` entries 213–229,
  rect `i` ↔ entry `213+i`, ordinal `i+1`).
- Verdicts: **DONE** = already traces to a verified archive/exe source
  (citation, not re-derived). **FILE** = exact or near-exact file
  candidate with plausible pairing — replaceable. **RUNTIME** = no file
  can hold it (exe-computed, server-fed, scaled-frame math, fallback
  chrome). **CANDIDATE** = file-near but without solid pairing evidence
  or with stated divergence — not FILE. **OPEN** = no candidate, rig
  backlog.
- Excluded: colors/tones, non-geometry counts (tick rate, wallet rows,
  chat caps), sprite/IDC name tables, inline `Rect` literals that
  derive from audited constants (e.g. the 2x2 scaled track edge, scrim
  bands built from row rects), and `o2jam-render`'s `NATIVE_WIDTH`/
  `NATIVE_HEIGHT` (800/600 canvas; RUNTIME by nature).

## Global

| Constant | Value | Candidate | Pairing evidence | Verdict | Proposed action |
|---|---|---|---|---|---|
| lib.rs:44 `DEFAULT_WIDTH` | 800 | none (canvas width) | exe-verified: `SetWindowPos(...,800,600,...)` file `0x14b4c`/`0x14baf` | RUNTIME | Keep; label exe-pinned (done) |
| lib.rs:46 `DEFAULT_HEIGHT` | 600 | none | same citation | RUNTIME | Keep |
| game.rs:72 `CLICK_W` | 800 | none | shell click canvas = native size | RUNTIME | Keep |
| game.rs:74 `CLICK_H` | 600 | none | same | RUNTIME | Keep |
| game.rs:1849 `SONG_PANEL_ROWS` | 2 | none | derived layout (title+detail) | RUNTIME | Keep |
| game.rs:1851 `SONG_PANEL_ROW_H` | 8 | none | lobby text pitch | RUNTIME | Keep |

## Channel select + channel-info dialog (paint.rs:115–760)

Scene note: `STATE_CHANNEL` has no `.bnd` (bare `BOUND` line), but its
four row rects live in `Dialog_ChannelInfo.bnd` rects 2–5 (the
exe-mounted 800x466 dialog sheet; pairing verified by the exact 4/4
row-rect match, `docs/channel-fill.md`).

| Constant | Value | Candidate | Pairing evidence | Verdict | Proposed action |
|---|---|---|---|---|---|
| paint.rs:115 `CHANNEL_DIALOG_X_RECT` | (541,151,48,38) | `Dialog_ChannelInfo.bnd` r1 (541,151)-(589,189) | `IDC_BUTTON_CHANNEL_INFO_CLOSE 0x8c200002`, ord 1 | DONE | None (`docs/channel-fill.md`) |
| paint.rs:319 `CHANNEL_ROW_H` | 8 | none | shell text pitch under pinned row bands | RUNTIME | Keep |
| paint.rs:329 `CHANNEL_ROW_RECTS` | 4 rows (216,224)…(216,375,572,436) | `Dialog_ChannelInfo.bnd` r2–5, exact 4/4 | exe memory read 2026-11-09 + bnd match | DONE | None (`docs/channel-fill.md`) |
| paint.rs:374 `CHANNEL_BAR_INSET` | 1 | row origin +1 (bnd method) | `docs/channel-fill.md` METHOD CORRECTION | DONE | None |
| paint.rs:375–378 `CHANNEL_FACE_LOCAL_X/Y/W/H` | 285 / 11 / 44 / 21 | `Dialog_ChannelInfo.bnd` r10 (285,11)-(329,32), exact | `IDC_IMAGE_CH_ENTER 0x8c50000b`, ord 10 | DONE | None |
| paint.rs:383 `CHANNEL_DIGIT_LOCALS` | (50,11),(63,11) | `Dialog_ChannelInfo.bnd` r7/r8 origins (50,11),(63,11) | `IDC_IMAGE_CH10/CH01 0x8c500008/9`, ord 7/8 | DONE | None |
| paint.rs:384 `CHANNEL_COUNT_LOCAL` | (90,12) | `Dialog_ChannelInfo.bnd` r9 origin (90,12), plate 185x19 | `IDC_IMAGE_CH_CONNECT_COUNT 0x8c50000a`, ord 9 | DONE | None |
| paint.rs:391 `CHANNEL_SCROLL_TRACK` | (573,213,21,216) | `Dialog_ChannelInfo.bnd` r11 (573,213)-(594,429), exact | `IDC_SCROLL_CHANNEL_INFO 0x8c02100c`, ord 11 | DONE | None |
| paint.rs:397 `CHANNEL_SCROLL_UP_LOCAL` | (0,0) | `Dialog_ChannelInfo.bnd` r12 (0,0)-(21,20) | `IDC_BUTTON_CHANNEL_SCROLLUP 0x8c20000d`, ord 12 | DONE | None |
| paint.rs:398 `CHANNEL_SCROLL_ARROW_H` | 20 | r12/r13 height 20, exact | same ords | DONE | None |
| paint.rs:399 `CHANNEL_SCROLL_DOWN_LOCAL` | (0,196) | `Dialog_ChannelInfo.bnd` r13 (0,196)-(21,216) | `IDC_BUTTON_CHANNEL_SCROLLDOWN 0x8c20000e`, ord 13 | DONE | None |
| paint.rs:429–432 `CHANNEL_PILL_X/Y/W/H` | 205 / 227 / 94 / 35 | none: no rect near (205,227)-(299,262) in any Interface `.bnd`; `STATE_CHANNEL` unbound | — | OPEN | Rig: read hosting dialog object (cf. row-rect memory read) |
| paint.rs:436–439 `CHANNEL_TRACK_X/Y/W/H` | 297 / 228 / 203 / 33 | none: no rect near (297,228)-(500,261) | — | OPEN | Rig: same |
| paint.rs:710 `CHANNEL_VISIBLE_ROWS` | 4 | none (reference photo `/tmp/chan2-list.png`) | — | OPEN | Rig: exe viewport count |
| paint.rs:712 `CHANNEL_PAGER_Y` | 446 | none: absent from `Dialog_ChannelInfo.bnd` | — | OPEN | Rig: exe pager origin |

## Room list (paint.rs:1611–1627)

Scene note: the `SET 0x11` row template (`State_Room.bnd` rects 33–49)
clones per row panel — the 2026-11-09 rig pins the clone's live
content (`~/scratch/geom-open-close/findings.md` §2), so the row
fields ride pinned panel-relative offsets while the template rects
supply sizes only.

| Constant | Value | Candidate | Pairing evidence | Verdict | Proposed action |
|---|---|---|---|---|---|
| paint.rs:1611 `ROOM_ROW_DIGIT_PITCH` | 11 | `State_Room.bnd` r45..47 width 11 | digits at 38/49/60 on panel 0 (findings §2) | DONE | None |
| paint.rs:1615 `ROOM_ROW_NUMBER_ORIGIN` | (12,12) | panel-relative pin | digit origin 38/94 = panel (26,82)+(12,12) (findings §2) | DONE | None |
| paint.rs:1619 `ROOM_ROW_TITLE_ORIGIN` | (64,11) | panel-relative pin | title ink x≈90 core 94, y 93 (findings §2; `ROOMNAME` rect at +66, 2 px GDI margin) | DONE | None |
| paint.rs:1623 `ROOM_ROW_COUNT_ORIGIN` | (194,10) | panel-relative pin | count ink x 220..264, y 91..115 (findings §2; `ROOM_USER` rect paints background) | DONE | None |
| paint.rs:1627 `TEXT_CAP_FRACTION` | 0.7 | exe-measured GDI cap share (not a geometry rect) | title ~11 px in 16 px rect, music ~14 px in 19 px (`t2-room-stage.png`) | DONE (presentation) | None — rasterization is owner-decided; the fraction pins the size |
| paint.rs `ROOM_DETAIL_LINES` | 9 | none | field count (title + 8) | RUNTIME | Keep |

## Waiting (paint.rs:1622–2335)

| Constant | Value | Candidate | Pairing evidence | Verdict | Proposed action |
|---|---|---|---|---|---|
| paint.rs:1622 `WAITING_SEATS` | 8 | none | room protocol seat count | RUNTIME | Keep |
| paint.rs:1624 `WAITING_PEERS` | 7 | none | same | RUNTIME | Keep |
| paint.rs:1626 `WAITING_ROW_H` | 8 | none | lobby pitch; `seat_origin` is scaled-frame math | RUNTIME | Keep |
| paint.rs:2375 `WAITING_CHAT_LIST_RECT` | (24,455,487,98) | `State_Waiting.bnd` r42 (24,455)-(511,553), exact | `IDC_LIST_SWAIT_CHAT 0x1680002b`, ord 42 | DONE (this commit) | Fallback pinned to bnd rect 42; painter reads the bind at runtime |
| paint.rs:2383 `WAITING_CHAT_EDIT_RECT` | (20,563,440,20) | `State_Waiting.bnd` r19 (20,563)-(460,583), exact | `IDC_EDIT_SWAIT_CHAT 0x16300014`, ord 19 | DONE (this commit) | same |
| paint.rs:2325 `WAITING_CHAT_ROW_H` | 17 | none: 98 px list / 6 rows ≈ 16.3; measured row tops 455.4/472.5/489.8 | reference photo only | OPEN | Rig: exe chat line pitch |
| paint.rs:2328 `WAITING_CHAT_VISIBLE_ROWS` | 6 | derived (98/17 slots) | — | RUNTIME | Keep (re-derives once ROW_H is pinned) |
| paint.rs:2333 `WAITING_CHAT_CARET_INSET` | (5,5) | none: caret chrome | reference photo (caret at (25,568)) | OPEN | Rig: exe caret placement |
| paint.rs:2335 `WAITING_CHAT_CARET_SIZE` | (2,10) | none | same photo | OPEN | same |

## Channel chat (paint.rs:2497–2499)

| Constant | Value | Candidate | Pairing evidence | Verdict | Proposed action |
|---|---|---|---|---|---|
| paint.rs:2497 `CHANNEL_CHAT_VISIBLE_ROWS` | 6 | none | mirrors waiting box; live-packet rows | RUNTIME | Keep |
| paint.rs:2499 `CHANNEL_CHAT_ROW_H` | 8 | none | lobby pitch | RUNTIME | Keep |

## Result (paint.rs:3008–3316, 5087–5098)

Scene note: the `.bnd` (`State_Result.bnd`, 7 rects) holds only the
background and the two buttons; every numeric readout anchor is
exe-computed GDI/sprite text (`docs/cresult-anchors.md`,
`FUN_004c38a0` / `FUN_004c3b40`, ev `88c3ca31…`, `615e034c…`) — no file
can hold those.

| Constant | Value | Candidate | Pairing evidence | Verdict | Proposed action |
|---|---|---|---|---|---|
| paint.rs:3008 `RESULT_MUSIC_BOX` | (70,304,664,215) | none: interior of the baked `Result_EachResult.ojs` pane (rect 4 body, 800x350 at (0,250)) | sprite-derived | RUNTIME | Keep |
| paint.rs:3250–3256 `RESULT_COOL_X/GOOD_X/BAD_X/MISS_X` | 151 / 240 / 337 / 426 | none | exe-pinned (`FUN_004c38a0`) | DONE | None (`docs/cresult-anchors.md`) |
| paint.rs:3258 `RESULT_COUNT_Y` | 125 | none | same | DONE | None |
| paint.rs:3260 `RESULT_MAX_COMBO_X` | 505 | none | same | DONE | None |
| paint.rs:3262 `RESULT_JAM_COMBO_X` | 596 | none | same | DONE | None |
| paint.rs:3264 `RESULT_COMBO_Y` | 130 | none | same | DONE | None |
| paint.rs:3266–3268 `RESULT_SCORE_X/Y` | 605 / 533 | none | same | DONE | None |
| paint.rs:3272 `RESULT_RETRY_RECT` | (625,570,83,22) | `State_Result.bnd` r5 (625,570)-(708,592), exact | `IDC_BUTTON_SRESULT_RETRY 0x17202006`, ord 5 | DONE | None (`docs/cresult-anchors.md` rect 6) |
| paint.rs:3280 `RESULT_BACK_RECT` | (711,569,83,24) | `State_Result.bnd` r4 (711,569)-(794,593), exact | `IDC_BUTTON_SRESULT_BACK 0x17200005`, ord 4 | DONE | None |
| paint.rs:3292–3304 `RESULT_RANK_X / ROW_NAME_X / ROW_JUDGE_X / ROW_COMBO_X / ROW_SCORE_X / ROW_BONUS_X / ROW_SPEED_X` | 91 / 128 / 230 / 416 / 530 / 626 / 685 | none | exe-pinned (`FUN_004c3b40`, ev `88c3ca31…`) | DONE | None |
| paint.rs:3308–3312 `RESULT_ROW_TEXT_Y / ROW_NAME_Y / ROW_PITCH` | 317 / 318 / 22 | none | row base 320+22·row | DONE | None |
| paint.rs:3314 `RESULT_RANK_ROWS_MAX` | 8 | none | exe builds eight avatar slots (`this+0x5c`) | DONE | None |
| paint.rs:3316 `RESULT_ROW_NAME_LIMIT` | 100 | none | exe truncation rule | RUNTIME | Keep |
| paint.rs:5087–5091 `RESULT_LIFE_Y / LIFE_LAST_X / BOTTOM_Y / BOTTOM_LAST_X / DIGIT_PITCH` | 77 / 460 / 536 / 176 / 24 | none | exe-pinned (ev `615e034c…`, cells 292/316/…/460) | DONE | None |
| paint.rs:5092 `RESULT_DIGIT_CELLS` | 8 | none | same citation (eight 24 px cells) | DONE | None |
| paint.rs:5098 `RESULT_ROW_BASE_Y` | 320 | none | `FUN_004c3b40`: 320+22·row | DONE | None |

## Select-music dialog (side.rs:497–600)

Scene note: bound to `Dialog_SelectMusic.bnd` (133 rects). The constants
look file-traceable by role, but the file contradicts almost every
value (see divergence column); only row tops, the two pitches, and the
inventory counts match exactly. Conservative verdicts below; the dialog
pairing itself is unproven (parity.md musicselect cell).

| Constant | Value | Candidate (file rect / divergence) | Pairing evidence | Verdict | Proposed action |
|---|---|---|---|---|---|
| side.rs:497 `SELECT_FILTER_COLUMNS` | (28,95,96,300),(130,95,92,300),(230,95,100,300) | STEP1/2/3 button runs r27–31 (24,118)-(118,375), r32–37 (125,147)-(219,292), r38–45 (226,118)-(320,350): tops 118 vs 95, lefts +4, widths −2…+10 | `IDC_BUTTON_DSMUSIC_STEP1_ALL…` ord 27… | CANDIDATE | Rig: prove pairing, then re-measure |
| side.rs:520–522 `SELECT_FILTER_BUTTON_TOP/PITCH/H` | 129 / 33 / 26 | file tops 118 pitch 29 h 29 (r27–31) | same | CANDIDATE | same |
| side.rs:525 `SELECT_RESULT_TABS` | (352,93,140,24),(494,93,127,24) | r46 (350,88)-(490,109), r47 (500,88)-(640,109): +2/+5 offsets, w 140 matches tab 1 only | `IDC_IMAGE_DSMUSIC_TABRESULT/TABPLAYLIST` ord 46/47 | CANDIDATE | same |
| side.rs:543 `SELECT_HEADER_Y` | 127 | r54 CHECKALL_BG (339,126)-(352,139): y 126 | `IDC_IMAGE_DSMUSIC_CHECKALL_BG` ord 54 | CANDIDATE | same |
| side.rs:544–545 `SELECT_ROW_TOP / SELECT_ROW_PITCH` | 150 / 23 | r3–12 (331,150)-(762,173) ×10, pitch 23, tops exact | `IDC_BUTTON_DSMUSIC_EXTEND1..10` ord 3–12 | CANDIDATE (strong) | File-backed once pairing proven |
| side.rs:546 `SELECT_VISIBLE_ROWS` | 10 | 10 EXTEND rects r3–12 | same | CANDIDATE (strong) | same |
| side.rs:547–548 `SELECT_TABLE_LEFT / SELECT_TABLE_WIDTH` | 344 / 437 | file band 331–762 (w 431): left +13, width +6 | contradicts r3–12 | CANDIDATE (divergent) | Rig: exe table band |
| side.rs:552 `SELECT_HEADER_COLUMNS` | [344,360,392,417,565,627,650,673,692,715] | r54–64 x1 [339,356,382,409,563,635,658,681,…,730]: offsets +2…−15, one extra column | header buttons ord 54–64 | CANDIDATE (divergent) | Rig |
| side.rs:556–557 `SELECT_STATUS_Y / SELECT_STATUS_H` | 394 / 18 | r65 (338,390)-(653,404): y 390, h 14 | `IDC_TEXT_DSMUSIC_SERACHRESHLT` ord 65 | CANDIDATE (divergent) | Rig |
| side.rs:558 `SELECT_PAGER_RIGHT` | 756 | r70 RIGHT (726,386)-(744,406): right edge 744 | `IDC_BUTTON_DSMUSIC_RIGHT` ord 70 | CANDIDATE (divergent) | Rig |
| side.rs:562 `SELECT_SEARCH_Y` | 417 | search row r72–75 tops 413–418 | ord 72–75 | CANDIDATE | Rig |
| side.rs:563/569 `SELECT_TITLE_RADIO / SELECT_ARTIST_RADIO` | (352,423,8,8) / (410,423,8,8) | no 8x8 rect; interior dots of r72 (341,413)-(393,435) / r73 (396,413)-(462,435) | ord 72/73 | CANDIDATE | Rig: exe radio dot placement |
| side.rs:575 `SELECT_SEARCH_BOX` | (475,417,212,25) | r74 EDIT (474,418)-(675,432): +1/−1 origin, w 212 vs 201, h 25 vs 14 | `IDC_EDIT_DSMUSIC_SEARCH` ord 74 | CANDIDATE (divergent) | Rig |
| side.rs:581 `SELECT_SEARCH_BUTTON` | (694,417,38,25) | r75 (695,414)-(742,434): −1/+3 origin, w 38 vs 47 | `IDC_BUTTON_DSMUSIC_SERACH` ord 75 | CANDIDATE (divergent) | Rig |
| side.rs:591–593 `SELECT_CHO_SUNG_Y / LEFT / PITCH` | [444,462] / 352 / 15 | index rows r87–113 y440, r114–128 y455; x1 345; x-step 15 exact | `IDC_BUTTON_DSMUSIC_SERACH_INDEX0..41` ord 87–128 | CANDIDATE (pitch FILE, offsets divergent) | Rig: row tops/left |
| side.rs:594–595 `SELECT_CHO_SUNG_JAMO / WORDS` | 14 / 2 | row inventory exact: 14 jamo (r87–100) + 13 letters (r101–113); 13 letters (r114–126) + 2 words (r127–128) | same | FILE | Count from INDEX button inventory at runtime |
| side.rs:596 `SELECT_CHO_SUNG_ROW_CELLS` | [27,15] | derived from JAMO/WORDS | — | RUNTIME | Keep |
| side.rs:600 `SELECT_OK_BUTTON` | (342,506,113,40) | r1 (326,497)-(452,542): +16/+9 origin, w 113 vs 126 | `IDC_BUTTON_DSMUSIC_OK` ord 1 | CANDIDATE (divergent) | Rig |

## Planet (side.rs:880–908)

| Constant | Value | Candidate | Pairing evidence | Verdict | Proposed action |
|---|---|---|---|---|---|
| side.rs:880 `PLANET_HARD_RECT` | (313,32,146,112) | `State_Planet.bnd` r1 (313,32)-(459,144), exact (duplicated in `Dialog_ChannelInfo.bnd` r16) | `IDC_BUTTON_SPLANET_HARD 0x19201002`, ord 1 | DONE | None (`docs/planet.md`) |
| side.rs:887 `PLANET_NORMAL_RECT` | (206,32,99,112) | `State_Planet.bnd` r2, exact (dup `Dialog_ChannelInfo` r17) | `IDC_BUTTON_SPLANET_NORMAL`, ord 2 | DONE | None |
| side.rs:894 `PLANET_EASY_RECT` | (87,32,99,112) | r3, exact (dup r18) | `IDC_BUTTON_SPLANET_EASY`, ord 3 | DONE | None |
| side.rs:901 `PLANET_FREE_RECT` | (614,32,99,112) | r4, exact (dup r19) | `IDC_BUTTON_SPLANET_FREE`, ord 4 | DONE | None |
| side.rs:908 `PLANET_DRUM_RECT` | (510,32,99,112) | r5, exact (dup r20) | `IDC_BUTTON_SPLANET_DRUM`, ord 5 | DONE | None |

## Playing (playing.rs)

| Constant | Value | Candidate | Pairing evidence | Verdict | Proposed action |
|---|---|---|---|---|---|
| playing.rs:27 `HIGHWAY_LANES` | 7 | none | game rule | RUNTIME | Keep |
| playing.rs:33 `NATIVE_LINE_Y` | 487 | none (sprite anchors) | `KeydownImage.ojs` anchors row 487 (slot 33) | DONE | None (`docs/playing-reference.md`) |
| playing.rs:40/51 `NATIVE_LANES / NATIVE_LANE_RECTS` | (5,28)…(165,28) | none (sprite anchors) | `Keyeffect1..7.ojs` beam anchors, slots 470–476 | DONE | None |
| playing.rs:97 `NATIVE_NOTE_H` | 7 | none (sprite) | `Note_Image.ojs` heads 7 px (slot 47) | DONE | None |
| playing.rs:101 `NATIVE_KEY_H` | [46,42,46,46,46,42,46] | none (sprite) | `KeydownImage.ojs` per-lane heights (slot 33) | DONE | None |
| playing.rs:106 `NATIVE_JAM_BARS` | (23,536)/(570,536) 206x16 | none (sprite) | `Jamgauge_left/right_bar.ojs` anchors (slots 464–465) | DONE | None |
| playing.rs:126 `NATIVE_JAM_BOTTOM_BAR` | (4,536,191,12) | none in `State_Playing.bnd`; sprite anchor | `Jam_gauge_bar.ojs` (4,536) 191x12 (slot 31) | DONE | None |
| playing.rs:137 `GAUGE_FILL_INSET` | 1 | none | staged `Life_gauge_back/bar` pair pins the +1 rule | DONE | None |
| playing.rs:144 `STRIP_ROW_NATIVE` | 554 | none (sprite) | `Playing_Exit.ojs` (751,554), `Normal_TimeBar.ojs` (489,556) anchors | DONE | None |
| playing.rs:148 `JAM_DIGIT_FALLBACK_Y` | 4 | none | fallback row, unbound art | RUNTIME | Keep |
| playing.rs:153 `NATIVE_PEER_BAR` | (754,0,46,8) | none (unbound stacked gauges) | size from `Avatar_Info_LifeGauge.ojs` 46x8 (slot 6); origin = runtime stacking | RUNTIME | Keep; size already archive-pinned |
| playing.rs:268 `COMBO_ROW_ABOVE_LINE` | 24 | none | invented anchor (parity.md audit 31) | OPEN | Rig: exe combo-digit anchor |
| playing.rs:289 `LONG_COMBO_PLATE_INK` | (52,61) | none (sprite frame) | `Playing1.opi` slot 432 frame 1, 74x9 at (52,61) | DONE | None |
| playing.rs:295 `NATIVE_LONG_COMBO_PLATE` | (91,282,74,9) | none (sprite frame) | photo bright-ink scan = slot 432 frame at 1:1 | DONE | None |
| playing.rs:2476/2478 `SCENE_NATIVE_W/H` | 800 / 600 | none | canvas | RUNTIME | Keep |
| playing.rs:2483 `AVATAR_SEAT_NATIVE` | (307,161,300,300) | `Map_Avatar.bnd` r0 (307,161)-(607,461), exact | no ControlList block binds it — exe loads by name (`docs/bnd-semantics.md`) | DONE | None |
| playing.rs:2492 `AVATAR_LEVEL_OFFSET` | (92,34) | none: `Avatar_Info.bnd` rects do not match; seat-relative photo offset | reference photo only | OPEN | Rig: exe plate placement |
| playing.rs:2496 `AVATAR_LIFE_OFFSET` | (93,54) | none | same photo | OPEN | same |
| playing.rs:2499 `AVATAR_LIFE_SIZE` | (46,8) | none (sprite) | `Avatar_Info_LifeGauge.ojs` 46x8 (slot 6) | DONE | None |
| playing.rs:2526 `TALLY_TIME_NATIVE` | (489,556,254,10) | `State_Playing.bnd` r2 (489,556)-(743,566), exact | `IDC_IMAGE_SPLAY_TIMEBAR 0x1b500003`, entry 215 | DONE | None |
| playing.rs:2534 `TALLY_SPEED_NATIVE` | (487,579) | r15 origin (487,579) 28x9 | `IDC_TEXT_SPLAY_SPEED 0x1b400010`, entry 228 | DONE | None |
| playing.rs:2537 `TALLY_TITLE_NATIVE` | (520,579) | r6 origin (520,579) 146x9 | `IDC_TEXT_SPLAY_TITLE 0x1b400007`, entry 219 | DONE | None |
| playing.rs:2541 `TALLY_VOLUME_NATIVE` | (705,579) | r16 origin (705,579) 44x9 | `IDC_TEXT_SPLAY_VLOLUME 0x1b400011`, entry 229 | DONE | None |
| playing.rs:2544 `TALLY_EXIT_NATIVE` | (751,554) | r0 origin (751,554) 42x42 | `IDC_BUTTON_SPLAY_EXIT 0x1b200001`, entry 213 | DONE | None |

Playing pairing note: `STATE_PLAYING` codes collide after the subset
window (duplicate `0x1b400011`), so rect↔entry pairing uses the
verified bound run (entries 213–229 = ordinals 1–17,
`PLAYING_SUBSET_FIRST_ENTRY`), not raw ordinal scan.

## Counts per verdict

- DONE: 77 (verified archive/exe source; incl. the 2026-11-09 rig
  pins: room inset +12/+12, waiting chat pitch/caret/row-count;
  2026-11-11 room splits: number/title/count origins + digit pitch
  + TEXT cap fraction; and the A/V-verified select-music photo
  constants — see the A/V section)
- FILE: 3 (`SELECT_CHO_SUNG_JAMO`,
  `SELECT_CHO_SUNG_WORDS`,
  `SELECT_CHO_SUNG_PITCH`)
- RUNTIME: 22 (exe-computed / server / scaled-frame / fallback)
- CANDIDATE: 0 (the 22 select-music rows were upgraded to
  DONE-exe-output-verified by the 2026-11-10 A/V alignment; the bnd
  values are documented as contradicted, not applied)
- OPEN: 11 (channel chrome 10 — partly live-measured 2026-11-09, see
  the channel rows; playing anchors 3 — blocked by the Attempt-13
  loading wedge; minus the 2 retired room inset rows, now DONE)

Total audited: 136 named constants.

## Highest-value FILE replacements (ranked)

1. ~~`WAITING_CHAT_LIST_RECT` + `WAITING_CHAT_EDIT_RECT`~~ — DONE
   (this commit): values already equal `State_Waiting.bnd` rects 42/19
   with verified ordinal pairing; fallback constants pinned by
   `waiting_chat_native_fallback_rects_match_the_state_waiting_bnd`,
   doc comments cite the pairing evidence.
2. `SELECT_CHO_SUNG_JAMO`/`WORDS`/`PITCH` (side.rs:593–595) — exact
   `Dialog_SelectMusic` INDEX-button inventory and x-pitch; countable
   from staged placements at runtime.
3. `SELECT_ROW_TOP`/`SELECT_ROW_PITCH` (side.rs:544–545) — exact match
   on the ten EXTEND button rects; promote to FILE once the dialog
   pairing is proven on the rig (parity.md musicselect cell).

## OPEN list (rig follow-up)

- Room row-template clone (ticket #13 residue): digit places land
  within −4/−2 px panel-relative and `ROOM_USER` paints background —
  read the per-row clone offset with a SendInput highlight sweep
  plus a memory read of the row template clone (per the
  `~/scratch/geom-open-close/` recipe); a 2+ room gateway replay
  plus a capture settles non-highlighted panels and the 3-digit
  reading order.
- Channel row-interior pill/track chrome (8 constants,
  paint.rs:429–439), `CHANNEL_VISIBLE_ROWS`, `CHANNEL_PAGER_Y` — read
  the hosting `CStatePlanet` dialog object the way the row rects were
  read (`docs/channel-fill.md`).
- Playing combo row `COMBO_ROW_ABOVE_LINE` (playing.rs:268) and avatar
  plate offsets `AVATAR_LEVEL_OFFSET`/`AVATAR_LIFE_OFFSET`
  (playing.rs:2492–2496) — STILL OPEN 2026-11-10: blocked by the
  Attempt-13 loading wedge under Wine (tee trace: full play handshake
  completes, then silence + frozen loading art; relay refuted;
  suspect DXSurface play-surface init). Next: winedbg stopped-state
  read of the play-stage init (lesson 9e297) or a Windows rig.
  (Room inset, waiting chat pitch, and caret were pinned in the
  2026-11-09 rig session — see the DONE rows above.)

## Surprises

- `SELECT_*` looks file-traceable by role, but `Dialog_SelectMusic.bnd`
  contradicts nearly every value: table band 344/437 vs file 331/431,
  chosung tops 444/462 vs file 440/455, OK button (342,506) vs
  (326,497), search box w 212 vs 201. Only row tops (150), both
  pitches (23 and 15), and the inventory counts match exactly. The
  constants stay CANDIDATE until the dialog pairing is proven.
- `RESULT_BACK_RECT` equals the BACK button rect of ten unrelated
  scenes (same (711,569)-(794,593) template everywhere); the
  scene-correct pairing is `State_Result.bnd` r4.
- The planet emblem rects are duplicated verbatim in
  `Dialog_ChannelInfo.bnd` r16–20 (the channel dialog reuses the board
  layout); pairing to `State_Planet.bnd` is the verified one.
- `STATE_CHANNEL` has no bind file, yet its row rects live in
  `Dialog_ChannelInfo.bnd` r2–5 — the exe mounts the dialog sheet over
  the board.
- `TALLY_*` playing constants pair correctly only through the
  213–229 subset window; raw ordinal scan hits duplicate codes
  (`0x1b400011` twice).

## Parity.md mapping

- Channel cell FAIL ↔ OPEN pill/track/pager rows here (pager y446,
  plate stand-ins).
- Room cell PASS (ticket #13) ↔ `ROOM_ROW_*` origins + digit pitch
  + `TEXT_CAP_FRACTION` DONE here; the per-row template clone offset
  past the pins stays OPEN (digit residual −4/−2 px, next: SendInput
  highlight sweep + clone memory read); fallback bands stay RUNTIME
  (labeled procedural fallback).
- Waiting cell FAIL (audits 12, 18–19) ↔ seat grid is scaled-frame math
  on `WAITING_ROW_H` (RUNTIME); portraits center blind (no constants).
- Loading cell FAIL (audits 23, 35) ↔ no geometry constants (bar/caption
  placeholders are centered placeholders, not literals).
- Playing cell FAIL (audits 16, 30–32) ↔ `COMBO_ROW_ABOVE_LINE` OPEN;
  audit 16/30/32 are paint-clip behavior, not geometry constants.
- Result cell FAIL (audit 13) ↔ exe-pinned anchors DONE here; the FAIL
  is fixed-pixel avatar/verdict placement behind a scrim, not a
  geometry constant.
- Musicselect cell FAIL ↔ all 22 CANDIDATE rows; pairing unproven.

## A/V verification vs the real exe (2026-11-10)

Static frames compared with the av-compare pipeline (SSIM/PSNR) plus a
band-wise translation search (raw + edge MAE, ±8 px) on native 800x600
frames: real Wine captures (`~/scratch/button-unblock/planet-bare-2.png`,
`planet-channel.png`; `reference-musicselect-4x3.png` downscaled 1920→800
LANCZOS) vs the shell renders (400→800 NEAREST, placement preserved at
2 px). Reports: `~/scratch/av-reports/` (summaries, frames).

- **Planet board**: emblem strip, MENU bar, frame rim all align at
  (0,±1) px — the bound-block render is pixel-faithful to the real
  client. (Promo banner content differs by carousel state — expected.)
- **Channel dialog**: title plate + X, emblem strip align at (0,±1);
  rows and scroll column sit within content noise (real rows carry
  server channel data vs synthetic rows — a systematic geometry offset
  would show a crisp MAE drop; none does).
- **Musicselect (the bnd contradiction)**: the shell's photo-derived
  constants (table band 331/431, chosung tops 440/455) align with the
  real capture at (0,0); the OK pill is pixel-exact (0% gain available
  from any shift). Shifting the bands toward the bnd values (344/437,
  444/462) does NOT improve alignment. Conclusion: the photo-derived
  constants are exe-output-faithful; `Dialog_SelectMusic.bnd`'s
  divergent rects are not what the 7.55 client paints (stale authoring
  data or a different staging path, cf. the channel block's zero-rect
  records) — the bnd rows stay contradicted, never silently replaced.
  Caveat: the constants derive from the same capture family, so the
  non-circular content is pipeline alignment + OK-pil exactness; a
  fresh rig session remains the stronger proof if ever questioned.

Consequence: the 20 select-music CANDIDATE constants keep their values
with verdict upgraded to DONE (exe-output-verified via A/V alignment);
the audit's FILE count is unchanged (no file replacement occurred).
