# Scene parity checklist — 7.55 client vs `o2jam-shell`

Reference client stays read-only (`AngelO2Jam.exe`, 5,029,888 B,
SHA-256 `83a04541…72c823b`). Every 7.55 claim below traces to
`docs/claims.md` (the oracle); anything else is marked **OPEN**.
Placeholder shell art is a known mismatch, marked **FAIL** — never a pass.

Verdicts per `GLOSSARY.md`: **PASS** = exe evidence + implementation + test.
**FAIL** = known gap. **OPEN** = untraced or unclaimed with a next step.

Harness: `crates/o2jam-shell/tests/scenes.rs` asserts art-backed
composition per scene (a staged figure behind the live UI: figure
ink present, UI ink on top, frame never one flat fill) plus the flat
fallback (non-uniform card-over-background scenes, per-scene
markers) and the full transition matrix plus input/timing edges
through the public API (`ScreenGame` incl. `load_scene`/`load_splash`,
`paint_screen`, `ScreenFlow`, `can_enter`) only. Staged figures stay
synthetic in the repo (real `.ojs`/`.bnd` bytes stay out), but the
local-only real-art soak now runs: `cargo test -p recomp2jam shots`
stages genuine `Interface1`/`Playing1` art from git-ignored
`install-root` (10/10 green, zero skips, 2026-11-07), and the six
PNGs below are genuine-art frames. Composition cells stay FAIL where
invented placement remains (cited per cell), never for lack of art.

## `STATE_AVI` — logo splash (`CStateAvi`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | First visible screen; one-entry block `IDC_BACK_LOGO` → `O2mediaLogo.ojs` (lifecycle §5); sole `.bnd` rect `(0,0)-(800,600)` is verbatim full-bleed (bnd-semantics) | Logo stages through the bound `STATE_AVI` record — one staging path, no splash slot (`load_splash`/`load_avi_logo` removed; `setup_splash` stages the declaration). Guards: `avi_stages_its_logo_through_the_bound_pairing`, `avi_missing_bind_file_stages_nothing_with_a_miss`, `avi_paints_staged_logo_without_a_splash_slot`, `avi_paints_real_logo_through_the_bound_record` (asserts the `IDC_BACK_LOGO` placement), `splash_paints_logo_through_scene_art_only`; soak `present_archives_stage_key_scenes`; `avi.png` zero-diff in the shots oracle | PASS |
| Input | Splash dismiss key (exe dispatch unpinned) | Any lane-key press or canvas click dismisses onto the bare board (ours-by-design stand-in) | OPEN (next step: `rea` trace of the `CStateAvi` dismiss dispatch arms — not in the dossier set — plus a Wine SendInput key sweep against the boot splash; guard: `lane_key_dismisses_splash_into_planet_board`) |
| Timing | Logo hold duration (dispatch call not in dossier set) | 3 s stand-in (`SPLASH_AUTO_ADVANCE_MS`), then auto-advance onto the bare board | OPEN (next step: trace the splash timer dispatch or measure the hold under Wine with a null sink, then replace the stand-in; guard: `splash_auto_advances_at_timeout`) |
| Transitions | `Avi → Planet` (bare board) only | `can_enter` allows exactly that; boot proven through the windowed walk harness (key edge plus timer edge, screen trace plus state hash) | PASS (guards: `splash_boots_into_the_bare_planet_board`, `avi_boot_reaches_the_bare_planet_board_on_both_edges`) |

