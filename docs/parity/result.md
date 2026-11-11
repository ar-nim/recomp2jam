## `STATE_RESULT` — results (`CStateResult`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | (ev: docs/cresult-anchors.md, docs/result-variants.md) Decl `0x07`, 8 entries: `Result_WinLose.ojs`, `Result_NumFont_*.ojs`, Top/Bottom, Retry; builder `FUN_004c2390` (file `0xc2390`, ev `d6de3a7f…`) | Soak stages genuine panels — `result.png` shows real COOL/GOOD/BAD + music-name art with digits on top. Re-shot 2026-11-07 post-`04e2462`: the rank table now paints row 0 (`YOU`, `1/0/0/0`, `200`) at the pinned columns (guard: `result_shot_paints_the_rank_table_rows`). Ticket #17 (2026-11-11): digits paint at the exe GDI anchors (top-panel `FUN_004c38a0` columns, `docs/cresult-anchors.md` `ev_615e034c…`), the verdict banner paints the exe-selected `Result_WinLose.ojs` frame (`winlose_frame`, rule 1/2 of `docs/result-variants.md`), the grade plate paints the pinned threshold frame at (395,531) (`grade_frame`, rule 6), and the local avatar hangs its visible box at the row-0 slot (78,316) under the 652×22 band clip (row painter `FUN_004c3b40`, `ev_88c3ca31…`) — no fixed-pixel anchors remain. Guards: `result_avatar_paints_at_the_local_row_slot`, `result_avatar_clips_to_the_exe_row_band`, `result_avatar_hangs_its_visible_box_at_the_slot`, `result_paints_loaded_avatar_at_the_row_slot`; smoke `docs/smoke/result-hand-session.md`; shots oracle holds with only the four intended `result*.png` deltas (avatar leaves frame center for the row slot). All 7 `State_Result.bnd` rects are verified (S4) and cover only panels/buttons/grade plate (number anchors stay exe-pinned GDI — negative evidence: no rects exist for them); `Winner_image` is decl-unloaded network-variant art, never on the result scene (test-pinned out of `RESULT_FRAME_BODIES`). Still OPEN with named next steps: peer-row portraits — convention PINNED by #85 (2026-11-12, claim D21 resolved, docs/cresult-anchors.md "Peer-row portraits"): member record +0x31 (first 4043 seat byte) is a 1-based unguarded index into eight constant image-less slot objects at result this+0x5c (mode-selected u16 table, display mode 0xf/0x10); no member-equipment→figure join exists and the byte doubles as the FUN_004c49a0 team bucket; remaining arm = what the slot object renders + repainting the shell's exe-contradicting worn-figure row-0 avatar (follow-up #117), life-readout and bottom-counter digit cells (pinned geometry, font-body binding inference — next step: REA trace of the digit-writer callers at files `0xc44c0`/`0xc4630`), grade letters (zero `.text` refs, `.ojs`-driven — next step: live rig capture of the plate at controlled score bands). Scoping #71 (2026-11-12): every OPEN arm carries an execution issue — peer-row portraits #85 (evidence, claim D21) + #117 (shell), digit cells #86, grade letters ride the grade issues #78 (evidence) and #79 (shell) | PASS (R+G) (v: 9692d41) |

