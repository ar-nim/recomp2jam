## `STATE_WAITING` — pre-game room (`CStateWaiting`/`CLiveWaiting`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x63`, 117 entries, `State_Waiting.bnd`; 8 team buttons `0x16201004..0B` (`Wait_TeamA..H.ojs`), 8 avatar covers; song dialog decl `0x7A`/134 entries (lifecycle §8, gameplay room-teams) | Soak binds 116 rects verbatim under genuine art — `waiting.png` shows real avatar tiles with seats + `WAITING` stamp on top. Ticket #14 (2026-11-10): seats stage at the verbatim cover rects (`COVERAVATAR1..8`, rects 60..67) with live seat lines — occupied covers take the scrim plus role/level/name/ready line, empty covers stay untouched staged panels, locked covers name `LOCK`. Team highlight rides the staged `Wait_Team?.ojs` button rects (rects 3..10). Live header captions ink their verbatim `.bnd` rects: MAPNAME (map-cycle entry of the room word), BPM (catalog column of the song, `-0-` on mismatch), SKILLCOUNT (`7/42/999`-capped inventory words), MUSICNAME (catalog title else `M{id}`), WINNERSCORE (wire streak), SPEED_NUM (`X0.5`..`XR` chain), 1/2/3SKILL ring ids; chat rows plus the focus caret paint at the LIST/EDIT rects (file-traced rects 42/19). Ticket #40 (2026-11-11): every occupied seat paints its own staged worn figure — self from the fetched-character equipment, peers from their wire `RoomMember.equipment` — stacked through the shared `load_worn_figure` `avatar.opa` path; the launcher stages the archive source once (`--avatar-opa`) and every join restages, so the ready painter has its production caller; without a fetched character the self seat falls back to the lobby `room_avatar` (intended dual source, guarded precedence). The live session fetches `GetCharacterInfo` 2001 on channel login (tickets #30/#58; gate `live_boot_stages_everything_paint_and_dispatch_consume`), so every later join restages the self seat from live data. The no-archive fallback keeps the labeled invented grid (ADR-0003). The song dialog stages over the seats — see the musicselect row below. Still OPEN: `INSTNAME`/`NOTICE` stay unpainted (OPEN 1/2 per `caption-sources.md`). Guards: `waiting_covers_paint_per_seat_rects`, `staged_covers_name_champion_and_challenger`, `empty_covers_leave_the_staged_panels_untouched`, `team_highlight_marks_our_team_button`, `wins_box_shows_the_champion_streak`, `speed_value_anchors_to_the_speed_num_rect`, `skill_slots_show_staged_ring_ids_only`, `waiting_musicname_paints_the_catalog_title`, `waiting_avatar_slots_key_figures_by_seat`, `stage_waiting_figures_without_decodable_archive_stages_nothing`, `join_restages_and_clears_stale_seat_figures`, `waiting_staged_seat_beats_room_avatar_fallback`, `waiting_seat_avatars_paint_each_occupied_seat_slot`, `waiting_seat_avatars_skip_empty_and_locked_seats`, soak `waiting_layers_seats_over_bound_waiting_art`, shots `waiting_shot_paints_mapname_bpm_skillcount_captions` + `waiting_shot_paints_the_scripted_chat_rows`. Ticket #57 (2026-11-11): the staged top-band chrome buttons (scroll track, change-title, team A..H) pair one-to-one with their `State_Waiting.bnd` rects by the code-ordinal rule — re-derived in the guard from an independent ControlList + bind parse, not from the staging output — and no two overlap (the #40 template-at-origin family stays locked out; set clones keep their template-local rects and stay out of the band by role). Guard: `waiting_top_buttons_sit_on_their_own_block_rects`. The staged composition now renders the top bar matching the 7.55 capture (`ab-wait.png`: master-name plate, Ch./No. wells, mode plate, title, change-title button, free-user plate). OPEN (orchestrator arm): the :99 live frame showing the top buttons on separate rects — the rig run closes this ticket (ev: A27 pairing wrapper VA `0x48b09c` ev `3a869bb7…`; shots `waiting.png`) Ticket #303 (2026-11-18): the master emblem is `IDC_IMAGE_SWIAT_BOSSMARK 0x1650002F` / `Wait_BossMark.ojs`, seat-template rect `(1,1)` under EXTEND1, staged at `(38,56)`; the own wire slot no longer duplicates the creator flag onto a second seat (`docs/waiting-master-emblem.md`). Guards: `master_emblem_stages_only_on_the_creator_seat`, `master_emblem_stages_only_on_the_creator_seat_with_real_art` (both red-proved). Owner visual confirm stays OPEN. | PASS (R+G, v: b3bf2ca) (covers + captions + top-button rects; INSTNAME/NOTICE OPEN with named next steps, :99 frame OPEN with the orchestrator) |
| Input | Team toggle → `SetRoomTeam` 4004 + `Ready` 4008(0); host start; back button (gameplay room-teams: `FUN_004b22d0` file `0xb22d0`, ev `2c8d3964…`) | Team buttons wired (S5) and answering through their staged `.bnd` rects: the toggle-group map (`FUN_0048c810`) lights exactly the pressed team, ungated joins queue `SetRoomTeam` 4004 then `Ready` 4008 in exe order; host start runs the START readiness gate (master + every occupied peer ready) into Loading; BACK leaves for rooms through the pinned 3005 body (#734); menu-edge ESC Back stays ours-by-design (#735 statics find no Waiting scene key handler for the transition); art-less clicks land nowhere (no pinned fallback geometry). Guards: `click_staged_team_button_queues_4004_then_4008`, `team_select_lights_joined_and_queues_4004_then_4008`, `team_select_without_room_lights_only`, `team_broadcast_guards_slot_below_eight`, `waiting_hand_session_joins_team_readies_and_starts`, `waiting_confirm_hit_needs_staged_buttons`, `waiting_backs_out_to_room_list`. Still OPEN: the gate-helper table contents behind `FUN_00556680` (file `0x16680`) stay untraced (next step: REA trace of the handler table plus a live SendInput team-click sweep per the `~/scratch/planet-verify/` recipe); the 4004 byte-vs-dword builder case (next step: builder-switch dossier for opcode `0xfa4` plus a live gateway probe per the `SetRoomTeam` note in `o2jam-net/src/play.rs`); exe key codes for the team/confirm/back keys (next step: SendInput sweep on the live room) (ev: B27 `0xb22d0`/`0x48c810`/`0x556680`, ev `2c8d3964…`; guard `click_staged_team_button_queues_4004_then_4008`) | PASS (R+G, v: 9692d41) (click path; gate helper + builder width + key codes OPEN with named next steps) |
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

## #570 joined peer figure on the master seat (2026-11-20)

The roster wire half is already pinned by the 3004 guards in `session.rs`
(`master_seats_the_joining_peer_from_the_3004_broadcast`,
`join_broadcast_raises_the_headcount_from_the_pinned_3004_member_id`),
and the live two-client Rust capture `~/scratch/p379-live/` shows the
peer's name on cover 1. The remaining spec-row gap was paint parity:
the master must draw the figure in that seat, not just the status mark.

Guard: `joined_peer_figure_paints_in_the_master_seat_slot` (`session.rs`).
Live chain: `OnlineSession::paint` → `poll_waiting_chat` →
`drain_waiting_chat` → `apply_user_joined` →
`ScreenGame::seat_waiting_member` → `GameLoop::paint` →
`ScreenGame::paint_at` → `paint_waiting` → `paint_waiting_seat_avatars`.
The test stages a synthetic one-pixel figure and synthetic seat art, so
the hidden-install-root arm is N/A; the wire-to-seat staging is guarded
separately. RED proof: removing the `paint_waiting_seat_avatars` call
makes the seat pixel stay unpainted; restoring it passes.

No qualified exe frame pair covers the peer-figure pixels in this state;
the exe display rule stays OPEN per #312.

## #734 live BACK click sends the room leave (2026-11-21)

The #732 guard chain (scripted `menu_key(Back)` -> 3005) stayed green
while the real click arm only advanced the screen. The live path goes
`main.rs::run_play` -> `WindowHandler::deliver_snapshot` ->
`OnlineSession::ingest_snapshot` -> `dispatch_pointer_edge(Local)` ->
`ScreenGame::apply_pointer_down` -> `click_waiting_button`; that last
arm now queues the W8 `ExitWaiting` 3005 body (u16 exit type 0) on the
staged BACK `.bnd` rect; the click advances the shell at once and
`OnlineSession::poll_wire` sends the queued body before that frame's
wire drains. The Menu edge keeps its existing
direct send; both arms share `OnlineSession::send_waiting_leave`.

Guards: `waiting_back_click_sends_the_room_leave_on_the_live_snapshot_path`,
`waiting_back_click_disband_answer_removes_the_room_row`,
`waiting_back_click_disbands_the_staged_pick_before_a_reentry`,
`waiting_back_menu_edge_sends_the_room_leave_once`. All use a synthetic
gateway and staged rects, so the hidden-install-root arm is N/A.
RED proof: with the queue arm removed, the click guards fail (no 3005,
no 2007 row removal) while the scripted #732 guards stay green.

## #578 peer READY and chat on the master (2026-11-20)

Wire pins from the bootless scripted leg:
`~/scratch/close-wr/wire-only/peer.log` CP33 `S2C 4009 len=2 hex=0101`
(15:51:32) and CP34 `S2C 3012 len=16
hex=74657374657232005749524532393101` (15:51:37) — sender `tester2`,
text `WIRE291`, seat 1. The two-client frame pair is still OPEN (boot 5
died after CP06 on an observer timeout); these guards pin the wire
receive half only.

Guards in `session.rs`:
`peer_ready_echo_from_cp33_opens_the_master_start_gate` — live chain
`OnlineSession::paint` → `poll_waiting_chat` → `drain_waiting_chat` →
`apply_ready_echo` → `ScreenGame::set_member_ready` →
`start_gate_verdict` (`PeerNotReady` → `Startable`).

`peer_chat_from_cp34_lands_as_a_master_waiting_row` — live chain
`OnlineSession::paint` → `poll_waiting_chat` → `drain_waiting_chat` →
`apply_waiting_user_message` → `ScreenGame::push_waiting_user_message`
→ `paint_waiting_chat`.

RED proofs: removing the `4009` arm in `apply_waiting_frame` makes the
ready guard time out; removing the `3012` arm makes the chat guard time
out; restoring both passes. No install-root read (synthetic gateway).

## #556 peer READY pair on the wire and the master's readout (2026-11-21)

CP33 (`~/scratch/close-wr/wire-only/peer.log` 15:51:32) sends peer
`C2S 4004 team=0` then `C2S 4008 ready`, followed by `S2C 4009
len=2 hex=0101` on the master (CP33 `master.log`, same instant). The
shell staged the 4004/4008 pair on a team press but no session caller
drained that outbox, so the pair never reached the gateway. The next
wire poll now sends it, and the 4009 echo updates the staged seat, the
session ready mirror, and the joined-room readout before the master's
START gate runs.

Guard `peer_ready_click_echo_opens_the_master_gate_and_mirror` — live
chain: `OnlineSession::paint` → `poll_wire` → `flush_team_send` →
`SetRoomTeam` 4004 + `Ready` 4008 (peer) → `drain_waiting_chat` →
`apply_ready_echo` → `ScreenGame::set_member_ready` /
`joined_room` mirror → `start_gate_verdict` → `confirm_master_start`
→ `StartGame` 4010. The channel-readable 2009/2003 half stays OPEN
with the next step named in #291.

RED proofs: without `flush_team_send`, the guard's peer gateway never
reads 4004 and the master times out waiting for the echo; without the
mirror write, the guard fails on the joined-room readout assertion.
Restoring both passes. Synthetic gateway; hidden install-root is N/A.

## #561 solo START face and solo verdict arms (2026-11-21)

The SOLO-IDLE cell pins a solo master's START face at (680,143)
(`~/scratch/solo-playing/s1-master-ui/waiting-master.png`,
`85decf32…`, manifest `4bd221b8…`). The avatar's bare-arm half is
already pinned by #375 (`face_only_wire_kit_stages_the_default_bare_arms`
and the waiting-seat pixel guard, commit `8de4d28`). The remaining face
gap was the role source: the painter used the local seat id, while the
session's START send uses the announced master id. A solo room handed
over to seat 1 painted READY and sent START. The painter now takes the
announced seat from `ScreenGame`.

