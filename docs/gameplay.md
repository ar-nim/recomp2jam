# Gameplay mechanisms — in-exe verified (7.55)

How the client judges, combos, spends pills, and scores. File
layouts live in `file-formats/ojn.md`; event encoding in
`file-formats/ojn.md` (chart parser section). This doc is
behavior only: every claim names the function, file offset, and
REA evidence ID behind it.

## Judgement windows — VERIFIED

`FUN_00576540` (file offset `0x176540`, VA `0x576540`;
function dossier `ev_14992eb2…`, decompile `ev_5ecb1ce8…`).
Both judgement sites compare absolute `|song_ms − note_ms|`
(song ms via `FUN_0057a240` tick→ms interpolation, function
`ev_9f8e1c2b…` / decompile `ev_f170d5f6…`) against inclusive
bounds **6 / 18 / 25 ms**:
autoplay/miss site file `0x176646` (`ev_b492dfe0…`),
`0x176655` (`ev_c9145d1a…`), `0x176664` (`ev_da09b5c7…`) and
key-press site file `0x176b7f` (`ev_c31f6413…`),
`0x176b8e` (`ev_7611dac8…`), `0x176b9d` (`ev_3db1071b…`).
Tiers emit flags `0x10`/`0x20`/`0x40`/`0x80`
(Cool/Good/Bad/Miss). Implemented as
`o2jam-game::TimingWindows{6,18,25}` with synthetic tests.

## Combo rule — VERIFIED

Same function: tiers `0x10`/`0x20` continue combo (file
`0x176c86` `ev_63f07dfc…` / `0x176c8c` `ev_b7f03b4d…`),
`0x40`/Miss take the break path (file `0x176d8b`
`ev_55bfa19b…`) — matches user report (Bad and Miss break
combo); the long-note combo path (hold-bit `0x2000000`
handling) also exists. Grade strings (`ALL COMBO`, `COOL 95%`,
`MISS 10`, … at file offsets `0x247363`/`0x2473db`/`0x24765b`)
remain code-unreferenced (zero pointer refs to those VAs —
result-screen `.ojs` driven); `LongnoteMode` refs (`0x3a71c`,
`0x1a4744`, …) remain `option.ini` display settings (dead end).

## Pills — earn and spend VERIFIED

Pill stock lives at game object `+0x398c` (sub-object `+0x1820`
field `+0x216c`); a consecutive-Cool streak counter lives at
`+0x3990` (sub field `+0x2170`). Score path dossier
`ev_c168abac…`, independently re-verified from bytes (dispatch
at file `0x16e43f`, accessors at file `0x140890`/`0x140b80`/`0x140b50`,
producer emission sites below):
- Judgement→flag mapping is exe-grounded, not dossier-only:
  BOTH judgement sites emit the literals `0x10`/`0x20`/`0x40`
  for windows `≤6`/`≤18`/`≤25` ms — autoplay site file
  `0x176646`–`0x176673`, key-press site file `0x176b7f`–`0x176ba3`.
  Tightest window emits `0x10` (= Cool path: builds the streak);
  loosest non-miss tier emits `0x40` (= Bad path: resets +
  spends). Miss variants (`0x80`/`0x180`) touch neither field
  anywhere in file `0x16e400`–`0x16ea00` (region-wide absence).
- Each Cool adds 1 to the streak while pills < 5; crossing
  `> 14` (the 15th consecutive Cool) grants a pill (cap 5)
  and resets the counter. Good/Bad reset the streak.
  (Good path file `0x16e680` zeroes `+0x3990` with no pill logic.)
- On Bad (dispatch `0x40` → path file `0x16e7fd`) the streak
  field `+0x3990` is unconditionally reset to 0 via setter
  `0x540890` (file `0x140890`: `this+0x2170 = arg`, called with
  `this = obj+0x1820`, arg `0`), and pills > 0 spends one
  (`-1` via adder `0x540b50`, file `0x140b50`) to take the Cool
  scoring branch (arg `4` to scorer `0x570510`) instead of the
  pill-less branch (arg `2`); mode byte `6` exempts both
  earning and spending.

So pills come from Cool hits — 15 in a row mints one, up to
the max-5 stock. Miss spends no pill (verified absence of pill
logic on its path) but still breaks combo. Implemented as
`o2jam-game::CoolStreak` + `PillStock` with synthetic tests.

