# Client lifecycle (7.55) — process start to exit

Version pin: `AngelO2Jam.exe` == `OTwo.exe`, 5,029,888 bytes,
SHA-256 `83a04541…72c823b`, PDB `d:\O2JamSvn\trunk\Client\O2Jam_Client(2005)\Release\OTwo.pdb`.
In-exe evidence below uses virtual addresses; file offset = VA − `0x400000`.
REA snapshot: `extracted/rea-snapshot` (single-line JSON dossier bundle,
Ghidra 12.1.2, analysis digest `88a37df4…130c8d`, target SHA matches the exe).
Behavior is described in our own words; no disassembly is reproduced.

## Launcher vs client responsibilities

| Piece | Role | Evidence |
|---|---|---|
| Web login page + `NowSmartLauncher.ocx` (MFC ActiveX, `CNowSmartLauncherCtrl`, 462,848 B) | Auth, version check (`launcher_version` / `patcher_version` / `directx_version` props), patch download | URL templates `http://%s/O2Jam/pub/O2Jam/%s/%s%2.1f.zip`, `%s\%s%2.1f.zip`, `NowcomPatcher.exe`, all in-ocx strings |
| `NowcomPatcher.exe` | Applies the downloaded patch | Referenced by name in the OCX |
| Game-find web page | Server/channel/planet pick; passes `My_channel`, `My_planet`, `CTuser_*` table into the client script bridge (never argv) | `http://game1.o2jam.com/gamefind_new/gamefine_main.asp?USER_INDEX_ID=` (file `0x248398`); `var My_channel = "%d";` / `var My_planet = "%d";` / `CTuser_id[%d] = "%s";` (files `0x24df70`–`0x24e024`) |
| `AngelO2Jam.exe` / `OTwo.exe` (project name `OTwo`) | Everything below: archives, states, gameplay, netcode | `GetCommandLineA` import (stored to global `0x796be8`, call site file `0x1fb2e5`); `CCommandLineParser` RTTI (file `0x27d068`); startup gate reads section `IDENTIFICATION` keys (files `0x1ab47a`–`0x1ab939`) |

There is **no in-client login screen**: no `CStateLogin`/`CStateLogo` class,
no login strings in the exe. Session identity arrives as one RSA-encrypted
blob plus five gateway mirrors on the command line
(`OTwo.exe <blob> (|test|??|<addr>|<port>) x5`, Memoryer `game:start` model,
exe-cross-checked). The blob holds 19 two-digit-length-prefixed fields
(`AuthParameters`: FTP hosts/port/paths, game version, `USER_INDEX_ID`, username
token, password, level, gender, nickname, email, gateway address/port, PC room,
home URL, rank, notice URL, `MEMBERSHIP`). The gate resolves the
`IDENTIFICATION` keys `USER_ID1` / `PASSWORD1` / `USER_ID` / `PASSWORD` /
`USER_INDEX_ID` (files `0x1ab47a`–`0x1ab5d4`), five `IP%d` / `PORT%d` pairs
(file `0x1ab5f5` bounds the loop at 5), `PCROOM_USER`, and `MEMBERSHIP`
against the MONAWA / GAMEANGEL / NATEON / PDBOX / O2JAM tag chain
(files `0x1ab8a9`–`0x1ab939`). Channel and planet arrive only through the
`CTuser_*` / `My_channel` / `My_planet` script bridge, never argv.

## Ordered stage list

### 0. Process start
CRT entry at RVA `0x1fb3b8` (file offset `0x1fb3b8`, VA `0x5fb3b8`;
`MSVCR80` era startup) → `WinMain` (exact address not covered by the
dossier set — unknown). Imports used downstream: `CreateWindowA`,
`GetPrivateProfileIntA`/`GetPrivateProfileStringA`, `KERNEL32 ReadFile`,
`timeGetTime`, `DirectDraw` (3 refs), `CDX*` sprite/sound classes.

### 1. Archive open (`..\Utility\PackingFile.cpp`)
Opens, in this adjacency order at file `0x2490386` (VA `0x6490386`):
`Image\Playing1.opi`, `Image\Interface1.opi`, `Image\Avatar.opa`,
then `.\Interface\State.cpp` + `AlbumList.ojs` + `%s\Image\OJNList.dat`.
`CFileManager` / `CPackingFile` / `CResManager` are the owning classes.

