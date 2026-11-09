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
| Composition | 50 controls, decl `0x27`, **no `.bnd`** (bare `BOUND`); `Dialog_ChannelSelectBar.ojs`, back/rank buttons (lifecycle §6) | Soak stages genuine dialog art under the pinned no-fill rule: `Dialog_ChannelInfo.ojs` body at its record anchor (0,0) (exe-mounted sheet 800x468; the parsed body is one 800x466 layer — 2 px runtime delta recorded, uncompensated) and `ChannelInfo_CH_Enter.ojs` frame 0 at the faces (exe art 44x24; parsed frames 44x21). Row geometry is now **exe-pinned** (live Wine memory read 2026-11-09, `docs/channel-fill.md`): the hosting `CStatePlanet` dialog object at `parent+0xF6C` holds the four viewport slot rects (216,224,572,275) / (216,275,572,324) / (216,324,572,375) / (216,375,572,436) at dialog+`0x94`; paint, scrim, and hit-test all ride this table (tops inclusive, bottoms exclusive), and the enter-face inset (row top + 12 → face Y 236) is pinned the same way — the reference capture's `232+50i` anchors were these rects' interior and are retired. Block staging live-confirmed: 33 zero-rect records (state `0x12`), the `0x8C` dialog block carries 26 sprite-named records. Still reference-measured (exe-OPEN): face X origin 502 and the 48x24 face chrome (4 px around the 44x24 art), interior pill/track chrome (re-based on the pinned tops), pager origin. Unmounded (no measured/pinned mount): select bar 348x44, `CH_Back` 348x44, `Btn_Close` 48x40, `CH_ConnectCount` 188x20, `CH_Num` 16x20, scroll bodies 24x20/24x48 — sizes exe-pinned, positions OPEN (button view fields `+0x1E4/+0x1E8` blocked: no `CCButton` objects found, winedbg `cont` dead in wow64). Guards: `channel_row_hit_maps_the_exe_pinned_bands`, `channel_row_hit_mirrors_the_pinned_row_rects`, `channel_dialog_body_stages_behind_the_live_list`, `channel_enter_art_covers_the_measured_faces`, soak `channel_layers_list_over_real_dialog_art`, shots `channel_shot_stages_dialog_art_inside_the_measured_bands`. — still FAIL: face X/chrome, interior chrome, pager, and the unmounded parts above stay non-pinned; row content text is an ASCII stand-in (no CP949 glyphs). | FAIL |
| Input | Pick channel/planet → room (`My_channel`/`My_planet` bridge, file `0x24df70`–`0x24e024`) | Up/Down moves highlight (clamped), Confirm → Room | OPEN (no planet/solo fork) |
| Timing | None documented | n/a | PASS (vacuous) |
| Transitions | `Channel → Room`; hosted by unknown class (open question) | Implemented + tested | PASS |

## `STATE_ROOM` — multiplayer lobby (`CStateRoom`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x36`, 76 entries, `State_Room.bnd` (75 rects); mode filters, `Room_ButtonRoom.ojs` (lifecycle §7) | Soak binds 75 rects verbatim (`bind_verbatim_entries`: rect `i` → entry `i + 1`) under genuine panels — `room.png` shows real Korean room art with list + `ROOM` stamp on top. Still FAIL: panel rows paint at the reference-measured inset (panel corner +34/+1, Wine capture, exe-OPEN); fallback scrim/rows keep invented anchors (audit 10–11) and the nine `IDC_*` detail fields draw from rect corners with micro-font metrics, sizes discarded (audit 3, 24–25) | FAIL |
| Input | Join/create room; `DIALOG_CREATEROOM`, password dialog | Confirm → Waiting, Back → Channel; no create/password UI. Live 2026-11-09: quiet server sends an all-`Number`-0 slot table, so the documented `set_rooms` empty-slot filter leaves blank panels (no rows to click — correct, painter proves rows with data) | OPEN |
| Timing | None documented | n/a | PASS (vacuous) |
| Transitions | `Room → Waiting`, `Room → Channel` | Implemented + tested | PASS |