## Scoring formulas — VERIFIED bases, open multiplier

Scorer `FUN_00570510` (file `0x170510`), called with the
judgement class 4/3/2/1 for Cool/Good/Bad/Miss: Cool scores
`200 + 10*X`, Good scores `100 + 5*X` (word at game object
`+0x396e`), Bad-with-pill takes the Cool path verbatim
(BAD→COOL conversion confirmed), Bad-without-pill scores +4,
Miss scores −10 floored at zero (`-10` if the compared field
exceeds `0x0a`). This corrects the flat-100 Good of black-box
accounts (the `*5` term is real) and confirms their 200/4/−10
figures; HP deltas partially corroborated (`[-10,-7,-5]` Bad
row found exactly once in-shipped-bytes at file `0x35e844`).

`X` (word at `+0x396e`) IS written — correction of an earlier
"zero writes" claim that only grepped the `+0x396e` displacement
spelling and missed the sub-object-offset writes (`+0x214e` of the
`+0x1820` sub-object): bulk struct-sync `FUN_004e92a0` (file
`0x4e92a0`, same-offset cross-object copies of `+0x214e`/`+0x2150`/
`+0x2152`/`+0x2154`/`+0x2158`…, called from member-setup paths
allocating 8592 (`0x2190`)-byte slots), word stores (files
`0x14079e`/`0x140a37`/`0x1413c2`, cross-object copies), and
zeroing (files `0x13fb6f`/`0x13fd8e`). So `X` is populated from
other objects, never computed in the scorer — the "read from
some other variable" half of the report is confirmed.
The live incremented neighbor is `+0x3972` — but it is the
regular COMBO counter, not jam (player report corrected a
mislabel here): `+1` per Cool/Good hit via `FUN_00541620`
(file `0x141620`, gated on a flag bit and mode `!= 6`),
reset to `0` on Bad AND Miss (shared tail file `0x16ebbd`,
flag `0x40`/`0x80`), zeroed at init, max-tracked into `+0x3974`
by the scorer (file `0x1706da`–`0x1706ef`). This matches the
combo rule exactly (0x10/0x20 continue, 0x40/Miss break).
The JAM counter is `X` itself (`+0x396e`, sub `+0x214e`),
with a separate PROGRESS counter at `+0x396c` (sub `+0x214c`)
— player report confirmed instruction-by-instruction:
Cool adds `+2` (`push $0x2`, file `0x1705b0`), Good adds `+1`
(`push $0x1`, file `0x170628`), both via adder `0x5409f0`
(file `0x1409f0`); Bad/Miss reset progress to `0` via setter
`0x540770` on the shared low-branch tail (file `0x1708dd`).
The threshold table is VERIFIED identical for every
difficulty: `FUN_00570260` (file `0x170260`) fills
`+0xb66[i] = (i+1) * 50` for `i = 0..399` (`0x190`) at game
init — first jam level needs 50 progress, then 100, 150, …
(cumulative; no reset on level-up). The earlier reported
"30 points" is NOT in the exe — corrected. This table is
per JAM LEVEL (index `X`), not per difficulty: same table
for Easy/Normal/Hard. `X += 1` via adder `0x540a20` (file
`0x140a20`) on the grant, and `X` resets to `0` on Bad/Miss
(file `0x170936`, after snapshotting via `0x5407f0` — likely
maxJam). Jam multiplies the score (`200+10X`, `100+5X`); at
zero jam Cool/Good score flat.

## Room teams — UI + client send VERIFIED, scoring server-side

Multiplayer team play in 7.55 is room teams (battle mode), not
per-song parties. `o2jam-game::RoomTeams` mirrors the room state
(8 seats, per-seat team, per-seat submitted totals, winner pick)
with synthetic tests.

### UI (ControlList, `install-root/Image/*.opi`)

- `STATE_WAITING` (decl `0x63`, 117 entries): eight team buttons
  `IDC_BUTTON_SWAIT_TEAMA..TEAMH` (`0x16201004..0x1620100B`,
  `Wait_TeamA.ojs` … `Wait_TeamH.ojs`), eight avatar covers
  (`IDC_IMAGE_SWAIT_COVERAVATAR1..8`) and eight extend slots — one
  team toggle per seat, eight seats per room.
