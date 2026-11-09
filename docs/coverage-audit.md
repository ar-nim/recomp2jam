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
(bgRoom 53.8 s parity); mixer pending. Render (`o2jam-render`):
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
| Room / waiting | `CStateRoom` override inventory mapped (vtable `0x647BC4`); team mapping `FUN_0048C810` matched; S→C 4007 handler `0x55F470` analyzed | **PART** — flows ✅; seats grid fixed to 4×2 (Wine); teams in `o2jam-game` ✅; no create/password UI; 4007 refresh not shell-wired | Their `0x55F470` analysis (u8 idx `<8`, stride-`0x2190` slot refresh) corroborates our handler pin; waiting BGM id 6 + room BGM id 3 pinned, unimplemented |
| Playing / judgement / score / life / pills / long notes | Judge chain matched (`0x540A80/570510/576540/570260`); playing loops analyzed; `key_up_release` `0x576719` analyzed | **PART→IMPL in crate** — all rule layers ✅ (`TimingWindows`, scorer, pills, `LifeBar`, `HoldState`); NOT shell-wired; shell clock ≠ sample clock | Jam gauge math: their `jam_threshold_table` `0x570260` is matched — strongest lead for our OPEN gauge chain (`+0x396e/+0x3970` identity, rating sink `+0xa14` codes 1/9/14). Needs our own exe verification |
| Result | `CStateResult_v10`, builder `FUN_004C2390` matched | **PART** — art stages, rank row 0 paints; digits at fixed anchors (audit 13); grade mapping OPEN; SubmitScore fields known | Result BGM id `0x13` cue + retry BGM re-cue pinned via selector; WinLose art wiring (`Result_WinLose.ojs` `0x24b014`) ours to finish |
| Audio (BGM / mixer / ojm) | `SceneMusic` chain reconstructed (`0x551820→0x54BC50`, one-based `BGM.ojm`); `COjmSampleReader::render` analyzed; volume cfg matched | **PART** — vorbis decode + cpal clock ✅; mixer pending; **scene BGM: pinned today, no code** | Fresh pin (below) gives the full per-scene id table with their corroboration; implement selector + loop/once policy |
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
| MusicShop | `0x10` | loop | |
| Result | `0x13` | loop | retry re-cues |
| Song preview | 0 | — | |
| Silence | −1 | — | |

- **Dialog-open trigger: still OPEN** on both sides (dynamic mount).
  Our writer-site pin: dialog-open `0x4ED650` (file `0xed650`) drives
  the caption setter trio (`channel-fill.md` §dialog writers);
  decomp2jam's `CStatePlanet.cpp` documents only the bar mount into
  `+0x8C` (already pinned on our side). Next evidence step: trace the
  caller chain that mounts the `0x12` Channel block on `parent+0xF6C`
  when the phase flag at `parent+0xF64` flips to 1.

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
