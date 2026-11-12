# .bnd consumption census (#164)

One row per `.bnd` file shipped in the 7.55 archives: does our Rust
parse it, stage it, consume it — or why not.

## Inventory (exact, 2026-11-12)

- `strings install-root/O2Jam/Image/Interface1.opi | grep -aoE "(State|Dialog)_[A-Za-z0-9_]+\.bnd" | sort -u` → **48** files.
- `strings install-root/O2Jam/Image/Playing1.opi | grep -aoE "[A-Za-z0-9_]+\.bnd" | sort -u` → **4** files
  (`Avatar_Info`, `Map_Avatar`, `State_Playing`, `State_Playing_network`).
- Total: **52**. Counts matched (48 + 4); no `STOP` needed.

## How consumption works (read this before the table)

The Rust never hardcodes most `.bnd` file names. `resolve_bound_row`
(`crates/o2jam-shell/src/staging.rs:520`) reads the `BOUND "..."` line
of the staged `ControlList` Block at runtime and `read_bind`
(`staging.rs:791`) parses that entry via `o2jam_file::parse_bnd`
(`staging.rs:795`). So a file is **consumed-live** when its Block has a
live staging row: `STAGE_TABLE` (`staging.rs:164-266`) for states,
`DIALOG_TABLE` (`staging.rs:354-396`) for dialogs, both staged through
`stage_state` (`staging.rs:468`) / `stage_dialog` (`staging.rs:486`) →
`stage_bound_row` → `bind_scene_rects` (`staging.rs:554-560`).
Per-file evidence below cites the row plus the live caller that stages it
(`crates/recomp2jam/src/live_art.rs` is the live boot path; `shots.rs`
`side_stage_data` is the shipping screenshot walk).

## Buckets

- **A consumed-live**: parsed + staged + consumed on the live boot path
  or the shipping shots walk (non-test code). Evidence: `file:line`.
- **B tests-only**: referenced only from `#[test]` / `tests/` dirs /
  fixtures. Evidence: one citation. (Empty — see below.)
- **C unreferenced, known-dead legacy**: no Rust reference; documented as
  an unbound legacy section (no `BOUND` line mounts it) in
  `docs/bnd-semantics.md:60-75`. Not staged, not parsed.
- **D exe-dead per ADR-0012**: no mounted state in 7.55
  (`docs/adr/0012-assets-first-consumption.md`); skipped by verdict,
  with absence as the evidence. Not staged, not parsed.
- **E GAP**: unreferenced and NOT known-dead — the exe may mount it but
  our Rust does not. Listed verbatim in §Gaps; filing tickets is out of
  scope for this census (no issues filed from here).

Bucket B is empty: every file referenced from test code
(e.g. `State_Room.bnd` in the `resolve.rs:303` fixture,
`Dialog_SelectMusic.bnd` in `staging/tests.rs:1115`) is *also* staged on
a live path, so each lands in A. Nothing is tests-only.

## Interface1.opi (48 files)