## `STATE_CHANNEL` — channel select (no state class)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | 50 controls, decl `0x27`, **no `.bnd`** (bare `BOUND`); `Dialog_ChannelSelectBar.ojs`, back/rank buttons (lifecycle §6) | Soak stages genuine dialog art under the pinned no-fill rule: `Dialog_ChannelInfo.ojs` body at its record anchor (0,0) (exe-mounted sheet 800x468; the parsed body is one 800x466 layer — 2 px runtime delta recorded, uncompensated) and `ChannelInfo_CH_Enter.ojs` frame 0 at the faces (exe art 44x24; parsed frames 44x21). Row geometry is now **exe-pinned** (live Wine memory read 2026-11-09, `docs/channel-fill.md`): the hosting `CStatePlanet` dialog object at `parent+0xF6C` holds the four viewport slot rects (216,224,572,275) / (216,275,572,324) / (216,324,572,375) / (216,375,572,436) at dialog+`0x94`; paint, scrim, and hit-test all ride this table (tops inclusive, bottoms exclusive), and the enter-face inset (row top + 12 → face Y 236) is pinned the same way — the reference capture's `232+50i` anchors were these rects' interior and are retired. Composition correction 2026-11-09 (live captures, `docs/planet.md`): the dialog opens **OVER the planet board** (verified first screen) — the shell paints the staged bound `State_Planet` block (backdrop + emblem buttons + promo + MENU bar) beneath the opaque 800x466 dialog sheet, and the MENU bar band stays visible in the bottom strip (y>468); the pinned no-fill corner rule keeps pixels no art covers untouched, and without board art the strip below y466 stays byte-identical to the art-less frame (soak + shots guard both arms). Composition correction 2026-11-10 (`docs/channel-fill.md` "METHOD CORRECTION"): `Dialog_ChannelInfo.bnd` (27 rects) pairs to the `0x8C` block by code ordinal `(code & 0xFFF) - 1` — its rects 2-5 equal the exe-pinned row rects exactly (4/4), which verifies the pairing. The bnd now pins the X button (541,151)-(589,189), the row-bar mount (row origin +1 → bar at (217,225)), the bar-local children (digit slots (50,11)/(63,11) 13x18, count plate (90,12) 185x19, enter face (285,11) 44x21 — face of row 0 = (502,236), exact against the exe-pinned inset and the capture), and the scroll track (573,213)-(594,429) with arrows at scroll-local (0,0)/(0,196) and the 21x45 body between them. The painter stages `ChannelInfo_CH_Back.ojs` per row (replacing the procedural pill/track chrome while staged — the chrome is the no-archive fallback), the digit/count plates, the enter faces, and the scroll bodies; absent figures fall back to today's behavior. The board EXIT `Planet_Btn_Exit.ojs` (750,561)-(788,599) rides the already-staged bound planet block; hit-testing lands with S0.3 (dispatch below). Block staging live-confirmed: 33 zero-rect records (state `0x12`), the `0x8C` dialog block carries 26 sprite-named records. Hover highlight (live 2026-11-09): the code-mounted `Dialog_ChannelSelectBar.ojs` (348x44) is the list highlight — its sprite record is live-confirmed mounted at dialog+`0x8C` (h/w/pitch 44/348/696 at +0x170) and it paints at the hovered row's rect (216,row_top) 348x44 (hover responds to mouse motion alone); the painter paints the staged bar there and keeps the amber cursor only as the no-figure fallback. Fallback chrome wells are now live-measured (2026-11-09): number well (225,228) 71x32 (the old (205,227) 94x35 pill was 20 px off) and track well (298,229) 195x32; the bnd-pinned enter face ((502,row.t+12) 44x21), X (541,151)-(589,189), and scroll track (573,213)-(594,429) all agreed with live measurement within ±2 px (brightness-run tolerance). Pager: RESOLVED NEGATIVE 2026-11-10 — the bottom strip (y~439-466) is live-confirmed dialog-sheet rim chrome plus the scroll column's down arrow; the decoded sheet art bakes no pager glyphs, the bnd names no pager rects, and the block's `IDC_BUTTON_SCHANNEL_LEFT/RIGHT` (`0x12200005/06`) stage zero rects and are statically unreachable (dead authoring vocabulary). The 7.55 channel dialog has NO pager: list navigation is the scroll column alone; the shell's procedural pager readout is removed and the earlier capture measurement is recorded as a rim-ink misread. Bare-board geometry (2026-11-09 reference capture, planet scene) is pinned as `PLANET_BOARD_*` constants for future wiring (promo nav cluster (345,509) 107x27, OPTION (712,562), EXIT (753,562), MENU back-arrow (130,562) 31x32; no hit-testing yet). Still reference-measured (exe-OPEN): the 48x24 plate stand-ins. Guards: `channel_row_hit_maps_the_exe_pinned_bands`, `channel_row_hit_mirrors_the_pinned_row_rects`, `channel_dialog_x_hit_uses_the_bnd_rect`, `channel_enter_face_is_bar_local_per_row`, `channel_bar_art_covers_the_measured_pill_track_band`, `channel_scene_without_bar_figure_keeps_procedural_chrome`, `channel_dialog_body_stages_behind_the_live_list`, `channel_enter_art_covers_the_measured_faces`, `channel_select_bar_paints_at_the_highlighted_row`, `channel_highlight_falls_back_to_the_cursor_without_select_bar`, `channel_fallback_chrome_uses_live_measured_wells`, `planet_board_reference_geometry_is_pinned`, soak `channel_layers_list_over_real_dialog_art`, shots `channel_shot_stages_dialog_art_inside_the_measured_bands`. Ticket #12 (2026-11-11): live rows — the `CH_Num` tens/ones wells show selected digit frames (`(id/10)%10`, `id%10` over the wire `channel_id`; list-ordinal display stays OPEN), the `CH_ConnectCount` strip clips to the live population share of its 185x19 plate (floor rule is the labeled stand-in — segment quantization stays OPEN), FULL rows (active, `population >= capacity > 0`) show the baked `CH_Back` frame-2 FULL face with no enter face, and every other row shows bar frame 0 plus enter frame 0 (single-frame paints only: the loader stacks all frames as layers, and whole-figure paints smear). Faces ride above the bar (the frame-0 face well is opaque decoded art, not a cutout). The ASCII row string is gone on the staged path — live captures (`~/scratch/t-chanlist.png`, `q-chan.png`) pin that exe rows carry no text (digits, segments, and the enter face only), and the decoded sheet bakes its own CLOSE wells for slots past the live list, which the painter leaves untouched. No stand-ins remain on the staged path; the figure-less fallback keeps the procedural chrome plus the ASCII string, labeled. New guards: `channel_digits_select_tens_and_ones_frames`, `channel_fill_clips_to_population_share`, `channel_full_row_uses_full_frame_without_face`, `channel_staged_digits_replace_ascii_text`; OPEN notes: digit value source (ordinal vs id — next step: gateway replay with non-sequential ids plus a well capture), strip rounding (next step: replay with known pop/cap plus a segment capture), bar frame-1 trigger (no capture pins it), inactive-row rendering (all live captures show active rows — next step: replay with `active=false` plus a capture). Emblem captions (TEXT `0x19400007..0B` → bnd rects 6–10, strings pinned in `docs/planet.md`) paint via `o2jam-text` and EXIT/OPTION hit-test via the control-code dispatch table (S0.3; EXIT is bnd rect 15 — rect 26 cannot exist in the 17-rect file); boot lands on the bare board and a lit-emblem click opens the dialog (shell-gated; the exe edge stays OPEN per E1). | PASS |
| Input | Pick channel/planet → room (`My_channel`/`My_planet` bridge, file `0x24df70`–`0x24e024`) | Up/Down moves highlight (clamped), Confirm → Room, Back → Planet (Esc symmetry with the verified X close, guard `channel_back_closes_to_the_planet_board` plus `menu_backs_out_of_lobbies`) — entry-face click LIVE-VERIFIED 2026-11-09 (one click at (526,248) drives `03EC/0FBE/07D0/07D2/13A4` into `STATE_ROOM`, ~/scratch/planet-verify/); dialog X close and lit-emblem family switch verified the same session | PASS (click path) / OPEN (exe key codes: Up/Down/Confirm/Back wired and tested but the exe dispatch stays unpinned — next step: SendInput arrows/Enter/Esc sweep on the live dialog per the `~/scratch/planet-verify/` recipe, reading highlight via dialog+`0x94` and the phase flag at parent+`0xF64`) |
| Timing | None documented | n/a | N/A (no timed behavior) |
| Transitions | `Channel → Room`; hosted by unknown class (open question) | `Channel → Room` (row confirm / entry faces); dialog X (bnd-pinned rect (541,151)-(589,189); verified close click (577,171)) closes to `Planet`; hosted by `CStatePlanet` (live memory read 2026-11-09) | PASS |

