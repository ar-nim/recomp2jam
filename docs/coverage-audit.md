# Coverage audit — recomp2jam (implemented-vs-not) against the decomp2jam ledger

Date: 2026-11-10. All original-side addresses are **OTwo.exe 7.55**
(image base `0x400000`; *file offset = VA − 0x400000* throughout),
size 5,029,888 B, SHA-256 `83a04541…72c823b`. Behavior oracle: the exe.
Everything the other project contributes is **roadmap and corroboration
only** — see the clean-room boundary below.

## Clean-room boundary

- **recomp2jam** (this repo): clean-room Rust reimplementation.
  Behavior is described in our own words in `docs/`, pinned to the exe
  by VA/file offset + REA evidence IDs or live captures, then
  re-implemented. No original bytes, no disassembly, no decompiled
  code in the repo.
- **decomp2jam** (`~/Projects/open3jam/decomp2jam`): C/C++ rebuild
  transcribed from asm (reccmp "matched" = compiles byte-identical).
  **Not clean-room.** Its `docs/progress/known-functions.csv`
  (7,010 rows) and progress docs are used here only as (a) a coverage
  map of the exe ("which functions exist, where, roughly what role")
  and (b) a corroboration source for pins we hold independently. No
  code, asm, or structure transcription from it enters this repo; any
  behavior claim it inspires must be re-verified against the exe on
  our side before it is coded (owner rule: the exe arbitrates).

## Cross-check sample (5 rows) — their claims vs our pins

| # | decomp2jam claim (VA) | recomp2jam pin | Verdict |
|---|---|---|---|
| 1 | Scene-music selector `0x00551820` (35 B, `RET 8`) forwards `(a, b)` to a thiscall on the bank sub-object at `+0x340C0`, which latches flag bits, stores the id, and pulses a worker event (`src/ui/pair_5518_42ff.cpp`; host global `0x0076832C` in `include/UiHosts.h`) | Fresh pin: selector **file `0x151820` = VA `0x551820`**, signature `(id, loop)`; bank `soundmgr[0x76832c] + 0x340c0`; `Music/BGM.ojm` **one-based** records; negative id = silence | **AGREE** — VA, arity, bank sub-offset `+0x340C0`, host `0x76832C` all match |
| 2 | `CStateAvi::v4` (`0x004F5730`) issues `0x551820(8, 0)`; `CStatePlanet::v4` (`0x0048DD10`) issues `0x551820(9, 1)` (`src/ui/CStateAvi.cpp`, `src/ui/CStatePlanet.cpp`) | Fresh pin id table: Avi **8** (once), Planet **9** (loop) | **AGREE** — including the once-vs-loop distinction |
| 3 | `+0xF6C` is the state manager's current-state slot; every state stores there (`docs/progress/HANDOFF.md` tail-decode `0x478906`) | `docs/channel-fill.md` (live read 2026-11-09): mounted child at `parent+0xF6C` is the **`CStatePlanet`** object hosting the channel dialog; phase flag at `+0xF64` | **AGREE with refinement** — both true: the current state *is* CStatePlanet while the dialog is open; no contradiction |
| 4 | Judgement `0x576540` `key_consumer` **matched**; life add/clamp `0x540A80` **matched**; scorer `0x570510` `set_judge` **matched**; jam threshold table `0x570260` **matched** (`docs/progress/known-functions.csv`) | `docs/gameplay.md`: judgement windows 6/18/25 ms at file `0x176540` (VA `0x576540`, sites `0x176646/55/64`); life deltas + adder `0x540a80`; score bases in `FUN_00570510` (VA `0x570510`, arg 4/3/2/1 paths) | **AGREE** on every VA; only role *names* differ (`set_judge` vs scorer paths) — naming variance, not behavior |
| 5 | Channel-connect dispatcher `0x0058C6C0` read end-to-end: difficulty-button codes (`0x19201002`…) select host/port pairs from two tables + three host strings (`0x64E260/50/40`) with random 10 000–60 000 jitter (`HANDOFF.md`) | `docs/planet.md` / `docs/channel-fill.md` (live 2026-11-09): emblem clicks switch server channel families; entry-face click drives `03EC/0FBE/07D0/07D2/13A4` → `STATE_ROOM` | **AGREE** — their static read explains the mechanism our live capture observed; `FUN_0048C810` (our button→index map, `ev_13212723…`) is `matched` in their CSV |

