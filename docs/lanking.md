# STATE_LANKING screen (7.55) — evidence log

Ranking display, no state class (display-only block, like CHANNEL).
Read-only walks below; offsets are file offsets unless marked VA.
Version pin for all EXE rows: `AngelO2Jam.exe` 5,029,888 B, SHA-256
`83a04541…72c823b`.

## Block walk (2026-11-09, `ControlList_Interface.txt`)

`STATE_LANKING`, decl `0X37`, `BOUND "State_Lanking.bnd"`,
56 `IDC_` entries plus one 11-member `SET 0x0B 0X00041000` group:

| Codes | Entries | Art |
|---|---|---|
| `0X52000000` | `IDC_STATE_LANKING` (self) | — |
| `0X52100001` | `IDC_BACK_LANKING` | `State_Lanking.ojs` |
| `0X52200002` | EXIT | `Lanking_Btn_Back.ojs` |
| `0X52500003..06` | box-line frames | `Lanking_LeftBoxLine_*.ojs` |
| `0X52200007..08` | BATTLE / TOP100 | matching `.ojs` |
| `0X52200009..0E` | ALL/GOLD/BLACK/BLUE/RED/SILVER tiers | matching `.ojs` |
| `0X5220000F..12` | TITLE/DIFF/BPM/ARTIST sorts | matching `.ojs` |
| `0X52300013` | search edit | — (input OPEN) |
| `0X52200014` | search button | `Lanking_Btn_Search.ojs` |
| `0X52200015..18` | L/R pagers | matching `.ojs` |
| `0X52500019..20` | page-number art | `MShop_PageNumber.ojs` |
| `0X52400021..22` | GEM/CASH money texts | — (captions OPEN) |
| `0X52041023..27` | `LVIEW_EX_1..5` toggle views | — |
| `SET` ×11 | list-view rows: BACK/IMAGE/NEW art, TITLE/ ARTIST/EDITOR/GENRE/BPM/DIFF texts, LANK button, COLOR | `Lanking_LViewBox/New/Color.ojs` + `Lanking_Btn_Lank.ojs` |

## Bind file

`State_Lanking.bnd`: slot 1341, offset 61445182, 1174 B body =
`4 + 2 + 73×16` exact → 73 rects of 16 B. Rect contents unread
(next step).

## Data side (server assists)

`t_o2jam_user_ranking` table exists in the local database; the rank
fetch opcode and row fields stay OPEN. Money texts (`GEM/CASH`)
follow the wallet path (`docs/online-loop.md`); caption sources
stay OPEN like all TEXT entries.

## OPEN

1. Rect contents → placements — DONE (73 rects, ordinal pairing).
2. Shell screen + paint + flow + live staging — DONE
   (`Screen::Lanking`, `paint_lanking_scene`, all-reject flow,
   `stage_interface_scenes` + soak tests; side-screen clicks route
   through `click_lanking` since ticket #20).
3. Rank fetch opcode + row fields — DONE exe-side
   (`docs/overnight-evidence/u4-ranking-packs.md`: 2028
   `MusicPlayRankingAnswer` handler VA `0x55b230`, REA
   `ev_7f09b7fa…`; `RankEntry` fields rank/nickname/battles/wins/
   win-rate/delta/delta-mag). Wire ride stays ASSUMED (scripted
   stub behind `stage_lanking_board` — next step: live gateway
   replay confirming 2028 on the wire).
4. Category/sort/search/pager semantics — PARTIAL (ticket #20,
   2026-11-12): sorts TITLE/DIFF/BPM/ARTIST answer at
   `0x5220000F`..`0x12`, `LPAGE` at `0x15`/`0x16`, `RPAGE` at
   `0x17`/`0x18`; left rows tile the five staged `LVIEW_EX`
   hit areas, right rows the eleven staged `RVIEW_EX` slots
   (self line first, ASSUMED). Sort directions, pager
   left/right pairing, and the self-line read stay ASSUMED —
   next step: SendInput sweep on the live board per the
   `~/scratch/planet-verify/` recipe plus captures. Still
   display-only OPEN: BATTLE/TOP100 (`0x07`/`0x08`), tiers
   (`0x09`..`0x0E`), search edit+button (`0x13`/`0x14`),
   REFRESH (`0x33`), GEM/CASH texts (`0x21`/`0x22`), and the
   `RVIEW_EX` toggles (no consumer pins them).
5. Entry path (host class unknown, like CHANNEL) and exits — OPEN
   (side-screen visit/return only).
6. `LVIEW_EX` toggle views (`0x23`..`0x27`): these ARE the five
   left-row hit areas (staged 353x86 tiling) — selection wired
   since ticket #20; any further toggle semantics stay OPEN.
7. Song-to-board fetch join: picking a song names the
   `MUSICNAME` caption but fetches no board (2027 is never sent
   by the exe — next step: live gateway replay).
8. Per-song 2031 bests on the song rows: `MusicScoreListAnswer`
   2031 parses (count + width pinned) but never joins the
   catalog — next step: join against `OJNList.dat` by music id.