| Input | (ev: file `0x24a338`, relay `0xfaa`/`0xfb5`/`0xfca`) Retry → waiting loading; leave → room/channel; event-ticket web call (file `0x24a338`) | RETRY click hands back to Waiting, BACK click leaves for Room (Confirm/Back mirror the buttons; miss clicks land nowhere). Guards: `staged_retry_click_returns_to_waiting`, `staged_back_click_returns_to_room`, `result_keys_mirror_the_staged_buttons`. Wire shapes named by the scripted-gateway replay (#87, guard `result_wire_replay_names_the_retry_back_and_battle_entry_shapes`): retry = `StartGame` 4010 (`0xfaa`) empty frame `04 00 AA 0F` (live-observed empty from the exe, `docs/wine-boot.md`); back = `ExitPlaying` 4021 (`0xfb5`) empty frame `04 00 B5 0F` — the frame the result-entry drain already emits today; battle entry = `GetLiveState` 4042 (`0xfca`) empty frame `04 00 CA 0F` (byte-verified: sole `0x0FCA` push-immediate at file `0xc365c`, gated on the battle global `0x7789e6`, `docs/relay.md` §10). The solo enter painter's event-ticket web call is an HTTP GET to `o2jam.nopp.co.kr/…/ticket_add.php?USER_INDEX_ID=<index>` (file `0x24a338`) — not a gateway frame, so the scripted gateway cannot capture it and the shell owns no web seam. Wired by #88: RETRY/BACK presses (button and keyboard arms) and battle result entry queue the pinned empty bodies through the session bridge (`OnlineSession::flush_result_wire`; guards `result_retry_and_back_queue_the_pinned_relay_bodies`, `result_entry_stages_the_pinned_ticket_wire_call`, replay reads the retry press's `StartGame` and the back press's second `ExitPlaying`). Retry-while-siblings-ready stays the recorded server-side rule (block string VA `0x64a3a0`, `docs/cresult-anchors.md`), never reimplemented | OPEN (R+G) (v: 0112ae9) (gateway arms wired by #88 — the pinned empty bodies queue through the session bridge; still OPEN with named next steps: the event-ticket web call rides a web-seam decision, then a live request-line capture of `ticket_add.php?USER_INDEX_ID=<index>` and its `USER_INDEX_ID` value source; retry-while-siblings-ready gating observation on the live relay — the block message string VA `0x64a3a0` is pinned, the server rule is recorded, never reimplemented) |
| Timing | None documented | Paint-driven, immediate | N/A (R+G) (v: 9692d41) (no timed behavior — no dispatch exists to guard; next step: none) |
| Transitions | `Result → Waiting`, `Result → Room` | (ev: docs/smoke/result-hand-session.md) Implemented + tested (guards: `result_retries_to_waiting_and_leaves_to_room`, `menu_walks_full_session_to_results`, `flow_allows_every_documented_edge`) | PASS (R+G) (v: 9692d41) |
| Gameplay | Grade mapping: score→grade rule PINNED with #78 (B48): chooser `FUN_004c4dd0` climbs `score / grade_max_reference` through f64 thresholds 0.30/0.50/0.70/0.80/0.90/0.95 (byte-verified constants; 0.95 a single full-image occurrence) to plate frames 1–6; max combo == note-count word − 1 promotes 6→7; score == max gives 8; the reference `this+0x10d14` is the full word `0x778a00 × 200` + 10/25-group jam bonus (writer `FUN_004c22c0`, files `0xc1821`/`0xc1829`; 27-record floor). Grade strings `0x247363+` stay code-unreferenced (B4) — letters are `.ojs`-driven plate frames, no letter mapping exists to pin | implemented in `o2jam-game` (`Score::grade_ratio` / `is_combo_at_top` / `is_score_at_max` / `grade_frame` / `Score::grade_frame`) and painted by the shell at the pinned (395,531) plate anchor (rule 6). Guards: `grade_mapping_selects_the_pinned_frame_at_band_boundaries` (synthetic boundary sweep, o2jam-game), `grade_thresholds_climb_the_pinned_table`, `grade_promotes_combo_top_and_max_score`, `live_plate_climbs_one_frame_per_band`, `live_plate_promotes_combo_top_and_max_score`, `live_plate_stays_off_without_chart_or_ratio`, shots-level `live_plate_paints_its_frame_at_the_pinned_anchor`, `grade_plate_paints_the_selected_frame_at_its_pinned_anchor`, `sub_threshold_grades_keep_the_plate_off`, and the walked solo shot `solo_shot_paints_the_walked_grade_plate` (grade 8 for the one-Cool one-note walk) (ev: claims B4, B48; `docs/result-variants.md` rule 6) | PASS (R+G) (v: ad43134) (evidence #78, shell #79; residual OPEN: grade-letter art stays `.ojs`-driven per B4 — no letter mapping exists; the chooser’s score-global read is indirect, ladder rests on dossier ev `ev_615e034c…`/`ev_c61d99f8…`) |

## 2026-11-13 — #117: peer-row portrait chips per the pinned seat-byte convention

Resolved the documented contradiction from #85 (exe is oracle): the
shell's row-0 worn-equipment figure (`paint_result_avatar` +
`result_avatar_*` guards) is retired — the exe attaches a constant
slot object for EVERY row, including row 0, and never reads worn
equipment on this path (`docs/cresult-anchors.md`, "Peer-row
portraits", ev_d7951a4c…/ev_62d866b0…).

Implemented the convention in `o2jam-shell`:

- `ResultPortraitSlot` — one of the eight constant, image-less slot
  objects (vtable `0x6444d8`): mode-table u16 at `obj+0x15c` plus the
  builder-fixed `NUMERATOR = 56` (`obj+0x15e`, `trunc(22/100·256)`)
  and `RATIO = 0.22` (`obj+0x160`).
- `result_portrait_slots(display_mode_word)` — the mode-selected
  tables: `0xf` → `{0x7c02,…}`, `0x10` → `{0xf802,…}`; any other
  mode writes neither (files `0xc245d`–`0xc251a`).
- Seat-byte pick (`FUN_004c3b40`): member record `+0x31` (first 4043
  `MemberInfo` seat byte) is a 1-based index into the eight slots —
  byte 1 → element 0, byte 8 → element 7. Byte 0 is the
  `FUN_004c49a0` battle-team skip (file `0xc4a87`); the exe's index
  is unguarded (byte 0 / 9+ read out of bounds) and the shell
  resolves neither instead of inventing the out-of-range object.
- `ResultRankRow.seat_byte` rides the joined room's team byte raw per
  occupied peer seat. OPEN, named next steps: the waiting-room team
  rides 0–7 while the 4043 byte is consumed 1-based — whether the
  result copy already holds 1-based bytes settles with a populated
  battle-row capture; the local row's `+0x31` writer stays unpinned
  (row 0 resolves no chip, never a worn figure); the display-mode
  word (`+0x24`, getter file `0x15d2a`) has no pinned shell source,
  so `ResultPaint.portrait_slots` stages `None` and no chip paints —
  exactly the exe's no-table state for other modes.
- Slot art source OPEN (as named in #85): the eight objects are
  image-less; what the `0x6444d8` draw path renders from `+0x15c`
  (file `0x2260f`) is unsettled — next step: read that draw path or
  one rig capture of a populated battle row on :99. The paint path
  resolves the picks in exe attach order and paints nothing visible
  until the art source lands (guard
  `portrait_chip_resolution_paints_nothing_visible_yet`).

Guards: `portrait_slots_use_the_pinned_mode_tables`,
`portrait_slots_carry_the_builder_fixed_fields`,
`seat_byte_picks_the_slot_one_based`,
`seat_byte_zero_skips_the_row_as_the_battle_bucket`,
`seat_byte_above_eight_resolves_nothing`,
`portrait_chip_resolution_paints_nothing_visible_yet`,
`result_rows_carry_the_peer_seat_byte_for_portrait_chips`,
`result_screen_ignores_the_worn_figure_per_the_exe_convention`.
Retired: `result_paints_loaded_avatar_at_the_row_slot`,
`result_avatar_paints_at_the_local_row_slot`,
`result_avatar_clips_to_the_exe_row_band`,
`result_avatar_hangs_its_visible_box_at_the_slot`.
