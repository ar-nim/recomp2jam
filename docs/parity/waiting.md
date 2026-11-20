## `STATE_WAITING` — pre-game room (`CStateWaiting`/`CLiveWaiting`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x63`, 117 entries, `State_Waiting.bnd`; 8 team buttons `0x16201004..0B` (`Wait_TeamA..H.ojs`), 8 avatar covers; song dialog decl `0x7A`/134 entries (lifecycle §8, gameplay room-teams) | Soak binds 116 rects verbatim under genuine art — `waiting.png` shows real avatar tiles with seats + `WAITING` stamp on top. Ticket #14 (2026-11-10): seats stage at the verbatim cover rects (`COVERAVATAR1..8`, rects 60..67) with live seat lines — occupied covers take the scrim plus role/level/name/ready line, empty covers stay untouched staged panels, locked covers name `LOCK`. Team highlight rides the staged `Wait_Team?.ojs` button rects (rects 3..10). Live header captions ink their verbatim `.bnd` rects: MAPNAME (map-cycle entry of the room word), BPM (catalog column of the song, `-0-` on mismatch), SKILLCOUNT (`7/42/999`-capped inventory words), MUSICNAME (catalog title else `M{id}`), WINNERSCORE (wire streak), SPEED_NUM (`X0.5`..`XR` chain), 1/2/3SKILL ring ids; chat rows plus the focus caret paint at the LIST/EDIT rects (file-traced rects 42/19). Ticket #40 (2026-11-11): every occupied seat paints its own staged worn figure — self from the fetched-character equipment, peers from their wire `RoomMember.equipment` — stacked through the shared `load_worn_figure` `avatar.opa` path; the launcher stages the archive source once (`--avatar-opa`) and every join restages, so the ready painter has its production caller; without a fetched character the self seat falls back to the lobby `room_avatar` (intended dual source, guarded precedence). The live session fetches `GetCharacterInfo` 2001 on channel login (tickets #30/#58; gate `live_boot_stages_everything_paint_and_dispatch_consume`), so every later join restages the self seat from live data. The no-archive fallback keeps the labeled invented grid (ADR-0003). The song dialog stages over the seats — see the musicselect row below. Still OPEN: `INSTNAME`/`NOTICE` stay unpainted (OPEN 1/2 per `caption-sources.md`). Guards: `waiting_covers_paint_per_seat_rects`, `staged_covers_name_champion_and_challenger`, `empty_covers_leave_the_staged_panels_untouched`, `team_highlight_marks_our_team_button`, `wins_box_shows_the_champion_streak`, `speed_value_anchors_to_the_speed_num_rect`, `skill_slots_show_staged_ring_ids_only`, `waiting_musicname_paints_the_catalog_title`, `waiting_avatar_slots_key_figures_by_seat`, `stage_waiting_figures_without_decodable_archive_stages_nothing`, `join_restages_and_clears_stale_seat_figures`, `waiting_staged_seat_beats_room_avatar_fallback`, `waiting_seat_avatars_paint_each_occupied_seat_slot`, `waiting_seat_avatars_skip_empty_and_locked_seats`, soak `waiting_layers_seats_over_bound_waiting_art`, shots `waiting_shot_paints_mapname_bpm_skillcount_captions` + `waiting_shot_paints_the_scripted_chat_rows`. Ticket #57 (2026-11-11): the staged top-band chrome buttons (scroll track, change-title, team A..H) pair one-to-one with their `State_Waiting.bnd` rects by the code-ordinal rule — re-derived in the guard from an independent ControlList + bind parse, not from the staging output — and no two overlap (the #40 template-at-origin family stays locked out; set clones keep their template-local rects and stay out of the band by role). Guard: `waiting_top_buttons_sit_on_their_own_block_rects`. The staged composition now renders the top bar matching the 7.55 capture (`ab-wait.png`: master-name plate, Ch./No. wells, mode plate, title, change-title button, free-user plate). OPEN (orchestrator arm): the :99 live frame showing the top buttons on separate rects — the rig run closes this ticket (ev: A27 pairing wrapper VA `0x48b09c` ev `3a869bb7…`; shots `waiting.png`) Ticket #303 (2026-11-18): the master emblem is `IDC_IMAGE_SWIAT_BOSSMARK 0x1650002F` / `Wait_BossMark.ojs`, seat-template rect `(1,1)` under EXTEND1, staged at `(38,56)`; the own wire slot no longer duplicates the creator flag onto a second seat (`docs/waiting-master-emblem.md`). Guards: `master_emblem_stages_only_on_the_creator_seat`, `master_emblem_stages_only_on_the_creator_seat_with_real_art` (both red-proved). Owner visual confirm stays OPEN. | PASS (R+G, v: b3bf2ca) (covers + captions + top-button rects; INSTNAME/NOTICE OPEN with named next steps, :99 frame OPEN with the orchestrator) |
| Input | Team toggle → `SetRoomTeam` 4004 + `Ready` 4008(0); host start; back button (gameplay room-teams: `FUN_004b22d0` file `0xb22d0`, ev `2c8d3964…`) | Team buttons wired (S5) and answering through their staged `.bnd` rects: the toggle-group map (`FUN_0048c810`) lights exactly the pressed team, ungated joins queue `SetRoomTeam` 4004 then `Ready` 4008 in exe order; host start runs the START readiness gate (master + every occupied peer ready) into Loading; BACK leaves for rooms; art-less clicks land nowhere (no pinned fallback geometry). Guards: `click_staged_team_button_queues_4004_then_4008`, `team_select_lights_joined_and_queues_4004_then_4008`, `team_select_without_room_lights_only`, `team_broadcast_guards_slot_below_eight`, `waiting_hand_session_joins_team_readies_and_starts`, `waiting_confirm_hit_needs_staged_buttons`, `waiting_backs_out_to_room_list`. Still OPEN: the gate-helper table contents behind `FUN_00556680` (file `0x16680`) stay untraced (next step: REA trace of the handler table plus a live SendInput team-click sweep per the `~/scratch/planet-verify/` recipe); the 4004 byte-vs-dword builder case (next step: builder-switch dossier for opcode `0xfa4` plus a live gateway probe per the `SetRoomTeam` note in `o2jam-net/src/play.rs`); exe key codes for the team/confirm/back keys (next step: SendInput sweep on the live room) (ev: B27 `0xb22d0`/`0x48c810`/`0x556680`, ev `2c8d3964…`; guard `click_staged_team_button_queues_4004_then_4008`) | PASS (R+G, v: 9692d41) (click path; gate helper + builder width + key codes OPEN with named next steps) |
| Timing | `MusicLoading.ojs` + song clock arming at load (lifecycle §9.5) | n/a (see Loading) (ev: lifecycle §9.5; the LOADING.TIME gate-leg rig run) | OPEN (R+G, v: 9692d41) (next step: the Loading Timing gate-leg timing run below — same rig recipe) |
| Transitions | `Waiting → Loading`, `Waiting → Room` | Implemented + tested (guards: `waiting_hand_session_joins_team_readies_and_starts`, `waiting_backs_out_to_room_list`, `flow_allows_every_documented_edge`) (ev: guard `waiting_hand_session_joins_team_readies_and_starts` + `flow_allows_every_documented_edge`) | PASS (R, v: 9692d41) |
| Gameplay | 8 seats, team 0..7, winner art only (no per-team panel) | Seats + ready flags; champion/challenger roles, the wins box, and the team highlight paint from live room data (guards above); no per-team panels beyond the highlight. Live fold gap, narrowed 2026-11-09: the guest toggle folds into game slots and self-seat reads slot readiness, but live snapshots number members off the slot indices (wire: member 1, no slot 1), so the fold misses silently — paint proven, lookup convention OPEN (next step: gateway replay pinning the member→slot convention). Scoping #72 (2026-11-12): the room-team tie-break display order is minted as claim D26 and rides evidence issue #94 — static sweep 2026-11 sharpened it: no in-exe team-total comparison exists, the winner rides the server-written win-flag byte, display order is fixed seat/button order (ev: B42 member table `0x77ee90` stride `0x2190` ev `b16e614c…`; D16 reader sites `0x159d38`–`0x159f82`; result-variants.md §Team tie-break) | OPEN (R+G, v: 9692d41) |
| Music-select | Song dialog decl `0x7A`/134 entries (`DAILOG_SELECTMUSIC`, `CDSelectMusic`, lifecycle §8); host pick commits the room song (`MUSELECT 0x16200003` drops non-master clicks, file `0xb2622`); rows name the catalog songs | Ticket #15 (2026-11-11): the dialog stages over the seats through the bound arm (shared ordinal pairing — the every-bound-entry proof now covers `DAILOG_SELECTMUSIC`: 154 entries paint on their own rect origins, incl. the EXTEND rows behind the staged table bands; the multi-layer probe fix attributes overpaint to the topmost inked layer). A master row click commits the room song — the row's song id plus its difficulty slot become the `SetRoomMusic` 4000 body queued for the session bridge, the room state takes it at once (title/speed captions repaint), and only the picked row's title inks the active orange (stand-in highlight, exe rendering unpinned). Catalog rows read the CP949 decode (`docs/file-formats/ojn.md` already pinned CP949 titles; the parser read UTF-8-lossy tofu — fixed, so table/panel/caption ink carries real glyphs through the embedded Noto face). Catalog order, easy-first, ten visible rows; per-chart level/grade stay blank (never staged), 다운/듣기/정보 icons blank (no staged art). Guards: `select_row_hit_maps_the_search_table_bands`, `click_staged_select_row_commits_song_and_queues_4004`, `select_row_click_without_master_or_art_commits_nothing`, `waiting_hand_session_picks_song_then_starts`, `picked_select_row_highlights_its_title`, side `select_music_songs`/`select_music_rows` family, shots `musicselect_shot_repaints_waiting_with_song_panel` + `musicselect_fallback_catalog_lists_four_synthetic_songs` + `musicselect_dialog_text_layer_inks_the_staged_tab_slot` + `song_dialog_buttons_paint_their_staged_bodies`; smoke `docs/smoke/selectmusic-hand-session.md` (offline hand session: waiting→waiting→loading, 4000 staged). Ticket #272 (2026-11-16, owner playtest): the dialog's black rectangles were our own structure plates — caption scrims painted over the bound button bodies. The overlay is now glyphs only (`paint_select_music_text`): tab counts, status, pager and per-row title/artist/level at the sheet's `TEXT` slots (their `.bnd` rects; row slots = band origin + the `SET 0x0B` template rect), with the filter buttons, table header, INDEX letters and OK pill painted from their OPI bodies. The sheet's two hidden families (the random panel at the same rects as the STEP1 column, and the three `*GuidImg` help sheets) no longer stage (`Dialog::hidden_sprites`, `SELECT_MUSIC_HIDDEN_AT_OPEN`), which is what the reference capture shows. Still OPEN: the mode/help switch handlers behind `0x2720001A/1B` and `0x27200053/55`; the row note/grade icon frame per chart; dialog chrome clicks fall through (filters/pager/search/chosung/OK unevidenced — next step: rig chrome sweep plus captures, or dossier offsets); picked-row highlight rendering (next step: live row-click capture per the `~/scratch/planet-verify/` recipe); the queued 4000 has no session drain yet (next step: gateway replay through `SetRoomMusic` 4000 confirming the room echo); exe key codes (next step: SendInput sweep on the live dialog). Scoping #73 (2026-11-12): the xTd paired table (`0x2454d8`–`0x2455dc`, `?Td/?Sd/?Bd` codes + u16 rows + pointer table — the key-name-table guess is refuted by C18) is minted as claim C39; its consumer classification (#95) decides whether a cell is minted here or the table is ruled out as launcher/patcher-only (ev: `MUSELECT 0x16200003` file `0xb2622`; shots `musicselect_shot_repaints_waiting_with_song_panel`; smoke `docs/smoke/selectmusic-hand-session.md`) | PASS (R+G, v: 9692d41) (dialog + host pick; chrome + highlight rendering + wire send + key codes OPEN with named next steps) |

