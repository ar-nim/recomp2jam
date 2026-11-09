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

## Real board structure (2026-11-09, live captures) — player report CORROBORATED

Live Wine captures (local-only, `~/scratch/button-unblock/`:
`planet-dialog.png`, `planet-bare-1/2.png`, `grid-top.png`,
`w3-list0..19.png`, `exit-state*.png`; repeatable recipe in
`walk_planet.py`) establish what the first interactive screen actually
is:

- **The screen is the planet board**: five emblem buttons in the top
strip, the promotion carousel beneath, and the MENU bar. The channel
channel-select dialog (`0x8C`, `Dialog_ChannelInfo.ojs` sheet) opens
**on top** of it at boot (the launcher web bridge preselects
`My_channel`/`My_planet`) and its X button (native (557,195) — inside
the measured X zone) closes it to the bare board.
- **Emblem ↔ `.bnd` rect live confirmation**: the five button rects
pair to the captured emblems exactly — EASY `(87,32)-(186,144)` holds
the lit **H badge** `사람손(초급)`; NORMAL `(206,32)-(305,144)` the lit
**M badge** `고수손(중급)`; HARD `(313,32)-(459,144)` the lit winged
**G badge** `신의손(고급)`; the DRUM rect `(510,32)-(609,144)` hosts
the **dimmed QUEST emblem**; the FREE rect `(614,32)-(713,144)` hosts
the **dimmed TUTORIAL emblem**. The block's CP949 authoring names
(Hard/Normal/Easy/Free/Drum) are not what the shipped screen shows in
the rightmost two slots — QUEST/TUTORIAL occupy them, grayed out,
matching the `미정` (undecided) comments and the P0 unbound rule.
- **Player report ("5 planets, first 3 active, then paginated
channels")**: CORROBORATED as screen composition. Per-emblem behavior
(does each lit emblem open/switch the channel list?) is verified
separately (`~/scratch/planet-verify/`, report pending).
- **Power button** (MENU bar, bottom right) opens a quit-confirm
MESSAGE dialog `게임을 종료하시겠습니까?` with OK/CANCEL
(`exit-state.png`). Gear (option) and back-arrow destinations stay
OPEN.
- **Promo carousel** auto-cycles banners (Halloween NEW ITEM ↔ October
NEW MUSIC package) with the arrow cells at y511-535.

## Live verification (2026-11-09, `~/scratch/planet-verify/`) — behavior PINNED

Dynamic verification on the real client (SendInput helper; full
recipe and findings in `~/scratch/planet-verify/findings.md`):

- **One state, one flag**: in BOTH phases (dialog up / bare board)
the parent keeps the same `CStatePlanet` object mounted at
`parent+0xF6C` and the descriptor table at `parent+0xF80` is unchanged.
The dialog visibility is a phase flag at **`parent+0xF64`: 1 = dialog
shown, 0 = bare board**. `STATE_CHANNEL (0x12)` is never the current
state — the channel select is a mode of Planet. (The shell's
`Screen::Channel` ↔ `Screen::Planet` edges model exactly this flag
flip; the two enum values are phases of one state, not two states.)
- **The emblems are server channel FAMILIES**, not solo-mission
difficulties. The dialog auto-opens ~21 s after login with the EASY
family (H badge) active; the title plate always reads `Channel`.
  - EASY click (already-active family): no change.
  - NORMAL click: family switch — M badge selects, rows go blank (the
    test server lists only family `[0]` channels), a session reconnect
    follows in 1-2 s.
  - HARD click: blue popup `network problem, NORMAL mode only (LINE
    BATTLE disabled)`; no switch. Server-gated, not client logic.
  - From the bare board, an emblem click reopens the dialog;
    re-selecting the active family reconnects without re-fetching, so
    rows stay blank.
  - QUEST/TUTORIAL untested (dimmed); P0 unbound stands.
- **Room entry verified live**: one entry-face click (native (526,248))
on a fresh populated dialog produces server tokens `03EC/0FBE/07D0/
07D2/13A4` within 2 s and the screen becomes `STATE_ROOM`.
- **Working click recipe**: boot explorer desktop; XTEST (145,105)
  then (400,362); dialog up ~+21 s; clear the blue popup at (400,365)
  before canvas clicks; helper = warp, 1.5 s settle, verify cursor,
  SendInput; X-close (577,171); emblems (136,88)/(255,88)/(386,88).
  Under winedbg: PTY, `\r` ends commands, Ctrl-C as `\x03`, one stop
  only, queue all `x` reads.

Player report status: **VERIFIED** as screen composition and emblem
behavior (family select), with the family-population caveat (only
family `[0]` had channels on the test server).

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
6. What a family pick DOES — VERIFIED 2026-11-09 (live): the emblems
   are server channel families (see "Live verification"); the exe's
   own entry key into Planet is moot — Planet IS the boot board and
the channel dialog auto-opens ~21 s after login.
7. BGM start trigger and NPC voice role — CLOSED as no-trigger (2026-11-09,
   static scan): all 7 background tracks preload together at audio
   init (function region file `0x1a5710`..`0x1a5fe0` via the index loop
   file `0x150ae0` over the 7-entry table at file `0x27f730`,
   `Music/Planet.ojm` at file `0x27f850` = index 4); no per-state
   open and no Planet-entry/button caller exist, and track selection
   among opened streams is unidentified. The shell starts no BGM —
   correct absence, not a gap.
