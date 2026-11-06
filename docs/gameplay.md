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
`ev_c168abac…`:
- Each Cool adds 1 to the streak while pills < 5; crossing
  `> 14` (the 15th consecutive Cool) grants a pill (cap 5)
  and resets the counter. Good/Bad reset the streak.
- On Bad the streak resets and pills > 0 spends one (`-1`) to
  keep the combo instead of breaking it (mode byte `6`
  exempts both earning and spending).

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

`X` is NOT the live jam combo: exhaustive search finds ZERO
writes to `+0x396e` anywhere in the executable (only reads plus
bulk-init copies) — a runtime counter requires writes. `X` is
consistent with a per-song constant loaded at song setup
(e.g. a difficulty multiplier — the init chain at `0x58e2d0`
leads toward song setup, source untraced) or with dead scaling
(`X == 0`). `o2jam-game` implements the verified bases
(`Cool +200`, `Good +100`, `Bad +4`, `Miss −10` floored);
variable terms stay pending.

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

## Jam gauge — player-reported, UNVERIFIED in-exe

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
per-level size OPEN). `o2jam-game` implements this model
(`JamGauge`/`PillStock`/`Score::apply_rated`) with synthetic
tests; gauge constants, Bad/Miss progress handling,
bonus-on-completing-note ordering, and the per-jam point size
stay placeholders.