## `STATE_WAITING` — pre-game room (`CStateWaiting`/`CLiveWaiting`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x63`, 117 entries, `State_Waiting.bnd`; 8 team buttons `0x16201004..0B` (`Wait_TeamA..H.ojs`), 8 avatar covers; song dialog decl `0x7A`/134 entries (lifecycle §8, gameplay room-teams) | Soak binds 116 rects verbatim under genuine art — `waiting.png` shows real avatar tiles with seats + `WAITING` stamp on top. Re-shot 2026-11-07 with staged caption sources: MAPNAME (map-cycle entry of room word 5), BPM (catalog column of song 757), SKILLCOUNT (`7/42/999`-capped inventory) ink their verbatim `.bnd` rects, guarded by `waiting_shot_paints_mapname_bpm_skillcount_captions`; `INSTNAME`/`NOTICE` stay unpainted (OPEN 1/2 per `caption-sources.md`). Still FAIL: seats sit on an invented 2-col × 4-row grid (audit 12), portraits center blind (audit 18–19); no team buttons or covers. The song dialog now stages over the seats — see the musicselect row below — so the dialog half of this cell rides there | FAIL |
| Input | Team toggle → `SetRoomTeam` 4004 + `Ready` 4008(0); host start; back button (gameplay room-teams: `FUN_004b22d0` file `0xb22d0`, ev `2c8d3964…`) | Confirm → Loading, Back → Room; no team/send path | OPEN |
| Timing | `MusicLoading.ojs` + song clock arming at load (lifecycle §9.5) | n/a (see Loading) | OPEN |
| Transitions | `Waiting → Loading`, `Waiting → Room` | Implemented + tested | PASS |
| Gameplay | 8 seats, team 0..7, winner art only (no per-team panel) | Seats + ready flags; no teams/winner art. Live fold gap, narrowed 2026-11-09: the guest toggle folds into game slots and self-seat reads slot readiness, but live snapshots number members off the slot indices (wire: member 1, no slot 1), so the fold misses silently — paint proven, lookup convention OPEN | OPEN |

## Loading gate (song select → play)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | `MusicLoading.ojs`, Trebuchet/Arial fonts, song-table walk + clock arm (`FUN_004e47a0` file `0xe47a0`, ev `68d920c2…`); OJN decrypt + Vorbis init precede it (lifecycle §9) | Soak stages genuine planet art full-canvas (`loading.png`); bare `BOUND` makes full-bleed the documented fallback, but per-sprite native sizes are ignored (audit 23). Re-shot 2026-11-08: the walk threads the gate phases (entry plus staging/decrypt/decode/arm) through `loading_phase_status` with the staged catalog title/artist (song 757, `OJNList.dat` row through the lossy UTF-8 decode — CP949 bytes render as replacements around the ASCII `(O2 Ver.)[3K]` tag) and holds the clock-armed full fraction, so the frame proves the `with_status` path (252 px of new caption ink at the placeholder row, bar unchanged full). Guarded by `loading_stage_drives_all_gate_phases_with_catalog_caption` and `loading_shot_paints_staged_fraction_and_caption`. Still FAIL: bar and caption row are unbound centered placeholders (audit 35), no fonts, no OJN/Vorbis path | FAIL |
| Input | Host start triggers gate | Confirm → Playing, Back → Waiting | OPEN (no validation/staging) |
| Timing | Select validation `0x5718b0` → staging `0x4f8b80` → decrypt `0x412180` → codec `0x54db40` → load screen | Instant on Confirm | OPEN |
| Transitions | `Waiting → Loading → Playing`, `Loading → Waiting` | Implemented + tested | PASS |

