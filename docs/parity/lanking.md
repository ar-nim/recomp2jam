## `STATE_LANKING` — ranking board (`0x37`; display-only, no state class — like CHANNEL)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | `State_Lanking.bnd` 73 rects; two lists — five catalog songs per left page under the `LVIEW_EX_1`..`_5` hit areas (353x86 tiling the song box) with TITLE/DIFF/BPM/ARTIST sorts at `0x0F`..`0x12` and the `LPAGE` pager at `0x15`/`0x16`; the fetched `2028 MusicPlayRankingAnswer` lines on the right (self line on `RVIEW_EX_1`, ten board lines on `RVIEW_EX_2`..`_11`, `0x35`..`0x3F`) with its `RPAGE` pager at `0x17`/`0x18` (docs/lanking.md; the self-plus-ten shape and zero-count ten-fill rule corroborate per docs/overnight-evidence/u4-ranking-packs.md) | (ev: docs/lanking.md) Every row/rank cell inks its staged TEXT template shifted by its staged slot origin — no geometry literals; the 2028 board stages behind the scripted-stub seam (proven shell-side, ASSUMED wire-side); the shots oracle holds with only the intended `lanking.png` delta. Guards: `lanking_songs_read_catalog_fields_and_sort_keys`, `lanking_rank_cells_render_fetched_lines_and_blank_fill`, `lanking_rview_renders_the_staged_2028_board`, `lanking_sort_title_orders_out_of_sequence_catalogs`. OPEN arm: GEM/CASH texts stay data-gated | (R) PASS (v: 9692d41) |
| Input | Sorts at `0x0F`..`0x12`, pagers at `0x15`/`0x16`/`0x17`/`0x18`; row picks ride the left hit areas and the tiled LANK button and name the `MUSICNAME` caption (`0x34`) | (ev: docs/lanking.md) Sort clicks resort the staged board, the pager turns five-row pages, row picks name the caption, unstaged clicks land nowhere. Guards: `lanking_clicks_answer_the_exe_named_sort_and_pager_codes`, `lanking_row_picks_answer_the_left_hit_areas`, `lanking_sort_click_resorts_the_staged_board`, `lanking_pager_turns_five_row_pages`, `lanking_row_pick_names_the_musicname_caption`, `lanking_unstaged_clicks_land_nowhere`, `lanking_hand_session_sorts_pages_picks_and_renders`; smoke `docs/smoke/lanking-hand-session.md`. OPEN: BATTLE/TOP100, tier buttons, search, REFRESH, `RVIEW_EX` toggles, `LVIEW_EX` past five; sort directions, pager left/right pairing, and self-on-`RVIEW_EX_1` ASSUMED; song-to-board fetch join; per-song 2031 bests on song rows — next steps: SendInput sweep on the live board per the `~/scratch/planet-verify/` recipe; live gateway replay for the fetch join; join `MusicScoreListAnswer` 2031 against the catalog). Scoping #72 (2026-11-12): the 2031 per-byte difficulty order (claim D22, issue #89), the 2032 `+0x64`-slot transfer (claim D23, issue #90), and the U4 gate globals (claim D24, issue #91) are pinned; the tie-break display order (claim D26, issue #94) carries the remaining evidence issue under map #66 | (R) OPEN (v: 9692d41) |
| Timing | None documented | n/a | (R) N/A (no timed behavior — no dispatch exists to guard; next step: none) (v: 9692d41) |
| Transitions | Lobby ↔ Lanking via the side visit | (ev: docs/smoke/lanking-hand-session.md) `visit_side` stages the genuine bound art; the hand session sorts, pages, picks, and renders, then returns. Guards: `lanking_hand_session_sorts_pages_picks_and_renders`, `flow_allows_every_documented_edge` | (R) PASS (v: 9692d41) |

## Entry-path statics verdict (re-lanking, 2026-11-22)

The journey-matrix waves could not aim a live Lanking entry. Statics now
REFUTE every static path: `0x37` is only the ControlList decl —
`ChangeState` (VA `0x478C50`, file `0x78C50`, `ev_a18e4237…`) has 45 call
sites (`ev_1447e439…`) passing only ids 0/2–6/8–D/10; the state-install
switch (`0x477990`, `ev_5650f76b…`) has no Lanking factory arm (by
declaration order Lanking would be id `0xE`, whose jump slot points to
common cleanup; no caller uses `0xE`). Room dispatcher `0x48FD20`
(`ev_ac6983f7…`): ITEMSHOP `0x13200008`→state 3, MYROOM `0x13200009`→4,
MUSICSHOP `0x13200037`→9; MYBAND `0x1320003A` posts a CP949 "service
being prepared" message (NOT Lanking) and is suppressed at setup
(`ev_3b07b137…`). Music Shop TOP100 `0x1A20007D` (rect (248,0)-(347,37))
is hidden at setup (`ev_3b33843…`) with no click arm in dispatcher
`0x4C7F70` (`ev_26ac6479…`). No Lanking board code has literal refs.
2027 is never sent; 2030 is a Waiting music-select fetch; 2032 is
opcode-only from song pick; 2028 is S→C row fill only. No entry word.

**Verdict: NA-STATICS — no 7.55 control or wire word enters Lanking.**
Next evidence step (dynamic): watch the state manager `+0x50/+0x54` or
trace `0x478C50` on a server-attended account, per
`/home/arnim/scratch/re-lanking-findings.md`. This refutes the
journey-matrix "MYBAND opens Item Shop" confusion only in part: the
overlap of MYBAND's rect (411,1)-(482,33) with Item Shop (387,1)-(458,33)
explains the wave-3 5006/Item Shop result (the aim hit Item Shop's rect).