- `STATE_ROOM` (decl `0x36`, 76 entries): mode filter buttons
  `SHOW_ALLMODE` / `SHOW_NORMALMODE` / `SHOW_BATTLEMODE` and a
  per-row `Room_Mode.ojs` icon — battle rooms are listed and
  joined like normal ones.
- `DIALOG_CREATEROOM` (decl `0x09`): room mode is
  `NORMAL_MODE` vs `LIVEBATTLE_MODE` (plus `3KEY_MODE` / `7KEY_MODE`).
- Win/lose display: `Winner_image.ojs` in `STATE_PLAYING`
  (file `0x24b014`) and `Result_WinLose.ojs` in `STATE_RESULT`
  (file `0x24a35c`). No per-team score panel exists in either
  ControlList — the client renders only the final winner.

### Client behavior (in-exe, 7.55)

- Waiting-room button dispatch `FUN_004b22d0` (file `0xb22d0`,
  dossier `ev_2c8d3964…`): control codes `0x16201004..0x1620100B`
  take the team path; anything else (e.g. `START` `0x16202012`)
  falls through elsewhere.
- Index mapper `FUN_0048c810` (dossier `ev_13212723…`): walks
  the 8-wide button run, highlights exactly the pressed button,
  and returns `button − 0x16201004` (team 0..7).
- Gate helper `FUN_00556680` (dossier `ev_0a91311f…`): scans a
  handler table for an opcode id; when it (or the room-mode bytes
  at `0x77cd30..32`) blocks, the press only refreshes the local
  highlight — otherwise the client sends `SetRoomTeam` (4004) with
  the team index, then `Ready` (4008) with 0 (ready cleared on
  team switch).
- S→C team-change handler `FUN_0055f470` (file `0x15f470`,
  dossier `ev_1ff324b4…`): reads a u8 slot/team index (guarded
  `< 8`) plus an i32 member key, refreshes the 8-slot waiting UI
  through the stride-`0x2190` member table, and forwards
  (`0x12`, 4007, index, 0) into the UI event queue — 4007 is the
  team-change broadcast every client applies.
- Result loader `FUN_004c2390` (file `0xc2390`, dossier
  `ev_d6de3a7f…`) mounts `StateResult_Top/Bottom.ojs` plus
  `Result_WinLose.ojs` — the win/lose art the server verdict drives.
- Scoring stays server-side: per-member `UpdateGameStats` (4014)
  and `SubmitScore` (4016) carry cool/good/bad/miss/maxCombo/jam +
  score; team totals and the winner are computed remotely (no
  team-total arithmetic found in-exe).
- RTTI has no team classes — only `CLiveWaiting` / `CNormalWaiting`
  and `CStateLivePlaying`; ASCII `TEAM`/`MISSION` appear solely as
  CRT false positives. Team identity is numeric (0..7) everywhere.

## Level-up missions — NOT in the 7.55 client

When a player reaches a level threshold, no clear-something-to-level
screen, check, or packet exists in this build — level-up is
server-side (EXP accounting behind `SubmitScore` 4016), the client
only displays values the server sends. Evidence:

- Case-sensitive REA procedure search for
  `Mission|Quest|LevelUp|Team|Battle` over the Ghidra inventory
  returns nothing (dossier `ev_0ce27550…`).
- `STATE_PLANET`'s fifth button (`IDC_BUTTON_SPLANET_DRUM`) is
  commented `퀘스트(미정)` — "quest (undecided)" — and its backend
  was never wired: no `CStateDrum*` class exists and
  `STATE_DRUM_PLAYING` is an empty `0x00` stub.
- Player level/EXP are display text fields (`MYLEVEL`, `EXP`,
  `NEXTUPEXP`, `MYRANK` in `STATE_ROOM`/`STATE_MYROOM`); the
  room-screen fill (`FUN_00491af0`, dossier `ev_240c6f0e…`)
  formats `"Lv.%d:%s"` from a server-fed global with no
  threshold comparison or mission trigger.