### 2. State manager init (`.\Interface\State.cpp`)
`CState` (singleton: `Singleton<CState>` RTTI) + `CStateBase` own the
screen machine. `CControlList` parses `ControlList_Interface.txt`
(16 `STATE_` headers vs declared `NUMBER_OF_STATE 0x0F`) and
`ControlList_Playing.txt` (`STATE_PLAYING`, decl `0xE6`/230, 235 entries).
State ID codes ride the top byte of `IDC_` codes: `0x12` CHANNEL …
`0x1A` MUSICSHOP, `0x1D/0x1E` PAYMENT/2, `0x52` LANKING; stubs have none.

### 3. Settings load (`option.ini`)
`GetPrivateProfileIntA` (file `0x27c8e4`) + `GetPrivateProfileStringA`
(file `0x27c97c`) over this key table (file `0x245224`–`0x24529c`):

| Section | Key | File |
|---|---|---|
| `MODE` | `Mode` | `option.ini` |
| `NoteScreen` | `BlendHalf` | (same block) |
| `LONGNOTEMODE` | `LongnoteMode` | `Option.ini` |
| `NoteTail` / `NOTE` | `NoteType` | (same block) |

Display/note-render options only. No server, key-bind, or audio-volume
keys live in this table (unknown where those load — open question).

### 4. Net/session init
Game server `221.132.84.4`, ports via `PORT%d` / `IP%d` format strings;
hole-punch relay IPs `203.238.136.184/.182/.183`, plus `.52`;
`CHolePunchingCtrl::InsertHUPResult` logging; `CRelayServerConnect`;
music FTP `121.125.76.135/.136` (`O2JAM_MUSIC_FTP`); song enum
`music\..\music\*.ojn` (file `0x2490505`); web billing/event endpoints
under `game1.o2jam.com` and `o2jam.nopp.co.kr`.

### 5. `STATE_AVI` — logo splash (first visible screen)
One-entry block (`0x01`): `IDC_BACK_LOGO … "O2mediaLogo.ojs"`.
Class `CStateAvi`. No video-file references exist in the exe (no `.avi`
bytes at all), so the "movie" is a still-logo state. Entry: process init
done. Exit: splash timer/dismiss → `STATE_CHANNEL`. The exact dispatch
call is not in the dossier set (unknown).

### 6. `STATE_CHANNEL` — channel select
Block decl `0x27` (39), 50 entries, **no `.bnd` bind file** (bare `BOUND`
line — the only full state without one) and **no `CStateChannel` class**
in RTTI. Visible controls: `Dialog_ChannelSelectBar.ojs`,
`IDC_BUTTON_SCHANNEL_BACK/RANK/CONTESTRANK`. Entry: from AVI.
Exit: pick channel/planet → `STATE_ROOM` (or `STATE_PLANET` for solo).
RESOLVED 2026-11-09 (live memory read, `docs/channel-fill.md`): the
screen's dialog is hosted by a **`CStatePlanet`** object mounted at
`parent+0xF6C` under the `CState` singleton — there is no
`CStateChannel` class because Planet's state object serves the channel
screen. `DIALOG_CHANNEL_INFO` (`CDChannelInfo`) and `CCBtnChannel` serve
popups.
CORRECTION 2026-11-09 (live captures, `docs/planet.md`): the channel
select presents **as a dialog over the planet emblem board** — the
first interactive screen is the Planet board (five emblems + promo
carousel + MENU bar) with the `0x8C` channel dialog mounted on top at
boot; its X button (native (557,195)) closes to the bare board.
Whether the state machine nominally marks `0x12` current while the
dialog is up, or Planet stages the dialog directly, awaits the
current-state field read (in flight).

### 7. `STATE_ROOM` — multiplayer lobby (`CStateRoom`)
Decl `0x36`, 76 entries, `State_Room.bnd` (75 rects). Room list with
mode filters (`Room_Btn_Show_AllMode/NormalMode.ojs`), `Room_ButtonRoom.ojs`.
Entry: channel pick. Exit: join/create room → `STATE_WAITING`;
create via `DIALOG_CREATEROOM` (`CDCreateRoom`), locked rooms via
`DIALOG_INPUTPASSWORD` (`CDInputPassword`).