## `STATE_ROOM` — multiplayer lobby (`CStateRoom`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x36`, 76 entries, `State_Room.bnd` (75 rects); mode filters, `Room_ButtonRoom.ojs` (lifecycle §7) | Soak binds 75 rects verbatim (`bind_verbatim_entries`: rect `i` → entry `i + 1`) under genuine panels. Ticket #13 (2026-11-11): rows split into the exe-measured fields — number at panel +(12,12) through the staged `Room_Number.ojs` digit font (hundreds/tens/ones at the 11 px digit pitch; GDI number text in the title band while the font stages no figure), title at +(64,+11) sized to the `ROOMNAME` rect, seat count at +(194,+10) sized to the `ROOM_USER` height — replacing the old combined string (rig `t2-room-stage.png`, `~/scratch/geom-open-close/findings.md` §2: digits "000" at 38/49/60 y 94, title "GEOJOIN" x≈90 core 94 y 93..108, count "(1/8)" x 220..264 y 91..115). Detail fields paint from their template rects at real sizes: the exe's GDI cap share fills 70% of the rect height (title ~11 px in 16 px, music ~14 px in 19 px — `TEXT_CAP_FRACTION`, same capture), panel-relative to the highlighted panel; icon-rect labels stand down while their icon figures stage (icon pass owns the rect), and number/title/count never double-paint (rows own them; the count's `ROOM_USER` rect paints background in the capture). Misplaced absolute detail ink in the top bar is gone — `room.png` now carries the fields inside the panels. Guards: `room_rows_split_number_title_and_count_to_pinned_offsets`, `room_details_anchor_to_bnd_rects`, `room_scene_paints_number_digits_at_the_pinned_plate_origin`, `bound_room_panels_paint_numbered_rows`, `room_rows_anchor_to_staged_panels_in_two_columns`, `room_icons_paint_their_frames_at_the_rect_origins`, `room_icons_skip_untouched_frames_and_missing_art`, `room_scene_layers_icons_under_the_detail_ink`, soak `room_layers_rows_over_bound_room_art`, shots `room.png` delta (only intended file). Still OPEN: the per-row template clone offset past the pins (digit places land within −4/−2 px panel-relative — next step: SendInput highlight sweep plus memory read of the row template clone per the `~/scratch/geom-open-close/` recipe); details for non-highlighted populated panels (the capture shows one populated panel — next step: gateway replay seating 2+ rooms plus a capture); digit reading order past "000" (symmetric — next step: 3-digit room number plus capture); `ROOM_USER` rect content (background in the capture — count rides the row field); exe key codes; the Left/Right pager (untraced) | PASS (row fields + template details; clone offset + pager + multi-room details OPEN with named next steps) |
| Input | Join/create room; `DIALOG_CREATEROOM`, password dialog | Confirm → Waiting, Back → Channel; create/password forms wired (S5): `Dialog_CreateRoom`/`Dialog_Password` rects via code→action dispatch, `CreateRoomForm::commit` → `CreateRoom` 2004 (empty title gated), `PasswordForm` → `JoinRoom` 3002. Live 2026-11-09: quiet server sends an all-`Number`-0 slot table, so the documented `set_rooms` empty-slot filter leaves blank panels (no rows to click — correct, painter proves rows with data). Wire completion rides the session seam: dialog commits stage the 2004/3002 bodies (`take_create_room`/`take_join`) in the same shape the session bridge exchanges on its host/guest paths — proven shell-side, ASSUMED wire-side. Guards: `create_room_commit_needs_title_and_stages_2004`, `password_dialog_commits_join_and_enters_waiting`, `listed_rooms_keep_titled_zero_numbered_first_room`, `quiet_server_slot_table_leaves_no_rows_to_click`, `room_lobby_session_replays_like_its_twin`, `click_selects_room_row_and_enters_waiting`, `click_selects_staged_room_panel` | PASS (click path; wire send ASSUMED — next step: live gateway replay confirming 2004/3002 on the wire) / OPEN (exe key codes: Confirm/Back wired and tested but the exe dispatch stays unpinned — next step: SendInput sweep on the live lobby per the `~/scratch/planet-verify/` recipe) |
| Timing | None documented | n/a | N/A (no timed behavior) |
| Transitions | `Room → Waiting`, `Room → Channel` | Implemented + tested | PASS |

## `STATE_WAITING` — pre-game room (`CStateWaiting`/`CLiveWaiting`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x63`, 117 entries, `State_Waiting.bnd`; 8 team buttons `0x16201004..0B` (`Wait_TeamA..H.ojs`), 8 avatar covers; song dialog decl `0x7A`/134 entries (lifecycle §8, gameplay room-teams) | Soak binds 116 rects verbatim under genuine art — `waiting.png` shows real avatar tiles with seats + `WAITING` stamp on top. Ticket #14 (2026-11-10): seats stage at the verbatim cover rects (`COVERAVATAR1..8`, rects 60..67) with live seat lines — occupied covers take the scrim plus role/level/name/ready line, empty covers stay untouched staged panels, locked covers name `LOCK`. Team highlight rides the staged `Wait_Team?.ojs` button rects (rects 3..10). Live header captions ink their verbatim `.bnd` rects: MAPNAME (map-cycle entry of the room word), BPM (catalog column of the song, `-0-` on mismatch), SKILLCOUNT (`7/42/999`-capped inventory words), MUSICNAME (catalog title else `M{id}`), WINNERSCORE (wire streak), SPEED_NUM (`X0.5`..`XR` chain), 1/2/3SKILL ring ids; chat rows plus the focus caret paint at the LIST/EDIT rects (file-traced rects 42/19). The no-archive fallback keeps the labeled invented grid (ADR-0003). The song dialog stages over the seats — see the musicselect row below. Still OPEN: `INSTNAME`/`NOTICE` stay unpainted (OPEN 1/2 per `caption-sources.md`). Guards: `waiting_covers_paint_per_seat_rects`, `staged_covers_name_champion_and_challenger`, `empty_covers_leave_the_staged_panels_untouched`, `team_highlight_marks_our_team_button`, `wins_box_shows_the_champion_streak`, `speed_value_anchors_to_the_speed_num_rect`, `skill_slots_show_staged_ring_ids_only`, `waiting_musicname_paints_the_catalog_title`, soak `waiting_layers_seats_over_bound_waiting_art`, shots `waiting_shot_paints_mapname_bpm_skillcount_captions` + `waiting_shot_paints_the_scripted_chat_rows` | PASS (covers + captions; INSTNAME/NOTICE OPEN with named next steps) |
| Input | Team toggle → `SetRoomTeam` 4004 + `Ready` 4008(0); host start; back button (gameplay room-teams: `FUN_004b22d0` file `0xb22d0`, ev `2c8d3964…`) | Team buttons wired (S5) and answering through their staged `.bnd` rects: the toggle-group map (`FUN_0048c810`) lights exactly the pressed team, ungated joins queue `SetRoomTeam` 4004 then `Ready` 4008 in exe order; host start runs the START readiness gate (master + every occupied peer ready) into Loading; BACK leaves for rooms; art-less clicks land nowhere (no pinned fallback geometry). Guards: `click_staged_team_button_queues_4004_then_4008`, `team_select_lights_joined_and_queues_4004_then_4008`, `team_select_without_room_lights_only`, `team_broadcast_guards_slot_below_eight`, `waiting_hand_session_joins_team_readies_and_starts`, `waiting_confirm_hit_needs_staged_buttons`, `waiting_backs_out_to_room_list`. Still OPEN: the gate-helper table contents behind `FUN_00556680` (file `0x16680`) stay untraced (next step: REA trace of the handler table plus a live SendInput team-click sweep per the `~/scratch/planet-verify/` recipe); the 4004 byte-vs-dword builder case (next step: builder-switch dossier for opcode `0xfa4` plus a live gateway probe per the `SetRoomTeam` note in `o2jam-net/src/play.rs`); exe key codes for the team/confirm/back keys (next step: SendInput sweep on the live room) | PASS (click path; gate helper + builder width + key codes OPEN with named next steps) |
| Timing | `MusicLoading.ojs` + song clock arming at load (lifecycle §9.5) | n/a (see Loading) | OPEN |
| Transitions | `Waiting → Loading`, `Waiting → Room` | Implemented + tested | PASS |
| Gameplay | 8 seats, team 0..7, winner art only (no per-team panel) | Seats + ready flags; champion/challenger roles, the wins box, and the team highlight paint from live room data (guards above); no per-team panels beyond the highlight. Live fold gap, narrowed 2026-11-09: the guest toggle folds into game slots and self-seat reads slot readiness, but live snapshots number members off the slot indices (wire: member 1, no slot 1), so the fold misses silently — paint proven, lookup convention OPEN (next step: gateway replay pinning the member→slot convention) | OPEN |
| Music-select | Song dialog decl `0x7A`/134 entries (`DAILOG_SELECTMUSIC`, `CDSelectMusic`, lifecycle §8); host pick commits the room song (`MUSELECT 0x16200003` drops non-master clicks, file `0xb2622`); rows name the catalog songs | Ticket #15 (2026-11-11): the dialog stages over the seats through the bound arm (shared ordinal pairing — the every-bound-entry proof now covers `DAILOG_SELECTMUSIC`: 154 entries paint on their own rect origins, incl. the EXTEND rows behind the staged table bands; the multi-layer probe fix attributes overpaint to the topmost inked layer). A master row click commits the room song — the row's song id plus its difficulty slot become the `SetRoomMusic` 4000 body queued for the session bridge, the room state takes it at once (title/speed captions repaint), and only the picked row's letter inks the active orange (stand-in highlight, exe rendering unpinned). Catalog rows read the CP949 decode (`docs/file-formats/ojn.md` already pinned CP949 titles; the parser read UTF-8-lossy tofu — fixed, so table/panel/caption ink carries real glyphs through the embedded Noto face). Catalog order, easy-first, ten visible rows; per-chart level/grade stay blank (never staged), 다운/듣기/정보 icons blank (no staged art). Guards: `select_row_hit_maps_the_search_table_bands`, `click_staged_select_row_commits_song_and_queues_4004`, `select_row_click_without_master_or_art_commits_nothing`, `waiting_hand_session_picks_song_then_starts`, `picked_select_row_highlights_its_letter`, side `select_music_songs`/`select_music_rows` family, shots `musicselect_shot_repaints_waiting_with_song_panel` + `musicselect_fallback_catalog_lists_four_synthetic_songs` + `musicselect_dialog_structure_paints_search_tab_over_waiting`; smoke `docs/smoke/selectmusic-hand-session.md` (offline hand session: waiting→waiting→loading, 4000 staged). Still OPEN: dialog chrome clicks fall through (filters/pager/search/chosung/OK unevidenced — next step: rig chrome sweep plus captures, or dossier offsets); picked-row highlight rendering (next step: live row-click capture per the `~/scratch/planet-verify/` recipe); the queued 4000 has no session drain yet (next step: gateway replay through `SetRoomMusic` 4000 confirming the room echo); exe key codes (next step: SendInput sweep on the live dialog) | PASS (dialog + host pick; chrome + highlight rendering + wire send + key codes OPEN with named next steps) |

