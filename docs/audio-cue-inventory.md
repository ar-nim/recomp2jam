# Audio cue inventory — every state's track and cue set (7.55)

Scope: the complete music and cue set for each shipped state/screen, at the
exe-pinned send moments. This is an inventory, not a new implementation:
it settles "which record, which bank, which moment" for #907. Behavior is
in our own words; no disassembly or decompiled text is reproduced.

## Oracle and provenance

- OTwo.exe == AngelO2Jam.exe, 7.55, 5,029,888 B, SHA-256
  `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
  File offset = VA − `0x400000` in `.text`. All pins below are for this
  build; a different build is OPEN by definition.
- Static pins: REA MCP with Ghidra 12.2, target bound to the hash above
  (`binary_overview` identity `ev_6fb4f8cdb7c38fc0757e03d87ac32d5b96c20fe852c2d38cd7a083ba77aba1b6`).
  The #904 Result trace used REA 6.9.0 CLI / Ghidra 12.2 and is copied
  with its dossier IDs from `docs/coverage-audit.md` §"Fresh pin — Result
  worker bank".
- Git pins: scene selectors and hooks landed in `1ae1667`
  (`feat(audio): add scene BGM selector with exe-verified cue table`) and
  `5f2232b` (`feat(shell): finish scene BGM hooks for side screens and
  result entry`); the Result bank landed in `1bb4525` and was corrected
  in `4f1606e`. Guard names below were grep-verified in this worktree.
- Installed-archive entry names come from local-only OJM data (never
  committed), read with `o2jam-dump dump-ojm`. Names are evidence for a
  record's identity; the exe selects by cache slot, never by name
  (`docs/file-formats/ojm.md`).

## Record notation (read this before the tables)

The exe has three selectors, and they use different record spaces:

1. **Scene music** `0x551820(id, loop)` (file `0x151820`, evidence
   `ev_3ccc21224ad3c5e1bee5694a9a8df6d066912e73dc979128ed3cdf2e6e7ca471`).
   `id` is a worker cache slot. The boot loader `0x550db0`
   (`ev_75ba7fe104a34f019541c1ba1bb792b8b27aef67f0cc8048e7946b44e9986ba1`)
   fills `slot = boot-row base + fetch ordinal` and rejects `slot >= 0x17`.
   Row `+0` (file `0x27f728`) is the mode-2 base; row `+4` (file
   `0x27f72c`, `ev_f73477ba029f80c5d7bfcb0b5c9449dd2f5d9260ce880142338f1bfeda132518`)
   is the mode-3 base. Fresh stopped-state reads of those fields confirm
   row 0 = `0/0`, row 1 = `0x28/1`, row 2 = `0x2f/1`, row 5 = `0x33/0xd`
   (evidence `ev_9f7a9e1b5e184fbabea834966b64cfd9c57de9f6490944624f5fd48e36f894de`,
   `ev_ab972830fbff49c1b884a8efb8eee2e052f6d26b382c2d43067892ebe5f2acaa`,
   `ev_7ddf59c039ff60050ea9ae72fe4f19f2d610552cb035bcf1d0dfa64c3f843226`,
   `ev_4b26ea17821403813bbd585a413df6868e2cf0ad5b098d5cbde95fea3abaff1e`).
   "mode-3 entry I" below means zero-based fetch index I in
   `parse_new_variant_entries().mode3`.
2. **UI one-shots** `0x551870(id, mode)` (file `0x151870`, evidence
   `ev_de048ee24121e6981c9ed6aff2d88235f8af29ca70a5200d80c284c79282c52d`).
   `id` is the UI cache slot; ids `>= 0x34` are rejected. `bgEffect.ojm`
   (boot row 0, mode-2 base 0) puts mode-2 entry I in slot I; one-based
   archive entry is I+1. `O2PlanetNPC.ojm` (row 1, mode-2 base `0x28`)
   puts entry I in slot `0x28+I`. All 66 static call sites pass mode 0.
3. **Song/strike sounds** `0x5703f0` take a chart sample id, not a scene
   cache slot (`docs/file-formats/ojm.md`). They belong to Playing and are
   listed there, not folded into the lobby tables.

The fetch ordinal starts at zero. Fresh decompiler check of `0x54eca0`
shows the fetch counter at object `+0x15c` is reset to 0 and copied to the
entry id at `+0x150` before the post-fetch increment (evidence
`ev_2a873a12408328105111740b73af0e85776d1c8a9ee6e6dc388994c30e9f14ac`,
page `ev_a673c456f60f2b508d6cf5f0e950fd08ee617fc5b96699b4e597fcd46bda747f`).
Therefore slot 1 is mode-2 entry index 1 (`02`), not entry index 0
(`01`); this corrects a name column in the scratch survey (see
Contradictions).

## State inventory

"Entry music" is the cue at the state's v4 entry (or its named helper).
"Cue set" names effect ids in the interaction table below; `—` means a
static negative, not an inference. `OPEN` always carries a next step in
the OPEN ledger.

| State (exe header / host) | Entry music: id, loop, bank + mode + entry | Cue set at exe send moments | Status / evidence |
|---|---|---|---|
| `STATE_AVI 0x01` / `CStateAvi` | `(8, once)` → `BGM.ojm` mode-3 entry 7, `bgLogo.ogg`; call site file `0xF57D4` | Generic modal accept id `1` is hosted by the splash/modal handler family, but the exact AVI host call is not pinned | Music **PINNED** (`docs/coverage-audit.md` §Fresh pins; guard `avi_launch_cue_selects_the_logo_record_once`); OPEN-1 |
| `STATE_PLANET 0x11` + `STATE_CHANNEL 0x27` dialog over Planet / `CStatePlanet` | `(9, loop)` → `BGM.ojm` mode-3 entry 8, `bgMainRoom.ogg`; file `0x8DE28`, `CStatePlanet::v4 0x48DD10` | `1` emblem/family pick (`0x8E316`, `0x47476B`), `5` exit/confirm (`0x8E087`, `0x47488A`), `9` row pick (`0x47530D`), `0x23` channel BACK/EXIT (`0x8FF45`), `1` quick-join (`0x8FF8B`), `0xa` scene shortcut (`0x900FF`…`0x902F5`), `6` chat paging (`0x901F0`, `0x9024E`), `2` dialog decline; hover and 1005 refusal silent | **PINNED** (`docs/overnight-evidence/e1-channel-dialog-trigger.md`; #718 survey §2/§3/§4); dialog-open edge OPEN-10 |
| `STATE_ROOM 0x36` / `CStateRoom` | `(3, loop)` → `BGM.ojm` mode-3 entry 2, `bgRoom.ogg`; file `0x8EB87` | `9` row join (`0x47530D`); `0x23` + `0x29` exit+voice (`0x998F7`, `0x99906`); `0x29`/`0x1b`/`0x1c`/`0x18` enter/retry voices (`0x99BB7`…`0x9B090`); `1`/`2` generic dialogs | **PINNED** for listed ids (#718 survey §2, dossiers `ev_357d6b03…`, `ev_a02e8e06…`); create/password dialog cue set OPEN-2 |
| `STATE_WAITING 0x63` / `CNormalWaiting`, `CLiveWaiting` | `(6, loop)` → `BGM.ojm` mode-3 entry 5, `bgWaiting.ogg`; file `0xB62C3` | `0x23` + `0x18` exit/list pick (`0xA4057`, `0xA415E`); `0x20` confirm (`0xB235E`); START gate at `0x4B22D0` is pinned for wire, not for an effect call | **PINNED** for listed ids (#718 survey §2; `docs/music-availability.md` §4a); exact START-branch cue OPEN-2 |
| `STATE_SHOP 0xA4` / `CStateShop` | `(4, loop)` → `BGM.ojm` mode-3 entry 3, `bgItemShop.ogg`; file `0x987BE` | `0xa` tab/confirm (`0x98C8F`, `0x98F93`); `0x2c..0x2e` greeting voice (`0x99360`, dynamic index); `0x1c` equip/gender (`0x9941E`); `0x14`/`0x13` buy/sell (`0x9C653`, `0x9C853`); `0x28` finish (`0xA0357`); `0x22` confirm (`0xAC456`); `0x21` tab (`0xB24A1`); `6`/`0x23` dialog scroll (`0xB8344`…`0xB840C`) | **PINNED** statically (#718 survey §2, dossier `ev_5bee92ef…`); buy/sell and NPC-voice wiring OPEN (coverage-audit §Cue-map coverage) |
| `STATE_MUSICSHOP 0x6B` / `CStateMShop` | `(10, loop)` → `BGM.ojm` mode-3 entry 9, `bgMusicShop.ogg`; file `0xC6E51`. Decimal 10, not hex `0x10` | `0x23` side-scene confirm (`0xC827B`); shared shop-confirm/tab families (`0x4B2540`, `0x4B6620`) carry `0x22`/`0x21`; preview is OPEN | Music **PINNED** (exe push `0xA`); shared-handler state attribution and preview OPEN-2/OPEN-9 |
| `STATE_MYROOM 0x6A` / `CStateMyroom` | `(5, loop)` → `BGM.ojm` mode-3 entry 4, `bgMyroom.ogg`; file `0xA3911` | `0xd` WEBAVATAR (`0xA3ED4`, `docs/myroom.md` §WEBAVATAR); `0x1f` dress/equip (`0xA4AB9`); `0x23`/`0x18` exit/list (`0xA4057`, `0xA415E`) | **PINNED** (#718 survey §2, dossier `ev_94d5c641…`; `docs/myroom.md`) |
| `STATE_PAYMENT 0x09` / `STATE_PAYMENT2 0x0C` / `CDPayment` | `(3, loop)` → `BGM.ojm` mode-3 entry 2, `bgRoom.ogg` (same record as Room); `docs/coverage-audit.md` §Fresh pins | No `0x551870` call pinned in the web-billing branch set | Music **PINNED**; cue set OPEN-2 |
| `STATE_TUTORIAL 0x00` stub / `CStateTutorial` | No `BGM.ojm` cue. Exe state helper loads `Tutorial.ojm` at file `0xF0A50` and calls its playback helper at `0xF0AB0` (`ev_c9875194…`). Our shell cues `Tutorial.ojm` mode-3 entry 0 (`bgsplit-01.ogg`) at entry and advances record N at page deadline (`0x4F19E0` → `FUN_0042F870`, table `0x67E2C0`, file `0x27E2C0`) | No UI-effect call pinned in the tutorial handler set | Hook **PINNED** (`firststep_tutorial_entry_cues_the_pinned_tutorial_ojm_record`, `tutorial_ojm_advances_at_the_pinned_page_deadline`); which exe record sounds OPEN-3; cue set OPEN-4 |
| Loading gate (overlay, no state header) | No `0x551820` caller pinned; shell uses `-1` silence as a stand-in, not an exe claim | No `0x551870` call pinned in the gate chain (`0x4E47A0` → `0x5718B0` → `0x4F8B80` → `0x412180`) | **OPEN-5** (`docs/lifecycle.md` §9) |
| `STATE_PLAYING` / `CStateNormalPlaying`, `CStateLivePlaying` | No scene selector cue pinned; shell uses `-1`. Song audio is chart-timed through `0x5703f0`: M30 `o2ma*.ojm` music ids `980–991` (`[WAV980]mr1`…`[WAV991]mr12`), or mode-3 splits on new-variant charts | Note/strike ids come from the chart (`decode_note_value`: value − 1, bit26 → +`0x3E8`), not from the UI emitter | **OPEN-6** for per-song bank mapping/in-song gain; negative scene cue from coverage-audit §Cue-map coverage |
| `STATE_RESULT 0x07` / `CStateResult` | `(0x13, once)` → **worker slot `0x13`**, not a `BGM.ojm` record. `0x4C1B90` calls `0x551820(0x13, 0)` at file `0xC1CAF` (pushes `0xC1CA5`/`0xC1CA7`). Slot `0x13 = 0x0d + 6` → `BGMwinter.ojm` mode-3 entry 6, `bgResult.ogg`. Retry re-cues the selector | `0x23` BACK/RETRY (`0xC2D34`, `0xC2E98`); generic modal `1` (`0xCAA39`/`0xF5B43`); `0xd` "Result/other confirm" lacks a Result-specific host pin | Music **PINNED** (#904, `1bb4525`/`4f1606e`, guard `result_cue_plays_bg_result_from_the_winter_worker_bank`); OPEN-7 for the `0xd` row |
| `STATE_LANKING 0x37` (display-only, no state class) | No `0x551820` caller pinned; shell uses `-1` | No `0x551870` call pinned in the lanking block | **OPEN-8** (`docs/exe-state-inventory.md` state headers) |
| Option/song preview (selector caller, not a state) | Exe pushes `(0, loop)`; id 0 has no bank record and silence is the only defined outcome here | — | **OPEN-9** (`docs/coverage-audit.md` §Fresh pins; `crates/o2jam-shell/src/screen.rs` `bgm_cue` docs) |

## Scene record table (music only)

One-based cue id I over boot row 2 (`BGM.ojm`, mode-3 base 1) names mode-3
entry I−1. Installed names are local-only data facts:

| Cue id | Loop | Bank + mode + entry | Installed name | Call site / evidence |
|---:|---|---|---|---|
| 3 | loop | `BGM.ojm` mode-3 entry 2 | `bgRoom.ogg` | file `0x8EB87`; guard `shipped_bank_records_select_the_shipped_tracks` |
| 4 | loop | `BGM.ojm` mode-3 entry 3 | `bgItemShop.ogg` | file `0x987BE`; `scene_bgm_cues_follow_the_pinned_table` |
| 5 | loop | `BGM.ojm` mode-3 entry 4 | `bgMyroom.ogg` | file `0xA3911`; `side_bgm_cues_follow_the_pinned_table` |
| 6 | loop | `BGM.ojm` mode-3 entry 5 | `bgWaiting.ogg` | file `0xB62C3`; Payment/Payment2 share this id |
| 8 | once | `BGM.ojm` mode-3 entry 7 | `bgLogo.ogg` | file `0xF57D4`; `avi_launch_cue_selects_the_logo_record_once` |
| 9 | loop | `BGM.ojm` mode-3 entry 8 | `bgMainRoom.ogg` | file `0x8DE28`; Planet and Channel share this id |
| 10 | loop | `BGM.ojm` mode-3 entry 9 | `bgMusicShop.ogg` | file `0xC6E51`; `music_shop_cue_loops_record_ten` |
| `0x13` | once | `BGMwinter.ojm` mode-3 entry 6, worker slot `0x0d+6` | `bgResult.ogg` | file `0xC1CAF`; `result_cue_plays_bg_result_from_the_winter_worker_bank` |
| `-1` | — | no record | silence | `BGM_SILENCE_ID`; guards `negative_id_silences_a_playing_track`, `cue_silences_a_playing_track_and_keeps_a_silent_channel` |

Records 1/2/7 of `BGM.ojm` are blank; selecting them silences the channel
(`one_based_record_selects_the_matching_bank_entry`, data guard
`shipped_bank_records_select_the_shipped_tracks`). No cue names them.

## Interaction cue table (one-shots)

Every row is a direct `CALL 0x551870` with mode 0; the id is the cache
slot, and the bank+record column names what the boot loader puts in that
slot. Source: #718 static survey (`/home/arnim/scratch/ui-sfx-static-findings.md`
§1–§5), call inventory `ev_4272574b8958634793552f3ff8f347d8d3801103bd5afc139f505b650efdd1d2`.
The first-cluster shell wiring (#721) covers ids `1/2/5/9/0x23`; the rest
are pinned statically, not claimed as implemented.

| Effect id | Bank + mode + record (zero-based index; one-based entry) | Send moment(s) — VA / file | Evidence |
|---:|---|---|---|
| `1` | `bgEffect.ojm` mode-2 index 1 (entry 2, `02`) | `0x4345B0`, `0x434C00`, `0x45B2A4`, `0x47476B`, `0x48E316`, `0x48FF8B`, `0xCAA39`, `0xF5B43` | `ev_91b2769a…`, `ev_b794e46e…`, `ev_5c45e95b…`, `ev_521c21c8…`, `ev_0d169505…`, `ev_ac6983f7…`, `ev_2aecc6bf…`, `ev_1f01e424…` |
| `2` | `bgEffect.ojm` mode-2 index 2 (entry 3, `03`) | `0x4345FC`, `0x434C4C`, `0x438834`, `0x439FED`, `0x43B552`, `0x43F44B` | `ev_91b2769a…`, `ev_1607e1c8…`, `ev_07018794…`, `ev_ec5b747b…`, `ev_62ffe70b…` |
| `5` | `bgEffect.ojm` mode-2 index 5 (entry 6, `06`) | `0x47488A`, `0x8E087` | `ev_6784b125…`, `ev_ebe68503…` |
| `6` | `bgEffect.ojm` mode-2 index 6 (entry 7, `07`) | `0x4901F0`, `0x9024E`, `0xB8344`, `0xB839B` | `ev_ac6983f7…`, `ev_edb84748…` |
| `9` | `bgEffect.ojm` mode-2 index 9 (entry 10, `10`) | `0x435B90`…`0x4362AC`, `0x47530D` | `ev_0f381eee…`, `ev_a02e8e06…` |
| `0xa` | `bgEffect.ojm` mode-2 index 10 (entry 11, `11`) | `0x4900FF`, `0x49015A`, `0x4901B5`, `0x4902F5`, `0x90FA0`, `0x98C8F`, `0x98F93` | `ev_ac6983f7…`, `ev_64482c95…`, `ev_5bee92ef…` |
| `0xd` | `bgEffect.ojm` mode-2 index 13 (entry 14, `14`) | `0xA3ED4` (MyRoom WEBAVATAR); other handler `0x4A3ED4` has no pinned host | `ev_965342b8…`; `docs/myroom.md` §WEBAVATAR |
| `0xe` | `bgEffect.ojm` mode-2 index 14 (entry 15, `15`) | `0x493AB8` | `ev_b34db66e…` |
| `0x13` | `bgEffect.ojm` mode-2 index 19 (entry 20, `19_1`) | `0xA4C41` | `ev_e4fcbcbf…` |
| `0x14` | `bgEffect.ojm` mode-2 index 20 (entry 21, `19_2`) | `0x9C653`, `0xA4C21` | `ev_526b9960…`, `ev_00e91798…` |
| `0x18` | `bgEffect.ojm` mode-2 index 24 (entry 25, `25`) | `0x9B090`, `0xA415E` | `ev_357d6b03…`, `ev_138a6bae…` |
| `0x1b` | `bgEffect.ojm` mode-2 index 27 (entry 28, `23_남녀`) | `0x99BB7`, `0x9A3E2` | `ev_357d6b03…` |
| `0x1c` | `bgEffect.ojm` mode-2 index 28 (entry 29, `24_다른행성`) | `0x9941E`, `0x9A64F` | `ev_5bee92ef…`, `ev_357d6b03…` |
| `0x1f` | `bgEffect.ojm` mode-2 index 31 (entry 32, `27_dress`) | `0xA4AB9` | `ev_94d5c641…` |
| `0x20` | `bgEffect.ojm` mode-2 index 32 (entry 33, `33`) | `0xAC103`, `0xB235E` | `ev_7a46b073…`, `ev_ac6a40ed…` |
| `0x21` | `bgEffect.ojm` mode-2 index 33 (entry 34, `34`) | `0xB24A1` | `ev_ac6a40ed…` |
| `0x22` | `bgEffect.ojm` mode-2 index 34 (entry 35, `35`) | `0xAC456`, `0xB2673`, `0xB6A1C` | `ev_4cc0d025…`, `ev_219d02ea…`, `ev_8e8941ea…` |
| `0x23` | `bgEffect.ojm` mode-2 index 35 (entry 36, `36`) | `0x8FF45`, `0x998F7`, `0xA4057`, `0xB840C`, `0xC2D34`, `0xC2E98`, `0xC827B` | `ev_ac6983f7…`, `ev_357d6b03…`, `ev_138a6bae…`, `ev_edb84748…`, `ev_4c1e77d4…`, `ev_26ac6479…` |
| `0x28..0x2e` | `O2PlanetNPC.ojm` mode-2 index `0..6` (boot row 1 base `0x28`) | `0x99360` (dynamic `DAT_0067E018 + 0x28`), `0x99906` (`0x29`), `0x9A3E2`…`0x9B090` | `ev_5bee92ef…`, `ev_357d6b03…`; exact voice per press OPEN-11 |

Silent paths (pinned negatives, not missing rows): hover updaters `0x439000`
and `0x48E530`; queue posters `0x478D50`/`0x478C90`; the 1005
channel-refusal answer `0x55A110`; option reset/cancel `0x26200010/11`;
gated emblem and room-row branches listed in survey §5. Blank effect slots
`7, 8, 11, 12, 15–18, 26, 30, 36, 37` carry no sample and cannot sound.

## Shared-record rule and Avi once rule

- `SceneBgm::play_scene_music` pushes the exe pair unconditionally; the
  shell's `SceneBgm::cue` keeps a sounding track when the incoming
  selection is the same record, repeat mode, and gain
  (`crates/o2jam-audio/src/bgm.rs`). Guards:
  `same_record_cue_keeps_the_running_track`,
  `cue_switches_on_a_different_record`,
  `cue_restarts_when_the_backend_stopped_playing`,
  `cue_switches_on_a_repeat_mode_change`.
  Planet and its channel dialog both cue record 9; an emblem click must
  not restart `bgMainRoom.ogg` (ticket #27).
- Avi is `(8, once)`, not loop: guard
  `avi_launch_cue_selects_the_logo_record_once`; the exe's push is at file
  `0xF57D4`. A negative id silences (`negative_id_silences_a_playing_track`);
  one-based records are checked by
  `one_based_record_selects_the_matching_bank_entry`.
- Scene entries re-cue through `ScreenGame::enter_screen` /
  `visit_side` / `apply_tutorial_ojm_cue` (commits `1ae1667`, `5f2232b`);
  `recomp2jam::scene_bgm_cue` and `side_scene_bgm_cue` delegate to the
  shell tables. Guards: `scene_bgm_cues_follow_the_pinned_table`,
  `side_bgm_cues_follow_the_pinned_table`,
  `music_shop_cue_loops_record_ten`.

## Result `0x13` — matches #904

Live caller chain: state dispatch → `FUN_004C1B90` (file `0x1B90`,
`ev_1cbc946c4f582ac36fd98fa5e0bb05ebb4e4d102cfbda1aacf0740c62a9be16f`)
→ `FUN_00551820(0x13, 0)` (file `0x151820`; call `0xC1CAF`, pushes
`0xC1CA5`/`0xC1CA7`,
`ev_fa3f6f8b9b47e9c71918d9d900856410c3070ef80ef813da9626f4d186d15da3`)
→ bank sub-object `soundmgr + 0x340C0` (file `0x151832`)
→ `FUN_0054BC50` (`ev_78e65d82656b266c69966b80db8ed056ae97b06d35ba3b2b3231914113eef4a3`)
→ worker `FUN_0054BD60` (`ev_b34b89d839433a81104e6ad8be18bdb6b10440e872a93697a758c4487dd3bf09`).
Boot loader row 5 is `BGMwinter.ojm` (name file `0x27F898`,
`ev_9bbdb57205ab0ed5a98a941fb5005c7fe526eddb7da8574b7e29a10ab21b1836`),
mode-3 base `0xd` (file `0x27F894`,
`ev_884629791f26d1a5813e18d9488a3d01e6da5413a91122846ab881d470d7f4b1`);
mode-3 fetch ordinals are written at file `0x14EED9`. So slot `0x13`
is `0x0d + 6`: `BGMwinter.ojm` mode-3 entry 6, `bgResult.ogg`. Guards:
`result_slot_names_the_winter_bank_result_record`,
`result_cue_plays_bg_result_from_the_winter_worker_bank` (red-proven in
#904: removing the winter staging call failed the staged-slot assertion).
The stale guard
`finished_song_enters_result_and_cues_silence_until_its_bank_is_found`
stages a synthetic ten-slot bank and no longer claims an OPEN exe bank.

## Dialog-open cue trigger — OPEN, with a next step

Pinned: the Planet v4 cue `(9, loop)` covers both the bare board and the
channel dialog because `CStatePlanet` stays mounted at `parent+0xF6C`
while `parent+0xF64` is the phase flag. The emblem pick emits effect id 1
at `0x48E316`, not a scene re-cue; the dialog's stage virtual `0x474110`
calls only the dialog stager `0x432510` (callee dossiers
`ev_428d85156428fec2ded16691bfa0a2709a5f578c83ce3a55b6720198ca3c6a0d`,
`ev_298665c5d3b7b54c07b1d43a5427c5741acca29ce73e004916c0d640643c61b8`;
neither resolved callee set contains `0x551820` or `0x551870`). Static
evidence shows no separate dialog-open music cue; the emblem → visibility
edge itself is dynamic-only (`docs/overnight-evidence/e1-channel-dialog-trigger.md`).

**OPEN-10**: whether any virtual/indirect path re-cues the selector when
the dialog opens remains unproven (one unresolved call in `0x432510`'s
callee set). Next step: one winedbg stopped-state watch on
`parent+0xF64`/`+0xF6C` during an emblem click, launched on OTwo.exe
directly, with queued `x` reads before and after; pair it with one Wine
audio capture of scene record 9 to see whether the sample restarts. Until
then the shell's keep-running policy is the documented stand-in, not an
exe claim.

## OPEN ledger

| Id | Missing fact | Next evidence step |
|---|---|---|
| OPEN-1 | Which handler hosts the generic accept cue in `STATE_AVI` | Static xref/owner dossier for `0x4F5B30` and one splash-dismiss capture with an audio cue log |
| OPEN-2 | Cue set for Payment/Payment2, create/password dialogs, Waiting START branch, MusicShop preview and shared-handler attribution | Static callee sweep of each web/shop handler; then one live press per branch with a cue log |
| OPEN-3 | Which `Tutorial.ojm` mode-3 record the exe plays at entry and on page N | One owned-sink A/B capture with `Tutorial.ojm` record swaps while the tutorial page advances |
| OPEN-4 | UI-effect calls inside `CStateTutorial` handlers | Complete callee inventory for the tutorial input/update functions |
| OPEN-5 | Loading-gate music and cue set | Stopped-state read of `0x4E47A0` chain for `0x551820`/`0x551870` calls |
| OPEN-6 | Per-song Playing bank entry and in-song scene/gain path | One chart-timed play with the OJM dump and a cue log; compare `0x5703F0` strike ids to bank entries |
| OPEN-7 | Host for effect id `0xd` at `0x4A3ED4` | Owner dossier for that handler; do not attribute it to Result on the scratch label alone |
| OPEN-8 | Lanking music and cue set | Static xrefs from the `0x37` block builder, then one live Lanking frame with an audio cue log |
| OPEN-9 | PCM behind preview id `(0, loop)` | Trace the option/music-shop preview caller and identify the bank it resolves; id 0 currently silences |
| OPEN-10 | Dialog-open re-cue / visibility edge | Stopped-state memory watch plus Wine audio capture, as detailed above |
| OPEN-11 | Exact NPC voice for a dynamic greeting index | Wire/capture one greeting with the value at `DAT_0067E018` and resolve slot `0x28+index` |

## Contradictions kept visible (exe arbitrates)

1. **MusicShop id** — older notes and `docs/exe-state-inventory.md` wrote
   `0x10`; the exe pushes `0xA` at file `0xC6E51`. The exe arbitrates:
   decimal 10, `bgMusicShop.ogg` (`docs/coverage-audit.md` §Contradictions
   item 6). Already documented, not silently changed here.
2. **Result loop flag** — older audit wrote loop; the exe pushes 0 at
   file `0xC1CA5`. The exe arbitrates: once (coverage-audit item 6).
3. **Effect sample-name off-by-one** — the #718 scratch table labels slots
   `1/2/5` with names `01/02/05`. The exe fetch counter starts at zero
   (`0x54eca0`, evidence above), so those slots hold mode-2 entries 2/3/6
   (`02/03/06`). The slot→entry numbers `2/3/6/10/36` in
   `docs/coverage-audit.md` remain correct; only the parenthetical names
   in that sentence and the scratch name column are shifted. No code
   changes: the emitter selects by slot, never by name.
4. **Planet BGM trigger** — `docs/planet.md` item 7 called it a no-trigger;
   the v4 call site proves the entry cue exists. Superseded in
   `docs/coverage-audit.md` §Contradictions item 1.
5. **Dialog host** — `+0xF6C` is both the manager's current-state slot and
   the `CStatePlanet` dialog host; live reads reconcile them (coverage-audit
   item 4). No separate `CStateChannel` exists.

## Coverage summary

13 state/screen rows plus the preview selector caller: 9 state rows have
a fully pinned entry music record; 4 state rows plus preview have an
OPEN music/cue field; 11 numbered OPEN items track every remaining gap
with a next step.
Result's `0x13` entry matches #904 exactly (mode-3 entry 6 of
`BGMwinter.ojm`, worker slot `0x0d+6`), and the dialog-open trigger is
explicitly OPEN-10 with a named evidence step.
