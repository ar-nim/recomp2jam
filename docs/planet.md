# STATE_PLANET screen (7.55) — evidence log

Solo-mission select (`CStatePlanet`, `0x11`). Read-only walks below;
offsets are file offsets unless marked VA. Version pin for all EXE
rows: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`.

## Block walk (2026-11-09, `ControlList_Interface.txt`)

Slot 1595 (`Interface1.opi`, data offset 74438614, 90472 B, 1979
CP949 lines), lines 785–809: header `STATE_PLANET 0X11`, `{`,
`BOUND "State_Planet.bnd"`, then entries:

| Code | Entry | Art |
|---|---|---|
| `0X19000000` | `IDC_STATE_PLANET` (self) | — |
| `0X19100001` | `IDC_BACK_SPLANET_1` | `State_Planet.ojs` |
| `0X19201002..06` | `IDC_BUTTON_SPLANET_{HARD,NORMAL,EASY,FREE,DRUM}` | `Planet_Btn_{Hard,Normal,Easy,Free,Drum}.ojs` |
| `0X19400007..0B` | `IDC_TEXT_SPLANET_{HARD,NORMAL,EASY,FREE,DRUM}` | — (captions OPEN) |
| `0X1950000C` | `IDC_IMAGE_SPLANET_PROMOTION` | `Planet_Promotion.ojs` |
| `0X1920000D..0F` | promotion LEFT/RIGHT/PLAYSTOP | matching `.ojs` |
| `0X19200010..11` | EXIT / OPTION | matching `.ojs` |

Button comments (CP949): HARD `신의손`, NORMAL `고수손`, EASY
`사람손`, FREE `튜토리얼(미정)`, DRUM `퀘스트(미정)` — FREE/DRUM
backends never wired (gameplay level-up note, `ev_0ce27550…`).

Cross-checks against existing dossiers: button codes match the
planet dispatcher range `0x19201002..06` (file `0x8e280`); TEXT
codes match the `0x19400007` base in the bar-builder region
(files `0x8dec9`/`0x8dee4`/`0x8def0`). The Channel dialog reuses
`Planet_Btn_Hard/Normal` (`IDC_BUTTON_CHANNEL_INFO_HARD/NORMAL`,
lines 1962–63).

## Bind file

`State_Planet.bnd`: slot 1347, offset 61450578, 278 B body =
`4 + 2 + 17×16` exact → magic `FFFFFFFF` + u16 count + 17 rects
of 16 B (`docs/file-formats/opi.md` envelope). Rect contents
(2026-11-09, `4×i32` LE per rect):

| Rect | Box | Pairs (ordinal) |
|---|---|---|
| 0 | (0,0)-(800,600) | BACK (ordinal 0, full-bleed) |
| 1 | (313,32)-(459,144) | HARD button (1) |
| 2 | (206,32)-(305,144) | NORMAL button (2) |
| 3 | (87,32)-(186,144) | EASY button (3) |
| 4 | (614,32)-(713,144) | FREE button (4) |
| 5 | (510,32)-(609,144) | DRUM button (5) |
| 6..10 | y125-138 strips under buttons | TEXT labels (6..10) |
| 11 | (49,163)-(750,496) | PROMOTION image (11) |
| 12..14 | y511-535 small cells | promotion LEFT/RIGHT/PLAYSTOP (12..14) |
| 15..16 | (750,561)/(710,561) bottom-right | EXIT / OPTION (15..16) |

Ordinal pairing (`(code & 0xFFF) − 1`, memory `[0c6c3]`) reproduces
a coherent layout with no invention: note the buttons do NOT run
left-to-right in file order (HARD sits center at x313).

## Assets present (install-root, git-ignored)

- `Image/Interface1.opi`: `State_Planet.ojs` + buttons + `State_Planet.bnd`.
- `Music/Planet.ojm` + `Music/O2PlanetNPC.ojm` (BGM/NPC).

## OPEN

1. ~~Rect contents → placements (pairing getter `0x548640`).~~ DONE above.
2. Shell screen + paint + flow edges — DONE (`Screen::Planet`,
   `paint_planet_scene`, `Channel→Planet`/`Planet→Room`; walk cover in
   `shots.rs` via the verified edges).
3. Live staging — DONE (`stage_interface_scenes` mounts the bound
   block; 17 placements incl. self, `State_Planet.ojs` grown).
4. Button hit-testing + difficulty selection — DONE (2026-11-09):
   clicks hit-test at the five pinned `.bnd` button rects
   (`planet_hit`/`PlanetSelection` in `side.rs`); HARD/NORMAL/EASY
   record the pick; the picked band draws the shared amber cursor
   inside its rect as an OPEN stand-in (no highlight rendering is
   evidenced). FREE/DRUM clicks hit their rects but stay unbound and
   change nothing (P0 preserved, soak- and shot-guarded).
2. TEXT caption sources (`0X19400007..0B` never code-addressed yet).
3. ~~FREE/DRUM click behavior~~ DONE as permanent rejection (P0:
   no backend ever existed); what they would do stays OPEN forever.
4. Promotion PLAYSTOP/LEFT/RIGHT semantics — hit rects unwireable:
   the `.bnd` walk pins their y band (511–535) but no x extents.
5. EXIT/OPTION destinations — same: pinned origins (750,561)/(710,561)
   only, no sizes, so no hit rect and no action.
6. What a difficulty pick DOES on confirm (solo entry) and the exe's
   own entry key into Planet — the shot walk uses the verified
   flow edges via `enter_screen`, no input invented.
7. BGM start trigger and NPC voice role — CLOSED as no-trigger (2026-11-09,
   static scan): all 7 background tracks preload together at audio
   init (function region file `0x1a5710`..`0x1a5fe0` via the index loop
   file `0x150ae0` over the 7-entry table at file `0x27f730`,
   `Music/Planet.ojm` at file `0x27f850` = index 4); no per-state
   open and no Planet-entry/button caller exist, and track selection
   among opened streams is unidentified. The shell starts no BGM —
   correct absence, not a gap.