## #372 matched residual pass (2026-11-19)

Two player-visible residuals from the p343 annotated pack are fixed:

- **`top_header`**: the staged `Wait_RoomNumber.ojs` body has ten glyph
  layers, and the old backdrop pass stacked all of them in each well,
  so the wells read `Ch.99 / No.999`. `paint_waiting_header_numbers`
  now paints one selected layer per `CHNUM_10/01` and `RMNUM_100/010/001`
  well, from the resident 1003 `channel_id + 1` and the accepted 3002
  room number staged by `OnlineSession`. `Wait_Premium.ojs` and
  `Wait_FreeUser.ojs` no longer stack unconditionally: the exe base
  frame shows neither, and no wire field arms them here. Guards:
  `waiting_header_wells_show_one_wire_digit_each`,
  `waiting_header_class_chips_stay_unpainted_without_a_source`,
  `resident_channel_stages_the_waiting_header_number`,
  `room_confirm_joins_pending_room_and_enters_waiting`.
- **`seat_grid_left`**: `staged_seats` used to put a synthetic self seat
  on cover 0 and shift wire slots by one. The 3003 snapshot gives
  `member_id` without a guest member row, so the exe seats the peer
  slots by their own `slot.index` and the local member at cover
  `member_id`. The master star, PC badge and READY plate now land with
  the slot-0 master on the left and the guest on the right. Guards:
  `guest_sees_the_wire_master_on_the_left_cover`,
  `guest_self_seat_lands_at_its_member_id_cover`.
