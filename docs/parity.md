# Scene parity checklist — 7.55 client vs `o2jam-shell`

Reference client stays read-only (`AngelO2Jam.exe`, 5,029,888 B,
SHA-256 `83a04541…72c823b`). Every 7.55 claim below traces to
`docs/lifecycle.md` / `docs/gameplay.md` or the REA dossier behind
them; anything else is marked **OPEN**. Placeholder shell art is a
known mismatch, marked **FAIL** — never a pass.

Verdicts: **PASS** = exe evidence + implementation + test.
**FAIL** = known gap against verified 7.55 behavior.
**OPEN** = untraced in the exe or unimplemented and unclaimed.

Harness: `crates/o2jam-shell/tests/scenes.rs` asserts art-backed
composition per scene (a staged figure behind the live UI: figure
ink present, UI ink on top, frame never one flat fill) plus the flat
fallback (non-uniform card-over-background scenes, per-scene
markers) and the full transition matrix plus input/timing edges
through the public API (`ScreenGame` incl. `load_scene`/`load_splash`,
`paint_screen`, `ScreenFlow`, `can_enter`) only. Staged figures stay
synthetic in the repo (real `.ojs`/`.bnd` bytes stay out), but the
local-only real-art soak now runs: `cargo test -p recomp2jam shots`
stages genuine `Interface1`/`Playing1` art from git-ignored
`install-root` (10/10 green, zero skips, 2026-11-07), and the six
PNGs below are genuine-art frames. Composition cells stay FAIL where
invented placement remains (cited per cell), never for lack of art.

## `STATE_AVI` — logo splash (`CStateAvi`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | First visible screen; one-entry block `IDC_BACK_LOGO` → `O2mediaLogo.ojs` (lifecycle §5) | Genuine logo stages via `load_splash` in soak (`present_archives_stage_key_scenes` asserts `load_avi_logo` is_some; `avi.png` shows the NowCOM window capture); sole `.bnd` rect `(0,0)-(800,600)` is verbatim full-bleed (bnd-semantics). Still FAIL: splash bypasses `bind_scene_rects`, so the `IDC_BACK_LOGO` bind is unproven | FAIL |
| Input | Splash timer/dismiss → channel | Any lane-key press dismisses to Channel | OPEN (dismiss key/timer unconfirmed) |
| Timing | Logo hold duration unknown (dispatch call not in dossier set) | Immediate on key press | OPEN |
| Transitions | `Avi → Channel` only | `can_enter` allows exactly that; tested | PASS |

## `STATE_CHANNEL` — channel select (no state class)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | 50 controls, decl `0x27`, **no `.bnd`** (bare `BOUND`); `Dialog_ChannelSelectBar.ojs`, back/rank buttons (lifecycle §6) | Soak stages the bar full-canvas (`stage_single_sprite`), but `channel.png` still reads as fallback paint (dark frame, list + `CHANNEL` stamp only): the bar's real rect is one panel, not the canvas (audit 15), and list rows step at invented `h/4 + i*8` (audit 10–11). Still FAIL | FAIL |
| Input | Pick channel/planet → room (`My_channel`/`My_planet` bridge, file `0x24df70`–`0x24e024`) | Up/Down moves highlight (clamped), Confirm → Room | OPEN (no planet/solo fork) |
| Timing | None documented | n/a | PASS (vacuous) |
| Transitions | `Channel → Room`; hosted by unknown class (open question) | Implemented + tested | PASS |

## `STATE_ROOM` — multiplayer lobby (`CStateRoom`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x36`, 76 entries, `State_Room.bnd` (75 rects); mode filters, `Room_ButtonRoom.ojs` (lifecycle §7) | Soak binds 75 rects verbatim (`bind_verbatim_entries`: rect `i` → entry `i + 1`) under genuine panels — `room.png` shows real Korean room art with list + `ROOM` stamp on top. Still FAIL: list scrim/rows/cursor sit at invented quarter-frame anchors (audit 10–11) and the nine `IDC_*` detail fields draw from rect corners with micro-font metrics, sizes discarded (audit 3, 24–25) | FAIL |
| Input | Join/create room; `DIALOG_CREATEROOM`, password dialog | Confirm → Waiting, Back → Channel; no create/password UI | OPEN |
| Timing | None documented | n/a | PASS (vacuous) |
| Transitions | `Room → Waiting`, `Room → Channel` | Implemented + tested | PASS |