- What 7.55 *does* gate by level, client-visibly: rooms carry a
  `Room_LevelLimit.ojs` icon plus `LEVEL_RANGE` text per row
  (join rejected server-side via `JoinWaiting` 3002/3003), and the
  song-select dialog filters by chart-level bands (`초급 1–8` /
  `중급 8–19` / `고급 19–27` / `최고급 27+`, format `"X급 : LEVEL %s"`,
  file `0x24578e…`, strings dossier `ev_68760cac…`); candidate
  select validation `FUN_005718b0` (dossier `ev_66e60d16…`, see
  lifecycle doc) rejects unknown song/difficulty slots.
- No level-up-mission packet exists in the opcode map
  (`net-protocol.md`); character data arrives via
  `GetCharacterInfo` (2000/2001).

So a "clear X to level up" mission mode is either a server-side
rule of a different version or a player-report mix-up with
level-gated rooms and difficulty bands — document, don't implement.

## Lifebar — PARTIAL (memory fields OPEN, server sync VERIFIED)

What the exe proves vs what stays open. No life model is
implemented in `o2jam-game` (field unconfirmed — do not add
until the client-side field + writer are pinned).

Client UI anchors VERIFIED: `Life_gauge_right_bar.ojs` /
`Life_gauge_left_bar.ojs` (file `0x24b1e8`/`0x24b204` → VAs
`0x64b1e8`/`0x64b204`; xrefs from play-scene `.ojs` loader code
file `0xe7ca9`/`0xe7d10`, same loader family as the jam-gauge
bars at `0x4e7de2`/`0x4e7d7b`); grade string `"LIFE 100%"`
(file `0x2474df`). No "stage failed"/death strings exist —
fail is presumably bar-empty + `Image_GameOver_Image.ojs`
(file `0x24af72`), unconfirmed.

Server tracking VERIFIED (this is how peers' bars work),
with the type split nailed down via Mozart.Encore's
Memoryer server (clean-room C# reimplementation for v7.55;
its `GameUpdateStatsType` enum reads `Life = 0`, `Jam = 1`,
which explains the exe's two send sites — corroboration,
not ground truth):
- `UpdateGameStats` (game TCP 4014, `push $0xfae`) is sent
  mid-song from the score path itself: Cool tail (file
  `0x16e660`) and Good tail (file `0x16e7dd`) send with
  arg `1` = Jam (jam streams per-hit); a tick-driven site
  (function file `0x4eaec0`, reached when per-tick helper
  `0x570a50` returns 2, file `0x4ea990`) sends with arg `0`
  = Life (file `0x4eaf3b`), gated on globals byte `!= 6`.
  Bad/Miss tails in this region send no 4014. So life
  streams on a tick, jam streams per hit.
- Memoryer's `ScoreTracker` (hypothesis for our exe except
  where the exe confirms): life range 0–1000, validates
  `0 <= life <= 1000`, ignores updates once `Life == 0`,
  and on submit sets `Clear = Life > 0` — death (0) = fail.
  It broadcasts S→C `GameStatsUpdate` (4015) with
  `memberId + type + value + 8 member ranks` to the room —
  one packet carries both the peer lifebar and live ranks.
  The 4015 handler in our exe (where peer life would land,
  plausibly the 8-seat member array at VA `0x77ee90` +
  `index*8592` seen at file `0xeaf13`–`0xeaf1d`
  (VA `0x4eaf13`–`0x4eaf1d`)) is NOT yet located — OPEN.
  Re-checked 2026-11-06: whole-image scan finds no true `0x0FAF`
  word or immediate — zero `push 0xFAF` vs six `push 0xFAE`
  C→S send sites (file `0xeaf3b`/`0xeb07e`/`0xf05cb`/`0xf070b`/
  `0x16e660`/`0x16e7dd`); the only three `AF 0F` byte pairs
  are a lea-offset constant (file `0x749f`), an indexed-table
  read (file `0x1c1864`), and a multiply seed (file `0x1e8b15`).
  Re-hunted 2026-11-06 (see `docs/4015-hunt.md`): the 122-caller UI
  event-queue family (`0x478d50`, 61 opcodes incl. 4007/4013/4017/
  4018) has NO 4011/4014/4015/4016 forwarder — expected, since
  S→C 4015 is never pushed; strong but UNVERIFIED candidate at
  file `0x15fa20` (u8 slot `< 8` + u16 Life-`0`/Jam-`1` branch into
  member-slot setters file `0x140e0`/`0x140c0`, i.e. game `+0x3970`/
  `+0x396e`, plus an 8×8 refresh loop) lacks an REA id, so no
  model is added.
  The tick send site's member-slot computation (index scaled by
  `0x2190` = 8592 onto base VA `0x77ee90`) is re-verified in
  the tick function at VA `0x4eaf00` (dossier
  `ev_b16e614c…`). No peer-life store found; no model added. Relay TCP op 5 mirrors UpdateGameStats
  C→S in Live mode, with `CO2JamMissPack`/`CO2JamCoolPack`
  combo broadcasts (ops 6/7, ctors file `0x16dfe0`/`0x16e080`).
