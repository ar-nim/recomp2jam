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
| `0X1920000D/0E`, `0X1920C00F` | promotion LEFT/RIGHT/PLAYSTOP | matching `.ojs` |
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
| 12 | (353,511)-(379,535) 26x24 | promotion LEFT (ordinal `(0xD & 0xFFF)−1 = 12`) |
| 13 | (421,511)-(447,535) 26x24 | promotion RIGHT (ordinal 13) |
| 14 | (387,511)-(413,535) 26x24 | promotion PLAYSTOP (ordinal 14) |
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

## Emblem caption strings EXE-PINNED (2026-11-10)

The five TEXT controls (`IDC_TEXT_SPLANET_{HARD,NORMAL,EASY,FREE,DRUM}`
`0x19400007..0B`) paint at `State_Planet.bnd` rects 6..10 (94x13 strips
at y125-138, ordinal pairing). Their runtime strings ship composed in
the exe (offset-only scan):

| Control | String | File offset | VA |
|---|---|---|---|
| TEXT EASY (`0x19400009`) | `사람손 (초급)` | `0x247ae0` | `0x647ae0` |
| TEXT NORMAL (`0x19400008`) | `고수손 (중급)` | `0x247af0` | `0x647af0` |
| TEXT HARD (`0x19400007`) | `신의손 (고급)` | `0x247b00` | `0x647b00` |
| TEXT DRUM (`0x1940000B`) | `퀘스트` | `0x247acc` | `0x647acc` |
| TEXT FREE (`0x1940000A`) | `튜토리얼` | `0x247ad4` | `0x647ad4` |

The ControlList authoring comments (`//신의손` etc., lines 790-799) are
the uncomposed hand names; the exe strings carry hand name + ` (` +
level band + `)` already composed. Level bands `초급/중급/고급` also sit
at `0x245414/0x24540c/0x2453fe` (beside `최고급/전체` — the music-select
filter labels). Rendering follows the owner font decision (modern
cross-platform rasterizer, decision e8d01); positions/sizes ride the
bnd rects. Glyph rasterization is presentation; the strings and rects
are gameplay-pinned as above.

1. ~~Rect contents → placements (pairing getter `0x548640`).~~ DONE above.

## Difficulty-pick highlight — EXE-RENDERING PINNED, shell OPEN (2026-11-11)

Live-capture diff (`~/scratch/planet-verify/s1-after-easy.png` vs
`s1-after-normal.png`, native 800x600): the exe renders the pick as
an **orange select border hugging the picked badge** — EASY-picked
border bbox x102–191/y49–116 (button rect (87,32)-(186,144)),
NORMAL-picked x221–290/y55–116 (button rect (206,32)-(305,144)).
The border does NOT follow the button rect (EASY border max-x 191
exceeds the rect edge 186); it hugs the badge art, whose extents are
unpinned. The shell keeps the amber-cursor stand-in (painted at
`(rect.x+10, rect.y+h/2)`, visible in the window capture as a small
amber tick left of the picked badge).

Next step: pin the border rule against the staged badge-art extents
(`Planet_Btn_{Hard,Normal,Easy}.ojs` figure sizes at their rect
origins) via capture alignment, then replace the cursor. Static
handler offset for the highlight paint is unlocated (no dossier
entry); do not invent border geometry meanwhile.

## Board hover highlight — PINNED per control, shell paints it (2026-11-11)

Wine rig hover sweep (`docs/hover.md`, full provenance there):
7.55 `AngelO2Jam.exe` 5,029,888 B SHA-256
`83a04541…72c823b`, Wine 11.21, Xvfb `:104` + wine desktop `o2jam`
800x600, `PULSE_SINK=wine_null`, SendInput absolute motion only
(XTEST never reaches the canvas), offA/on/offB capture triples
under `~/scratch/hover-sweep/`, byte-identical changed-pixel counts
across two independent runs.

| control | code | hover arms | diff bbox (native px) | changed px |
|---|---|---|---|---|
| HARD emblem | `0x19201002` | 84x18 plate band of body frame 0, no frame swap | 343,44-427,62 | 1456 |
| NORMAL emblem | `0x19201003` | same plate band, body frame 0 | 213,44-297,62 | 1477 |
| EASY emblem | `0x19201004` | same plate band, body frame 0 | 94,44-178,62 | 1476 |
| FREE (QUEST) | `0x19201005` | static | none | 0 |
| DRUM (TUTORIAL) | `0x19201006` | static | none | 0 |
| OPTION gear | `0x19200011` | body frame 0 at full colour | 714,565-743,594 | 615 |
| EXIT disc | `0x19200010` | body frame 0 at full colour | 754,565-783,594 | 654 |
| PROMO LEFT | `0x1920000D` | body frame 0 at full colour | 358,514-371,529 | 120 |
| PROMO PLAYSTOP | `0x1920C00F` | **frame 1** (rest is frame 0) | 392,515-407,529 | 224 |
| PROMO RIGHT | `0x1920000E` | body frame 0 at full colour | 428,514-441,529 | 120 |