## Loading gate (song select → play)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | `MusicLoading.ojs`, Trebuchet/Arial fonts, song-table walk + clock arm (`FUN_004e47a0` file `0xe47a0`, ev `68d920c2…`); OJN decrypt + Vorbis init precede it (lifecycle §9) | Soak stages genuine planet art full-canvas (`loading.png`); bare `BOUND` makes full-bleed the documented fallback, but per-sprite native sizes are ignored (audit 23). Re-shot 2026-11-08: the walk threads the gate phases (entry plus staging/decrypt/decode/arm) through `loading_phase_status` with the staged catalog title/artist (song 757, `OJNList.dat` row through the CP949 decode (ticket #15 — real glyphs) and holds the clock-armed full fraction, so the frame proves the `with_status` path (252 px of new caption ink at the placeholder row, bar unchanged full). Ticket #16 (2026-11-10): the caption inks through the exe-pinned Trebuchet 27x65 cell (`Cell::Loading`, `0x4e47a0` `+0x524` per `docs/loading-type.md`/`docs/gdi-text.md`); Arial 7x14 keeps no cell (OPEN). The OJN/Vorbis legs are implemented file-side (`o2jam_file::loading`: select validation, staging extract names, OJN open, sample-format and channel checks; CLI `prepare_chart`/`load_gated_level`) and the walked OJN cover stages over the planet fallback (`stage_loading_cover`/`has_loading_cover`). Bar and caption rows stay OPEN placeholders by negative evidence: no `.bnd` names the gate (bnd-audit 35) and the sprite-55 walk pins `MusicLoading.ojs` to one 800x600 body with no sub-rect — centered rows, never literals (ADR-0003). Guards: `loading_stage_drives_all_gate_phases_with_catalog_caption`, `loading_shot_paints_staged_fraction_and_caption`, `loading_hand_session_holds_then_arms_and_hands_off`, `loading_caption_routes_through_the_display_cell`, `loading_cell_pins_the_gate_trebuchet_face`, `loading_paints_the_staged_walk_fraction`, `loading_paints_the_staged_song_caption`; smoke `docs/smoke/loading-hand-session.md` (hand-driven passage `loading→loading→playing` with trace hashes). Still OPEN: placeholder rects (next step: live gate capture plus memory read of the overlay mount, or a dossier offset for the bar fill call); Arial cell; phase wall-clock pacing (Timing row) | PASS (gate art + fonts + legs; rects + pacing OPEN with named next steps) |
| Input | Host start triggers gate | Confirm is the S7 gate: it holds in Loading until the armed walk (validation `0x5718b0`, staging/decrypt/decode/arm phases, audio rig) passes; failures latch and hold the scene honest. Guard: `loading_hand_session_holds_then_arms_and_hands_off` (hand-driven passage with trace) plus the hold/failure/armlatch leg guards | PASS (gate; live Wine validation of the full chain blocked by E2) |
| Timing | Select validation `0x5718b0` → staging `0x4f8b80` → decrypt `0x412180` → codec `0x54db40` → load screen | Gate phases threaded through `loading_phase_status` (S7), clock armed on the sample clock at Playing entry | OPEN (phase wall-clock pacing under Wine unverified — next step: rig run timing each gate leg per the `~/scratch/planet-verify/` recipe on a null sink, then replace the fraction steps with measured holds) |
| Transitions | `Waiting → Loading → Playing`, `Loading → Waiting` | Implemented + tested | PASS |