Guards:
`solo_promoted_master_paints_the_start_face_not_the_guest_ready_face` —
live chain: `OnlineSession::paint` (3007 promotion) →
`ScreenGame::promote_waiting_master` → `ScreenGame::paint` →
`paint_waiting_scene_with_confirm` → `paint_waiting_confirm_face`.
`solo_master_gate_has_no_peer_only_verdict_arm` — live chain:
`OnlineSession::menu_key` (Waiting Confirm) → `confirm_master_start` →
`ScreenGame::press_start` → `start_gate_verdict` (the -1/-2/-3/-4/-5
arms all read an occupied peer; a solo room has none).

RED proofs: forcing the painter's master seat to 0 fails the face guard
on the armed START record; counting the local seat as a peer fails the
verdict guard on the first NoMusic scenario. Restoring both passes.
Synthetic three-record art and gateway; hidden install-root is N/A.

## #576 member removal, promotion, and final disband (2026-11-21)

Wire pins CP35/CP37/CP38 in
`~/scratch/close-wr/wire-only/{master,peer,observer}.log`: peer exit
`3007 body=0100000000000000` keeps the row; master exit with a resident
peer `3007 body=0001000000000000` promotes seat 1 without 2007; last
exit `2007 body=00000000` removes it. The CP35/CP38 branch is pinned by
`evidence_room_3007_keeps_the_row_and_2007_removes_it`; the CP37 seat
and role swap by `master_leave_promotes_the_resident_peer_to_room_master`
plus the new readout guard. The session's joined-room mirror cleared the
departed seat but left the old master flag on it, so a later re-stage
from that snapshot (a 4003 arena echo) restored the departed master.
The 3007 arm now moves the master flag in the mirror too.