## `STATE_PLAYING` — gameplay (`CStateNormalPlaying`/`CStateLivePlaying`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Real UI in `Playing1.opi` (235 entries, `State_Playing.bnd` — bound run is entries 213–229 `EXIT`…`VOLUME`, decl `0xE6`; bnd-semantics); life/jam gauge bars, `Winner_image.ojs` (lifecycle §10, gameplay lifebar/jam sections) | Highway now derives from staged art, not frame fractions: uneven lanes `(5,28)…(165,28)`, judgement row 487, 7 px heads, per-lane receptor heights, jam bars `(23,536)`/`(570,536)` 206×16, peer 46×8 (5cb01a2; audit 4–9 superseded); soak proves it (`play_skin_decodes_every_lane_when_present`, `skinned_highway_replaces_procedural_note_ink` green). 2026-11-07 re-shoot: `playing.png` regenerated byte-identical to e9fd95f (no pipeline drift), then region-checked against `reference-4x3.png`. Per-region: **highway panel** structurally matches (lanes, receptors, staged JAM bar + strip combo/score; big COMBO/word/LONGNOTE digits stay small-font at invented anchors — audit 31); **edge gauges** match (staged life/jam gauge art at record anchors, both flanks within photo tolerance); **stage scene** matches with vintage deltas (stage-5 studio stands for the photo's unshipped rooftop, documented in shots; figure extent-centered in the `Map_Avatar.bnd` seat, `Lv.43` plate + amber gauge at photo offsets, HOT TIME badge staged); **bottom strip** partial (counts/TIME/speed/title/EXIT + staged panel paint; `VOL 58` now staged too — shots `playing_shot_paints_the_staged_volume_readout` red→green; big `0005` play-time digits over the bar stay on an assumed anchor — audit 31 — and procedural white TIME fill rides over the staged bar art, audit-16 family). Residuals documented, no invented covers: staged `Note_BG.ojs` right-side red panel shows at (570,458)–(774,478) where the photo's venue shows deck (exe cover untraced); staged strip panel is blue metallic vs the photo's dark strip (venue art vintage); BO/EA05 edge labels unpainted. **Mirror** 2026-11-08: the walk seats the wire peer's gauge through the session bridge (`apply_peer_echo` slot 1 `STAT_LIFE` full — the same stats-then-ranks order `OnlineSession` folds 4015 echoes through), so `playing.png`/`playing-native.png` carry the live green fill at the slot-1 rect (754,8,46,8; 213/368 exact-green native px, majority-green guarded by shots `playing_shot_paints_session_fed_peer_gauge`); the walked strikes both rate Cool, whose Hard +1s saturate, so local life stays `LIFE_MAX` behind the avatar bar (same guard). The same regen refreshed `waiting.png` by 329 px in the chat band with zero fixture change — pre-existing drift (reproduces with the gauge change stashed), not from this work. Still FAIL: gauge appended full-canvas over the bound window (audit 16), judgement/combo/jam digits at invented anchors (audit 31), linear scroll map (audit 32), curtain clips a frame box not lane columns (audit 30) | FAIL |
| Input | 7-lane keys, judgement vs `\|song−note\|` ≤ 6/18/25 ms → flags `0x10/0x20/0x40/0x80` (gameplay judgement, file `0x176540`) | Lane strikes hit `LivePlay`; windows live in `o2jam-game`, shell uses wall clock | OPEN (shell clock not sample clock) |
| Timing | Sample-accurate song clock; per-frame bg-effect loop `0x550db0` (ev `4164031c…`) | 60 Hz `FrameClock`; paint-advance to Result at chart end tested | OPEN |
| Transitions | `Loading → Playing → Result` | Implemented + tested, incl. auto-advance | PASS |
| Gameplay | Cool/Good continue combo, Bad/Miss break; pills (15-Cool mint, spend-on-Bad); score `200+10X/100+5X/4/−10`; life max 1000, per-difficulty deltas Cool +3/+2/+1, Good +2/+1/0, Bad −10/−7/−5, Miss −50/−40/−30, dead gate at 0 (gameplay, all VERIFIED; life: switch `0x170fa2` ev `feebe490…`, scorer `0x170510` → adder `0x540a80`) | Implemented + tested in `o2jam-game` (`LifeBar` + `LifeDeltas`: difficulty sets, 0..=1000 saturation, dead gate), not wired to shell scenes | OPEN (shell-side) |

## `STATE_RESULT` — results (`CStateResult`)

| Dimension | 7.55 expectation (evidence) | Shell status | Verdict |
|---|---|---|---|
| Composition | Decl `0x07`, 8 entries: `Result_WinLose.ojs`, `Result_NumFont_*.ojs`, Top/Bottom, Retry; builder `FUN_004c2390` (file `0xc2390`, ev `d6de3a7f…`) | Soak stages genuine panels — `result.png` shows real COOL/GOOD/BAD + music-name art with digits on top. Re-shot 2026-11-07 post-`04e2462`: the rank table now paints row 0 (`YOU`, `1/0/0/0`, `200`) at the pinned columns (guard: `result_shot_paints_the_rank_table_row`). Still FAIL: avatar/digits/verdict sit at fixed-pixel anchors `(16,16)`/`(16,32)`/`(16,48)` behind a 64×52 scrim (audit 13), ignoring the 7 `State_Result.bnd` rects; no WinLose art, grade letters out (mapping OPEN) | FAIL |
| Input | Retry → waiting loading; leave → room/channel; event-ticket web call (file `0x24a338`) | Confirm → Waiting, Back → Room | OPEN (no ticket call) |
| Timing | None documented | Paint-driven, immediate | PASS (vacuous) |
| Transitions | `Result → Waiting`, `Result → Room` | Implemented + tested | PASS |