- `SubmitScore` (4016, `push $0xfb0`) layout corroborated
  field-for-field by Memoryer's `ScoreSubmissionRequest`
  (`u16 cool/good/bad/miss/maxCombo/jam/maxJam`, `u32`
  score, `u8 life`, speed, `i32 longNoteScore`); in-exe
  sites (e.g. file `0x4eb018`) are thin `vtable+0x84`
  wrappers, so the life field read happens downstream —
  field offset OPEN.

Client memory fields OPEN:
- Candidate signed table at file `0x35e1a0`–`0x35ec90`
  (175 rows × 4 × `i32`, VA `0x75e1a0`; values +30…−30,
  contains the player-reported Bad row `(-10,−7,−5,…)` at file
  `0x35e844`) is *unconfirmed*: no absolute reference from
  `.text` (objdump-wide grep), so its reader — if any — goes
  through a pointer. Dimensions unexplained (175 rows). Do
  NOT implement from it.
- Scorer `FUN_00570510` and per-judgement setup `FUN_0056ddf0`
  touch score-domain accumulators only (`+0x3962`/`+0x396c`/
  `+0x3980` words via adders `0x540900`/`0x5409f0`/`0x540a50`;
  judgement dispatch touches only `+0x398c`/`+0x3990`/
  `+0x1853`); no scaled-index table read found in the score
  path. Life write site unlocated.
- Next steps: locate the 4015 handler (peer-life store +
  probable local-life mirror), or the SubmitScore field
  reader; then the delta-table reader, if the candidate is
  real, should fall out via the life field's writers.

## Jam gauge — player-reported, UNVERIFIED in-exe (negative evidence recorded)

UI anchors located: `Jamgauge_right_bar.ojs` /
`Jamgauge_left_bar.ojs` (VAs `0x64b1b8`/`0x64b1d0`, search
`ev_2ff06089…`; xrefs `ev_9a778412…`/`ev_d006b5ee…` → code
`0x4e7de2`/`0x4e7d7b` inside the play-scene `.ojs` loader
`FUN_004e7850`, dossier `ev_40f99221…`),
`Playing_Effect_Jam` (`0x64b0fc`, `ev_a17b38d2…`; xrefs
`ev_b0f57cc5…` → `0x4e822a`/`0x4ee9dc`/`0x4f2d5a`),
`Note_JamNum.ojs` (`0x64c0fc`, `ev_49a71dd5…`; xref
`ev_bda08173…` → `0x50d0ae`), plus `JAM` shop/grade strings
near `0x24745e`/`0x24762b`/`0x2476b3` and the `jamcard.asp` +
`Itemdata_Korea.dat`/`ItemManager` shop leads. The gauge math
below is player-reported (UNVERIFIED in-exe): Cool feeds 2,
Good feeds 1, Bad/Miss feed nothing; 30 progress completes one
jam combo with overflow carrying into the next cycle (31 → 1);
pills (max 5 held) are spent instead of breaking the combo on
Bad/Miss; higher jam combo scores more (direction confirmed,
per-level size OPEN). No gauge model is implemented: unconfirmed
behavior stays out of code; see `JamGauge`-free `apply_rated`.

