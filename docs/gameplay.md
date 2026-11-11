# Gameplay mechanisms — in-exe verified (7.55)

How the client judges, combos, spends pills, and scores. File
layouts live in `file-formats/ojn.md`; event encoding in
`file-formats/ojn.md` (chart parser section). This doc is
behavior only: every claim names the function, file offset, and
REA evidence ID behind it. Rows feed `docs/claims.md` (the oracle).

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
handling) also exists — its counter routing is pinned in the
"Long-note combo path" section below, which also documents why
`+0x3972` is the long-note combo, not this rule's per-hit counter. Grade strings (`ALL COMBO`, `COOL 95%`,
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

Per-judgement order VERIFIED (re-checked 2026-11-11, dossier
`ev_ed3fbddbb6ac91c280b283895863db8bb043d925d6742217fd883b98241fed46`):
the life adder runs first (calls at files `0x17057c`/`0x1705f6`),
then the score adder reads the *current* `X` (Cool `200 + 10*X`
at files `0x17058f`–`0x1705ab`, Good `100 + 5*X` at
`0x170609`–`0x170623`), then the jam feed (+2/+1, files `0x1705b0`/
`0x170628`), then the strict-greater grant check (files
`0x1706f9`–`0x170727`). The granting note therefore scores at the
OLD level — `o2jam-game::Score` now scores before it feeds.

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
- F2 auto-select — REFUTED (no first-unused-team scan, no F2→4004
  path in 7.55). Team choice is manual button press only
  (`button − 0x16201004` → 4004). Evidence: zero `cmp`/`push`
  against VK_F2 `0x71` as a key (whole-image scan finds two code
  `push $0x71` sites, both unrelated — file `0xb072c`
  (`push 0x71` + vtable `+0x50` in waiting-entry highlight
  `FUN_004b06a0`) and file `0x1a6979` (`LoadAcceleratorsA`
  resource 113)); `GetKeyState`/`GetAsyncKeyState` consumers check
  only `VK_SHIFT`/`VK_CONTROL` (`0x10`/`0x11` at files `0x11984b`/
  `0x1198b6`/`0x119b10`); the DirectInput scancode table
  (file `0x173290`, dossier `ev_fc65241f…`) feeds in-play lanes
  via `FUN_00544b50`, not waiting; all four true `push $0xfa4`
  (4004) sites are the manual paths — files `0xb2465`/`0xb24c0`
  (button dispatch, team = mapper result) and `0xb4de0`/`0xae447`
  (waiting setup, team 0 default under the `0x77cd31` gate) —
  with no 8-wide occupancy loop feeding any of them.

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

## Lifebar — VERIFIED local chain (memory field + deltas + application site; server sync VERIFIED)

What the exe proves vs what stays open. The local life model is
implemented in `o2jam-game` as `LifeBar`/`LifeDeltas` (field, delta
sets, application site and death gate all pinned — see the
"Life bar — VERIFIED" section below).

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

Client memory fields — FOUND (re-verified from bytes 2026-11-07):
- Life field: the u16 word at game `+0x3970`, reached as word `+0x2150`
  on the score sub-object `this+0x1820` (`0x1820 + 0x2150 = 0x3970`).
  The earlier "no writes to `+0x3970`" negative evidence was an
  artifact of scanning whole-object offsets: every write goes through
  the sub-object word `+0x2150` (same pattern as `+0x396e` via
  `+0x214e`). Reads of the absolute offset do exist (`0x17051c`,
  `0x17093e`, `0x170967` in the scorer; the `0x17094d` compare is the
  1000 clamp).
- Write site: adder `0x540a80` (file `0x140a80`) — adds the i32 arg
  into the u16 word (write `0x140a97`), clamps to 1000 when a positive
  delta lands above 1000 (files `0x140ab6`–`0x140abf`), snaps to 0 when
  a negative delta's u16 wrap lands above 1000 (files
  `0x140acd`–`0x140ae0`; the wrap only happens on underflow, so the
  effective range saturates over 0..=1000). Raw setter `0x5407b0`
  (file `0x1407b0`) stores a u16 arg; file `0x4dae7c` pushes 1000 into
  it on the global game object VA `0x77cd00` (bar start).
