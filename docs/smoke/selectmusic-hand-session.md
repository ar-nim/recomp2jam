# Music-select hand session — smoke note (2026-11-11)

Offline shell run (no gateway): the master hand-drives a song pick
through the real window input seam (`GameLoop::mouse_click` on the
native 800x600 canvas — the same seam the windowed binary feeds),
against a staged dialog (`load_select_music_scene`) and a staged
catalog (synthetic `OJNList.dat` row: song 757 `First`/`Band`,
`set_song_catalog`), with the START readiness gate armed
(`waiting_room()`: master, peers ready).

## What was clicked, what happened

Waiting with the dialog staged; row 1 click (400, 184 — Normal slot
of song 757, audited table band `SELECT_ROW_TOP` 150 pitch 23)
commits the room song (`SetRoomMusic { music_id: 757, difficulty: 1,
speed: 3 }` — speed survives from the staged selection) and queues
the 4000 body for the session bridge (`take_music_send`). The table
repaints with only the picked row's difficulty letter in the
active-tab orange (stand-in highlight). Confirm then runs the START
readiness gate into Loading.

Evidence: screen path `waiting → waiting → loading`, state hashes
`7792a697e1a94dfe` (pre-pick) → `b8c43476c182a3f1` (post-pick) →
`d85c781b256218be` (Loading held). Guard:
`waiting_hand_session_picks_song_then_starts` (plus
`click_staged_select_row_commits_song_and_queues_4004`,
`select_row_click_without_master_or_art_commits_nothing`,
`picked_select_row_highlights_its_letter`).

## Deltas found (new rows, never silent fixes)

1. **Picked-row highlight renders the active-tab orange stand-in.**
   The exe's picked-row rendering is unpinned — no capture pins it.
   Next step: SendInput row-click sweep on the live dialog per the
   `~/scratch/planet-verify/` recipe, capture the picked row, then
   replace the stand-in.
2. **Dialog chrome clicks (filters, pager, search, chosung, OK) fall
   through to the seat buttons below.** No consumer pins the chrome,
   so it stays display-only and BACK stays reachable. Next step: rig
   sweep of each chrome cell (filter press, pager step, OK commit)
   plus captures, or dossier offsets for the chrome handlers.
3. **The 4000 body queues shell-side only.** `take_music_send` stages
   the send like the team's 4004/4008 pair; no session drain sends
   it yet. Next step: gateway replay draining the queued pick
   through `SetRoomMusic` 4000 (plus 4002/4030 follow-ups per
   `OnlineSession::select_song`) and confirming the room echo.
4. **Exe key codes for the dialog stay unpinned** (same OPEN as the
   team keys). Next step: SendInput arrow/Enter sweep on the live
   dialog, reading the highlight via captures.