Negative evidence (objdump static analysis, 7.55
`AngelO2Jam.exe` = `OTwo.exe`, md5 `34b789ff…`): zero `cmp $0x1e`
(30) sites anywhere in the rating/judgement range VA
`0x56xxxx`–`0x58xxxx` — the only five in `.text` sit in unrelated
loader/CRT regions (`0x5249a2`/`0x5249b7`/`0x52b7a0`,
`0x5c03fa`/`0x5c413d`). `FUN_00570510` writes no gauge field
(only the judgement store `+0xb58`, combo-ish words `+0x39b4`/
`+0x3974`, and display virtual calls). The `+0xa14` rating-event
dispatcher (`0x56e170`, jump table `0x56ec5c`) clamps against
`0x1e0` (480) with a `0xf` (15) compare — neither is the claimed
30-combo threshold. Gauge numbers stay OPEN (player-reported only).

## Skill-ring modifiers — VERIFIED storage + lane path, item-text visibility

Playable effect rings from the avatar shop. Storage is the room,
not `option.ini`: `option.ini` near file `0x245234` holds only
`MODE`/`NoteScreen`/`BlendHalf`/`LONGNOTEMODE`/`NoteTail`/`NOTE`/
`NoteType` (verified, no modifier keys), and the executable holds no
`RANDOM`/`PANIC`/`SUDDEN`/`HIDDEN`/`DARK`/`MIRROR`/`SHUFFLE` gameplay
strings (only `Random` room UI at file `0x2457e8` plus unrelated
overlay/COM matches — 6 hits total).

### Ring catalog (shipped item text, CP949, VERIFIED bytes)

`Itemdata_Korea.dat` in `Image/avatar.opa` (740945 B at `0x885008c`)
lists consumable effect rings `11A080xx` in x10/x30/x50/x∞ use counts
plus permanent flavor rings (`70A`/`71A` series, prose only).
Effect lines (translations of the item text):

| Code | Name | Effect (item text) | Bonus (score/exp/jam) |
|---|---|---|---|
| `11A08001` | 미러반지 (mirror) | flips the chart left-right | +500 / 2% / 3% |
| `11A08002` | 랜덤반지 (random) | shuffles the whole chart order | +1000 / 4% / 6% |
| `11A08004` | 히든반지 (hidden) | covers the bottom 20% of the chart | +1000 / 2% / 3% |
| `11A08005` | 서든반지 (sudden) | shows only the bottom 20% | +1500 / 4% / 6% |
| `11A08006` | 다크반지 (dark) | shows only the middle 20% | +2500 / 8% / 12% |
| `11A08007` | 슈퍼랜덤반지 (super-random = panic) | random effect every measure | +2000 / 8% / 12% |
| `11A08009` | 리버스다크반지 (reverse-dark) | covers the middle 20% | +2000 / 6% / 9% |
| `11A08010` | 천사의빛반지 (angel-light) | dark effect moves up/down | +3000 / 10% / 15% |
| `11A08011` | 검은안개반지 (black-fog) | reverse-dark moves up/down | +3500 / 12% / 18% |
| `11A08012` | 절대잼짱반지 | keeps the room in live battle | (no chart effect) |

(`11A08003`/`11A08008` are absent — numbering gaps, not missing data.)

### Storage chain (VERIFIED in-exe)

Room field `+0x20d` → session byte at VA `0x7789e4` (file `0x3789e4`,
in a small option block `0x7789d8`–`0x7789f4` defaulted by
`FUN_00495110`, file `0x95110`, dossier `ev_5b520d70…`, default 2).
The room message `0x24200002` handler `FUN_00436360` (file `0x36360`,
dossier `ev_6838819a…`) serializes `+0x20d`/`+0x20e`/`+0x205` over the
network and mirrors them into the session bytes (67 static references
to `0x7789e4`; value 2 takes a player-count gate in one room path).
Which ring selects which byte value is translated room-side — OPEN.

### Lane application point (VERIFIED in-exe)

- Setup `FUN_00571c30` (file `0x171c30`, dossier `ev_41695a9b…`)
  seeds the CRT generator (`_srand` at VA `0x5f7229`, file bytes
  `0x1f7229`; `_rand` is the single MSVC site at file `0x1f7236`,
  dossier `ev_4948ed69…`, 21 callers) and builds the source→destination
  lane table: full reversal for mirror (no randomness), the N-lane
  swap builder `FUN_00573940` (file `0x173940`, dossier `ev_e65f8622…`:
  identity fill, then one `rand() % width` swap per lane) for byte 2,
  the 3-lane builder `FUN_00573a10` (file `0x173a10`, dossier
  `ev_327cea5d…`: 4 slots skipping slot 1, `rand() % 3` draws with a
  swap-through-slot-2 rule on a drawn 1) for byte 0, identity
  otherwise. Shuffle width (`+0x82c`) starts at 7 (file `0x1717e3`).