- Per-judgement consumer: scorer `FUN_00570510` (file `0x170510`).
  Tier arg byte 4/3/2/1 = Cool/Good/Bad/Miss (pinned by the in-branch
  score constants 200+10X / 100+5X / +4 / −10). Each tier block reads
  the difficulty delta field — Cool `+0xe88` (file `0x17056c`), Good
  `+0xe8c` (`0x1705e6`), Bad `+0xe90` (`0x170806`), Miss `+0xe94`
  (`0x170861`) — and calls adder `0x540a80` on `+0x1820` (calls
  `0x17057c`/`0x1705f6`/`0x170816`/`0x170871`). Entry snapshot of the
  life word (`0x17051c`) gates every tier block (tests
  `0x170557`/`0x1705d1`/`0x1707f1`/`0x17083c`): while life reads 0,
  counts/score/jam/combo/life all freeze and only a per-note display
  word clears (`0x17099b`). Tail: life > 1000 → zeroed via `0x5407b0`
  (`0x17094d`–`0x17095f`); any change fires the gauge UI event
  (`0x17097e`–`0x170997`, event 0x66/0x12 via vtable).
- Delta-set selection: `FUN_00570f90` (files `0x170fa2`–`0x171056`,
  ev `feebe490…`), called at game init (difficulty store `this+0x34`
  file `0x1701f2`, call `0x170206`) — arms Easy {+3,+2,−10,−50},
  Normal {+2,+1,−7,−40}, Hard {+1,0,−5,−30} as i32 immediates; details
  in the "Life bar — VERIFIED" section below. Implemented as
  `LifeBar` + `LifeDeltas`.
- The candidate signed table at file `0x35e1a0`–`0x35ec90` stays
  unattributed: the verified switch uses code immediates, not that
  table, and no `.text` reader is known for it. Do NOT implement from
  it.
- Next steps: locate the 4015 handler (peer-life store + probable
  local-life mirror), or the SubmitScore field reader.

## Jam gauge — SUPERSEDED (oracle: Scoring formulas above + `docs/claims.md` B9)

The 30-progress model below is retained as the refuted report. Truth lives in
Scoring formulas (Cool `+2` / Good `+1`, `(i+1)*50` cumulative, `X+=1`, resets
on Bad/Miss) and in code (`JamState`/`JamGauge`, `jam_threshold_for`).

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
`Itemdata_Korea.dat`/`ItemManager` shop leads. The 30-progress math below is the refuted player report (kept so nobody
re-adds it): Cool 2, Good 1, 30 completes one jam, overflow carries. Do not
code from this section; code from Scoring formulas + B9 (`JamState`/`JamGauge`).

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
30-combo threshold. Gauge numbers now live in Scoring formulas + B9. The scan below stays as
negative evidence against the 30-model only.

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

## Life bar — VERIFIED (delta sets + application site; implemented as `LifeBar`)

Max 1000 (u16 life word, game `+0x3970`). Per-judgement deltas by
difficulty, VERIFIED 2026-11-07 against the shipped exe (md5
`34b789ff557a9bf6a1f275010dcaaceb`): the difficulty switch
`FUN_00570f90` (files `0x170fa2`–`0x171056`, ev `feebe490…`) loads one
arm of four i32 immediates into delta fields `+0xe88`/`+0xe8c`/
`+0xe90`/`+0xe94` (Cool/Good/Bad/Miss) at game init (difficulty store
`this+0x34` file `0x1701f2`, setter call `0x170206`):

| Judgement | Easy | Normal | Hard |
|---|---|---|---|
| Cool | +3 | +2 | +1 |
| Good | +2 | +1 | 0 |
| Bad | −10 | −7 | −5 |
| Miss | −50 | −40 | −30 |

Byte sites per arm — Easy files `0x170fbc`/`0x170fc9`/`0x170fd6`/
`0x170fe3`, Normal `0x170ff2`/`0x170fff`/`0x17100c`/`0x171019`, Hard
`0x171028`/`0x171035`/`0x171042`/`0x17104f`; a difficulty outside
0..=2 takes the fallthrough and selects nothing (`0x170fb4`).