What this means for the painter: the emblems never swap frames —
the hover reveals the body's own plate band (the band pixels match
the body's frame-0 pixels at that offset within ±30/255; frame 1 is
the *selected family* art, a click state, not hover). The discs and
arrows restore their own frame 0 over a dimmed rest; only PLAYSTOP
swaps frames. FREE/DRUM render dim and never react.

Ticket #137 lit-state rule (PINNED 2026-11-12, always-dim,
unconditional): the QUEST (DRUM rect `0x19201006`) and TUTORIAL
(FREE rect `0x19201005`) emblems are dim in every observed state
and no state lights them — rest, hover, press, and
family-selected/dialog-open. Evidence: (a) the hover sweep
(`docs/hover.md`) — rest matches body frame 0 at only 55-56%
(dim, disabled) and the in-rect hover diff is byte-identical
offA/on for both rects (re-verified); (b) the `clean-emblem`
press series (`~/scratch/close-backlog/gui-drive/captures/`, 7
frames rest→dialog-open) — both rects byte-identical across all
frames while the dialog region changes, so selecting an H/M/G
family does not touch them; (c) canvas brightness runs at ~0.32x
the staged frame-0 opaque mean for both bodies (dimmed, not lit);
(d) static: the exe emblem dispatcher covers all five codes
uniformly (toggle group, family index `pressed - 0x19201002`, no
per-emblem frame branch —
`docs/overnight-evidence/e1-channel-dialog-trigger.md`), and
`OTwo.exe` carries no quest/event backend strings, so the dim is
paint-time disabled rendering with no live condition behind it.
The exact dim coefficients stay OPEN (the #113/#26 rest-dim
transform, sharpened-OPEN); the shell paints the rest stack at
full colour until #26 lands (ignored guard below names it).