Sample verdict: **5/5 agreement**, no refutation. Their ledger is a
reliable roadmap at the address level; behavior text still needs our
own exe verification per the boundary rule.

## Side A — decomp2jam ledger totals

`known-functions.csv`: 7,010 rows → **matched 6,219 (88.7%)**,
**analyzed 785 (11.2%)**, parked 2.

| kind | total | matched | analyzed | Named game-logic content |
|---|---:|---:|---:|---|
| game | 3,362 | 2,922 | 440 | bulk of O2Jam logic incl. leaf batches, msg_* handlers |
| core | 2,038 | 1,864 | 174 | state machine (`0x477990` factory, slots `+0xF6C`), CControlList, CStateBase family, CInputManager, CKeyConfig, messenger, net queue/dispatch, CSystem |
| crt / stl / func / function | 1,215 | 1,126 | 89 | compiler runtime, not game behavior |
| render | 73 | 58 | 15 | CBlend16, conv565, note_draw, dev_step, mode_set |
| libjpeg / zlib / libvorbis / boost | 138 | 120 | 18 | vendored libraries |
| sequencer | 23 | 17 | 6 | `note_spawner_a/b/c` `0x575C20/70/D00`, `auto_sound_driver` `0x5763D0` (matched), `auto_miss_drain` `0x577630` (matched), tempo envelope `0x57A4E0`, `tick_to_ms` `0x57A240`, `visible_window_scan` `0x577920`, `sequencer_start` `0x5702B0`, `ojn_header_validate` `0x5718B0`, `chart_compile` `0x571D60`, `ojm_load` `0x5734E0` (analyzed) |
| judge | 6 | 4 | 2 | `life_add_clamp` `0x540A80`, `jam_threshold_table` `0x570260`, `set_judge` `0x570510`, `key_consumer` `0x576540` (all matched); `seq_callback` `0x56E170`, `key_up_release` `0x576719` (analyzed) |
| audio | 20 | 13 | 7 | `SceneMusic` (`0x551820` chain, `BGM.ojm` one-based), `OjmReader`/`COjmSampleReader::render`, `sample_play_once/looped`, `play_note_sound` `0x5703F0`, cfg volumes `0x551580/16D0/17A0` |
| ui | 14 | 9 | 5 | CStateAvi, CStatePlanet, CStateRoom, CStateShop(v19), CStateTutorial(v16), CStateResult(v10), CStatePayment(+2) — v4 entries and helpers |
| interface | 5 | — | 5 | `NormalWaiting`, `LiveWaiting` |
| net | 6 | 3 | 3 | packet refcount, `CPacket_Str`, `CmdHost::send` `0x417930`, `client_768358`; connect dispatcher `0x58C6C0` (analyzed/read) |
| avatar | 9 | 8 | 1 | `CPlayerInfo` ctors/fields/del, `ItemMgr` |
| playing | 5 | 0 | 5 | `normal_playing_loop` `0x4EF03D`, `live_playing_loop` `0x4E85F0`, `song_begin` `0x4DB6E0`, `music_timer_tick` `0x4EA800`, `play_state_load_init` `0x5700C0` |
| state | 2 | 0 | 2 | the two playing loops (same as above) |

**Their state-class roster** (`docs/progress/STATE-TABLE.md`, all from
RTTI + the factory jump table at `0x4789C4`): ids 2–13 mapped —
`CStateRoom` `0x48E700`, `CStateShop` `0x497F60`, `CStateMyroom`
`0x4A3560`, `CStateResult` `0x4C1730`, `CStateAvi` `0x4F5630`,
`CStatePlanet` `0x48DB90`, `CStateMShop` `0x4C6760`,
`CStateNormalPlaying` `0x4EB610`, `CStateTutorial` `0x4F07F0`,
`CStatePayment` `0x4F33F0`, `CStatePayment2` `0x4F45E0`; id 5 builds
`CNormalWaiting` `0x4B0550` / `CLiveWaiting` `0x4AAA70` over
`CStateWaiting` `0x4B5D10`; `CStateLivePlaying` `0x4E3340` has no
factory caller (created by the playing state itself). Same conclusion
as our `docs/lifecycle.md`: **no `CStateChannel`, no `CStateLanking`,
no `CStateDrum` classes exist** — channel/lanking are screens inside
`CStatePlanet`/`CStateResult`, drum is a stub. Two independent
projects, same negative findings.

