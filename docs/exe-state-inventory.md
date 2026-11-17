# Exe state and dialog census (7.55)

This note records the complete census of state-like screens and dialog
blocks in the shipped 7.55 client. It is the exe-side answer to "which
states still need a spec?". Read it with `GLOSSARY.md`: a **State** is the
mounted client object behind a scene, a **Dialog** is a sheet over a
state, and a **Block** is a named ControlList run.

## Version pins

- `OTwo.exe` == `AngelO2Jam.exe`, 5,029,888 B, sha256
  `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` (7.55).
- `Interface1.opi`, 75,100,702 B, sha256
  `d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`.
  `ControlList_Interface.txt` sits at archive data offset 74,438,614,
  90,472 B, CP949, 1,979 lines.
- `Playing1.opi`, 221,737,026 B, holds `ControlList_Playing.txt`.
- File offset = VA minus `0x400000`. Rect ordinals pair by
  `(entry.code & 0xFFF) - 1` (VA `0x548640`, file `0x148640`, dossier
  `ev_3a869bb7…`, `docs/bnd-semantics.md`).

Method: read-only parse of the two ControlList texts, a read-only string
sweep of the pinned exe, the `.bnd` entry-name census, and the spec
bodies on the tracker. No disassembly or copyrighted bytes are
reproduced here.

## State headers

`NUMBER_OF_STATE 0x0F` declares 15, but the shipped Interface
ControlList has 16 `STATE_` headers; four are `0x00` stubs. The Playing
ControlList declares one more state. `STATE_CHANNEL` has no `.bnd`
bind line; every other full block does.

| Header | Kind | Bind / class pin | Spec |
|---|---|---|---|
| `STATE_AVI 0x01` | state | `State_Avi.bnd` 1 rect; `CStateAvi` RTTI file `0x27e370`; BGM cue id 8 once (`0x4f5730`) | #287 |
| `STATE_PLANET 0x11` | state | `State_Planet.bnd` 17; `CStatePlanet` `0x27dfc0`; cue id 9 loop (`0x48dd10`) | #288 |
| `STATE_CHANNEL 0x27` | dialog over Planet | 50 entries, no `.bnd`; `Dialog_ChannelInfo.bnd` 27; host `CStatePlanet` at `parent+0xF6C`, phase flag `parent+0xF64` | #289 |
| `STATE_ROOM 0x36` | state | `State_Room.bnd` 75; `CStateRoom` `0x27dfe4` | #290 |
| `STATE_WAITING 0x63` | state | `State_Waiting.bnd` 116; `CStateWaiting` `0x27e06c`; `CLiveWaiting` `0x27e1dc` | #291 |
| Loading gate | overlay, no header | `0x4e47a0` (file `0xe47a0`, ev `68d920c2…`): validate `0x5718b0`, stage `0x4f8b80`, decrypt `0x412180`, codec `0x54db40`; `MusicLoading.ojs` | #292 |
| `STATE_PLAYING 0x00` | state stub | real UI in Playing1: 235 entries, 17 rects; `CStateNormalPlaying` `0x27e2a4`, `CStateLivePlaying` `0x27e1dc`; command switch `0x498bc0` | #293 |
| `STATE_RESULT 0x07` | state | `State_Result.bnd` 7; `CStateResult` `0x27e0b4`; builder `0x4c2390` | #294 |
| `STATE_SHOP 0xA4` | side state | `State_ItemShop.bnd` 186; `CStateShop` `0x27e004`; cue id 4 | #178 |
| `STATE_MUSICSHOP 0x6B` | side state | `State_MShop.bnd` 138; `CStateMShop` `0x27e0d4`; cue id `0x10` | #179 |
| `STATE_MYROOM 0x6A` | side state | `State_Myroom.bnd` 111; `CStateMyroom` `0x27e030`; cue id 5 | #180 |
| `STATE_LANKING 0x37` | side state | `State_Lanking.bnd` 73; no `CStateLanking` class in RTTI | #295 (closed) |
| `STATE_PAYMENT 0x09` | side state | `State_Payment.bnd` 9; web billing `payment_input.aspx` | #296 |
| `STATE_PAYMENT2 0x0C` | side state | `State_Payment2.bnd` 12; `gift_input.aspx` | #297 |
| `STATE_TUTORIAL 0x00` | state stub with class | `CStateTutorial` RTTI `0x27e300`, factory `0x4f07f0`; `Playing_TutorialBG.ojs` file `0x24ad1c`; `KeyEffect_Tutorial_%d.ojs` file `0x24ad00`, stage `0x4dd290`; `Tutorial.ojm` file `0x24b524`; `Tutorial.ojn` id 8888 | #313 |
| `STATE_DRUM_PLAYING 0x00` | dead stub | no `CStateDrum*` class; drum art only | excluded |
| `STATE_EXIT 0x00` | teardown | no `CStateExit` class; `ExitThread` path | excluded |