- Gameplay planner `FUN_00574330` (file `0x174330`, dossier
  `ev_d880e00e…`, `param_1 == 3` path) rebuilds the table at every
  plain measure and once per hold-note span (hold payload bits
  `0x2000000`/`0x1000000` mark span measures — corroborating the
  `0x172a4f`/`0x173061` hold survey in `file-formats/ojn.md`), then
  spawns notes through the map into the lane objects (`+0x42c`,
  stride `0xC`). Both shuffle bytes rebuild per measure in-exe, so
  the item text's "whole chart" (random) vs "every measure"
  (super-random) distinction is NOT corroborated — the planner
  re-randomises in both cases.
- `o2jam-game` implements this as `LaneMap` (identity/mirror/both
  shuffle builders over an injected word source, chart remap) plus
  `MeasureReshuffle` (rebuild every plain measure, once per hold
  span) with synthetic tests.

### Visibility application point (item-text geometry, in-exe clips OPEN)

The play-scene setup `FUN_0056f900` (file `0x16f900`, dossier
`ev_0f6f6d1b…`) copies static visibility params (table VA `0x680038`)
when the modifier byte is 0 or 2, and the play-scene loader gates
effect-sprite selection on the byte (effect ids `0x1ea` at file
`0xe89a4` and `0x205` at file `0xe89fb` when it equals 2, with
mirror `== 0` gates nearby). The lead address `0x54b9f0` is only a
generic resource-slot setter (dossier `ev_45cf9963…`) — endpoint, not
clip code. The exact clip math is untraced, so `o2jam-render`
implements the item-text geometry data-driven (`LaneCurtain` +
`visible_spans` + `blit_rgb555_clipped`, synthetic tests): hidden
keeps the top 80%, sudden keeps the bottom 20%, dark keeps the middle
20%, reverse-dark keeps the top and bottom 40%.

## Life bar — player-reported values, implemented

Max 1000 (possibly percent). Per-judgement deltas by difficulty,
per player report (2026-11-07, UNVERIFIED in-exe):

| Judgement | Easy | Normal | Hard |
|---|---|---|---|
| Cool | +3 | +2 | +1 |
| Good | +2 | +1 | 0 |
| Bad | 0 | 0 | 0 |
| Miss | −50 | −40 | −30 |

At 0 the player is dead: no more input, song ends, back to the
waiting state. Binary note: `[3,2,1]`/`[2,1,0]` fragments exist
in-shipped-bytes but are unattributed to any code path, and the
unique `[-10,-7,-5]` triple contradicts Bad=no-change, so it
belongs elsewhere until proven. No life model is implemented:
unconfirmed behavior stays out of code.

Negative evidence (objdump static analysis, same binary): no
writes to the candidate life word (game object `+0x3970`)
exist nowhere in `.text` — only reads — so per-judgement life
arithmetic (`+3/+2/+1`, `−50/−40/−30`) has no in-exe application
site. (The sibling claim once made about `+0x396e` was wrong:
that word IS written via sub-offset `+0x214e` copies — see the
scoring section. `+0x3970` has no such writes.) The
`+0x3970` compare in `FUN_00570510` (file `0x57094d`) is a clamp
against `0x3e8` (1000), not a delta. The `[-10,-7,-5]` triple
(file `0x35e844`, VA `0x75e800`, in `.data`) sits inside a larger
signed-i32 delta table (repeated `−30`/`−26`/`−20`/… rows) with zero
absolute xrefs in `.text` — unattributed to any code path. Verdict:
per-difficulty life deltas stay OPEN (counter-evidence: no write
site; Bad=0 unconfirmed either way).

## Long-note release rule — player-reported

No long-note options exist (at most a cosmetic long-note-combo
toggle). Long notes are chart data: the head must be hit, the tail
MUST be released exactly at its end — missing the release is a
miss, and releasing mid-hold is a miss. Implemented nowhere yet:
hold detection needs the in-exe release path (bit-26/0x2f50 sites
never fire on shipped charts).

## Win rule — player-reported

