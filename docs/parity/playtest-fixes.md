## Playtest fix pass (2026-11-10) — hand-playtest tickets #24–#41

Work landed on `main` after the hand playtest. Each entry names the
guard tests that hold it and what stays OPEN with its next step.

- **Frame clear (tickets #25, #39).** Every frame now starts on device
  black (`FRAME_CLEAR_COLOR` in `ScreenGame::paint_at`). Pinned by the
  7.55 planet capture `/home/arnim/scratch/planet-shot.png`: it is black
  in exactly the 30 725 px (6.40 %) the staged `STATE_PLANET` art
  leaves uncovered (100 % of the hole mask). Stale room ink under the
  board and white splash patches are gone. Guards: the channel
  no-fill pair (`empty_channel_art_paints_no_fill`,
  `empty_channel_art_clears_stale_splash_pixels`).
- **`SET` template clones (tickets #35, #40, #28 rows).** The exe
  instantiates `SET` children per parent control and writes back
  parent-offset rects (`docs/ui-loader.md` step 8, helper `0x5487B0`);
  staging now expands the same clones (`PlacementRole::Clone`,
  `avatar::expand_set_clones`) and scenes paint only the clones of live
  rows/seats. Measured against `ac-room.png` (room members at
  panel + rect, correlation `+0,+0`) and `ad-wait.png`
  (`Wait_BossMark` (38,56), `Wait_PCRoomMark` (131,56),
  `Wait_ReadyText` (60,183)). Guards:
  `room_number_clones_paint_their_digit_frames_in_place`,
  `room_icons_paint_their_frames_at_the_rect_origins`,
  `every_bound_entry_paints_at_its_placement_origin` (covers every
  bound entry of the real archives), `waiting_seat_clone_live`
  behaviour via the waiting guards below.
- **Shared sprite bodies position by rect (ticket #40).** A body used
  by several placements (`Wait_RoomNumber.ojs` feeds five digit wells)
  cannot author an absolute anchor, so its rects position it
  (`avatar::shared_sprite_names`).
- **No scene title over genuine art (ticket #36).** The `ROOM`/`CHANNEL`
  stamp paints only on the art-less fallback; the room capture carries
  no such text.
- **Waiting seat indicators (ticket #40).** Seat-template bodies reach
  the frame only for occupied seats and each armed indicator selects
  its own frame from wire data (`SeatIndicator`: team plate frame =
  team + 1 (eight layers = teams A–H), master star, ready plate,
  no-song mark, playing banner). The local player's worn figure paints
  in their seat slot, and since 2026-11-11 every occupied peer seat paints its staged worn figure (launcher `--avatar-opa` source, restaged on every join; the painter was ready, now it has its production caller) — the live self seat still rides the `room_avatar` fallback until the session fetches character data (OPEN next step above). Bodies with no pinned flag stay dark with named next steps
  (PC-room mark, download pair, P2P banner, gem mark).
- **Room user panel (ticket #30).** The nickname plate
  (`Lv.<level> <name>`, as `ac-room.png` shows), level, exp, cash
  point, penalty count/level wells and the worn figure all paint from
  `CharacterInfo` 2001; the grade and band wells carry the exe's own
  `준비중..` placeholder. OPEN: `Next Up Exp.` (level-up table) and
  `MYRANK`/`NOTICE_TEXT`/`DESC` (web-served).
- **Room chrome dispatch (tickets #32, #34).** One control table
  (`ROOM_CONTROLS`) replaces the three hand-written arms: ITEMSHOP,
  MYROOM, MUSICSHOP and MYBAND open their side views; every other
  staged button records its press with its next evidence step instead
  of dropping it. `STATE_ROOM` carries **no** tutorial control, so #34
  closes as "not a room control" with the reporter question posted.
- **Room lounge chat (ticket #37).** The room lobby shares the
  main-room feed with the Channel scene (`LobbyChatPanel`, 2012 out,
  2013/2014/2016/2019/2029 in) and paints inside the staged
  `IDC_LIST_SROOM_CHAT`/`IDC_EDIT_SROOM_CHAT` rects. OPEN: chat tab
  filters and the row pitch (borrowed from the waiting box).
- **Scene BGM keeps a sounding record (ticket #27).** A cue that repeats
  the latched record in the same repeat mode at the same volume leaves
  the running track alone (`SceneBgm::cue` -> `CueOutcome::KeepRunning`),
  so Planet -> Channel no longer restarts record 9. Guards:
  `emblem_round_trip_keeps_the_shared_board_track_running`,
  `different_track_and_silence_still_cue_on_the_scene_change`.
- **Pointer state (tickets #33, #26).** `GameLoop::mouse_moved` carries
  native pixels into the game; the channel select bar follows the
  hovered row and falls back to the keyboard highlight (both halves
  live-confirmed). Room hover (ticket #33, 2026-11-11): the resolver
  maps the pointer onto staged room chrome (`hovered_room_control`,
  dialogs/modals/sides cover it first) and the painter repaints the
  hovered button at full body frame 0 — the rig-pinned hover state
  (`docs/hover.md` 2026-11-10 sweep: 17 of 18 buttons match frame 0 at
  100%; the active filter/tab keeps frame 1). Row panels stay static
  by negative evidence (0 changed px, no `Room_MOver.ojs` overlay).
  The rest-state desaturation transform stays OPEN (measured, not
  explained — next step: stopped-state winedbg draw-state read).
  Planet hover frames stay OPEN for ticket #26, which consumes the
  same pointer state. Guards: `hovered_room_control_resolves_the_staged_chrome_button`,
  `hovered_room_control_clears_off_the_room_screen`,
  `hovered_room_button_repaints_its_first_frame`,
  `room_row_panels_stay_static_under_the_pointer`,
  `room_hover_hides_behind_the_open_dialog`.
- **Splash hold (ticket #24).** The Avi cue is record 8 (`bgLogo.ogg`,
  3.923 s by `o2jam-dump` + decode timing), so the 3 s stand-in cuts
  0.92 s early. The hold length and the arming event stay OPEN: the two
  no-input boot captures (`~/scratch/avi-hold/`) show a white desktop,
  not the logo, so frames alone cannot pin it. Next step: boot with a
  reachable gateway, >= 10 Hz frames, and a live state-id read per
  frame.
- **`e2e` evidence (ticket #41).** The walk rasterizes the native
  800x600 canvas and box-downscales to 400x300 like the `shots` walk; it
  used to paint straight into a small frame, which drew staged art at
  double size and clipped it (false scene evidence).