**Their gaps (no delta from them for us):** playing-state bodies
(loops analyzed only), `CStateMyroom`/`CStateMShop` state bodies,
shop-send district (blocked on hub `0x52E1A0`), messenger-update
district (hub `0x594AA0`), dialog-open path — their `CStatePlanet.cpp`
documents the bar mount (`O2DXImageFind(… "Dialog_ChannelSelectBar.ojs" …)` into `+0x8C` inside `0x48DD10`, which we already pin at
`docs/channel-fill.md`) but **not** the `Dialog_ChannelInfo` open
trigger. Our own pin: dialog-open `0x4ED650` (file `0xed650`) is the
writer site (`docs/channel-fill.md` §dialog writers); the dynamic
mount call itself stays OPEN on both sides.

## Side B — recomp2jam implemented systems (evidence)

Parsers (`o2jam-file`, all tested against real 7.55 data):
OJM M30/OMC/new-variant ✅ (`parse_m30`, `parse_omc`,
`parse_new_variant_entries`, `decode_new_variant_pcm`,
`recover_new_variant_sample`); OPI/OPA ✅ (`parse_opi(_slots)`;
inner formats: CP949, sprite-55 + strided combo, `.bnd` rects,
ControlList↔rect pairing `(code&0xFFF)-1`, 38/38 Interface);
OJN old header + new decrypt ✅ (`decrypt_ojn_new_format`,
`parse_ojn_levels`, `build_level_chart`, 14/14 walks); JPEG table
driver ✅. Game rules (`o2jam-game`, pure + tested): judgement
`TimingWindows{6,18,25}` ✅; score/combo/pills ✅ (`CoolStreak`,
`PillStock`, `apply_rated`); `LifeBar` + `LifeDeltas` ✅
(0..=1000 saturation, dead gate — `docs/gameplay.md` §Life bar
VERIFIED); long-note `HoldState` ✅ (hold machine + release-tail
rating + tick bonus `+10/+1 combo` VERIFIED `docs/gameplay.md`);
lane-effect map reshuffle ✅; room teams ✅; `PeerBars` 4015 mirror ✅.
Audio (`o2jam-audio`): lewton decode + cpal backend + sample clock ✅
(bgRoom 53.8 s parity); scene mixer landed; scene-BGM selector and the
first pinned UI-effect cluster wired (#721, #307). Render (`o2jam-render`):
software compositor (RGB555 blit/clip/colorkey) ✅; wgpu TODO.
Shell (`o2jam-shell`): all 7 main scenes + 9 side states flow through
`ScreenGame`/`load_scene`/`ScreenFlow` ✅ (14 PASS cells,
`docs/parity.md`); composition cells FAIL on invented placement
(audits 3, 10–16, 18–19, 23–25, 30–32); input via winit keymap ✅;
exe key bindings OPEN. Net (`o2jam-net`): protocol library — client,
command/frame/message, rooms, play, relay, shop, gift, wallet,
character, strings, inflate (225 pub items) + `docs/net-protocol.md`
endpoint/opcode map; shell fold covers 3012/3013 chat and 4015
peer-life echo; the rest is documented, not shell-wired.

## Audit matrix

Legend — our status: **IMPL** = implemented + tested; **PART** = some
layers implemented; **DOC** = documented/pinned only, no code;
**NONE** = not started; **OOS** = out of scope.

| Subsystem | decomp2jam coverage | recomp2jam status | DELTA (what they know that we have not implemented) |
|---|---|---|---|
| States / flow | Factory `0x477990` + jump table `0x4789C4` matched; 12 classes placed by RTTI; tails decoded (`HANDOFF.md`) | **PART** — flows/transitions for all scenes ✅ (`parity.md` 14 PASS); state-class identity pins agree | Their per-state v4 body roster pinpoints where scene-entry behavior (BGM cue, control setup) lives; we have entry *behavior* for channel only |
| Scene rendering + UI | CStateBase family, text controls, pagers, web ctrl matched; CDX boundary doc (their vendored CDX too old for mesh/texture) | **PART** — bnd rect staging ✅ (all side screens), composition FAIL (invented placement, audits cited) | Their matched text-control setters (`0x50F520/680/6A0/6F0` families, callee counts 96/37/38) corroborate our setter-trio pin; digit/combo/score anchor binding still ours to pin |
| Channel / planet behavior | `CStatePlanet::v4` `0x48DD10` reconstructed; connect dispatcher `0x58C6C0` read; emblem→server-family mapping static-read | **PART** — live-pinned geometry/hover/host (`channel-fill.md`); entry-face click verified; emblem→family verified live | Their static read of `0x58C6C0` (host/port tables + jitter) can guide our next live trace; BGM cue for Planet (id 9, loop) now pinned, unimplemented |
| Room / waiting | `CStateRoom` override inventory mapped (vtable `0x647BC4`); team mapping `FUN_0048C810` matched; S→C 4007 handler `0x55F470` analyzed | **PART** — flows ✅; seats grid fixed to 4×2 (Wine); teams in `o2jam-game` ✅; no create/password UI; 4007 refresh not shell-wired | Their `0x55F470` analysis (u8 idx `<8`, stride-`0x2190` slot refresh) corroborates our handler pin; waiting BGM id 6 + room BGM id 3 pinned and implemented via `Screen::bgm_cue` (#307) |
| Playing / judgement / score / life / pills / long notes | Judge chain matched (`0x540A80/570510/576540/570260`); playing loops analyzed; `key_up_release` `0x576719` analyzed | **PART→IMPL in crate** — all rule layers ✅ (`TimingWindows`, scorer, pills, `LifeBar`, `HoldState`); NOT shell-wired; shell clock ≠ sample clock | Jam gauge math: their `jam_threshold_table` `0x570260` is matched — strongest lead for our OPEN gauge chain (`+0x396e/+0x3970` identity, rating sink `+0xa14` codes 1/9/14). Needs our own exe verification |
| Result | `CStateResult_v10`, builder `FUN_004C2390` matched | **PART** — art stages, rank row 0 paints; digits at fixed anchors (audit 13); grade mapping OPEN; SubmitScore fields known | Result BGM id `0x13` once + retry BGM re-cue pinned via selector; worker bank now pinned to `BGMwinter.ojm` mode-3 ordinal 6 (`bgResult`) by static trace (#904); WinLose art wiring (`Result_WinLose.ojs` `0x24b014`) ours to finish |
| Audio (BGM / mixer / ojm) | `SceneMusic` chain reconstructed (`0x551820→0x54BC50`, one-based `BGM.ojm`); `COjmSampleReader::render` analyzed; volume cfg matched | **IMPL (scene + first UI cluster)** — vorbis decode + cpal clock ✅; mixer landed; scene selector + per-scene table ✅; UI slots `1/2/5/9/0x23` wired on handler branches ✅ (#721); `bgEffect` bank table pinned below; Result's `bgResult` staged from `BGMwinter.ojm` (#904) | Scene ids and UI first cluster are guarded (below); the remaining #718 interaction clusters (buy/sell, NPC voice) and per-volume SFX levels stay pinned/OPEN |
| Net protocol | Thin: 6 net rows + queue/dispatch under core; `CPacket_Str::GetPacket` analyzed; `CmdHost::send` matched | **PART** — `o2jam-net` protocol library ✅ + endpoint/opcode doc; shell fold: chat + peer-life only | Both sides thin on *handlers*; their `msg_*` matched set (12 handlers under core) is a checklist for our remaining S→C handler pins |
| File formats | OJM walk (`m30_walk_a/b/c` analyzed); `ojn_header_validate` `0x5718B0` analyzed; asset gates | **IMPL** — OJM/OJN/OPI/OPA parsers ✅ tested (this is our strongest column) | `.oja` frames, outlier pixel codecs, OJNList trailer `;HD` semantics — open on both sides |
| Input | `CInputManager`, `Input.cpp`, devices, clear_flags matched | **PART** — winit keymap ✅; exe binds OPEN; latency budget OPEN | Their matched `CKeyConfig::loadKeys/syncKeys` `0x543510/7F0` corroborates our option.ini key-table pin (file `0x245224-0x2452a8`); xTd paired table decode still OPEN |
| Options / messenger / shop flows | Messenger msg_* handlers matched; shop-send district deferred (hub `0x52E1A0`); `CPlayerInfo`/`ItemMgr` matched | **DOC** — SetInfodata.ojs + Itemdata_Korea.dat mapped; all buy/gift/cart/search flows OPEN | Their *deferred* shop/messenger districts confirm these flows are genuinely hard, not skipped; no shortcut available from their side |

## Fresh pins — scene BGM selector (this audit)

- **Selector**: file `0x151820` = VA `0x551820`, signature `(id, loop)`;
  body forwards to the bank sub-object `soundmgr[0x76832c] + 0x340c0`
  (event-latch + worker wake); `Music/BGM.ojm` records are
  **one-based**; id −1 = silence. Corroborated independently by
  decomp2jam (`src/ui/pair_5518_42ff.cpp`, `src/audio/SceneMusic.cpp`).
- **Scene → id table** (per-state v4 entry issues the cue; loop flag
  semantics once vs restart):

| Scene | id | loop | Note |
|---|---:|---|---|
| Avi (logo splash) | 8 | 0 — play once | `CStateAvi::v4` `0x4F5730` |
| Planet / channel | 9 | 1 — loop | `CStatePlanet::v4` `0x48DD10` |
| Room | 3 | loop | |
| Payment / Payment2 | 3 | loop | |
| Shop | 4 | loop | |
| Myroom | 5 | loop | |
| Waiting | 6 | loop | |
| MusicShop | 10 (decimal) | loop | exe push `0xA` at file `0xC6E51`; the earlier audit's `0x10` shorthand is refuted |
| Result | `0x13` | once | exe call at file `0xC1CAF` (pushes at `0xC1CA5`/`0xC1CA7`); the earlier audit's "loop" is refuted. `0x13` is a **worker cache slot**, not a `BGM.ojm` record: slot `0xD + 6` holds `bgResult.ogg`, mode-3 entry 6 of `BGMwinter.ojm` (#904, static trace below) |
| Song preview | 0 | — | |
| Silence | −1 | — | |

- **Dialog-open trigger: still OPEN** on both sides (dynamic mount).
  Our writer-site pin: dialog-open `0x4ED650` (file `0xed650`) drives
  the caption setter trio (`channel-fill.md` §dialog writers);
  decomp2jam's `CStatePlanet.cpp` documents only the bar mount into
  `+0x8C` (already pinned on our side). Next evidence step: trace the
  caller chain that mounts the `0x12` Channel block on `parent+0xF6C`
  when the phase flag at `parent+0xF64` flips to 1.

## Cue-map coverage (2026-11-21, #307)

Scope: the per-state cue map, the static UI-effect map, and the bank table
that turns an effect id into a cache slot. All pins below are for OTwo.exe
7.55 (hash and image base above); no new code landed with this audit.

- **Scene cues — pinned and implemented.** The scene → `(id, loop)` table
  above is the single source in `Screen::bgm_cue` and
  `SideScreen::bgm_cue`; `recomp2jam::scene_bgm_cue` delegates to it. Every
  entry re-cues through `ScreenGame::enter_screen`/`visit_side`; the exe's
  cue sites file `0xF57D4` (Avi 8 once), `0x8DE28` (Planet 9 loop),
  `0xC6E51` (MusicShop 10), and `0xC1CAF` (call; pushes at `0xC1CA5`/`0xC1CA7`,
  Result `0x13` once) arbitrate over the earlier audit shorthand.
  Guards: `avi_launch_cue_selects_the_logo_record_once`,
  `scene_bgm_cues_follow_the_pinned_table`, `music_shop_cue_loops_record_ten`,
  `result_cue_plays_bg_result_from_the_winter_worker_bank`,
  `same_record_cue_keeps_the_running_track`, `cue_switches_on_a_different_record`.
  Loading/Playing/Lanking and the side screens with no pinned caller cue
  silence; Result's worker bank is pinned and staged (#904 below).
- **UI-effect map — pinned (#718).** The emitter is `0x551870` (file
  `0x151870`): ids `>= 0x34` are rejected, the id indexes a cache slot,
  and an active sample is stopped before the new one starts (dossier
  `ev_de048ee24121e6981c9ed6aff2d88235f8af29ca70a5200d80c284c79282c52d`).
  The static survey resolved 66 direct call sites in 37 handlers
  (inventory `ev_4272574b8958634793552f3ff8f347d8d3801103bd5afc139f505b650efdd1d2`;
  per-handler evidence and the full interaction → id table live at
  `/home/arnim/scratch/ui-sfx-static-findings.md`, sections 1–5). Sound
  sits in the producer branch, never in the queue posters `0x478d50`/
  `0x478c90`, the hover updaters `0x439000`/`0x48e530`, or the 1005
  channel-refusal answer `0x55a110`; those silent paths are pinned with
  skip reasons in that survey.
- **Bank table — pinned and implemented.** The boot table at file
  `0x27f728` has stride `0x48` and row-`+0` bases
  `[0, 0x28, 0x2f, 0x2f, 0x2f, 0x33, 0x33]` (dossier
  `ev_a9fb8aefefa70e2a45105090df82976fd44527a81a7029fd83d4f64e1a2bfd3e`).
  The fetcher `0x54eca0` writes the fetch ordinal at buffer `+0x2c` for
  both mode-2 (file `0x14edfe`) and mode-3 (file `0x14eed9`) entries;
  the bank loader `0x550db0` adds the bank's base to it. Row field `+0`
  (file `0x27f728`) is the **mode-2** base; row field `+4` (file
  `0x27f72c`) is the **mode-3** base. The UI-effect branch reads `+0`;
  the worker-music branch reads `+4`. Thus `bgEffect.ojm` (bank 0, mode-2
  base 0) maps slots `1/2/5/9/0x23` to entries `2/3/6/10/36`;
  slots `>= 0x34` and blank entries never sound. The correction to the
  old "field never read" note is committed in `docs/file-formats/ojm.md`.
  Guards: `boot_bank_table_names_all_seven_exe_archives`,
  `pinned_effects_round_trip_through_their_cache_slots`,
  `pinned_ui_effect_ids_name_the_exe_handler_cues`,
  `installed_effect_bank_decodes_the_first_cluster_slots` (data-dependent,
  skips cleanly with install-root hidden).
- **Wiring — implemented for the first cluster (#721).** `ui_effect_cue`
  beside `scene_bgm_cue` names the pinned ids; shell handler branches call
  `ScreenGame::cue_ui_effect` → `AudioBackend::play_key` (never
  `set_scene_bgm`), with ids recorded in CueLog's second vector and the
  decoded `bgEffect` PCM staged by `load_ui_effect_bank`. Scenario guards
  name live handler chains: `planet_emblem_pick_cues_effect_one`,
  `hovering_an_emblem_cues_no_effect`,
  `refused_channel_enter_cues_no_effect_after_the_row_pick`,
  `slider_apply_cues_two_but_reset_and_cancel_do_not`,
  `option_tab_switch_and_channel_exit_cue_their_pinned_ids`,
  `planet_exit_and_dialog_ok_cancel_cue_five_one_and_two`,
  `effect_cue_walk_records_the_pinned_sequence_in_cuelog`,
  `a_cue_without_a_staged_bank_entry_fabricates_no_sample`. The one-shot
  bus mixes over the continuous BGM (`bgm_keeps_its_sample_sequence_while_a_ui_effect_cue_plays`).
- **OPEN, not claimed by this coverage:** buy/sell `0x14/0x13` and NPC
  voice `0x28..0x2e` are pinned in #718 but have no wiring ticket; per-volume
  SFX levels (`0x67ed60`), the emitter's second-argument semantics, and
  audio capture by ear stay OPEN with the next evidence steps in the #718
  survey. The Result `0x13` worker bank is no longer OPEN (#904 below).

## Fresh pin — Result worker bank (#904, static)

Pinned to OTwo.exe 7.55, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`;
REA 6.9.0 CLI with Ghidra 12.2. File offset = VA − `0x400000` in `.text`;
data row offsets below come from `address-to-file-offset`.

Live caller chain: state dispatch → `FUN_004c1b90` (VA `0x4C1B90`,
file `0x1b90`; `ev_1cbc946c4f582ac36fd98fa5e0bb05ebb4e4d102cfbda1aacf0740c62a9be16f`)
→ `FUN_00551820(0x13, 0)` (file `0x151820`; call at `0xC1CAF`,
pushes at `0xC1CA5`/`0xC1CA7`;
`ev_fa3f6f8b9b47e9c71918d9d900856410c3070ef80ef813da9626f4d186d15da3`)
→ bank sub-object at `soundmgr + 0x340c0` (file `0x151832`, `add ecx,0x340c0`)
→ `FUN_0054bc50` (`ev_78e65d82656b266c69966b80db8ed056ae97b06d35ba3b2b3231914113eef4a3`):
stores the id at worker `+0x84`, sets the once/loop bit at `+0x80`, wakes
worker event `+0x74` → worker `FUN_0054bd60`
(`ev_b34b89d839433a81104e6ad8be18bdb6b10440e872a93697a758c4487dd3bf09`)
reads `+0x84` and indexes the worker pointer array at `worker + id * 4`
(file `0x10c0b2`; disassembly in scratch only).

Who fills that slot: boot loader `FUN_00550db0`
(`ev_75ba7fe104a34f019541c1ba1bb792b8b27aef67f0cc8048e7946b44e9986ba1`)
walks all seven bank rows at `0x67f730` (stride `0x48`). For **mode-3**
entries (fetcher flag `0x80000000` at buffer `+0x28`, set at file
`0x14eeef`; evidence `ev_1730117d444d1e97bd846434c86c110dd135126c5306b83c377705a84ebd88e5`)
it computes `slot = row(+4) + fetch ordinal`, rejects `slot >= 0x17`,
decodes the body with `FUN_0054f580`, and registers it in the worker array
through `FUN_0054b9f0` with `ecx = soundmgr + 0x340c0` (call at file
`0x10eee8`; setter evidence
`ev_1a6261453c321dc23c57b469e44abc211f9993f2bdf488d8d449fa8f60efe374`).
The mode-3 base field is file `0x27f72c` (VA `0x67f72c`, stride `0x48`;
`ev_f73477ba029f80c5d7bfcb0b5c9449dd2f5d9260ce880142338f1bfeda132518`);
row 5 is `Music/BGMWinter.ojm` (name at file `0x27f898`,
`ev_9bbdb57205ab0ed5a98a941fb5005c7fe526eddb7da8574b7e29a10ab21b1836`)
with base `0x0d` (file `0x27f894`,
`ev_884629791f26d1a5813e18d9488a3d01e6da5413a91122846ab881d470d7f4b1`).
Fetch ordinals are written for mode-3 too (file `0x14eed9`), so
`0x13 = 0x0d + 6`: slot 19 is `BGMwinter.ojm` mode-3 entry 6,
`bgResult.ogg` (installed archive, 10 mode-3 records; `o2jam-dump dump-ojm`).
The other rows fill slots `0/1/11/12` (bgEffect sample, BGM records 1..10,
Event, Planet); no other row covers slot `0x13`. `SceneBgm` indexes the
staged array by cue id − 1, so the loader stores slot `N` at array index
`N − 1`; slot 0 has no cue. Guard
`result_cue_plays_bg_result_from_the_winter_worker_bank` skips cleanly
without game data and was red-proven: skipping the winter staging call
made it FAIL at the staged-slot assertion; restoring the call made it PASS.

Contradiction kept visible: the older bank-table bullet called row `+0`
the base for all banks and wrote the last two rows as `3, 3`; those are
32-bit `0x33` values in the mode-2 field. The exe's mode-3 field is `+4`;
every scene cue, including Result, reads the mode-3 worker array. The
stale shell guard `finished_song_enters_result_and_cues_silence_until_its_bank_is_found`
still passes because it stages only a synthetic ten-slot bank, not the
seven-bank worker layout; it no longer claims an OPEN exe bank.

## Contradictions found (exe arbitrates)

1. **Planet BGM trigger** — our `docs/planet.md` item 7 says "BGM
   start trigger … CLOSED as no-trigger (static scan): no per-state
   open … track selection unidentified." decomp2jam's reconstructed
   `CStateAvi::v4` / `CStatePlanet::v4` plus this audit's fresh pin
   prove the opposite: **every state's v4 entry cues the selector**
   `0x551820(id, loop)`. The static scan missed the event-forwarding
   selector. The exe arbitrates: the trigger exists; `planet.md` item
   7 is **superseded** (contradiction documented here, not silently
   resolved).
2. **Life bar implemented?** — `docs/progress.md` (Nov 6) says "NOT
   implemented"; `docs/gameplay.md` (Nov 8) + `parity.md` (Nov 9) say
   implemented as `LifeBar`/`LifeDeltas`, and the code exists
   (`crates/o2jam-game/src/lib.rs`). Code arbitrates: **implemented,
   not shell-wired**. `progress.md` row is stale.
3. **Long-note bonus OPEN?** — same pattern: `progress.md` says OPEN;
   `gameplay.md` §Long-note hold bonus says VERIFIED (+10/tick,
   +1 combo) with `HoldState` in code. Code arbitrates: **verified
   and implemented**; `progress.md` row is stale.
4. **`+0xF6C` semantics** — decomp2jam: manager current-state slot;
   recomp2jam: mounted dialog host is `CStatePlanet`. Reconciled by
   the live read: the current state **is** the planet object while
   the dialog is open. Agreement with refinement, recorded.
5. **Scorer naming** — decomp2jam calls `0x570510` `set_judge`; our
   evidence shows the rated add paths (Cool 200 / Good 100 / Bad 4 /
   Miss −10). Same VA, same family; naming variance only.
6. **Our own audit shorthand** — the original Fresh-pins table listed
   MusicShop `0x10` (hex shorthand) and Result loop; the exe pushes
   `0xA` at file `0xC6E51` and `0` at `0xC1CA5` before the `0x13` push
   at `0xC1CA7` and the call at `0xC1CAF`. The exe arbitrates: the table
   above and the code table (`Screen::bgm_cue`) use decimal 10 and once.
   Tracked here, not silently resolved (#307).

## Delta list — ranked by owner-visible value

1. **Scene BGM playback.** Every lobby screen is silent today. The
   selector + full id table + once/loop policy are now pinned and
   double-sourced. Highest visible value, smallest evidence risk.
2. **Jam gauge.** On screen in every song; our math is OPEN. Their
   matched `jam_threshold_table` (`0x570260`) + `+0x396e/+0x3970`
   candidate chain give the exact next evidence step (verify in exe,
   then implement; unconfirmed behavior stays out of code).
3. **Wire life/jam/long-notes into the shell playing scene.**
   `LifeBar`/`HoldState`/windows live in `o2jam-game` but the shell
   uses wall clock and no gauge drain; dead-at-0 gate invisible.
4. **Playing/result digit anchors + WinLose art + grade mapping.**
   bnd-pinned rects exist (result 7 rects, playing EXIT…VOLUME run);
   grade strings provably code-unreferenced (mission-side) — pin the
   mapping before painting.
5. **Waiting/room UI completion**: team buttons + covers
   (`Wait_TeamA..H.ojs`, 4004/4008 send path — their `0x48C810`
   matched corroborates), create/password dialogs, 4007 slot refresh
   wiring, result SubmitScore/FinalizeRank.
6. **Rating-event sink per-code semantics** (`+0xa14`, attach
   `0x56E170`, codes 1/9/14) — unlocks jam/life server sync and BGM
   channel truths.
7. **Loading gate validation sequence** (`0x5718B0` → `0x4F8B80` →
   `0x412180` → `0x54DB40`; their `ojn_header_validate` analyzed) —
   visible as load behavior, needed for honest loading screen.
8. **Exe key bindings + DirectInput latency budget** (option.ini
   table verified; their `CKeyConfig` matched corroborates).
9. **Side-state data joins** (shop buy/gift/cart, musicshop, myroom
   equip, lanking) — hard on both sides (their districts deferred);
   no free lunch.
10. **File-format tail**: OJNList `;HD` trailer, `.oja` frames,
    outlier codecs; then wgpu blits + screenshot-diff harness.

## Suggested next slices (evidence-first, behavior-first)

1. **BGM slice**: verify the selector body + id table in the exe
   ourselves (one session, winedbg stopped-state reads at
   `0x551820`/`0x54BC50` + the v4 call sites `0x4F5730`/`0x48DD10`);
   then implement `SceneBgm` in `o2jam-audio` + per-scene cues in
   `o2jam-shell`; test with real `BGM.ojm` (skip-clean without
   install-root).
2. **Jam-gauge slice**: exe-verify `0x570260` table + the
   `+0x396e/+0x3970` writers; only then code the gauge; fold into
   playing scene with the life bar.
3. **Shell-wiring slice**: sample-clock the shell loop, wire
   `LifeBar`/`HoldState`/jam into `LivePlay`, dead-gate → waiting.
4. **Waiting/room slice**: 4004/4008/4007 handlers + team buttons +
   create/password dialogs, live-server replay against the quiet
   server fixture.
5. **Result slice**: bnd anchors for digits, WinLose art, grade
   mapping evidence, SubmitScore/FinalizeRank flow.

Each slice ends green: `cargo fmt --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace` (both with
install-root present and hidden), committed and pushed.
