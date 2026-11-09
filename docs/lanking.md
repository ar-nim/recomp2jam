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
   `stage_interface_scenes` + soak tests; clicks land nowhere per P0).
2. Rank fetch opcode + row fields.
3. Category/sort/search/pager semantics (display-only candidates).
4. Entry path (host class unknown, like CHANNEL) and exits.
5. LVlEW_EX toggle views.