## Side states (reachable from lobbies; not in shell)

All **OPEN** (no shell screen, no implementation — documented so no
pass is ever claimed):

- `STATE_SHOP` / `STATE_MUSICSHOP` (`CStateShop`/`CStateMShop`,
  `0xA4`/`0x6B`; `Payment` back-ends, `CMyBag`/`CCart`). Screens,
  paint (staged art or flat card, no rows/data), and all-reject
  flow live; buy/gift/cart/search/sort/pager flows and entry
  unwired (all OPEN).
- `STATE_MYROOM` (`0x6A`; `CItemManager`, `CPlayerInfo`). Screen,
  paint (staged anchor art or flat card, no slots/captions), and
  all-reject flow live; slot selection, equip, pager, gift flows,
  captions, entry unwired (all OPEN). Bind file declared but absent:
  art paints at sprite anchors, never rects.
- `STATE_PLANET` (`CStatePlanet`, `0x11`; BGM `Music/Planet.ojm`;
  quest button commented undecided — no drum backend). Screen,
  paint (staged art or flat card, no captions/cursor/selection),
  and flow edges (`Channel→Planet`, `Planet→Room`) live; buttons
  and BGM unwired, entry UI awaits tab semantics (all OPEN).
- `STATE_LANKING` (`0x37`; display-only, no state class — like CHANNEL).
  Screen, paint (staged art or flat card, no rows/data), and
  all-reject flow live; rank fetch, sorts, search, pagers, entry
  unwired (all OPEN).
- `STATE_PAYMENT` / `STATE_PAYMENT2` (web billing views).
- `STATE_TUTORIAL` (stub `0x00`; entry path unknown).
- `STATE_DRUM_PLAYING` / `STATE_EXIT` (stubs `0x00`; drum never wired,
  no `CStateDrum*` class; exit is teardown, not a screen).

## Reference-photo verify pass (2026-11-07 re-shoot)

All seven PNGs regenerated via `cargo run -q -p recomp2jam -- shots`
(`cargo test -p recomp2jam` 75/75 green, incl. the new
`result_shot_paints_the_rank_table_row` guard and the three
musicselect guards). Per-region read is
ink-presence per band (shot std vs photo std — content differs by
scenario, so structure, not pixels, is compared):