## `STATE_WAITING` — pre-game room (`CStateWaiting`/`CLiveWaiting`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x63`, 117 entries, `State_Waiting.bnd`; 8 team buttons `0x16201004..0B` (`Wait_TeamA..H.ojs`), 8 avatar covers; song dialog decl `0x7A`/134 entries (lifecycle §8, gameplay room-teams) | Soak binds 116 rects verbatim under genuine art — `waiting.png` shows real avatar tiles with seats + `WAITING` stamp on top. Still FAIL: seats sit on an invented 2-col × 4-row grid (audit 12), portraits center blind (audit 18–19); no team buttons, covers, or song dialog | FAIL |
| Input | Team toggle → `SetRoomTeam` 4004 + `Ready` 4008(0); host start; back button (gameplay room-teams: `FUN_004b22d0` file `0xb22d0`, ev `2c8d3964…`) | Confirm → Loading, Back → Room; no team/send path | OPEN |
| Timing | `MusicLoading.ojs` + song clock arming at load (lifecycle §9.5) | n/a (see Loading) | OPEN |
| Transitions | `Waiting → Loading`, `Waiting → Room` | Implemented + tested | PASS |
| Gameplay | 8 seats, team 0..7, winner art only (no per-team panel) | Seats + ready flags; no teams/winner art | OPEN |

## Loading gate (song select → play)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | `MusicLoading.ojs`, Trebuchet/Arial fonts, song-table walk + clock arm (`FUN_004e47a0` file `0xe47a0`, ev `68d920c2…`); OJN decrypt + Vorbis init precede it (lifecycle §9) | Soak stages genuine planet art full-canvas (`loading.png`); bare `BOUND` makes full-bleed the documented fallback, but per-sprite native sizes are ignored (audit 23). Still FAIL: bar is an unbound centered placeholder track/fill (audit 35) with no progress fraction, no fonts, no OJN/Vorbis path | FAIL |
| Input | Host start triggers gate | Confirm → Playing, Back → Waiting | OPEN (no validation/staging) |
| Timing | Select validation `0x5718b0` → staging `0x4f8b80` → decrypt `0x412180` → codec `0x54db40` → load screen | Instant on Confirm | OPEN |
| Transitions | `Waiting → Loading → Playing`, `Loading → Waiting` | Implemented + tested | PASS |

## `STATE_PLAYING` — gameplay (`CStateNormalPlaying`/`CStateLivePlaying`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Real UI in `Playing1.opi` (235 entries, `State_Playing.bnd` — bound run is entries 213–229 `EXIT`…`VOLUME`, decl `0xE6`; bnd-semantics); life/jam gauge bars, `Winner_image.ojs` (lifecycle §10, gameplay lifebar/jam sections) | Highway now derives from staged art, not frame fractions: uneven lanes `(5,28)…(165,28)`, judgement row 487, 7 px heads, per-lane receptor heights, jam bars `(23,536)`/`(570,536)` 206×16, peer 46×8 (5cb01a2; audit 4–9 superseded); soak proves it (`play_skin_decodes_every_lane_when_present`, `skinned_highway_replaces_procedural_note_ink` green; `playing.png` shows left-hugging lanes + staged burst/gauge ink). Still FAIL: gauge appended full-canvas over the bound window (audit 16), judgement/combo/jam digits at invented anchors (audit 31), linear scroll map (audit 32), curtain clips a frame box not lane columns (audit 30) | FAIL |
| Input | 7-lane keys, judgement vs `\|song−note\|` ≤ 6/18/25 ms → flags `0x10/0x20/0x40/0x80` (gameplay judgement, file `0x176540`) | Lane strikes hit `LivePlay`; windows live in `o2jam-game`, shell uses wall clock | OPEN (shell clock not sample clock) |
| Timing | Sample-accurate song clock; per-frame bg-effect loop `0x550db0` (ev `4164031c…`) | 60 Hz `FrameClock`; paint-advance to Result at chart end tested | OPEN |
| Transitions | `Loading → Playing → Result` | Implemented + tested, incl. auto-advance | PASS |
| Gameplay | Cool/Good continue combo, Bad/Miss break; pills (15-Cool mint, spend-on-Bad); score `200+10X/100+5X/4/−10`; life max 1000, per-difficulty deltas Cool +3/+2/+1, Good +2/+1/0, Bad −10/−7/−5, Miss −50/−40/−30, dead gate at 0 (gameplay, all VERIFIED; life: switch `0x170fa2` ev `feebe490…`, scorer `0x170510` → adder `0x540a80`) | Implemented + tested in `o2jam-game` (`LifeBar` + `LifeDeltas`: difficulty sets, 0..=1000 saturation, dead gate), not wired to shell scenes | OPEN (shell-side) |