Guard `leave_handoff_updates_the_joined_room_master_readout` — live
chain: `OnlineSession::paint` (Waiting) → `poll_waiting_chat` →
`drain_waiting_chat` → `apply_user_left` → `JoinedRoom::clear_member`
and the promoted-seat master flag → `ScreenGame::promote_waiting_master`.
RED proof: removing the mirror's promotion loop leaves the seat clear
but fails on the promoted member's master flag; restoring passes.
Synthetic gateway; hidden install-root is N/A.

## #559 DOWNLOAD badges — 4035 arm after #838, A/B still BLOCKED (2026-11-22)

Wire pins in `docs/music-availability.md` §2 and the #291 DOWNLOAD
cell: matched-ab-339 `4037 0004` proof `0580bc8d…`, p343 proof
`20fbe170…`, plus 4026/4037/3009 and 4039. The c958cc2 paint arm
(4037 state + 4039 progress) is REFUTED by boot-wave3: exe member frames
stay badge-free at 42 %/100 %, while the #773 proxy frame for S2C
`4035 0200000104` paints the local seat. Statics: the 4035 case in each
Waiting dispatcher (normal `0x4b1d42`, live `0x4aba51`; REA
`ev_18a485d7…` / `ev_0593bf27…`) calls the seat refresh and, for the local
member, `StartMusicDownload`; the 4037 case only refreshes/retries.