| Shot vs photo | Region verdicts |
|---|---|
| `playing.png` vs `reference-4x3.png` | HOLD, byte-identical to the e9fd95f bytes (no pipeline drift). Highway panel (59.5 vs 78.7), edge gauges (40.8 vs 64.9), avatar scene (67.6 vs 41.5), bottom strip (75.3 vs 54.0) — every band carries staged ink on both sides. Residuals unchanged: stage-5 studio stands for the unshipped rooftop, `Note_BG` red panel at (570,458)–(774,478) with no exe cover, blue-metallic strip vs the photo's dark strip, BO/EA05 labels unpainted |
| `waiting.png` vs `reference-waiting-4x3.png` | REFRESHED 2026-11-07 (scripted 3012/3013 chat fixture — see below). Top panel (59.6 vs 65.6), seat grid (55.5 vs 53.0), right column (59.4 vs 61.4) unchanged; bottom chat band carries two live rows plus the input caret (bracket-form amber rows at the photo-pinned 17 px pitch — see the 2026-11-08 chat-box verify below). Caption refresh 2026-11-07 (see below): only `waiting.png` changed again — 682 px confined to the six caption rects (MUSICNAME/MAPNAME/BPM/3×SKILLCOUNT), every other shot byte-identical, no pipeline drift. Residuals: invented 2-col x 4-row seat grid, no team buttons/covers (the song dialog lives in `musicselect.png` below); chat texts are ASCII stand-ins, no paging/scroll |
| `result.png` vs `reference-result-4x3.png` | REFRESHED (was stale). Score dome (48.6 vs 50.7), verdict banner (46.5 vs 48.7), rank table (85.3 vs 48.5), footer (59.1 vs 51.2). The checked-in PNG predated the rank-row wiring (`04e2462`) — its row band was empty; the re-shoot carries row 0 (`YOU`, `1/0/0/0`, `200`) at the pinned columns, guarded by `result_shot_paints_the_rank_table_row` (proven to fail on wrong text). Scenario deltas vs the photo stay: GOOD! solo arm vs LOSE, one local row vs two wire rows, P plate vs C |
| `musicselect.png` vs `reference-musicselect-4x3.png` | NEW (was UNCOVERED — no shot mapped to it). The walk now repaints Waiting through the host song flow (installed 7.55 562-song catalog, pick id 757 Hard X3.5; 4-song synthetic fallback without game data) under genuine `DAILOG_SELECTMUSIC` art, and the staged structure paints over it: filter columns (75.0 vs 183.4), search table (61.3 vs 83.2), status + search row (76.4 vs 114.7), chosung + OK (84.3 vs 128.1) — staged ink in every band on both sides. Still FAIL: per-chart level and grade cells draw the unknown dash (never staged), the 다운/듣기/정보 icon cells stay blank (no staged art), chosung hangul + word cells stay blank (no CP949 glyphs), the dialog bind pairing is unproven (the every-bound-entry proof covers Playing + Room only), and pager/search/chosung are display-only. Guarded by `musicselect_shot_repaints_waiting_with_song_panel`, `musicselect_fallback_catalog_lists_four_synthetic_songs`, `musicselect_dialog_structure_paints_search_tab_over_waiting` |
| `channel.png` | REFRESHED 2026-11-10: rows/hit-test/scrim now ride the exe-pinned slot rects (live memory read: CStatePlanet dialog+`0x94`) with the enter face at row-top + 12; dialog art staging unchanged from 2026-11-09 (800x466 parsed body at (0,0), 44x21 enter frames at the faces, live list/cursor/pager on top). Still FAIL: face X/chrome, interior chrome, and pager stay reference-measured; select-bar/back/close/num/count/scroll parts unmounded (sizes pinned, positions OPEN) |
| `room.png` | HOLD, byte-identical (no drift). No reference photo covers it |
| `loading.png` | REFRESHED 2026-11-08: the walk holds the clock-armed full fraction with the staged catalog caption (song 757) — 252 px of new caption ink at the placeholder row, bar unchanged full; guarded by the two loading-stage tests. No reference photo covers it; staged `MusicLoading.ojs` art + green bar under the no-fill rule (empty stage would paint nothing, staged art is unaffected) |

Chat refresh 2026-11-07: only `waiting.png` changed (1104 bytes
of two chat rows at the list origin); every other shot byte-identical
— no pipeline drift. Earlier deltas stand: `musicselect.png` is new;
the rank-row refresh added 129 pixels of row-0 ink.

Chat-box verify 2026-11-08 (chat band vs
`reference-waiting-4x3.png`, photo pixels pinned per item; failing
tests first, then the shared painter): re-shot `waiting.png` (chat
band rows 225–250 + caret block) and `musicselect.png` (the same
input strip's caret, 10 px). `playing.png`/`playing-native.png`
regen diffs reproduce at HEAD with this change stashed — pre-
existing drift (committed bytes predate a HEAD jam-effect change),
not from this work. Per-item:

| Item | Verdict |
|---|---|
| Composition (sender/text rows at the list origin) | FIXED — user rows bracket the nickname right-aligned in a 13-wide field (`[{sender:>13}] text`), the photo's `[        Lujia] YOOO!!!` shape (was `sender: text`); rows paint bare on the box art, per-row scrim bands removed. Residuals: field width pinned by the one photo row (2-byte-name padding OPEN); `[ADMIN]` prefix stays a stand-in (the photo's admin notice shows no sender); the photo's `/r` command echo has no shell source |
| Ink colors | FIXED — user rows amber (photo user row reads warm while known-white ink in the same photo calibrates neutral; was white); admin stays amber. Residuals: distinct user-vs-admin hues unverifiable in the lossy photo; caret white per photo |
| Row stacking | FIXED — pitch 17 native px (photo row tops 455.4/472.5/489.8; was the lobby 8 px), 6 visible slots fill the 98 px `LIST` rect exactly (was 12); fixture reordered admin→user so the peer's row sits last, matching photo rows 2–3. Chronological top-down confirmed both sides |
| Draft line | FIXED — focus caret now paints: 2×10 white bar inset (5,5) of `EDIT_SWAIT_CHAT` rect 19, riding the typed width (photo shows the caret on an empty input; we painted nothing). Residuals: draft-text origin unevidenced (photo input is empty); `game.rs`'s "empty histories paint nothing" sentence predates the caret (out-of-scope file — follow-up) |