### 8. `STATE_WAITING` — pre-game room (`CStateWaiting`)
Decl `0x63`, 117 entries, `State_Waiting.bnd`. Two flavors in RTTI:
`CLiveWaiting` + `CStateWaiting` vs `CNormalWaiting`. Host picks music via
`DAILOG_SELECTMUSIC` (`CDSelectMusic`, decl `0x7A`, 134 entries — the
largest dialog; `CCBtnSelectMusic`, `MusicLoading.ojs`, `AlbumList.ojs`
album art). Entry: room join. Exit: host starts → loading → `STATE_PLAYING`;
back button → `STATE_ROOM`.

### 9. Loading gate (song select → play)
Dossier-covered, in order:
1. Select validation `0x5718b0` (file `0x1718b0`, ev `66e60d16…`) — checks
   the candidate against the song table (stride `0x1b4`), id mask, and
   difficulty slot; rejects on mismatch.
2. Staging `0x4f8b80` (file `0xf8b80`, ev `d2a69b45…`) — creates
   `.\Music` + `.\Music\TEMP`, formats `MUSIC\Temp\%s%d_%d` extract names.
3. OJN open/decrypt `0x412180` (file `0x12180`, ev `011eab73…`) via
   guarded wrappers `0x412890` (ev `ffc5053b…`) / `0x412a50`
   (ev `23e09c2f…`); `"new"`-magic payloads get byte-reversal plus the
   period-stride XOR (`0x54fac0` xors `0x696d616e`, ev `5f650bc0…`).
   File helpers: `0x412020` (open, ev `f0b58cef…`), `0x412620` (read,
   ev `04362901…`, 12 call sites), `0x4125b0` (ev `55fd256a…`),
   `0x4069b0` (guarded `ReadFile`, ev `740cefda…`, 20 callers).
4. Codec init `0x54db40` (file `0x14db40`, ev `0ee1373d…`, 35 callees incl.
   Vorbis table setup) + sample decode init `0x54d460` (file `0x14d460`,
   ev `3f154198…`, validates 8/16-bit mono/stereo, tags `DISTRIBUTOR`).
5. Loading screen `0x4e47a0` (file `0xe47a0`, ev `68d920c2…`) — creates
   `Trebuchet MS`/`Arial` fonts, shows `MusicLoading.ojs`, walks the song
   table and arms the song clock, then hands off to playback.

### 10. `STATE_PLAYING` — gameplay
Interface side is the `0x00` stub (real UI lives in Playing1.opi:
`STATE_PLAYING`, 235 entries, `State_Playing.bnd`); classes
`CStateNormalPlaying` / `CStateLivePlaying` / `CPlayingGame`.
Dossier-covered sequence:
1. Song start `0x56ffa0` (file `0x16ffa0`, ev `f3ce92fb…`) — builds
   `MUSIC\%s` from the picked entry, strips the extension, walks the song
   list, then calls small init `0x570f90` (ev `c91297e1…`) and big setup.
2. Playback setup `0x571d60` (file `0x171d60`, ev `c0325c85…`, 891 asm
   lines) — calls the OJN wrapper `0x412890`, validation `0x5718b0`,
   event pump setup, 25-slot init `0x573890` (ev `1d728802…`), and exit
   paths.
3. Per-frame background-effect loop `0x550db0` (file `0x150db0`,
   ev `4164031c…`) — pumps the event queue via `0x54ebe0`/`0x54eca0`
   (recursive queue consumer, ev `1601a757…`, reads records through
   `0x412620`), dispatches codes `< 0x34` through record reader `0x54ef60`
   (file `0x14ef60`, ev `f96b0fe5…`) and `< 0x17` through `0x54f580`
   (file `0x14f580`, ev `fdb31596…`); both fan out to field setters
   `0x54f860` (ev `16bc56aa…`) / `0x54f920` (ev `5077e97e…`) /
   `0x54f9e0` (bit toggle, ev `7c51d53e…`) / `0x54fa70`
   (ev `a236b85a…`), with record-type dispatch `0x54ca60`
   (ev `edda1b93…`) and record alloc `0x54b9f0` (ev `45cf9963…`).
   Entry BGM ref `Music/BGEffect.ojm` (file `0x27f730`).