| file | bucket | evidence | notes |
|---|---|---|---|
| Dialog_AlbumInfo.bnd | C | `docs/bnd-semantics.md:60-75` (7 unbound `Dialog_*`) | no Rust reference; no `BOUND` mounts it |
| Dialog_ChangeNickName.bnd | E | none — §Gaps | — |
| Dialog_ChangeTitle.bnd | E | none — §Gaps | — |
| Dialog_ChannelInfo.bnd | A | `staging.rs:361` (`Dialog::ChannelInfo` row) + `live_art.rs:126` (`stage_channel_dialog`) | staged live; painter mounts figures at anchors (`paint.rs:200`) |
| Dialog_ChargeCash.bnd | E | none — §Gaps | — |
| Dialog_Conference.bnd | E | none — §Gaps | — |
| Dialog_CreateRoom.bnd | A | `staging.rs:386` (`Dialog::CreateRoom` row) + `live_art.rs:108` (`stage_room_dialogs`) | modal owns Room input live (`game.rs:1635`) |
| Dialog_EmotionHelp.bnd | E | none — §Gaps | — |
| Dialog_Event.bnd | E | none — §Gaps | — |
| Dialog_Exit.bnd | C | `docs/bnd-semantics.md:60-75` | no Rust reference; no `BOUND` mounts it |
| Dialog_GiftView.bnd | E | none — §Gaps | — |
| Dialog_Information.bnd | E | none — §Gaps | only a doc-comment mention of its `.ojs` (`o2jam-render/src/lib.rs:794`); the `.bnd` is never parsed |
| Dialog_ItemInfo.bnd | C | `docs/bnd-semantics.md:60-75` | no Rust reference; no `BOUND` mounts it |
| Dialog_Messenger_FriendADD.bnd | E | none — §Gaps | — |
| Dialog_Messenger_Information.bnd | E | none — §Gaps | — |
| Dialog_Messenger_Main.bnd | E | none — §Gaps | — |
| Dialog_Messenger_Note.bnd | E | none — §Gaps | — |
| Dialog_Messenger_PopUpMenu.bnd | E | none — §Gaps | — |
| Dialog_Messenger_Question.bnd | E | none — §Gaps | — |
| Dialog_MusicInfo.bnd | E | none — §Gaps | — |
| Dialog_MyAlbum.bnd | C | `docs/bnd-semantics.md:60-75` | no Rust reference; no `BOUND` mounts it |
| Dialog_Option_Game.bnd | A | `staging.rs:376` (`Dialog::PlanetOptionGame` row, block `DIALOG_OPTION_GAME` at `side.rs:2550`) + `live_art.rs:87` (`stage_planet_dialogs`) | tab switching live (`side.rs:2610`, `game.rs:3490`) |
| Dialog_Option_Key.bnd | A | `staging.rs:366` (`Dialog::PlanetOption` row) + `live_art.rs:87` | first tab (`side.rs:2717`); install-guarded stage test at `game.rs:13876` |
| Dialog_Option_Sound.bnd | A | `staging.rs:371` (`Dialog::PlanetOptionSound` row) + `live_art.rs:87` | — |
| Dialog_Password.bnd | A | `staging.rs:391` (`Dialog::InputPassword` row) + `live_art.rs:108` | modal owns Room input live (`game.rs:1640`) |
| Dialog_Question1.bnd | A | `staging.rs:381` (`Dialog::PlanetQuit` row) + `live_art.rs:87` (`QUIT_DIALOG_BIND_FILE`, `side.rs:2734`) | quit-confirm staged live |
| Dialog_Question2.bnd | E | none — §Gaps | exe instantiates a Yes/No dialog app-wide (`docs/cxo2-reference.md:38`); our Rust never stages this sheet |
| Dialog_SelectAlbum.bnd | C | `docs/bnd-semantics.md:60-75` | no Rust reference; no `BOUND` mounts it |
| Dialog_SelectMusic.bnd | A | `staging.rs:356` (`Dialog::SelectMusic` row) + `live_art.rs:144` (`stage_select_dialog_art`) | staged over Waiting seats live |
| Dialog_Shopping_Add.bnd | E | none — §Gaps | — |
| Dialog_SkillInfo.bnd | E | none — §Gaps | — |
| Dialog_Top100_Buy.bnd | C | `docs/bnd-semantics.md:60-75` | no Rust reference; no `BOUND` mounts it |
| Dialog_UserInfo.bnd | C | `docs/bnd-semantics.md:60-75` | no Rust reference; no `BOUND` mounts it |
| Dialog_Whisper.bnd | E | none — §Gaps | — |
| State_Avi.bnd | A | `staging.rs:245` (`State::Avi` row, Bound) + `live_art.rs:58` (`stage_interface_scenes`) | single `IDC_BACK_LOGO` entry, verbatim full-bleed (`staging.rs:144`) |
| State_Bulletin.bnd | D | `docs/adr/0012-assets-first-consumption.md` (no mounted state in 7.55) | zero Rust references; skipped by verdict |
| State_ItemShop.bnd | A | `staging.rs:182` (`State::Shop` row) + `shots.rs:266` (`side_stage_data` via `state_of_side`) | shop board paints live (`game.rs:4599`); `SHOP_BIND_FILE` at `side.rs:31` |
| State_Lanking.bnd | A | `staging.rs:210` (`State::Lanking` row) + `shots.rs:266` | lanking board paints live (`game.rs:4586`); `LANKING_BIND_FILE` at `side.rs:47` |
| State_MShop.bnd | A | `staging.rs:189` (`State::MusicShop` row) + `shots.rs:266` | music-shop board paints live (`game.rs:4613`); `MUSIC_SHOP_BIND_FILE` at `side.rs:35` |
| State_MyCapsule.bnd | D | `docs/adr/0012-assets-first-consumption.md` | zero Rust references; skipped by verdict |
| State_Myroom.bnd | A | `staging.rs:196` (`State::MyRoom` row) + `shots.rs:266` | avatar room paints live (`game.rs:4629`); `MYROOM_BIND_FILE` at `side.rs:39` |
| State_Payment.bnd | A | `staging.rs:224` (`State::Payment` row) + `shots.rs:266` (`side_stage_data` merges Payment + Payment2) | billing overlay paints live (`game.rs:4580`); live boot does not mount it — staged through the side walk; `PAYMENT_BIND_FILE` at `side.rs:51` |
| State_Payment2.bnd | A | `staging.rs:231` (`State::Payment2` row) + `shots.rs:266` | same as Payment; `PAYMENT2_BIND_FILE` at `side.rs:55` |
| State_Planet.bnd | A | `staging.rs:203` (`State::Planet` row) + `live_art.rs:58` | planet board is the live hub (`PLANET_BIND_FILE` at `side.rs:43`) |
| State_Result.bnd | A | `staging.rs:217` (`State::Result` row) + `live_art.rs:58` | result banner merges through the row (`RESULT_BIND_FILE` at `paint.rs:74`) |
| State_Room.bnd | A | `staging.rs:168` (`State::Room` row) + `live_art.rs:58` | room lobby is the live entry (`ROOM_BIND_FILE` at `avatar.rs:656`); test fixture at `resolve.rs:303` is supplementary |
| State_Top100.bnd | D | `docs/adr/0012-assets-first-consumption.md` | zero Rust references; skipped by verdict |
| State_Waiting.bnd | A | `staging.rs:175` (`State::Waiting` row) + `live_art.rs:58` | waiting seats stage live (`WAITING_BIND_FILE` at `paint.rs:72`) |