Guards: `waiting_chat_row_text_brackets_the_sender_field`,
`waiting_chat_rows_stack_at_the_reference_pitch`,
`waiting_chat_paints_user_rows_in_amber_at_the_list_rect`,
`waiting_chat_without_live_rows_paints_only_the_caret`,
`waiting_chat_paints_the_caret_on_an_empty_input`,
`scripted_chat_fixture_delivers_admin_then_user_rows`
(renamed), `waiting_shot_paints_the_scripted_chat_rows` — all
proven red before the fix (compile-red for the new painter API,
order-red for the fixture swap).

Caption refresh 2026-11-07: the waiting walk now stages the song
catalog plus inventory before the shot (room song 757, counts
`7/42/1500` with the slot-3 999 cap), so MAPNAME/BPM/SKILLCOUNT
ink their `.bnd` rects instead of the bare `M0`/`-0-`/`  0`
defaults. Old-vs-new diff is 682 px, all inside the six caption
boxes — MUSICNAME native (573,15), MAPNAME (620,448), BPM
(552,504), skill counts (712,208/246/284) — each cluster matching
its rect; no other band moved. Guarded by
`waiting_shot_paints_mapname_bpm_skillcount_captions` (exact text
at each rect, proven against the staged sources).

## Waiting / music-select region verify (2026-11-07 caption re-shoot)

Both PNGs regenerated via `cargo run -q -p recomp2jam -- shots`
(`cargo test -p recomp2jam` 97 green, incl. the three chat guards
plus the new caption guard below); `musicselect.png` byte-identical
(no drift there), `waiting.png` refreshed with staged captions.
Bands below are column-
restricted luminance std (shot vs photo — structure, not pixels):

### `waiting.png` vs `reference-waiting-4x3.png` — REFRESHED, still FAIL