Exit: song end / quit → `STATE_RESULT` (networked) with the
`Playing_network_exit` sprite + `State_Playing_network.bnd`.

### 11. `STATE_RESULT` — results (`CStateResult`)
Decl `0x07`, 8 entries: `Result_WinLose.ojs`, `Result_NumFont_*.ojs`,
`StateResult_Top/Bottom.ojs`, `Retry` control, plus an event-ticket web
call (`o2jam.nopp.co.kr/…/ticket_add.php?USER_INDEX_ID=`, file `0x24a338`).
Entry: play finish. Exit: retry → `STATE_WAITING` loading; leave →
`STATE_ROOM`/`STATE_CHANNEL`.

### 12. Side states (reachable from channel/room/waiting lobbies)
- `STATE_SHOP` (`CStateShop`, `0xA4`, item shop) and `STATE_MUSICSHOP`
  (`CStateMShop`, `0x6B`, song shop; `jsBuyMusic` bridge) share `Payment`
  back-ends and `CMyBag`/`CMyCart`/`CItemManager`/`CBillManager`.
- `STATE_MYROOM` (`0x6A`, avatar room; `CItemManager`, `CPlayerInfo`).
- `STATE_PLANET` (`CStatePlanet`, `0x11`, solo missions; difficulty buttons
  HARD/NORMAL/EASY/FREE; BGM `Music/Planet.ojm`, NPC `Music/O2PlanetNPC.ojm`).
- `STATE_LANKING` (`0x37`; no `CStateLanking` class — display-only block,
  like CHANNEL).
- `STATE_PAYMENT`/`STATE_PAYMENT2` (`CDPayment` classes, web billing views:
  `…/np/billing/payment/payment_input.aspx`, `…/billing/gift/gift_input.aspx`).
- `STATE_TUTORIAL` (`CStateTutorial`, stub `0x00`; `KeyEffect_Tutorial_%d`
  + `Playing_TutorialBG.ojs` art in Playing1.opi; entry path unknown).
- 27 dialogs (`DIALOG_*` + `DAILOG_SELECTMUSIC` typo) map 1:1 to `CD*`
  classes (`CDInformation` … `CDChannelInfo`); option dialogs `CDOption` /
  `CDOptionGame` / `CDOptionSound` edit the §3 keys.

### 13. `STATE_DRUM_PLAYING` / `STATE_EXIT` — stubs
Both `0x00` with no block. No `CStateDrum*` class exists, so drum play was
never wired (only art remains: `DrumKeyDownEffect.ojt`,
`State_Playing.bnd` refs `DrumKeyDownEffect`). No `CStateExit` class
either: `EXIT` is teardown (window close → `ExitThread`, imports present),
not a screen. Drum entry trigger and exact exit path are unknown.

## Transitions summary

```text
launcher(web login+patch) --cmdline--> CRT entry --> archives --> CState init
  --> option.ini --> net/session --> STATE_AVI(logo) --> STATE_PLANET
  board (emblems + promo) with the STATE_CHANNEL dialog mounted on top
  --> STATE_ROOM --create/join--> STATE_WAITING --host start--> LOADING GATE
  --> STATE_PLAYING --> STATE_RESULT --retry--> WAITING | --leave--> ROOM
side: CHANNEL/ROOM/WAITING <-> SHOP | MYROOM | MUSICSHOP | PLANET | LANKING
      | PAYMENT* | TUTORIAL ; anywhere --> EXIT(teardown)
```

Login has no box: web page → command line → `CTuser_*`/`My_channel` bridge.

## Unknowns (marked, not guessed)
1. `WinMain` address and the exact first-state assignment call (no dossier).
2. Which class hosts `STATE_LANKING` (no `CState*` match). Channel's
   host is RESOLVED: a `CStatePlanet` object at `parent+0xF6C`
   (live memory read 2026-11-09, `docs/channel-fill.md`).
3. `DRUM_PLAYING` trigger — likely dead; no class, stub-only block.
4. Option keys beyond the §3 table (key binds, volumes, resolution).
5. Playing `State_Playing.bnd` 17-rect ↔ 235-entry subset rule (see `opi.md`).
6. Second header numbers (`0X01`…`0X04` on 10 blocks) — semantics TBD.
7. Underscore-prefixed Playing sprite refs (`_Note_Miss.ojt` …) — see `opi.md`.