- **Symmetric member visibility (#379)**: the guest reads the master
  from 3003 slots; the master's 2006/2003 host snapshot had no slot rows
  and its drain skipped 3004 `UserJoinWaiting` (0x0BBC), so a joining
  peer never appeared. The drain now decodes 3004/3007/4009, seats and
  clears peers, and the host snapshot carries seven empty covers.
  Guards: `master_seats_the_joining_peer_from_the_3004_broadcast`,
  `joining_peer_seats_the_master_from_the_3003_answer`,
  `leave_broadcast_removes_the_{peer,master}_seat_on_the_{master,guest}`,
  `ready_echo_lights_the_{joining_peer,master}_seat_on_the_{master,guest}`,
  `waiting_back_sends_the_room_leave_on_the_live_gateway`. Live taps and
  the screenshot pair: `~/scratch/p379-live/`
  (`master-after-join.png`, `guest-after-join.png`,
  `master-after-ready.png`, `master-after-leave.png`). OPEN: a master
  leaving promotes `new_master_id` on the wire; our guest clears the
  departed cover but does not swap its START/READY role. Next step:
  gate that role swap against a live host-leave capture.

Matched rerun against the p343 exe frame: `top_header` SSIM 0.809921 →
0.859627, `seat_grid_left` 0.878300 → 0.893151. Both stay below 0.95
for named presentation residuals: glyph size/rasterization on the same
WINEJOIN title and manner-level caption (owner decision e8d01), and
worn-avatar bodies/rings plus name-plate ink. No seat-order or badge
residual remains.

Post-rebase note: `origin/main` later made stored colorkey `0` opaque in
`o2jam-render`, so a fresh matched leg now shows a full-width black
`Lanking_NoticeBoard.oji` band over the header (`top_header` 0.451423).
That band is outside #372; the pre-rebase build of this patch scored
0.859627 with no band. It needs its own exe pin before any verdict.

## #524 role-owned song rect (2026-11-20)

The owner decision from #278 stands: the master paints
`IDC_BUTTON_SWAIT_MUSELECT 0x16200003` / `Wait_MusicSelect.ojs` (the blue
곡선택 button) at bound `.bnd` rect 2, and a peer paints
`IDC_IMAGE_SWAIT_COVERMUSIC 0x1650001d` / `Wait_Cover_Music.ojs` (BPM
cover) at rect 28; `ALBUMMODE 0x1650001c` paints for neither role. Staging
keeps both bodies for every role; `waiting_control_armed_for_role` in
`o2jam-shell/src/screens/paint/waiting.rs` is the single home for the swap.

Live-path guard: `waiting_song_rect_shows_select_button_for_host_and_cover_for_peer`
(`recomp2jam/src/live_art.rs`, chain `main.rs::run_play` → `setup_online` →
`OnlineSession::stage_live_art` → `stage_live_scenes` →
`stage_interface_scenes(State::Waiting)` → `ScreenGame::paint_at` →
`paint_waiting` → `paint_waiting_scene` → the role filter). It compares the
whole staged song region against in-process references that keep one named
body at a time; no capture pixel is baked in. Seam guards:
`waiting_song_rect_bodies_swap_with_the_role` and
`host_paints_the_select_button_while_peer_keeps_the_bpm_cover`.

Red-proof (re-run on this branch): with the two role arms replaced by
unconditional `true`, the live guard fails at the host/peer region
comparison; restored arms pass. The exe display rule remains **OPEN**
(#312): no qualified capture pins what the 7.55 exe paints in this state,
so this note records the owner-decided shell rule only.

Story #550 (MUSELECT opens for the master while peers keep the BPM
cover) is carried by the same live guard and by the seam guards
`waiting_mu_select_opens_song_dialog_for_master` and
`waiting_mu_select_is_noop_for_non_master` in the shell. The static pins
are the MUSELECT handler at file `0xb2622` and the role-2 popup gate at
file `0xaaee0`; the qualified exe master dialog frame stays OPEN (#312).

## #538 master sees the joining peer (2026-11-20)

The parked fix from #379 landed as `75c81be` on `cb2/tt-379`: the master's
host snapshot carries seven empty slots, and the waiting drain decodes
`UserJoinWaiting` 3004 (0x0BBC) into cover `member_id`. Live guards:
`master_seats_the_joining_peer_from_the_3004_broadcast` and
`joining_peer_seats_the_master_from_the_3003_answer`
(`recomp2jam/src/session.rs`, chain `OnlineSession::paint` →
`poll_waiting_chat` → `drain_waiting_chat` → `apply_user_joined` →
`ScreenGame::set_joined_room` → `paint_waiting_scene`). Both are red-proved
in `~/scratch/p379-findings.md` (evidence batches 3–5).

Qualified two-client Rust-client capture (`~/scratch/p379-live/`, private
Xvfb :381/:382, two socket taps, audio contained; `--claim none`, so no
exe-oracle claim):

- tap `tap-master.log` sha256
  `d66d693db0166cf80a94db46cd0347dc25caedff48560fc9e7baf329b8201ca7`,
  line 27 at 12:51:52.034: `S2C 3004 (0x0BBC)` len 103, member 1
  `tester2`, team 1.
- master pixel frame `master-after-join.png` sha256
  `d42617e16df0f108c81f98878f4b535406fa8e55d55495bb2b86de66cda7d219`
  names `P.ly:tester2` on cover 1; the peer frame
  `guest-after-join.png` sha256
  `2ac57af46b300caa2b677019e8fc427175597fbedcb792db5dfabb3b3dc06856`
  shows the master on cover 0.

So the roster half of the story is evidenced on both clients with the
matching wire frame. The peer's worn figure is a separate residual: #570
owns the red status icon where the peer body sits. No qualified exe pack
exists for this leg; the exe display rule stays OPEN per #312.

## #414 native two-Confirm drive (2026-11-20)

One native-client run with a pass-through gateway tap (`15020 -> 15010`,
`--claim none`; no exe frame is claimed). Pack
`~/scratch/waiting-414/afk01/` (`rig-afk`, display :150, private null sink,
`route=sink`, `on_default_sink=0`, `preboot_breach=0` in `audio/manifest`):

- scene sequence `planet → channel → planet → channel → room → waiting →
  loading → playing` (`artifacts/scene-sequence.txt` sha256
  `d5ad2bf9fc63d33605991040074c2d3537c2457457df6bd27376f12ee56c3791`);
  the extra planet/channel pair is the NORMAL-mode popup dismissal leg.
  `frame-000026-playing.png` sha256
  `e05c0812148827b4636068aa6f0444758a8acb50e499db3255b404d89f58fbbd`
  shows the Highway and HUD in Playing.
- wire slice `~/scratch/waiting-414/wire-chain.log` sha256
  `ec05197952acfd09c3456d0d28c5eecb801d0630c762edf6cd80d5a1552ee312`:
  `C2S 4000 len=4` → `S2C 4001`, `C2S 4002` → `S2C 4003`,
  `C2S 4030` → `S2C 4031` (02:43:04), then `C2S 4010` → `S2C 4011`
  (02:43:22), `C2S 4012 len=5` → `S2C 4013`, followed by 4014/4015 and
  2007. This is the two-Confirm chain from t280 and
  `scripts/planet-to-waiting.sh`, now in one native run that reaches
  Playing.

This is a native-drive pin only. No qualified exe frame covers the
sequence; the exe comparison stays OPEN per #312.