The player report had Bad=0 — the code says −10/−7/−5. That also
resolves the old `[-10,-7,-5]` triple mystery: the exact triple IS the
Bad column, stored as switch immediates — the `0x35e1a0` data table
(which merely contains a copy) stays unattributed.

Application site VERIFIED (re-verified from bytes): scorer
`FUN_00570510` (file `0x170510`) reads the tier's delta field
(files `0x17056c`/`0x1705e6`/`0x170806`/`0x170861`) and calls adder
`0x540a80` (files `0x17057c`/`0x1705f6`/`0x170816`/`0x170871`), which
updates the u16 word `+0x2150` on sub-object `+0x1820` = game
`+0x3970` with 1000-clamp on positive overflow (`0x140ab6`–`0x140abf`)
and 0-snap on negative underflow via u16 wrap (`0x140adf`). Tail:
life > 1000 → zeroed (`0x17094d`–`0x17095f`, setter `0x1407b0`); any
change fires the gauge UI event (`0x17097e`–`0x170997`).

Death rule VERIFIED: the scorer snapshots the life word at entry
(`0x17051c`) and skips every tier block while it reads 0
(`0x170557`/`0x1705d1`/`0x1707f1`/`0x17083c`) — counts, score, jam,
combo and life all freeze; only a per-note display word clears
(`0x17099b`). The killing judgement still applies its delta (gate is
pre-note). Bar start VERIFIED: 1000 via setter `0x5407b0` (file
`0x4dae7c`).

Implemented as `o2jam-game::LifeBar` + `LifeDeltas` (saturation over
0..=1000, dead gate, difficulty selection), wired into `Score::apply`
and `Score::apply_rated`. Pill-saved Bad feeds life like a Cool —
the dispatch calls the scorer with tier arg 4 (file `0x16e880`,
dossier `ev_c168abac…`). The dead-state pill path is untraced (the
pill dispatch runs upstream of the scorer); the model freezes pill
spend too — implementation choice, not an exe claim.

## Long-note release rule — player report CORRECTED twice by trace