Guards: `acquire_music_4035_arms_only_the_local_badge_from_the_gateway`,
`ready_4035_clears_the_local_badge_from_the_gateway` and
`acquire_music_4035_paints_the_local_seat_badge_and_bar` pin the arm;
`music_state_4037_and_progress_4039_leave_the_local_badge_unarmed_from_the_gateway`
and `music_state_4037_and_progress_4039_leave_the_seat_chrome_dark` pin
the negative. Live chain: session frame ingest -> `poll_waiting_chat` ->
`drain_waiting_chat` -> `apply_acquire_music` -> `answer_acquire_music`
-> `ScreenGame::arm_local_download_badge` -> `paint_at` ->
`paint_waiting_scene_with_confirm` -> `staged_seats` ->
`paint_waiting_seat_marks`. The 4039 readout guard
`ready_state_clears_the_seats_download_progress_and_opens_start` still
pins 4037 Ready clearing the wire percentage, not the paint arm.

BLOCKED: the player-visible A/B claim still needs one qualified
two-client capture with exe 7.55 (sha256 `83a04541…72c823b`) and Rust
frames of the *same* Waiting room on the 4035 arm, before either
client's START verdict, with a gateway tap showing 4035/4037/4039.
Until that pair exists, no pixel-equality claim is made and the #291
story row stays unchecked.