| Region | Shot | Photo | Verdict + residual |
|---|---|---|---|
| Top panel (title/Ch/Wins bar) | 51.2 | 56.7 | HOLD — staged art + title/song/wins ink on both; shot room is the `Jam` duo (no Ch/No/MX3 line, song caption now resolves the staged catalog title for 757 instead of bare `M0`) — scenario delta |
| Seat grid (covers, champion row) | 45.0 | 47.7 | HOLD — staged covers + seat text on both; shot names no CHAMPION/CHALLENGER roles (joined room carries no champion) and centers portrait text blind (audit 18–19); seats still on staged cover rects, team grid highlight only |
| Right column (TEAM/SKILL/SPEED) | 43.1 | 49.7 | REFRESHED — staged panel + button/speed ink on both; BPM now inks the catalog column of song 757 (was `-0-`) and the three SKILLCOUNT rects ink the staged inventory (`  7`/` 42`/`999`-capped); team highlight is the live amber cursor over the staged `Wait_Team?.ojs`, no covers; skills paint ids only, counts ride the staged inventory (no wire field) |
| Bottom chat/song (chat, arrow, Random, BACK) | 60.1 | 57.8 | REFRESHED — the history box stages two scripted 3012/3013 rows in the photo stacking (amber bracket-form rows at the 17 px pitch, admin notice on top, `Lujia`'s user row last) plus the input-strip caret (2026-11-08 chat-box verify below); Random cards and BACK stay staged art (no card/nav state rides the snapshot). MAPNAME now inks the map-cycle entry of room word 5 at its `.bnd` rect (was bare). Guarded by `scripted_chat_fixture_delivers_admin_then_user_rows`, `waiting_walk_without_staged_chat_holds_no_history`, `waiting_shot_paints_the_scripted_chat_rows`, `waiting_shot_paints_mapname_bpm_skillcount_captions` |

Composition stays FAIL (invented placement where noted above; no team-button bodies or covers staged).

### `musicselect.png` vs `reference-musicselect-4x3.png` — HOLD, still FAIL

| Region | Shot | Photo | Verdict + residual |
|---|---|---|---|
| Filter columns (level/wheel/genre) | 24.6 | 61.5 | HOLD — staged art + column/button ink on both; lower shot std is the 562-song catalog vs the photo's 1608 plus ASCII stand-ins (no CP949 hangul); pager/search/chosung display-only |
| Search table (label/note/title/artist/lvl/icons/grade) | 25.4 | 30.8 | HOLD — 10 staged rows + header on both; per-chart level draws the unknown dash, grade blank, 다운/듣기/정보 icon cells blank (no staged art) |
| Status + search row | 26.2 | 39.0 | HOLD — `ALL -> ALL SEARCH RESULT.` + pager `1/<pages>` + title/artist radios + edit + search button on both; photo page `3/161`, shot page 1 (no paging input staged) |
| Chosung + OK | 37.4 | 46.3 | HOLD — `A`–`Z` letters + OK pill on both; hangul jamo + 숫자/기타 word cells blank (micro-font has no CP949 glyphs), dialog bind pairing unproven |

Dialog stays FAIL (blank level/grade/icon/chosung cells, unproven bind pairing, display-only pager/search/chosung).

## Wine capture verify pass (2026-11-08 room/waiting/loading)

Shot pipeline regenerated via `cargo run -q -p recomp2jam -- shots`
(`cargo test -p o2jam-shell -p recomp2jam` green, `cargo fmt --check`
clean): all PNGs byte-identical — the fallback-only change below
touches no staged path. Compared `docs/screenshots/room.png`,
`waiting.png`, `loading.png` against `~/scratch/aa-room.png`,
`ac-room.png` (room), `ab-wait.png`, `ad-wait.png` (waiting),
`ab-load.png`, `z-load.png` (loading) plus siblings:

| Scene | Verdict vs Wine |
|---|---|
| Room | FAIL — real lists 2 columns x 3 rows of panels (number/title/HX1/3KEY/green bar/WAITING badge, detail/avatar/chat right, modal OK dialog over it); shell rows stay single-column quarter-frame ink over staged panels, details at rect corners, no create/password/modal UI. No shot drift (staged path unchanged) |
| Waiting | FAIL but IMPROVED — staged path matches the real 4-across x 2-down seats, TEAM A–H block, SKILL/SPEED column, BPM box, Random cards, BACK. Fixed this pass: the art-less fallback grid was an invented 2-col x 4-row (`seat_origin`); now 4x2 per the captures (new `waiting_seat_grid_is_four_columns_by_two_rows`, red on the old grid). Residuals: portraits center blind (audit 18–19), no team-button bodies/covers, ASCII chat, song dialog lives in `musicselect.png` |
| Loading | FAIL — real is a song-illustration full-bleed (bead progress pills, 3-KEY badge, Artist/Remix/Pattern credits); shell stages planet `MusicLoading.ojs` art with the centered bar + caption placeholders (audit 35), song caption only. No invented pass; bar/caption rects unmoved |

## Scoreboard

Verify pass 2026-11-08 (chat-box verify on top of the
2026-11-07 passes: post no-fill, chat refresh, caption refresh):
2 HOLD (playing + room byte-identical at their last
verified pass; the 2026-11-08 regen carries pre-existing
`playing.png` drift — see the chat-box verify note), 5 REFRESHED
(result rank rows; channel art-less re-shoot; waiting scripted
chat rows + staged captions; waiting chat band re-verified and
corrected; loading live fraction + caption), 1 NEW (musicselect song dialog). Detail in "Reference-photo verify
pass" above.

- Scenes covered: 7 main + musicselect dialog + loading gate + 9 side states.
- PASS: boot-into-splash, 800×600 default (`SetWindowPos` file
  `0x14b4c`/`0x14baf`), all 11 documented transition edges (plus
  rejections), vacuous-timing scenes — 14 cells.
- FAIL (known placement gaps): 8 composition cells (genuine art now
  stages in the local soak — shots green, seven PNGs refreshed 2026-11-07 —
  but invented placement remains per cell: audit 3, 10–16, 18–19,
  23–25, 30–32; audit 4–9 superseded by the verbatim highway 5cb01a2).
  The `load_scene` layering behind them stays harness-proven (22/22).
- OPEN: all input/timing/gameplay cells above plus every side state —
  next evidence targets are the loading-gate validation sequence
  (`0x5718b0` → `0x4f8b80` → `0x412180`), the peer-life 4015 handler,
  and per-cell rect bindings (channel bar/rows, room `IDC_*` fields,
  waiting seats/teams, result digits, playing digits/scroll).