A screen can be real without a class: Channel and Lanking have blocks.
A class can be real without a full block: Tutorial has a `0x00` stub.

## Dialog blocks

`NUMBER_OF_DIALOG 0x1B` declares 27 blocks. The full BOUND table is in
`docs/file-formats/opi.md`. Host-specific sheets belong to their host
state spec. Three cross-cutting families own separate specs per the
owner decision of 2026-11-17:

- Options: `DIALOG_OPTION_KEY 0x19` (25 rects), `SOUND 0x16` (22),
  `GAME 0x17` (23); `CDOption*` RTTI `0x27d8c4`/`0x27d8e0`/`0x27d900`;
  `docs/options.md`; spec #314.
- Messenger: `MAIN 0x3E` (77), `FRIENDADD 0x06` (6), `QUESTION 0x07` (7),
  `NOTE 0x0D` (13), `POPUPMENU 0x06` (6), `INFORMATION 0x05` (5);
  `CDMessenger*` RTTI `0x27db0c`–`0x27dbbc`; `CCMessengerHandler` file
  `0x280690`; host buttons Room `0x13200048`, Shop `0x142000B8`, MyRoom
  `0x1520006D`, Waiting `0x1620006A`, MShop `0x1A200088`; spec #315.
- App-wide message and utility dialogs: `INFORMATION`, `QUESTION1/2`,
  `WHISPER`, `EMOTION`, `CHANGETITLE`, `CHANGENICKNAME`, `MUSICINFO`,
  `SKILLIINFO`, `EVENT`, `CONFERENCE`, `CHARGECASH`, `GIFTVIEW`,
  `SHOPPINGBAG_ADD`; spec #316. Many are bucket E/GAP in
  `docs/bnd-consumption.md`; the mount census is that spec's first leg.

`DAILOG_SELECTMUSIC` (exe typo, `0x7A`, 134 entries, 133 rects) stays in
#291; create/password sheets stay in #290; ChannelInfo stays in #289.

## Dead legacy stems (no spec)

`StateRoom.bnd` (76 rects), `State_Bulletin.bnd`,
`State_MyCapsule.bnd`, `State_Top100.bnd` and seven unbound `Dialog_*`
sheets (`AlbumInfo`, `Exit`, `ItemInfo`, `MyAlbum`, `SelectAlbum`,
`Top100_Buy`, `UserInfo`) have no BOUND line, no state header and no
live consumer. ADR-0012 records them exe-dead. Live TOP100 is a
Music Shop chrome arm (`State_MShop.bnd` `0x1A20007D..84`,
`docs/musicshop.md`) plus an OPEN Lanking button. `MYCAPSULE` room
chrome `0x13200038` is always hidden (`docs/parity/room.md`, #169).

## Contradiction (exe arbitrates)

`docs/parity/playtest-fixes.md:52` says #34 closed as "STATE_ROOM
carries no tutorial control". Shipped `ControlList_Interface.txt` line
148 names `IDC_BUTTON_SROOM_FIRSTSTEP 0X13200032
"Room_FirstStep.ojs"`; its rect is `(660,1)-(720,33)` (`GLOSSARY.md`
FIRSTSTEP). The chrome exists. The link from FIRSTSTEP to
`CStateTutorial` stays OPEN, as does the dimmed Planet TUTORIAL plate
`0x19201006` (`docs/planet.md`). #317 records the correction and the
revised Tutorial entry next step.
