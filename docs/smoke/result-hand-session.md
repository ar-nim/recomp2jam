# Result hand session — smoke note (2026-11-11)

Offline shell run (no gateway): the hand drives Confirm through the
public key seam (`ScreenGame::menu_key(MenuKey::Confirm)` — the same
edge the session bridge feeds) for the lobby walk, strikes the
scripted one-note chart once at its onset (existing song-end edge —
unmodified, final-boss ticket owns gameplay), then clicks the staged
RETRY/BACK buttons at their bound rects. Sound contained: no audio
rig staged, no backend attached.

## What was done, what happened

Walk Avi→Planet→Channel→Room→Waiting→Loading→Playing on the
scripted chart, strike once, paint to song end: Result with the
solo banner (frame 4, image 3), one rank row (`YOU`, `1/0/0/0`),
and the walked grade plate. RETRY click at (666,581) hands back to
Waiting; a fresh walk to Result plus BACK click at (752,582)
leaves for the room list. A miss click at (0,0) lands nowhere.

Evidence: screen path `result → waiting` (retry) and
`result → room` (back), state hashes `1c6914d5e9018cc1`
(Result) → `58c8f8afdeb0e08a` (Waiting) and `b83332e71853c9d5`
(Room). Guards: `staged_retry_click_returns_to_waiting`,
`staged_back_click_returns_to_room`,
`result_keys_mirror_the_staged_buttons`,
`result_entry_stages_no_ticket_wire_call`
(plus the avatar-slot guards `result_avatar_paints_at_the_local_row_slot`,
`result_avatar_clips_to_the_exe_row_band`,
`result_avatar_hangs_its_visible_box_at_the_slot`,
`result_paints_loaded_avatar_at_the_row_slot`).

## Deltas found (new rows, never silent fixes)

1. **Avatar leaves the frame center for the exe row slot.** Member
   avatars sit at x=78, y=316+22·row under a 652×22 clip (row
   painter `FUN_004c3b40`, `docs/cresult-anchors.md`
   `ev_88c3ca31…`); the shell stages one avatar — the local
   player's — so it hangs its visible box at the row-0 slot (new
   `AvatarFigure::visible_origin`, same adaptation family as the
   Playing extent-centered seats). Shots delta: `result*.png`
   carry the slot ink (row-0 band x 90..126, y 316..338 native)
   with the old center ink gone — the only oracle deltas.
2. **Peer rows paint no avatar (OPEN).** The member-table avatar-id
   byte rides server-side (relay-adjacent); no portrait bodies are
   staged. Next step: gateway replay pinning the member→avatar-id
   convention plus a rig capture of a populated battle row, then
   stage portraits per row.
3. **Solo event-ticket web call stays unwired (OPEN).** The enter
   painter fires it outside battle mode
   (`docs/cresult-anchors.md`, `FUN_004c1b90`); the shell has no
   web seam, so staged entry queues nothing (guarded). Battle
   `0xfca` post plus retry `0xfaa` / back `0xfb5` relay events are
   likewise local-only. Next step: live gateway replay naming the
   wire shape, then queue the bodies through the session bridge.
4. **Grade letters stay unpainted (OPEN).** The plate runs the
   pinned threshold table (`grade_frame`, `FUN_004c4dd0`) at
   (395,531); grade strings carry zero `.text` refs (full-image
   LE32 scan) — result grades are `.ojs`-driven. Next step: live
   rig capture of the grade plate at controlled score bands.
5. **Life-readout and bottom-counter digit cells stay unpainted
   (OPEN).** Pinned geometry (24 px pitch, digit-1 +5 kern,
   `ev_615e034c…`) but the font-body binding is inference
   (`PointNum.ojs` metrics match, no located reader). Next step:
   REA trace of the digit-writer callers at files `0xc44c0`/
   `0xc4630` pinning the body, per `docs/cresult-anchors.md`
   OPEN 1.
6. **Rig photo check timeboxed.** The reference photo is the
   battle variant; band-level luminance could not separate the
   avatar column from panel art without the alignment pipeline.
   The slot rests on the static dossier pins above, not on a new
   capture. Next step: av-compare pipeline alignment of a live
   battle-row capture when a gateway replay is available.