## Playing1.opi (4 files)

| file | bucket | evidence | notes |
|---|---|---|---|
| Avatar_Info.bnd | E | none — §Gaps | exe loads it by name (`docs/bnd-semantics.md`, file `0x24aba7`); our Rust mentions only its `.ojs` bodies in doc comments (`playing.rs:150`, `avatar.rs:1134`) — the `.bnd` is never parsed |
| Map_Avatar.bnd | E | none — §Gaps | exe loads it by name (same evidence); our Rust cites only `Map_Avatar.bnd` rect 0 in a doc comment (`playing.rs:2819`) — never parsed |
| State_Playing.bnd | A | `staging.rs:238` (`State::Playing` row, PlayingSubset) + `live_art.rs:157` (`stage_playing_scenes`) | 17-rect walk stages live (`PLAYING_BIND_FILE` at `paint.rs:70`) |
| State_Playing_network.bnd | E | none — §Gaps | 19-rect exit-path variant (`playing.rs:2850`, `paint.rs:4943` doc comments); our Rust stages only the 17-rect sheet — the network sheet is never parsed |

## Counts

- A consumed-live: **20** (19 Interface + 1 Playing)
- B tests-only: **0**
- C unreferenced, known-dead legacy: **7**
- D exe-dead per ADR-0012: **3**
- E GAP: **22** (19 Interface + 3 Playing)
- Total: **52** ✓

## Gaps (§E, verbatim)

`Dialog_ChangeNickName.bnd`, `Dialog_ChangeTitle.bnd`,
`Dialog_ChargeCash.bnd`, `Dialog_Conference.bnd`,
`Dialog_EmotionHelp.bnd`, `Dialog_Event.bnd`, `Dialog_GiftView.bnd`,
`Dialog_Information.bnd`, `Dialog_Messenger_FriendADD.bnd`,
`Dialog_Messenger_Information.bnd`, `Dialog_Messenger_Main.bnd`,
`Dialog_Messenger_Note.bnd`, `Dialog_Messenger_PopUpMenu.bnd`,
`Dialog_Messenger_Question.bnd`, `Dialog_MusicInfo.bnd`,
`Dialog_Question2.bnd`, `Dialog_Shopping_Add.bnd`,
`Dialog_SkillInfo.bnd`, `Dialog_Whisper.bnd`, `Avatar_Info.bnd`,
`Map_Avatar.bnd`, `State_Playing_network.bnd`.

Next evidence step for each E row: confirm in the exe census whether a
live `ControlList` `BOUND` line (or a literal load, as pinned for the
three Playing files) mounts it; if yes, add a `STAGE_TABLE` /
`DIALOG_TABLE` row and a scenario-named install-guarded test; if no,
move it to C/D with the absence evidence. No issues filed from this
census (per ticket).