## #291 enrichment — key-mode auto-pick (#757)

`FUN_004b42d0` (VA `0x4b42d0`, file `0xb42d0`, EV
`ev_63f94a48…`) is the in-Waiting pick arm called from 3007
(`UserLeaveWaiting`, call VA `0x4b0bf2`) and the 0xfb6/4022 seat-state
arm (call VA `0x4b0f1c`), both in `FUN_004b0a50`. It maps key mode
0/1/2 to 3/5/7 lanes, keeps a standing song whose catalog row carries
availability bit `8` and the current difficulty's chart bit, then
walks catalog rows (stride `0x1b4`) that carry the lane byte `+0x1b0`,
the availability bit, the chart bit from the `0x680724` table
(`1`/`2`/`4`, EV `ev_920e926b…`), and a level word `+0x1a8` of zero or
at most global `0x7798bc`. A match writes the song id to `0x7789d8`
and sends 4000; a miss arms BACK `0x16200002` and posts the string at
VA `0x6498b0` (EV `ev_3f58de30…`, CP949 bytes read at EV
`ev_18132d9a…`).

`ScreenGame::auto_pick_waiting_song` mirrors that arm: the shell
catalog row carries a lane count (`0` when no local source staged),
the difficulty bit uses the pinned `1`/`2`/`4` table, and the level
ceiling defaults to "no cap" until a source is staged. A match queues
the existing `SetRoomMusic` 4000 body; a miss arms the BACK byte and
posts the pinned notice. `OnlineSession::apply_auto_pick_after_seat_change`
calls the arm from both 3007 and 4022 in `apply_waiting_frame`; the
master-only gate follows the exe's role split. Guards:
`auto_pick_uses_the_room_key_mode_lane_count`,
`auto_pick_skips_rows_that_fail_the_pinned_filters`,
`auto_pick_miss_arms_back_and_posts_the_notice` (synthetic catalog,
hidden-install-root safe), and the live-frame guard
`master_auto_picks_a_matching_chart_when_a_3007_leave_arrives`.