Grades exist; WIN/LOSE art goes to rank #1 by highest score,
solo or team. Grade thresholds OPEN. Negative evidence: zero
pointer refs in `.text` to any grade-string VA (file `0x247363`/
`0x2473db`/`0x24765b`, `JAM` strings `0x24745e`/`0x24762b`/`0x2476b3`)
— result-screen grades are `.ojs`-driven; no rank-comparison
arithmetic located in-exe. `Winner_image`/`Result_WinLose` each occur
once in-shipped-bytes (UI names only).

## Renderer default — VERIFIED 800x600, effectively fullscreen-only

7.55 targets 800x600: `SetWindowPos(hwnd, 0, 0, 0, 800, 600,
SWP_SHOWWINDOW)` at file `0x14b4c`/`0x14baf` (via the USER32 IAT slot
`0x63f3a0` = `SetWindowPos`), with matching 800/600 pushes at
`0x4169xx`/`0x416bxx`/`0x416cxx` (`AdjustWindowRect`/`MoveWindow`
path); `DDRAW.dll!DirectDrawCreateEx` is imported (IAT `0x63f01c`,
thunk `0x5adccc`) and called from the CDX display init
(`0x413080`/`0x413c2f`). Window creation forks at file `0x41c0b` on
a runtime flag bit (`+0x1c` of the global at VA `0x768128`): default
`WS_POPUP|WS_VISIBLE` (`0x90000000`, fullscreen) vs `WS_CAPTION`
(`0xC00000`, windowed) — but no config/registry/option writer for
that flag exists in 7.55, so the windowed branch is unreachable in
practice. Implemented as `o2jam-shell::{DEFAULT_WIDTH,
DEFAULT_HEIGHT}` = 800x600 with a synthetic test.

## Long-note release rule — player-reported, in-exe path identified but not traced

Player holds from the note head; the tail MUST be released — missing
the release is a miss, releasing early is a miss. Verified to exist:
hold-bit handling and per-measure span markers in-exe, but the exact
release-point judgement was not traced (never fires on shipped
charts). Implemented as documented behavior only, not yet in code.
Hold-bit sites pinned by objdump (same binary): `cmp $0x2000000`
at `0x576105`/`0x576c42`, `and $0x2000000` at `0x5766cb`,
`and $0x1000000` at `0x576994` — all inside `FUN_00576540`.

## Long-note hold bonus — VERIFIED (+10 per tick, +1 combo)

While a hold stays active, each playback tick feeds a bonus
accumulator plus combo. Chain, in the player's own words:
the parser fix-up `FUN_00572f50` (file `0x172f50`) marks hold
spans (payload bit 26 test at file `0x173061`; survey at file
`0x172a4f`); a judged hold head raises the per-lane hold-active
flag at game `+0x9f8` (head writes at files `0x17617b`/`0x176c78`/
`0x17726b`, gated on the hold-bit `0x2000000` tests at files
`0x1766cb`/`0x176c42`/`0x1771f5`, with the hold judgement
recursed as rating event `0xe` — sends at files
`0x16e5d4`/`0x16e751`/`0x16e8ce`/`0x16e91d`/`0x1766ef` into the
16-way dispatcher at file `0x16e170`, hold case at file `0x16ebbd`);
then the per-frame hold scan `FUN_00578110` (file `0x178110`,
dossier `ev_737dfe7b…`, Ghidra 12.1.2, same binary md5
`34b789ff…`) checks the flag per lane (file `0x178194`) and, while
held, advances the lane cursor by `+6` (file `0x1781f6`) and runs
the combo helper (call at file `0x178200`), which writes `+10` to
the hold accumulator (game `+0x3978`, sub-object `+0x2158`, write
at file `0x14163c`) and `+1` combo (game `+0x3972`, sub `+0x2152`,
write at file `0x141652`; mode byte `6` exempts only the `+10`).
Implemented as `o2jam-game::LongNoteBonus` (`hold_tick` = one
`+10` tick, gated on hold-active by the caller) with synthetic
tests. The accumulator is the lead candidate for the dedicated
`longNoteScore` slot in `SubmitScore` (4016) / `UpdateGameStats`
(4014) — layout corroborated in `net-protocol.md`, but the
submit-side read runs through a vtable path, so that link stays
corroborated, not proven.