Shell (ticket #26): `GameLoop::hovered_planet_action` resolves the
control under the pointer through the staged dispatch table
(ticket #33's pointer state, no second store; silent off-board, off-
phase, and behind a modal), and `paint_planet_hover` repaints the
pinned pixels over the untouched rest stack through the shared
`paint_placement_layer` hover painter (ticket #33, extended with
the plate clip) — emblem layer 0 clipped to
`planet_plate_clip` over the staged button rect, discs and arrows
layer 0 whole, PLAYSTOP layer 1 (fallback layer 0), static plates
nothing, missing art nothing. Unhover clears by repaint.
Guards: `hovered_planet_action_tracks_pointer_motion`,
`planet_hover_stays_off_while_a_modal_owns_the_board`,
`playstop_hover_arms_the_frame_swap_cell`,
`hovered_emblem_reveals_the_pinned_plate_band`,
`quest_and_tutorial_emblems_ignore_hover`,
`quest_and_tutorial_emblems_hold_rest_across_press_and_selection`,
`quest_and_tutorial_rest_paints_the_dimmed_frame` (ignored,
red-by-design until #26's rest-dim),
`promo_playstop_hover_paints_the_second_frame`,
`gear_hover_restores_the_first_frame`,
`planet_hover_without_staged_figure_paints_nothing`.
The amber selection cursor stays labeled stand-in (section above).

Promo code correction (2026-11-11): the shell had STOP and RIGHT
crossed (`PLANET_PROMO_STOP_CODE 0x...0E`, `RIGHT 0x...0F` — the
latter names a code the block never declares). The archive
`ControlList_Interface.txt` `STATE_PLANET` declares
`IDC_BUTTON_SPLANET_PROMOTION_LEFT 0X1920000D`,
`..._RIGHT 0X1920000E`, `..._PLAYSTOP 0X1920C00F` — ordinals 12/13/14
pair to rects 12/13/14 exactly, and the body anchors agree
byte-exact (`Left (353,511)`, `Right (421,511)`, `PlayStop
(387,511)`). Fixed to those values; the crossed click dispatch is
corrected by the same change (guard above pins the archive codes
literally). Record-count note, resolved 2026-11-11: `Planet_Btn_Promotion_PlayStop.ojs`
decodes to 6 raw 26x24 records at (387,511), not 2 — and the layer
order is now pinned by direct record-vs-capture match over all six
records (rest = rec0 624/624, hover = rec1 624/624), so the shell
stages all records as layers and the hover takes layer 1.
`docs/hover.md` tabulates the corrected count.

OPEN (mechanism, not observable): the rest-dim rendering path
(measured on every button, explained by none) and whether the exe
clips the plate band at rest or paints it only on hover — next step
for both is the stopped-state winedbg button-descriptor read
(parent `+0xF80`, stride `0x118`, `docs/reversing.md` protocol).
The rest stack underneath the hover is untouched by this ticket.
2. Shell screen + paint + flow edges — DONE (`Screen::Planet`,
   `paint_planet_scene`, `Channel→Planet`/`Planet→Room`; walk cover in
   `shots.rs` via the verified edges).
3. Live staging — DONE (`stage_interface_scenes` mounts the bound
   block; 17 placements incl. self, `State_Planet.ojs` grown).
4. Button hit-testing + difficulty selection — DONE (2026-11-09):
   clicks hit-test at the five pinned `.bnd` button rects
   (`planet_hit`/`PlanetSelection` in `side.rs`); HARD/NORMAL/EASY
   record the pick; the picked band keeps the shared amber cursor
   as an OPEN stand-in — the exe rendering (orange select border
   hugging the picked badge) is capture-pinned with bbox evidence
   in "Difficulty-pick highlight" above, next step named there.
   FREE/DRUM clicks hit their rects but stay unbound and
   change nothing (P0 preserved, soak- and shot-guarded).
2. ~~TEXT caption sources (`0X19400007..0B` never code-addressed yet).~~
   DONE 2026-11-11: the five `PLANET_CAPTIONS` entries carry the
   pinned CP949 bytes and paint through `o2jam-text` into their
   staged `.bnd` rects (`paint_planet_captions`; no placement, no
   caption); decode + paint + system-face glyph guards in the
   shell, boot-board window capture carries real Hangul captions
   (`docs/smoke/planet-board-session.md`).
3. ~~FREE/DRUM click behavior~~ DONE as permanent rejection (P0:
   no backend ever existed); what they would do stays OPEN forever.
4. ~~Promotion PLAYSTOP/LEFT/RIGHT semantics — hit rects unwireable:
   the `.bnd` walk pins their y band (511–535) but no x extents.~~
   Hit rects PINNED 2026-11-11 (parsed bytes, ordinal pairing):
   rect 12 LEFT (353,511)-(379,535), rect 13 RIGHT (421,511)-(447,535),
   rect 14 PLAYSTOP (387,511)-(413,535) — the `< ☐ >` nav cluster
   over the promo banner (window capture `docs/smoke/planet-board-session.md`).
   The shell dispatches them through staged placements only
   (`PlanetAction::Promo`, recorded press, ADR-0003: no fallback
   literals); carousel semantics stay OPEN forever (no consumer
   evidenced).
5. ~~EXIT/OPTION destinations — same: pinned origins (750,561)/(710,561)
   only, no sizes, so no hit rect and no action.~~ DONE: ordinal
   pairing gives full rects — EXIT rect 15 (750,561)-(788,599),
   OPTION rect 16 (710,561)-(748,599), verified against the shipped
   bytes 2026-11-10; both dispatch (quit-confirm / options dialog)
   with shell tests + window capture (`docs/smoke/planet-board-session.md`).
6. What a family pick DOES — VERIFIED 2026-11-09 (live): the emblems
   are server channel families (see "Live verification"); the exe's
   own entry key into Planet is moot — Planet IS the boot board and
the channel dialog auto-opens ~21 s after login.
7. BGM start trigger — SUPERSEDED 2026-11-10 (the earlier "no-trigger"
   static-scan verdict was a scan-shape miss, not a negative): the
   selector `0x151820(id, loop)` (VA `0x551820`) is called from every
   state's v4 entry with immediate ids — Avi→8 (once), Planet→9
   (loop), Room/Payment/Payment2→3, Shop→4, Myroom→5, Waiting→6,
   MusicShop→0x10, Result→0x13, option preview→0, transitions→−1
   (silence); bank `soundmgr[0x76832c]+0x340c0`; `Music/BGM.ojm`
   one-based records. NPC voice role stays OPEN. See
   `~/scratch/bgm-evidence/findings.md` and `docs/coverage-audit.md`.