OPEN (ticket's next step): the writer of catalog byte `+0x1b0` and
global `0x7798bc` are untraced, so staged rows use `lanes = 0` until a
pinned source lands; no lane/level value is invented. A 3KEY/5KEY/7KEY
pick leg with a gateway tap and matched frames stays open.

## #291 enrichment — 3001 key-mode change (#758)

Case `0xbb9` of `FUN_004b0a50` (EV `ev_e1395126…`, file `0xb0fc7`)
compares new/old key-mode bytes at VAs `0x4b0ff6`/`0x4b1004`. On a
change, and only for the master mode byte, it clears its two select
vectors (stride `0x238` and stride `8`, destructors at VAs
`0x452a00`/`0x452bd0`), resets the two dialog globals, copies the
high nibble of song id `0x7789d8` into `0x77890c` (file `0xb10d2`),
and sends 4000 at VA `0x4b10de`. No live 3001 key-change frame exists:
the body layout is OPEN, and `o2jam_net::KeyModeChanged` decodes only
the one byte the shell arm needs.

`ScreenGame::apply_key_mode_change` stages that arm: clear the stale
pick/query/page, mirror the high nibble, and queue the standing room
song as 4000; an equal key mode is a no-op.
`OnlineSession::apply_key_mode_changed` calls it from the 3001 match
in `apply_waiting_frame`. Guards:
`key_mode_change_clears_stale_rows_and_resends_the_room_song` and
`key_mode_change_requeues_the_room_song_after_clearing_rows`.
## #765 role-gated chrome beyond the song rect (2026-11-21)
Static pins: `FUN_004b4a00` VA `0x4b4a00` (evidence `ev_c5b7cdce…`)
arms START, MAPLEFT/RIGHT, MUSELECT, the six `1/2/3SKILLLEFT/RIGHT`
buttons and MASTER_SKILLSEL for the master, and clears the song and
skill covers; `FUN_004b4d30` VA `0x4b4d30` (evidence `ev_1365b919…`)
disables those controls for guests, frames READY and shows the covers
with a per-slot skill frame. The song-rect display rule stays the
owner decision recorded above, and the guest's owned-item vector fill
stays OPEN (`docs/caption-sources.md` OPEN 4).
`waiting_control_armed_for_role` now owns the whole pinned set. The
live-path guard `master_paints_map_and_skill_buttons_that_a_guest_hides`
stages synthetic bodies from boot (`walk_to_waiting` → `load_scene` →
`paint_at` → `paint_waiting` → `paint_waiting_scene_with_confirm`) and
checks that the master paints all eight bodies where the guest paints
none. RED proof: without the role arms the guest painted all eight.
The seam table pin lists the three skill-cover ids as guest-only. No
qualified exe frame pair exists yet for the guest cover frame family;
that capture step stays OPEN.
## #766 map arrows and MAPNAME (2026-11-21)
Static pins: `IDC_BUTTON_SWAIT_MAPLEFT/RIGHT 0x1620000C/0D` are
master-armed by `FUN_004b4a00` (evidence `ev_c5b7cdce…`) and guest-disabled
by `FUN_004b4d30` (evidence `ev_1365b919…`). The press dispatcher calls
`FUN_004bb570` with -1/+1 (evidence `ev_c9d69d84…`, call site
`ev_d729fea2…`); the normal branch steps the room word, skips four rdata
stage words (`ev_df240aa2…`), and sends 0xfa2. `MAPNAME` still resolves
through the low room word to the pinned cycle table (OPEN 3 about the
separate `0x4ad350` applier is unchanged).
The shell stages both arrow rects with the role gate from #765. A
master press updates the joined room word from the staged `Wait_Map.ojs`
frame count (exe default 14 when no body stages), queues `SetRoomArena`
4002, and the next caption paint resolves the new word. Guards:
`master_map_arrow_click_cycles_the_room_word_and_queues_4002` (synthetic
Waiting art, no game data), `map_arrow_cycle_skips_the_pinned_stages_and_leaves_random_wrap_open`
(pure cycle table), and `map_arrow_click_sends_4002_from_the_live_boot_path`
(fake gateway, boot path through `OnlineSession::paint`). RED proof:
before the click arm the room word stayed at 7. The random wrap arm
(`FUN_004bb2c0`) stays OPEN with a live arrow-click capture at a cycle
boundary as the next evidence step.
## #767 skill selectors and counters (2026-11-21)
Static pin: `FUN_004bb760` (VA `0x4bb760`, evidence `ev_826be834…`)
decrements slot 1's selection word with wrap to `len-1` on LEFT,
increments with wrap to 0 on RIGHT, then stages the owned-item art
(id at vector `+4`) and prints its count (vector `+8`, width 3, slots
2–3 capped at 999). Slots 2/3 use the same handler shape at
`0x4bbb30`/`0x4bbf20`. The handler writes no wire field, and the
server fill of the three owned-item vectors is still OPEN
(`docs/caption-sources.md` OPEN 4).
The shell stages three local owned-item lists; empty lists keep the
existing ring-id/count sources, so no packet invents items. A staged
`1/2/3SKILLLEFT/RIGHT` click cycles the selection and the count and
id repaint from the selected item. Guards:
`skill_selector_clicks_cycle_owned_items_with_the_exe_wrap` (boot walk
-> staged rect -> `click_waiting` -> `WaitingSkillSlots::cycle`) and
`cycled_skill_slot_repaints_its_selected_id_and_count` (paint seam,
synthetic probes, no game data). RED proof: before the click arm the
selection stayed at the first item. The wire fill and
selection-change field stay OPEN with a gateway tap and a two-client
frame pair as the next evidence step.

