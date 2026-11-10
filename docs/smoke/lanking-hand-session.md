# Lanking hand session — smoke note (2026-11-12)

Offline shell run (no gateway): the board opens from Waiting through
the side-screen seam (`visit_side(SideScreen::Lanking)`), against
staged side art (`load_side_scene` with the 73 genuine
`STATE_LANKING` placements) and a staged 12-song synthetic catalog
(`set_lanking_catalog`), with a scripted 2028 stub board behind the
fake-gateway seam (`stage_lanking_board` — ASSUMED wire-side, never
exe proof). Clicks drive the public `click_lanking` seam (the same
seam the windowed binary feeds through `GameLoop::mouse_click` on
the native 800x600 canvas); paint runs the side overlay
(`paint_lanking_board` over `side_placements`).

## What was clicked, what happened

Board open from Waiting; TITLE sort click (15, 12 — the staged
`IDC_BUTTON_LANKING_TITLE` rect at its exe code `0x5220000F`)
resorts the twelve rows and resets the pager; pager NEXT click
(85, 12 — staged `IDC_BUTTON_LANKING_LPAGE_RIGHT`,
`0x52200016`) turns to page 1 (5/5/2 over the five staged
`LVIEW_EX` hit areas); first-row hit-area click (705, 105 —
staged `IDC_BUTTON_LVIEW_EX_1`, `0x52041023`) picks Song005 and
the picked title repaints into the staged `MUSICNAME` rect; the
scripted 2028 board (one ranked line) stages the right list; Back
(`leave_side`) returns to Waiting.

Evidence: screen path `waiting → lanking → waiting`, state hashes
`37b73ce7f0f5401c` (board open) → `1995ccf49f75e194`
(TITLE sorted) → `1eee7fdb426c2215` (page 1) →
`eaff134468d23c85` (Song005 picked) → `4fdabf7febac1f18`
(2028 stub staged). Guard:
`lanking_hand_session_sorts_pages_picks_and_renders` (plus
`lanking_sort_click_resorts_the_staged_board`,
`lanking_sort_title_orders_out_of_sequence_catalogs`,
`lanking_pager_turns_five_row_pages`,
`lanking_row_pick_names_the_musicname_caption`,
`lanking_rview_renders_the_staged_2028_board`,
`lanking_unstaged_clicks_land_nowhere` and the four
`side_tests` dispatch/slot/song/cell guards).

## Deltas found (new rows, never silent fixes)

1. **Sort directions read ascending (ASSUMED).** The exe codes name
   the keys, never the order. Next step: SendInput sort-click sweep
   on the live board per the `~/scratch/planet-verify/` recipe,
   capture the row order, then pin the direction.
2. **Pager left/right pairing reads left=back, right=forward
   (ASSUMED).** Next step: same rig sweep on the four pager
   buttons plus captures.
3. **Self line on `RVIEW_EX_1` (ASSUMED).** The answer leads with
   self and the first toggle stands apart (y=125 vs the 191+18k
   run), but no capture pins the read. Next step: rig capture of
   the board with a known self rank.
4. **The `RVIEW_EX` toggles stay display-only.** No consumer pins
   them, so clicks land nowhere and row picks ride the left hit
   areas. Next step: live row-click capture (same recipe).
5. **The 2028 board stages shell-side only.** `stage_lanking_board`
   takes the parsed answer like the team's 4004/4008 pair stages
   sends; no session drain feeds it yet. Next step: gateway replay
   confirming 2028 on the wire plus the song-to-board join (2027
   is never sent by the exe).
6. **BATTLE/TOP100, tier buttons, search, REFRESH, and GEM/CASH
   stay display-only.** Next step: rig sweep of each control plus
   captures, or dossier offsets for their handlers.