## `STATE_RESULT` — results (`CStateResult`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x07`, 8 entries: `Result_WinLose.ojs`, `Result_NumFont_*.ojs`, Top/Bottom, Retry; builder `FUN_004c2390` (file `0xc2390`, ev `d6de3a7f…`) | Soak stages genuine panels — `result.png` shows real COOL/GOOD/BAD + music-name art with digits on top. Still FAIL: avatar/digits/verdict sit at fixed-pixel anchors `(16,16)`/`(16,32)`/`(16,48)` behind a 64×52 scrim (audit 13), ignoring the 7 `State_Result.bnd` rects; no WinLose art, grade letters out (mapping OPEN) | FAIL |
| Input | Retry → waiting loading; leave → room/channel; event-ticket web call (file `0x24a338`) | Confirm → Waiting, Back → Room | OPEN (no ticket call) |
| Timing | None documented | Paint-driven, immediate | PASS (vacuous) |
| Transitions | `Result → Waiting`, `Result → Room` | Implemented + tested | PASS |

## Side states (reachable from lobbies; not in shell)

All **OPEN** (no shell screen, no implementation — documented so no
pass is ever claimed):

- `STATE_SHOP` / `STATE_MUSICSHOP` (`CStateShop`/`CStateMShop`,
  `0xA4`/`0x6B`; `Payment` back-ends, `CMyBag`/`CCart`).
- `STATE_MYROOM` (`0x6A`; `CItemManager`, `CPlayerInfo`).
- `STATE_PLANET` (`CStatePlanet`, `0x11`; BGM `Music/Planet.ojm`;
  quest button commented undecided — no drum backend).
- `STATE_LANKING` (`0x37`; display-only, no state class — like CHANNEL).
- `STATE_PAYMENT` / `STATE_PAYMENT2` (web billing views).
- `STATE_TUTORIAL` (stub `0x00`; entry path unknown).
- `STATE_DRUM_PLAYING` / `STATE_EXIT` (stubs `0x00`; drum never wired,
  no `CStateDrum*` class; exit is teardown, not a screen).

## Scoreboard

- Scenes covered: 7 main + loading gate + 9 side states.
- PASS: boot-into-splash, 800×600 default (`SetWindowPos` file
  `0x14b4c`/`0x14baf`), all 11 documented transition edges (plus
  rejections), vacuous-timing scenes — 14 cells.
- FAIL (known placement gaps): 7 composition cells (genuine art now
  stages in the local soak — shots green, six PNGs refreshed 2026-11-07 —
  but invented placement remains per cell: audit 3, 10–16, 18–19,
  23–25, 30–32; audit 4–9 superseded by the verbatim highway 5cb01a2).
  The `load_scene` layering behind them stays harness-proven (22/22).
- OPEN: all input/timing/gameplay cells above plus every side state —
  next evidence targets are the loading-gate validation sequence
  (`0x5718b0` → `0x4f8b80` → `0x412180`), the peer-life 4015 handler,
  and per-cell rect bindings (channel bar/rows, room `IDC_*` fields,
  waiting seats/teams, result digits, playing digits/scroll).