## `STATE_PLAYING` — gameplay (`CStateNormalPlaying`/`CStateLivePlaying`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Real UI in `Playing1.opi` (235 entries, `State_Playing.bnd` — bound run is entries 213–229 `EXIT`…`VOLUME`, decl `0xE6`; bnd-semantics); life/jam gauge bars, `Winner_image.ojs` (lifecycle §10, gameplay lifebar/jam sections) | Highway now derives from staged art, not frame fractions: uneven lanes `(5,28)…(165,28)`, judgement row 487, 7 px heads, per-lane receptor heights, jam bars `(23,536)`/`(570,536)` 206×16, peer 46×8 (5cb01a2; audit 4–9 superseded); soak proves it (`play_skin_decodes_every_lane_when_present`, `skinned_highway_replaces_procedural_note_ink` green). 2026-11-07 re-shoot: `playing.png` regenerated byte-identical to e9fd95f (no pipeline drift), then region-checked against `reference-4x3.png`. Per-region: **highway panel** structurally matches (lanes, receptors, staged JAM bar + strip combo/score; big COMBO/word/LONGNOTE digits stay small-font at invented anchors — audit 31); **edge gauges** match (staged life/jam gauge art at record anchors, both flanks within photo tolerance); **stage scene** matches with vintage deltas (stage-5 studio stands for the photo's unshipped rooftop, documented in shots; figure extent-centered in the `Map_Avatar.bnd` seat, `Lv.43` plate + amber gauge at photo offsets, HOT TIME badge staged); **bottom strip** partial (counts/TIME/speed/title/EXIT + staged panel paint; `VOL 58` now staged too — shots `playing_shot_paints_the_staged_volume_readout` red→green; big `0005` play-time digits over the bar stay on an assumed anchor — audit 31 — and procedural white TIME fill rides over the staged bar art, audit-16 family). Residuals documented, no invented covers: staged `Note_BG.ojs` right-side red panel shows at (570,458)–(774,478) where the photo's venue shows deck (exe cover untraced); staged strip panel is blue metallic vs the photo's dark strip (venue art vintage); BO/EA05 edge labels unpainted. **Mirror** 2026-11-08: the walk seats the wire peer's gauge through the session bridge (`apply_peer_echo` slot 1 `STAT_LIFE` full — the same stats-then-ranks order `OnlineSession` folds 4015 echoes through), so `playing.png`/`playing-native.png` carry the live green fill at the slot-1 rect (754,8,46,8; 213/368 exact-green native px, majority-green guarded by shots `playing_shot_paints_session_fed_peer_gauge`); the walked strikes both rate Cool, whose Hard +1s saturate, so local life stays `LIFE_MAX` behind the avatar bar (same guard). The same regen refreshed `waiting.png` by 329 px in the chat band with zero fixture change — pre-existing drift (reproduces with the gauge change stashed), not from this work. Still FAIL: gauge appended full-canvas over the bound window (audit 16), tally strip (TIMEBAR/TITLE/SPEED/VOLUME/EXIT) now paints from the 17 parsed `State_Playing.bnd` rects by control id (S4; `TALLY_*` literals kept only as the no-archive fallback); combo/word/long-note digits stay OPEN (entries 136–145 have no rects — named next step: `rea` trace of the digit-frame setters for codes `0x1B500088`–`0x1B500092`), linear scroll map (audit 32), curtain clips a frame box not lane columns (audit 30) | FAIL |
| Input | 7-lane keys, judgement vs `\|song−note\|` ≤ 6/18/25 ms → flags `0x10/0x20/0x40/0x80` (gameplay judgement, file `0x176540`) | Lane strikes hit `LivePlay` on the sample clock (S3: backend samples minus the Playing-entry anchor; rate auto-synced from the device) | OPEN (live key-up routing) |
| Timing | Sample-accurate song clock; per-frame bg-effect loop `0x550db0` (ev `4164031c…`) | Song position is backend-sample-derived (S3, sample clock); per-frame bg-effect loop still unproven | OPEN (bg-effect loop) |
| Transitions | `Loading → Playing → Result` | Implemented + tested, incl. auto-advance | PASS |
| Gameplay | Cool/Good continue combo, Bad/Miss break; pills (15-Cool mint, spend-on-Bad); score `200+10X/100+5X/4/−10`; life max 1000, per-difficulty deltas Cool +3/+2/+1, Good +2/+1/0, Bad −10/−7/−5, Miss −50/−40/−30, dead gate at 0 (gameplay, all VERIFIED; life: switch `0x170fa2` ev `feebe490…`, scorer `0x170510` → adder `0x540a80`) | Implemented + tested in `o2jam-game` (`LifeBar` + `LifeDeltas`: difficulty sets, 0..=1000 saturation, dead gate), not wired to shell scenes | OPEN (shell-side) |

