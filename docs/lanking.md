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

## Notice-board colorkey path (#374, 2026-11-19)

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256
`83a04541…72c823b`. REA 6.9.0 / Ghidra 12.2 dossiers (local scratch
bundles under `~/scratch/p374-rea/`, no disassembly reproduced):

- Load-time source key: `DXSurface::Load` (file `0x4249F0`, dossier
  `ev_2bdb3b2c…`) reads the stored u16 key at `+0x11E` and hands it
  to frame-create `FUN_00422F60` (`0x422F60`, dossier
  `ev_38c8c255…`) as its fourth argument. Frame-create installs the
  DirectDraw source key only when that argument is nonzero;
  `FUN_0042BA00` (`0x42BA00`, dossier `ev_20d472f2…`) is the
  `SetColorKey` call with flags `8` and low==high==key. This is the
  #371/#373 rule for key-0 `.ojs` plates: black stays opaque.
- The only other `SetColorKey` caller is `FUN_00428C40` (dossier
  `ev_013bc070…`), a blank 800x512 surface builder reached from the
  Playing setup (`0x56F900`), not the notice-board art. A whole-image
  decompile sweep found no blit-time `SetColorKey(0)` on the
  notice-board path.
- `Lanking_NoticeBoard.oji` stores key `0` and uses zero words for
  transparent pixels (24,839 zero words, no `0x7D5F` words). All ten
  shipped `.oji` bodies have the same pair. The staged loader maps that
  pair to the canonical `0x7D5F` key at staging time
  (`load_staged_figure` / `with_prefixed_zero_key_as_colorkey`), so the
  renderer's load-time nonzero-key rule stays global while the notice
  board's zero words keep cutting out the header backdrop.

Guards: `waiting_notice_board_zero_words_leave_the_header_backdrop_visible`
(lanking band), `planet_zero_key_black_plates_cover_the_board_art` and
`channel_flanks_show_opaque_vinyl_through_zero_key_black_pixels`
(#371/#373), plus the renderer's `zero_color_key_leaves_black_pixels_opaque`.

## v3 paired-shot note (2026-11-23, #879)

The v3 `paired-shots-v3/states/lanking` pair is not a Lanking pair:
its rust frame is byte-identical to the itemshop-entry rust frame
(`beb29a99…568`), and both traces stop at the Item Shop. Re-driving the
rust leg to the real Lanking board with the non-overlapping MYBAND aim
`(470,17)` produces `~/scratch/fix-chrome-b/lanking-rust2/states/lanking.png`;
the exe cannot be driven there — statics (`~/scratch/re-lanking-findings.md`,
`ev_a33542ad…`, `ev_5650f76b…`, `ev_ac6983f7…b584086`) refute every
entry control and wire word. The 0.6726 pair therefore names Item Shop
chrome, not Lanking chrome. No Lanking painter change follows from it;
a valid A/B needs a newly discovered live entry path first.