The player report ("head hit, tail released exactly at its end, missing
or early release is a miss") is close to the final trace: the tail IS
rated against a release event, through the armed per-lane cursor
`+0x864[lane]` — but the rating is symmetric |release − tail time|
with the shared 6/18/25/≥26 windows, not "early release = miss" (a
late release past the Bad bound Misses too). The first trace's
negative ("no release-vs-tail judgement exists") was itself wrong: it
missed the `+0x864` indirection (arming file `0x176e60`, strike files
`0x176646`–`0x176687`, clear file `0x176709`). The full rule lives in
"Long-note hold machine — VERIFIED" below; `claims.md` B12 needs a
second CORRECTED verdict (this doc keeps the evidence).

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

## Long-note hold machine — VERIFIED (tail rates on the release strike)

Traced end to end in `OTwo.exe` md5 `34b789ff557a9bf6a1f275010dcaaceb`
(5,029,888 B, sha256 `83a045418b62…` as opened by REA; Ghidra 12.1.2,
REA 6.7.0; evidence ids below are per-function). Event plumbing: the
input forward `FUN_005714b0` (file `0x1714b0`, inverts its pressed
word) calls the ring producer `FUN_00577ff0` → `FUN_00579b80` (file
`0x179b80`), which edge-detects per lane via the pressed flags at
`+0x408[lane]` (set file `0x179c21`, cleared file `0x179beb`) and
pushes 8-byte events {lane, pressed, time} into the 128-slot ring at
`+0x400` (pop `FUN_00579c90`, file `0x179c90`; ring init file
`0x179b2c`). Remote play events carry the sender's timestamp and enter
through the same forward (network handler calls at files `0xe17c6`/
`0xe1877`/`0xe1b12`, wrapper at file `0x1714d1`/`0x1714ec`).

Per frame, the play advance `FUN_00571060` (file `0x171060`, dossier
`ev_d444398b…`, called at file `0xde7e0`) calls `FUN_00575d70`
(file `0x175d70`, dossier `ev_efdaac19…`): it gates on play state
`+0x828 == 2` (file `0x175d7c`), rebases `timeGetTime()` (IAT
`0x63f4f8`) on the song start `+0x834`, then dispatches on the mode
word at `0x77ee84` (file `0x175f07`) to the keydown judge — mode 0
`FUN_00576540` (via `FUN_00575c20`, call at file `0x175c58`); mode 1
runs auto-play `FUN_00577090`, else `FUN_00576540`, else — when
`+0xa1c` is set (file `0x175cca`) — the second judge `FUN_00575fa0`
(file `0x175fa0`, dossier `ev_3d707204…`, call at file `0x175cde`),
each followed by the expiry scan `FUN_005772f0` (file `0x1772f0`,
dossier `ev_0ece39cb…`); the scroll-line scan `FUN_005763d0` (file
`0x1763d0`, dossier `ev_292d6cf2…`), the song-end scan `FUN_00577630`
(file `0x177630`, dossier `ev_dbec9dc8…`), the pass-event scan
`FUN_00577920` (file `0x177920`, dossier `ev_86593220…`), and the
hold-bonus scan `FUN_00578110` (file `0x178110`, dossier
`ev_1baf7ef1…`).

1. **Head strike — VERIFIED.** Both judges tier |hit − record time|
   against the shared 6/18/25 bounds (compare chains files
   `0x176b7f`–`0x176ba7` and `0x176646`–`0x17667a`) and OR the tier
   bits `0x10`/`0x20`/`0x40` into the record's judged word `+0xe`
   (files `0x176687`/`0x176bd7`). The walk branch strikes too (file
   `0x176719`): it walks the lane's note list and judges the nearest
   unjudged record to now, closer-of before/after (distance pick files
   `0x176acd`–`0x176ae7`), excluding bit-24 tail records from
   candidacy (files `0x176837`/`0x176994`). With the pressed-word
   inversion (file `0x1714b0`) this branch rides the key-down; the
   docs' earlier "key-up" label for it predates the polarity finding
   in item 6.
2. **Head arms the hold — VERIFIED.** A record with
   `payload & 0x3000000 == 0x2000000` (bit 25 only = head; second-judge
   test file `0x176105`, keyup test file `0x176c42`) raises the
   per-lane hold-active flag `+0x9f8[lane] = 1` (second-judge keydown
   file `0x17617b`, keyup file `0x176d78` gated Cool/Good at files
   `0x176c86`–`0x176c90`, auto-play file `0x17726b`), latches the
   render cursor `+0x988` and head mirror `+0x9a4` from the head time,
   latches the hold span `+0x9c0` (head) / `+0x9dc` (paired tail, via
   the list advance at file `0x17612c`), feeds the combo helper
   `FUN_00541620` (files `0x17613d`/`0x176d86`), and — second-judge
   keydown only — re-aims the lane judge cursor `+0x864` at the tail
   record (file `0x1761c4`). Hold heads forward rating event `0xe`
   (bit-25 test at files `0x1766cb`/`0x176105`, sends at files
   `0x1766ef`/`0x1760d6`) into the render dispatcher `FUN_0056e170`
   (file `0x16e170`, dossier `ev_c168abac…`); its Bad/Miss case resets
   combo to 0 (file `0x16ebcf`).
3. **No release gate on the hold flag — VERIFIED (negative).** The
   hold flag is read exactly once (file `0x178194`, the bonus scan) and
   cleared only by the start/restart loops (files `0x18ec0b`/
   `0x18ed25`/`0x18ee3b`). No site reads the flag to rate a release;
   releasing mid-hold raises nothing — the bonus keeps ticking. The
   tail itself, however, IS release-rated through the `+0x864`
   indirection (item 6); the first trace's blanket negative
   ("no site judges a release against the tail time") is corrected
   there.
4. **Bonus over the span — VERIFIED.** `FUN_00578110` ticks `+10`/`+1`
   per frame while flag ∧ `span_start < now < span_end` (strict:
   files `0x1781c1`/`0x1781d7`) — see the hold-bonus section below.
5. **Expiry — VERIFIED rule, VERIFIED inert on shipped cursors.** The
   expiry scan `FUN_005772f0` (dossier `ev_16e274b8…`, raw
   `ev_02842ce1…`) would Miss any unjudged record once
   `now > record_time + 25` (grace file `0x1773bf`, Miss `0x80` file
   `0x17740f`, rating call file `0x177448`) and, while a head is
   unjudged, also marks its paired bit-24 tail `0x180` (files
   `0x177468`–`0x1774e2`). But the scan cannot reach any record: its
   per-lane walk runs on the cursor pair `+0x480[lane]`/`+0x484[lane]`
   (`{stop, current}` over the lane list), initialized at chart load to
   `{list head, list head}` (files `0x172d32`/`0x172d39`) and never
   advanced by any site — the judges and the scan advance stack copies
   only (walk advance `FUN_00579590` on `LEA ECX,[EBP−0x10]` locals;
   the scan's only save path keeps `current` when it equals the list
   head, file `0x177530`–`0x1775ec`). The walk's first record access
   therefore lands on the list-head node itself, whose payload+8 field
   holds the adjacent lane's link pointer — a value far above any song
   time — so the grace test breaks the walk immediately, every frame
   (files `0x1773bf`–`0x1773c5`). The Miss store and the paired-tail
   marking are unreachable on shipped cursors; a held tail can never be
   expired. A Bad keyup hit on a head still kills the paired tail
   `0x180` (files `0x176dab`–`0x176df6`), and the auto judges rate due
   records themselves (below).
6. **RESOLVED — held-tail consumption (second trace).** The tail is
   player-rated through the per-lane judge cursor `+0x864[lane]`:
   the strike walk (event byte 0) judges the nearest unjudged
   non-tail record; a Cool/Good hit on a hold head arms `+0x864`
   at the paired tail (raw store file `0x176e60`, evidence
   `ev_9faabadd…`) while raising `+0x9f8`; the other input branch
   (event byte 1) judges the record at `+0x864` — the tail — against
   the tail time with the shared windows (compare chain files
   `0x176646`–`0x17667a`, store file `0x176687`, cursor clear file
   `0x176709`), so |release − tail| ≥ 26 ms rates the tail `0x80`
   Miss and in-window releases rate Cool/Good/Bad. The input forward
   inverts the pressed word (file `0x1714b0`), so byte 0 rides the
   key-down and byte 1 the key-up: press strikes the head, release
   strikes the tail. In assist modes the auto judges
   `FUN_00577090`/`FUN_00575fa0` (evidence `ev_d6802fde…`/
   `ev_b83c2c24…`) rate every due unjudged record — tails included,
   no bit-24 exclusion — Cool `0x10` before any scan could see it.
   The dual-chain insert candidate is REFUTED: the two link words are
   the prev/next of ONE circular doubly-linked list (embedded
   sentinel `{first=+0, last=+4, size=+8}`; lane insert `FUN_00578990`
   inserts before `*(sentinel+4)`, evidence `ev_690d8393…`/
   `ev_c17d173f…`; loader `FUN_00571d60` evidence `ev_8659c2d0…`),
   and both directions cover the same records. OPEN refinements: (a)
   RESOLVED statically (fourth trace, no runtime needed): no terminator
   record exists — the stop-marker `*(sentinel+4)` is that lane's
   earliest real note, pinned there by the list discipline, and every
   walk excludes it structurally. The lane-list insert `FUN_00578990`
   (file `0x178990`, dossier `ev_40f34b57…`; node builder
   `FUN_00578ea0` file `0x178ea0`, `ev_8d4fea5e…`) links each record
   immediately before its position argument, and the only callers that
   pass a lane list pass `*(header+4)` as that position: the loader
   note loop reads it at file `0x172866` for its callsite `0x1728c3`
   (dossier `ev_4b4ccad8…`; insert xrefs `ev_44c16d41…`), the measure
   planner at files `0x174d4c`/`0x174fe2`/`0x1752e2`/`0x1755b9`/
   `0x17581a` — and that planner (`FUN_00574330`) is dead code: its
   whole body gates on `param == 3` (files `0x174374`–`0x174378`,
   `ev_1601e127…`) while its only caller, the loader tail at file
   `0x172c3d`, passes the difficulty index 0–2 (xrefs `ev_995dd46c…`);
   its splice helper `FUN_00579970` (file `0x179970`, `ev_63631c18…`)
   is reached only from that dead tail through `FUN_005782e0` (xrefs
   `ev_422d2542…`), and the load-end pairing sweep `FUN_00573b30`
   (`ev_edc4cc84…`) only reads. The loader — walking the OJN event
   stream in ascending measure order — is therefore the only lane-list
   writer: every insert lands immediately before the current
   `*(header+4)`, so the first insert (the lane's earliest note) takes
   the tail slot, stays there forever, and iteration from the header
   runs ascending in time, ending at that pinned earliest note — the
   walks' own end marker is a real note. Both walk consumers stop before reading that node:
   the judge candidate walks and the expiry scan test
   `current == *(header+4)` ahead of their first dereference (judge
   seed files `0x17674e`–`0x176762`, bound `0x1767e5` before deref
   `0x176805`, dossier `ev_14992eb2…`; scan cursors `0x17732f`/
   `0x177336`, seed `0x17736d`, bound `0x177393` before deref
   `0x1773b1`, dossier `ev_0ece39cb…`), and the iterator primitives
   refuse the tail node outright — `FUN_00578db0`/`FUN_00579590` fail
   their `current == *(container+4)` check at files `0x178dd6`–
   `0x178de6` and `0x1795b8`–`0x1795c8` (dossiers `ev_87bd7178…`/
   `ev_7c82f55b…`). So the earliest note of each lane is excluded from
   walk candidacy, and a held tail still cannot be expired; (b)
   RESOLVED — press/release polarity settled statically (third trace,
   no runtime needed). The queue drained by
   `FUN_004e15f0` carries {lane, kind, time}: kind 1 (press) reaches
   the forward as pressed argument 1 (push sites files `0xe17ae`/
   `0xe185f`, kept to the in-play states {5,6}) and kind 2 (release)
   as pressed argument 0 (push site file `0xe1afa`, ungated so
   releases always settle the edge flags) (dossier `ev_c01db4a8…`).
   The forward `FUN_005714b0` inverts the pressed word (file
   `0x1714b0`: pressed ≠ 0 forwards 0, == 0 forwards 1, dossier
   `ev_cea19f7d…`); `FUN_00577ff0` relays verbatim (dossier
   `ev_7e8cc6d7…`) and `FUN_00579b80` copies that value into the
   ring event byte (dossier `ev_248c9713…`) with per-lane pressed
   flags `+0x408[lane]` initialized to 1 (file `0x179b61`) — under
   the inversion that reads 'all keys start released', so the first
   key-down (forwarded 0) is the edge that fires: byte 0 = key-down
   = walk/head strike, byte 1 = key-up = `+0x864` tail judge. The
   flag-1 init also refutes the no-inversion reading: there every
   key-down edge would be swallowed at start. The keyboard side
   feeds the queue with the raw pressed bit: buffered DirectInput
   keyboard device (`GUID_SysKeyboard` data `0x652af0`, device setup
   `FUN_00542ab0` dossier `ev_eb523feb…`), `Option.ini` KEY_1–7
   scancode table (file `0x173290`, dossier `ev_fc65241f…`),
   scancode→lane poster `FUN_00544b50` pushing INPUT_STATE verbatim
   through `FUN_005429b0`; `GetKeyState`/`GetAsyncKeyState` imports
   (`0x63f42c`/`0x63f420`) touch only VK_SHIFT/VK_CONTROL modifiers
   (sites `0x5187b1`/`0x51984b`/`0x5198b6`/`0x519b10`). Verdict: the
   inversion is real and is what makes key-down judge the head and
   key-up judge the armed tail — the modeled wiring needs no change.

**Payload bit rules — VERIFIED (claim B14 CORRECTED).** Which payload
bits mark a record as hold head/tail, re-verified from bytes (same
binary as above; bit tests swept across the slot computer, judges,
scans, pairing sweep and parser — VAs `0x5703f0`–`0x578300` and
`0x572a4f`/`0x573061`):

- **Bit 25 (`0x2000000`) = hold head.** The equality form
  `payload & 0x3000000 == 0x2000000` (bit 25 set with bit 24 clear)
  runs in the second judge (files `0x1760ff`/`0x176105`, dossier
  `ev_3d707204…`), the keyup branch (files `0x176c3b`/`0x176c41`),
  the auto judge (files `0x1771ef`/`0x1771f5`), the expiry-scan head
  test (`0x17746d`) and the dead lane planner (`0x1748ce`); the mode-0
  keydown judge tests bit 25 alone (`and ecx, 0x2000000`, file
  `0x1766cb`) — a divergence that cannot fire, because the candidate
  walk excludes bit-24 records first (files
  `0x176837`/`0x176906`/`0x176994`).
- **Bit 24 (`0x1000000`) = paired tail.** The strike walks drop
  bit-24 records from candidacy (files
  `0x176837`/`0x176906`/`0x176994`); the expiry pairing scan marks the
  first bit-24 record after an unjudged head `0x180` (test file
  `0x1774b4`, store `0x1774e2`, dossier `ev_0ece39cb…`); a Bad keyup
  head hit kills it (`0x176dab`); the span latch advances the lane
  list once from the head to the next record (advance call `0x17612c`,
  latch stores `0x176197`/`0x1761af`); the strike-slot computer forks
  the tail spawn call on the bit (files `0x17043d`/`0x170487`); the
  pass-event scan re-tests it (files `0x177bc0`–`0x177f5b`); the
  load-end pairing sweep re-derives head→tail from the same bits
  (bit-25 tests `0x173c7f`/`0x173e23`, bit-24 test `0x173fd5`, dossier
  `ev_edc4cc84…`). Shipped tails keep bit 25 set too (flag word
  `0x300`, the B16 data walk); no site requires bit 25 on a tail.
- **Bit 26 (`0x4000000`) is judge-invisible — the B14 correction.**
  The sweep finds no bit-26 test in any judge or scan; its only
  payload consumers are the parser span-marker survey (bit-26 test
  `0x172a4f`, id `and edx, 0xFFF` at `0x172a64`, byte-table store
  `0x172a6a`), the post-parse fix-up (`0x173061`, fix-up at file
  `0x172f50`), and the strike-slot sample bank `+0x3E8` (files
  `0x170427`/`0x170431`, claim B17). Shipped playable payloads never
  set the bit. B14's "bit 25/26 = hold span/head semantics" therefore
  resolves to: bit 25 = head, bit 24 = tail, bit 26 = parser
  span/sample-bank marker. Refined by the #83 structural sweep
  (docs/ojn-bit26.md): bit 26 does ship — on 12 Tutorial kind-9 BGM
  records, the B22 autoplay presence marker; no shipped playable-lane
  payload sets it, and the lane-note semantics stay a synthetic-only
  OPEN (next step: rig chart swap reading the `+0x3E8` bank).

Modeled as `o2jam-game::HoldSpan` + `o2jam-game::HoldState`
(`judge_head`, `apply_head`, `judge_tail`, `hold_tick_due`,
`HoldSpan::is_expired`), with the payload-bit rules pinned as
`HoldSpan::marks_hold_head`/`marks_hold_tail` and the named masks
`HOLD_HEAD_TEST_MASK`/`HOLD_HEAD_TEST_BITS`/`HOLD_TAIL_TEST_MASK`
(tested on the shipped flag words `0x200`/`0x300`); the deprecated
`LongTail` release model lacks the verified arming gate and one-shot
cursor semantics and stays deprecated in favor of
`HoldState::judge_tail`. The list-insert ordering discipline itself is
pinned as `o2jam-game::LaneNoteList` + `LaneNoteList::walk`: the only
write is insert-before-end-marker (`FUN_00578990` links each record
immediately before the current `*(header+4)`, loader callsite
`0x1728c3`), so the first-inserted note stays the walks' end marker
forever, and the walk cursor refuses that slot before any read (files
`0x178dd6`–`0x178de6`) — iteration structurally ends at the earliest
note, tested on three-note, single-note and empty lanes.

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
16-way dispatcher at file `0x16e170`, whose Bad/Miss case (combo
reset) sits at file `0x16ebbd`);
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
not proven.

## Long-note combo path — VERIFIED (B46, #80)

Combo counter `+0x3972` (max `+0x3974`, B10) is incremented by
exactly one writer: the combo helper `FUN_00541620` (file
`0x141620`), called with the score sub-object (`this + 0x1820`;
`+0x1820 + 0x2152 = +0x3972`, `+0x2158 = +0x3978` score,
`+0x2154 = +0x3974` max). The helper adds `+1` combo always and
`+10` score only when the mode byte (`+0x1853`) is not `6`
(files `0x14162a`–`0x141659`). An exe-wide caller sweep (objdump
scan, #80) finds exactly six call sites, every one of them a
long-note arm — there is NO plain-note caller:

1. **Head strike (the judge arm).** Second-judge hold arming
   (file `0x176140`), mode-0 keyup hold arming gated Cool/Good
   (call at file `0x176d86`, tier gate files `0x176c86`–`0x176c90`),
   auto-play hold arming (file `0x177230`). The head's combo `+1`
   happens at the strike, outside the rating-event sink.
2. **Tail rating (the sink arm).** The sink dispatcher's code-1
   Cool/Good arms add combo only on the tail payload bit
   (`payload & 0x1000000`, files `0x16e5b0`–`0x16e5c1` and
   `0x16e72d`–`0x16e73e`), each followed by a code-14
   self-redispatch (tier `0x10`/`0x20`, files `0x16e5d4`–`0x16e5d6`
   and `0x16e751`–`0x16e753`). The release judge itself (files
   `0x176646`–`0x176714`) never calls the helper — the paired
   head/tail split is: head feeds at the judge, tail feeds in the
   sink.
3. **Hold tick (the bonus scan).** One call per held frame over the
   open span (call at file `0x178200`, span test files
   `0x1781c1`/`0x1781d7`). **No cap exists**: the scan never
   saturates the counter; it only syncs the max word behind by one
   (`+0x3974 = combo − 1` when greater, files `0x178208`–`0x178248`;
   the judgement store max-tracks the true value at files
   `0x1706af`–`0x1706ef`).

Breaks: the code-14 arm zeroes `+0x3972` on Bad/Miss words only
(file `0x16ebcf`, test files `0x16ebbd`–`0x16ebcf`). Cool/Good
never reset it.

**Contradiction with the earlier "Combo rule — VERIFIED" section
(above) and B10's "regular combo counter" label, documented per
AGENTS.md (exe is oracle):** the same sweep shows plain
(flag `0x00`) note judgements NEVER increment `+0x3972` — the
counter's only writers are the six long-note arms above plus the
three reset sites (init files `0x4de91e`/`0x4de9cc`, code-14
file `0x16ebcf`). So `+0x3972` is the **long-note combo**
(the count #84 paints under the LONGNOTE COMBO caption), not the
regular per-hit combo; the section above pins only the
continue/break *rule* (which holds everywhere: nothing resets on
Cool/Good, code-14 resets on Bad/Miss). Where the regular combo
lives for the big COMBO digits stays OPEN — named next step: the
digit-frame setters for codes `0x1B500088`–`0x1B500092`
(factory district `0x48B2D0`, HUD sprite loader `0x50D060`,
already scoping playing.md). The `o2jam-game` scorer keeps the
per-hit `+1` model for plain notes (`Score::apply_rated`)
pending that trace — flagged, not silently changed.

Implemented in `o2jam-game`: `Score::apply_rated_without_combo`
(the tail's combo-silent scorer path), `RatingSink::run_combo_helper`
(the shared adder behind tail arms and ticks), guards
`hold_head_continues_combo_once_at_the_judge_arm`,
`cool_tail_adds_exactly_one_combo_through_the_sink_helper`,
`good_tail_continues_combo_through_the_good_arm`,
`bad_tail_breaks_combo_only_through_the_bookkeeping_event`,
`hold_tick_bumps_combo_once_per_event_without_a_cap`.