## `STATE_RESULT` — results (`CStateResult`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x07`, 8 entries: `Result_WinLose.ojs`, `Result_NumFont_*.ojs`, Top/Bottom, Retry; builder `FUN_004c2390` (file `0xc2390`, ev `d6de3a7f…`) | Soak stages genuine panels — `result.png` shows real COOL/GOOD/BAD + music-name art with digits on top. Re-shot 2026-11-07 post-`04e2462`: the rank table now paints row 0 (`YOU`, `1/0/0/0`, `200`) at the pinned columns (guard: `result_shot_paints_the_rank_table_rows`). Ticket #17 (2026-11-11): digits paint at the exe GDI anchors (top-panel `FUN_004c38a0` columns, `docs/cresult-anchors.md` `ev_615e034c…`), the verdict banner paints the exe-selected `Result_WinLose.ojs` frame (`winlose_frame`, rule 1/2 of `docs/result-variants.md`), the grade plate paints the pinned threshold frame at (395,531) (`grade_frame`, rule 6), and the local avatar hangs its visible box at the row-0 slot (78,316) under the 652×22 band clip (row painter `FUN_004c3b40`, `ev_88c3ca31…`) — no fixed-pixel anchors remain. Guards: `result_avatar_paints_at_the_local_row_slot`, `result_avatar_clips_to_the_exe_row_band`, `result_avatar_hangs_its_visible_box_at_the_slot`, `result_paints_loaded_avatar_at_the_row_slot`; smoke `docs/smoke/result-hand-session.md`; shots oracle holds with only the four intended `result*.png` deltas (avatar leaves frame center for the row slot). All 7 `State_Result.bnd` rects are verified (S4) and cover only panels/buttons/grade plate (number anchors stay exe-pinned GDI — negative evidence: no rects exist for them); `Winner_image` is decl-unloaded network-variant art, never on the result scene (test-pinned out of `RESULT_FRAME_BODIES`). Still OPEN with named next steps: peer-row portraits (member-table avatar-id byte rides server-side — next step: gateway replay pinning the member→avatar convention plus a rig capture of a populated battle row), life-readout and bottom-counter digit cells (pinned geometry, font-body binding inference — next step: REA trace of the digit-writer callers at files `0xc44c0`/`0xc4630`), grade letters (zero `.text` refs, `.ojs`-driven — next step: live rig capture of the plate at controlled score bands) | PASS |
| Input | Retry → waiting loading; leave → room/channel; event-ticket web call (file `0x24a338`) | RETRY click hands back to Waiting, BACK click leaves for Room (Confirm/Back mirror the buttons; miss clicks land nowhere). Guards: `staged_retry_click_returns_to_waiting`, `staged_back_click_returns_to_room`, `result_keys_mirror_the_staged_buttons` | OPEN (no ticket call: the solo enter painter's event-ticket web call plus battle `0xfca` and retry `0xfaa`/back `0xfb5` relay events stay unwired — staged entry queues nothing, guard `result_entry_stages_no_ticket_wire_call`; next step: live gateway replay naming the wire shapes, then queue the bodies through the session bridge; retry-while-siblings-ready gating likewise waits on the live relay) |
| Timing | None documented | Paint-driven, immediate | N/A (no timed behavior) |
| Transitions | `Result → Waiting`, `Result → Room` | Implemented + tested | PASS |

## Side states (reachable from lobbies)

All **OPEN** for data/input (no fetch opcodes, no buy/gift/cart/search
flows — documented so no pass is ever claimed). Composition 2026-11-10:
the walk now visits every side screen from Waiting through
`visit_side` with its genuine bound art staged (`side_stage_data`:
`.bnd` placements paired by code ordinal plus decoded figures, the
painter consumes every placement through `paint_side_scene`), and
shots `shop.png`, `musicshop.png`, `myroom.png`, `lanking.png`,
`payment.png`, `tutorial.png` show the file-native frames. Without
`install-root` the side shots stay skipped (clean skip, CI arm) and
the walk list tests assert both arms. The musicselect chosung grid
now counts its row cells and x-pitch from the staged INDEX-button
inventory (`select_chosung_layout`, FILE per the audit; row tops and
left edge stay CANDIDATE constants). Contradiction correction:
`State_Myroom.bnd` **exists** on the shipped archive (111 rects,
full-bleed r0) — the older "bind file declared but absent" note was
wrong; Myroom stages bound like every other side screen.

- `STATE_SHOP` / `STATE_MUSICSHOP` (`CStateShop`/`CStateMShop`,
  `0xA4`/`0x6B`; `Payment` back-ends, `CMyBag`/`CCart`). Bound
  placements stage (ItemShop 186 rects, MShop 138 rects); every
  staged figure paints at its rect. Data rows (items, songs, prices,
  pager cells) stay unpainted stand-ins: no data join exists and the
  audit gives no FILE rows for these scenes. Buy/gift/cart/search/
  sort/pager flows and entry unwired (all OPEN).
- `STATE_MYROOM` (`0x6A`; `CItemManager`, `CPlayerInfo`). Bound
  placements stage (111 rects). Slot selection, equip, pager, gift
  flows, captions, entry unwired (all OPEN).
- `STATE_PLANET` (`CStatePlanet`, `0x11`; BGM `Music/Planet.ojm`;
  quest button commented undecided — no drum backend). Screen,
  paint (staged art or flat card, exe-pinned captions in real
  glyphs), and flow edges (`Channel→Planet` via the dialog X,
  `Planet→Channel` via a lit emblem or Confirm — live-verified
  2026-11-09, `docs/planet.md`) live; no bare-board → Room path is
  evidenced. Button clicks hit-test at the five pinned `.bnd`
  button rects and HARD/NORMAL/EASY record a difficulty pick; the
  promo `< ☐ >` cells answer clicks at their parsed rects (press
  records, staged placements only — ADR-0003); EXIT/OPTION discs
  dispatch at their pinned rects (quit-confirm / options dialog).
  FREE/DRUM clicks stay unbound (P0). Guards:
  `planet_emblem_captions_decode_to_the_pinned_strings`,
  `planet_captions_have_real_glyphs_in_the_system_font`,
  `planet_captions_paint_into_their_bnd_rects`,
  `planet_dispatch_maps_promo_cells_to_carousel_presses`,
  `planet_promo_cells_answer_clicks_at_their_staged_rects`,
  `promo_press_records_the_cell_without_leaving_the_board`,
  `lit_emblem_click_opens_the_channel_dialog_and_records_the_pick`,
  `planet_confirm_opens_the_channel_dialog`,
  `power_disc_opens_quit_confirm_and_ok_quits`,
  `gear_disc_opens_options_and_escape_closes_it`, plus the window
  board session (`docs/smoke/planet-board-session.md`: 19/19 trace
  events, `avi → planet → channel → planet → channel`).
  PASS (board behavior) / OPEN (highlight rendering — exe orange
  select border capture-pinned, shell keeps the amber cursor with a
  named next step in `docs/planet.md`; promo carousel semantics;
  bare-board → Room/solo path; exe entry key; BGM trigger
  superseded per `docs/planet.md` OPEN 7).
- `STATE_LANKING` (`0x37`; display-only, no state class — like CHANNEL).
  Bound placements stage (73 rects). Rank rows, sorts, search, and
  pagers stay unpainted (fetch opcode unknown — all OPEN).
- `STATE_PAYMENT` / `STATE_PAYMENT2` (web billing views). Both
  billing views stage and merge (9 + 12 rects); no web call wired.
- `STATE_TUTORIAL` (stub `0x00`; entry path unknown). The
  `Playing_TutorialBG.ojs` backdrop stages full-canvas from `Playing1`.
- `STATE_DRUM_PLAYING` / `STATE_EXIT` (stubs `0x00`; drum never wired,
  no `CStateDrum*` class; exit is teardown, not a screen).

## Reference-photo verify pass (2026-11-07 re-shoot)

All seven PNGs regenerated via `cargo run -q -p recomp2jam -- shots`
(`cargo test -p recomp2jam` 75/75 green, incl. the new
`result_shot_paints_the_rank_table_rows` guard and the three
musicselect guards). Per-region read is
ink-presence per band (shot std vs photo std — content differs by
scenario, so structure, not pixels, is compared):

| Shot vs photo | Region verdicts |
|---|---|
| `playing.png` vs `reference-4x3.png` | HOLD, byte-identical to the e9fd95f bytes (no pipeline drift). Highway panel (59.5 vs 78.7), edge gauges (40.8 vs 64.9), avatar scene (67.6 vs 41.5), bottom strip (75.3 vs 54.0) — every band carries staged ink on both sides. Residuals unchanged: stage-5 studio stands for the unshipped rooftop, `Note_BG` red panel at (570,458)–(774,478) with no exe cover, blue-metallic strip vs the photo's dark strip, BO/EA05 labels unpainted |
| `waiting.png` vs `reference-waiting-4x3.png` | REFRESHED 2026-11-07 (scripted 3012/3013 chat fixture — see below). Top panel (59.6 vs 65.6), seat grid (55.5 vs 53.0), right column (59.4 vs 61.4) unchanged; bottom chat band carries two live rows plus the input caret (bracket-form amber rows at the photo-pinned 17 px pitch — see the 2026-11-08 chat-box verify below). Caption refresh 2026-11-07 (see below): only `waiting.png` changed again — 682 px confined to the six caption rects (MUSICNAME/MAPNAME/BPM/3×SKILLCOUNT), every other shot byte-identical, no pipeline drift. Residuals: invented 2-col x 4-row seat grid, no team buttons/covers (the song dialog lives in `musicselect.png` below); chat texts are ASCII stand-ins, no paging/scroll |
| `result.png` vs `reference-result-4x3.png` | REFRESHED (was stale). Score dome (48.6 vs 50.7), verdict banner (46.5 vs 48.7), rank table (85.3 vs 48.5), footer (59.1 vs 51.2). The checked-in PNG predated the rank-row wiring (`04e2462`) — its row band was empty; the re-shoot carries row 0 (`YOU`, `1/0/0/0`, `200`) at the pinned columns, guarded by `result_shot_paints_the_rank_table_rows` (proven to fail on wrong text). Scenario deltas vs the photo stay: GOOD! solo arm vs LOSE, one local row vs two wire rows, P plate vs C. REFRESHED 2026-11-11 (ticket #17): the worn avatar leaves frame center for the local row-0 slot (78,316) under the 652×22 band clip — `result*.png` carry the slot ink (row-0 band x 90..126, y 316..338 native) with the old center ink gone; photo's battle-row portraits ride the member table (OPEN). |
| `musicselect.png` vs `reference-musicselect-4x3.png` | NEW (was UNCOVERED — no shot mapped to it). The walk now repaints Waiting through the host song flow (installed 7.55 562-song catalog, pick id 757 Hard X3.5; 4-song synthetic fallback without game data) under genuine `DAILOG_SELECTMUSIC` art, and the staged structure paints over it: filter columns (75.0 vs 183.4), search table (61.3 vs 83.2), status + search row (76.4 vs 114.7), chosung + OK (84.3 vs 128.1) — staged ink in every band on both sides. Still FAIL: per-chart level and grade cells draw the unknown dash (never staged), the 다운/듣기/정보 icon cells stay blank (no staged art), and pager/search/chosung are display-only. REFRESHED 2026-11-11 (ticket #15): titles/artists now ink real CP949 glyphs (the parser read UTF-8-lossy tofu — fixed), the picked row highlights orange (stand-in), the bind pairing is proven (154-entry proof incl. the dialog), and a master row click commits the 4000 pick — see the new `STATE_WAITING` music-select row (PASS with named OPENs). Chosung jamo ink through the embedded face (the old micro-font gap is closed); word cells stay blank (no caption source). Guarded by `musicselect_shot_repaints_waiting_with_song_panel`, `musicselect_fallback_catalog_lists_four_synthetic_songs`, `musicselect_dialog_structure_paints_search_tab_over_waiting` plus the ticket-15 click/highlight guards |
| `channel.png` | REFRESHED 2026-11-10 (file-native geometry + hover bar): rows/X/face/scroll all bnd-pinned (pairing exe-verified), hover highlight is the code-mounted `Dialog_ChannelSelectBar` at the hovered row, fallback wells live-measured, dialog over the board with the MENU bar beneath. Pager removed — resolved negative (no pager in the 7.55 dialog; scroll column only). Still FAIL: plate stand-ins and row content text (ASCII, no CP949 glyphs) |
| `room.png` | HOLD, byte-identical (no drift). No reference photo covers it |
| `planet.png` | REFRESHED 2026-11-10 (re-scoped walk): now the BARE board — the walk shoots it after the dialog X (reference-measured zone, native (557,195) modulo chrome) closes the dialog; the staged 17-placement bound block (backdrop, five buttons, promotion image, MENU bar) paints with no dialog over it. A following lit-emblem (HARD) click reopens the channel dialog and records the pick (LIVE-VERIFIED 2026-11-09: the emblems are server
channel families, EASY active at boot ~21 s after login; guarded by `lit_emblem_click_reopens_the_channel_dialog`); the amber-cursor stand-in is covered by returning to the board (`planet_shot_stages_bound_art_with_the_picked_band`). REFRESHED 2026-11-11: captions now paint (exe-pinned strings in real glyphs, guarded), the picked band keeps the amber-cursor stand-in (exe orange select border capture-pinned, shell OPEN with a named next step — `docs/planet.md` "Difficulty-pick highlight"), FREE/DRUM stay unbound (P0), promo cells answer clicks at parsed rects (press records, carousel semantics OPEN), EXIT/OPTION dispatch (quit-confirm / options dialog); no reference photo covers it |
| `loading.png` | REFRESHED 2026-11-08: the walk holds the clock-armed full fraction with the staged catalog caption (song 757) — 252 px of new caption ink at the placeholder row, bar unchanged full; guarded by the two loading-stage tests. No reference photo covers it; staged `MusicLoading.ojs` art + green bar under the no-fill rule (empty stage would paint nothing, staged art is unaffected) |

Chat refresh 2026-11-07: only `waiting.png` changed (1104 bytes
of two chat rows at the list origin); every other shot byte-identical
— no pipeline drift. Earlier deltas stand: `musicselect.png` is new;
the rank-row refresh added 129 pixels of row-0 ink.

Chat-box verify 2026-11-08 (chat band vs
`reference-waiting-4x3.png`, photo pixels pinned per item; failing
tests first, then the shared painter): re-shot `waiting.png` (chat
band rows 225–250 + caret block) and `musicselect.png` (the same
input strip's caret, 10 px). `playing.png`/`playing-native.png`
regen diffs reproduce at HEAD with this change stashed — pre-
existing drift (committed bytes predate a HEAD jam-effect change),
not from this work. Per-item:

| Item | Verdict |
|---|---|
| Composition (sender/text rows at the list origin) | FIXED — user rows bracket the nickname right-aligned in a 13-wide field (`[{sender:>13}] text`), the photo's `[        Lujia] YOOO!!!` shape (was `sender: text`); rows paint bare on the box art, per-row scrim bands removed. Residuals: field width pinned by the one photo row (2-byte-name padding OPEN); `[ADMIN]` prefix stays a stand-in (the photo's admin notice shows no sender); the photo's `/r` command echo has no shell source |
| Ink colors | FIXED — user rows amber (photo user row reads warm while known-white ink in the same photo calibrates neutral; was white); admin stays amber. Residuals: distinct user-vs-admin hues unverifiable in the lossy photo; caret white per photo |
| Row stacking | FIXED — pitch 17 native px (photo row tops 455.4/472.5/489.8; was the lobby 8 px), 6 visible slots fill the 98 px `LIST` rect exactly (was 12); fixture reordered admin→user so the peer's row sits last, matching photo rows 2–3. Chronological top-down confirmed both sides |
| Draft line | FIXED — focus caret now paints: 2×10 white bar inset (5,5) of `EDIT_SWAIT_CHAT` rect 19, riding the typed width (photo shows the caret on an empty input; we painted nothing). Residuals: draft-text origin unevidenced (photo input is empty); `game.rs`'s "empty histories paint nothing" sentence predates the caret (out-of-scope file — follow-up) |

Guards: `waiting_chat_row_text_brackets_the_sender_field`,
`waiting_chat_rows_stack_at_the_reference_pitch`,
`waiting_chat_paints_user_rows_in_amber_at_the_list_rect`,
`waiting_chat_without_live_rows_paints_only_the_caret`,
`waiting_chat_paints_the_caret_on_an_empty_input`,
`scripted_chat_fixture_delivers_admin_then_user_rows`
(renamed), `waiting_shot_paints_the_scripted_chat_rows` — all
proven red before the fix (compile-red for the new painter API,
order-red for the fixture swap).

Caption refresh 2026-11-07: the waiting walk now stages the song
catalog plus inventory before the shot (room song 757, counts
`7/42/1500` with the slot-3 999 cap), so MAPNAME/BPM/SKILLCOUNT
ink their `.bnd` rects instead of the bare `M0`/`-0-`/`  0`
defaults. Old-vs-new diff is 682 px, all inside the six caption
boxes — MUSICNAME native (573,15), MAPNAME (620,448), BPM
(552,504), skill counts (712,208/246/284) — each cluster matching
its rect; no other band moved. Guarded by
`waiting_shot_paints_mapname_bpm_skillcount_captions` (exact text
at each rect, proven against the staged sources).

## Waiting / music-select region verify (2026-11-07 caption re-shoot)

Both PNGs regenerated via `cargo run -q -p recomp2jam -- shots`
(`cargo test -p recomp2jam` 97 green, incl. the three chat guards
plus the new caption guard below); `musicselect.png` byte-identical
(no drift there), `waiting.png` refreshed with staged captions.
Bands below are column-
restricted luminance std (shot vs photo — structure, not pixels):

### `waiting.png` vs `reference-waiting-4x3.png` — REFRESHED, still FAIL

| Region | Shot | Photo | Verdict + residual |
|---|---|---|---|
| Top panel (title/Ch/Wins bar) | 51.2 | 56.7 | HOLD — staged art + title/song/wins ink on both; shot room is the `Jam` duo (no Ch/No/MX3 line, song caption now resolves the staged catalog title for 757 instead of bare `M0`) — scenario delta |
| Seat grid (covers, champion row) | 45.0 | 47.7 | HOLD — staged covers + seat text on both; shot names no CHAMPION/CHALLENGER roles (joined room carries no champion) and centers portrait text blind (audit 18–19); seats still on staged cover rects, team grid highlight only |
| Right column (TEAM/SKILL/SPEED) | 43.1 | 49.7 | REFRESHED — staged panel + button/speed ink on both; BPM now inks the catalog column of song 757 (was `-0-`) and the three SKILLCOUNT rects ink the staged inventory (`  7`/` 42`/`999`-capped); team highlight is the live amber cursor over the staged `Wait_Team?.ojs`, no covers; skills paint ids only, counts ride the staged inventory (no wire field) |
| Bottom chat/song (chat, arrow, Random, BACK) | 60.1 | 57.8 | REFRESHED — the history box stages two scripted 3012/3013 rows in the photo stacking (amber bracket-form rows at the 17 px pitch, admin notice on top, `Lujia`'s user row last) plus the input-strip caret (2026-11-08 chat-box verify below); Random cards and BACK stay staged art (no card/nav state rides the snapshot). MAPNAME now inks the map-cycle entry of room word 5 at its `.bnd` rect (was bare). Guarded by `scripted_chat_fixture_delivers_admin_then_user_rows`, `waiting_walk_without_staged_chat_holds_no_history`, `waiting_shot_paints_the_scripted_chat_rows`, `waiting_shot_paints_mapname_bpm_skillcount_captions` |

Composition stays FAIL (invented placement where noted above; no team-button bodies or covers staged).

### `musicselect.png` vs `reference-musicselect-4x3.png` — REFRESHED (ticket #15), structure HOLD

Shot bytes changed (274 px, all inside the title cells plus the chosen-song panel — the CP949 title decode, ticket #15); every band below still carries staged ink on both sides, so the structure verdicts hold with the behavior residuals corrected.

| Region | Shot | Photo | Verdict + residual |
|---|---|---|---|
| Filter columns (level/wheel/genre) | 24.6 | 61.5 | HOLD — staged art + column/button ink on both; lower shot std is the 562-song catalog vs the photo's 1608 plus ASCII stand-ins; pager/search/chosung display-only |
| Search table (label/note/title/artist/lvl/icons/grade) | 25.4 | 30.8 | HOLD — 10 staged rows + header on both; titles/artists now ink real CP949 glyphs (ticket #15 rows read Korean where the exe draws text); per-chart level still draws the unknown dash (never staged), grade blank, 다운/듣기/정보 icon cells blank (no staged art); the picked row's letter inks the active orange (stand-in highlight, exe rendering OPEN) |
| Status + search row | 26.2 | 39.0 | HOLD — `ALL -> ALL SEARCH RESULT.` + pager `1/<pages>` + title/artist radios + edit + search button on both; photo page `3/161`, shot page 1 (no paging input staged) |
| Chosung + OK | 37.4 | 46.3 | HOLD — `A`–`Z` letters + OK pill on both; hangul jamo now ink through the embedded face (ticket #15 — the old micro-font gap is closed), 숫자/기타 word cells stay blank (no caption source); dialog bind pairing proven (154-entry proof incl. `DAILOG_SELECTMUSIC`, ticket #15) |

Dialog behavior rides the new `STATE_WAITING` music-select row (PASS with named OPENs); the photo-structural residuals above stay (level/grade dashes, icon blanks, display-only pager/search/chosung/OK).

## Wine capture verify pass (2026-11-08 room/waiting/loading)

Shot pipeline regenerated via `cargo run -q -p recomp2jam -- shots`
(`cargo test -p o2jam-shell -p recomp2jam` green, `cargo fmt --check`
clean): all PNGs byte-identical — the fallback-only change below
touches no staged path. Compared `docs/screenshots/room.png`,
`waiting.png`, `loading.png` against `~/scratch/aa-room.png`,
`ac-room.png` (room), `ab-wait.png`, `ad-wait.png` (waiting),
`ab-load.png`, `z-load.png` (loading) plus siblings:

| Scene | Verdict vs Wine |
|---|---|
| Room | FAIL — real lists 2 columns x 3 rows of panels (number/title/HX1/3KEY/green bar/WAITING badge, detail/avatar/chat right, modal OK dialog over it); shell rows stay single-column quarter-frame ink over staged panels, details at rect corners, no create/password/modal UI. No shot drift (staged path unchanged) |
| Waiting | FAIL but IMPROVED — staged path matches the real 4-across x 2-down seats, TEAM A–H block, SKILL/SPEED column, BPM box, Random cards, BACK. Fixed this pass: the art-less fallback grid was an invented 2-col x 4-row (`seat_origin`); now 4x2 per the captures (new `waiting_seat_grid_is_four_columns_by_two_rows`, red on the old grid). Residuals: portraits center blind (audit 18–19), no team-button bodies/covers, ASCII chat, song dialog lives in `musicselect.png` |
| Loading | FAIL — real is a song-illustration full-bleed (bead progress pills, 3-KEY badge, Artist/Remix/Pattern credits); shell stages planet `MusicLoading.ojs` art with the centered bar + caption placeholders (audit 35), song caption only. No invented pass; bar/caption rects unmoved |

## Scoreboard

Verify pass 2026-11-08 (chat-box verify on top of the
2026-11-07 passes: post no-fill, chat refresh, caption refresh):
2 HOLD (playing + room byte-identical at their last
verified pass; the 2026-11-08 regen carries pre-existing
`playing.png` drift — see the chat-box verify note), 5 REFRESHED
(result rank rows; channel art-less re-shoot; waiting scripted
chat rows + staged captions; waiting chat band re-verified and
corrected; loading live fraction + caption), 1 NEW (musicselect song dialog). Detail in "Reference-photo verify
pass" above.

Ticket #15 re-shoot (2026-11-11, CP949 title decode + picked-row highlight): 4 REFRESHED, 15 HOLD.
`musicselect.png` (+274 px: title cells of Korean catalog rows + chosen-song panel line, real glyphs for tofu),
`waiting.png` and `waiting-guest.png` (+177 px each: MUSICNAME caption title), `loading.png` (+4169 px: loading
caption title/artist) — every diff cluster named in the sections above; all other shots byte-identical (no
pipeline drift). The `STATE_WAITING` music-select row flips to PASS with guards named; the `musicselect.png`
photo rows above carry the corrected residuals.

- Scenes covered: 7 main + musicselect dialog + loading gate + 9 side states.
- PASS: boot-into-splash, 800×600 default (`SetWindowPos` file
  `0x14b4c`/`0x14baf`), all 11 documented transition edges (plus
  rejections) — 11 cells. No-timing scenes read N/A, never PASS.
- FAIL (known placement gaps): 8 composition cells (genuine art now
  stages in the local soak — shots green, seven PNGs refreshed 2026-11-07 —
  but invented placement remains per cell: audit 3, 10–16, 18–19,
  23–25, 30–32; audit 4–9 superseded by the verbatim highway 5cb01a2).
  The `load_scene` layering behind them stays harness-proven (22/22).
- OPEN: all input/timing/gameplay cells above plus every side state —
  next evidence targets are the loading-gate validation sequence
  (`0x5718b0` → `0x4f8b80` → `0x412180`), the peer-life 4015 handler,
  and per-cell rect bindings (channel bar/rows, room `IDC_*` fields,
  waiting seats/teams, result digits, playing digits/scroll).
