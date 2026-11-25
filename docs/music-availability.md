# Unavailable songs — availability model, notice, post-START download (#276)

Version pin for every EXE row below: `OTwo.exe` 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`
(file offset = VA − `0x400000`). Method: static byte scans with GNU
objdump 2.47, radare2 6.2.2, Python 3.14.10 over a read-only copy; no
Wine, no winedbg, no captures. Memoryer rows (`~/mozart-src`) are the
wire-contract reference, labelled corroboration; the exe stays the
behavior oracle. Wire tables: `docs/net-protocol.md`; shop screen:
`docs/musicshop.md`; catalog format: `docs/file-formats/ojn.md`,
`docs/ojnlist-trailer.md`.

Owner symptom (2026-11-16 playtest): unavailable songs are selectable in
our client; in the exe, picking one tells the player a download is
needed, and the song downloads after START.

## 1. Availability model — local file presence (PINNED)

A song is available when its `o2ma<id>.ojn` file exists under
`<install>\Music\`. The client learns this from a directory scan, not
from a server flag.

- Song table: base `0x77952c`, end `0x779530`, stride `0x1b4`, filled by
  the OJNList reader district (`docs/ojnlist-trailer.md`).
- That district enumerates `<exe dir>\music\*.ojn`:
  `SetCurrentDirectoryA` (IAT `0x63f288`, call VA `0x47c84c`),
  `FindFirstFileA` (IAT `0x63f28c`, VA `0x47c8ce`), `FindNextFileA`
  (IAT `0x63f290`, VA `0x47cd83`), `FindClose` (IAT `0x63f294`, VA
  `0x47cdcf`). Pattern strings: `\music\*.ojn` (VA `0x647a44`),
  `music\` (VA `0x647a3c`), `_` (`0x647a34`), `.` (`0x647a38`).
- Each hit is split at `.`, `strtol` yields the music id (VA
  `0x47c9f1`), `or $0x8000` marks it installed (VA `0x47cbec`), and
  VA `0x540150` stores the packed id into the **local member seat**
  `0x77cd00`: `id & 0xfff` at `seat+0x38+idx*4`, `(id & 0xf000) >> 12`
  at `seat+0x1038+idx*4`; the count lands at `seat+0x34` via VA
  `0x540130` (VA `0x47cdc3`). Installed flag value is therefore `0x8`.
- Per-song availability bits ride the song table at `entry+0x15c`:
  bits 0-2 = per-difficulty charts (mask table VA `0x680724` =
  `1`/`2`/`4` = Easy/Normal/Hard), bit 3 (`0x8`) = files installed.
  Reader VAs: `0x436f48`, `0x48fea4`, `0x494869`, `0x4adb6e`,
  `0x4adbd4`, `0x4add2e`, `0x4af65e`, `0x4b44f6`, `0x4be7ab`,
  `0x4be808`, `0x4bed7b`, `0x4bef4b` and neighbours.
- The server does not add songs. The music-list handler district at VA
  `0x561080` matches 12-byte wire rows against the local table by id
  and stamps `entry+0x15c = 0x7` (all three difficulties) before fanning
  out on the 5-way wheel-bucket jump table at VA `0x562e2c`; unmatched
  ids are skipped. So a server row can only *annotate* a song the client
  already knows.

Consequence: a song in `OJNList.dat` with no `o2ma<id>.ojn` on disk is
"not downloaded". Nothing else marks it.

## 2. Wire state — MusicState (PINNED)

- Enum (Memoryer `Encore.Data/Metadata/MusicState.cs`): `Ready 0`,
  `NoMusic 1`, `NoAccess 2`, `Downloading 4`.
- `4025 UpdateMusicState` C→S (one byte): 8 send sites, files `0xab613`,
  `0xab7ab`, `0xaba29`, `0xb0dcd`, `0xb0faa`, `0xb17b4`, `0xb1d1a`,
  `0xb6a9e`; each uses the pinned send idiom (`push 0; push 0xfb9` then
  vtable `+0x5c`).
- **When those 8 sites fire (PINNED, slice 3).** Both waiting-room
  classes are event handlers over `(id, member, arg)`; three sites sit
  in `CLiveWaiting`'s handler (VA `0x4ab330`, RTTI name from radare2
  `aaa`), four in `CNormalWaiting`'s (VA `0x4b0a50`), and one in the
  handler both classes forward unhandled event ids to (VA `0x4b6620`).
  The moment each site fires, by event id:

  | site (file) | class | event | moment |
  |---|---|---|---|
  | `0xab60c` | CLiveWaiting | 4001 WaitingMusicChanged | the new room song was just staged in the waiting music UI |
  | `0xab7a4` | CLiveWaiting | 4022 UserLeaveGame | after the leaving member's seat reconciliation |
  | `0xaba22` | CLiveWaiting | 4024 WaitingSkillChanged | after the skill refresh |
  | `0xb0dc6` | CNormalWaiting | 4001 WaitingMusicChanged | same, with its master-only notice branch first |
  | `0xb0fa3` | CNormalWaiting | 4022 UserLeaveGame | same as the live-mode twin |
  | `0xb17ad` | CNormalWaiting | 4024 WaitingSkillChanged | same as the live-mode twin |
  | `0xb1d13` | CNormalWaiting | 4058 WaitingSkillExChanged | after the skill-ring refresh |
  | `0xb6a97` | both (fallback) | internal UI event `0x10000006` | download queue finished: sends 4025 only when no transfer ran (`scene+0x4fc == 0`), else sends 4036 |

  Consequences: no 4025 fires on a select pick, on a room join, or on
  receiving 4035/4037/4039. The room song the byte is computed for is
  global `0x7789d8`, written by the 4001 network handler VA `0x55f2d0`
  (file `0x15f2d0`): it reads a `u16` music id, one difficulty byte and
  one speed byte, stores the id with its variant nibble lifted to bits
  28-31, then queues the UI events that reach the two handlers above.
  Event ids read from Memoryer's `Commands.cs` (corroboration); the
  sites, the case tables (`0x4abc8c`/`0x4abcb0` live mode,
  `0x4b1f24`/`0x4b1f48` normal mode) and the handler chain are exe pins
  (objdump 2.47 + radare2 6.2.2 slices over the read-only 7.55 copy).
- The byte is computed by VA `0x4be690` (called by every one of those
  sites, files `0xab60c`…`0xb6a97`): take the room's staged music id
  `0x7789d8 & 0x0fffffff`, find its table entry, then
  - `entry+0x15c & 0x7 != 0` → `Ready(0)` when bit `0x8` is set, else
    `NoMusic(1)` (VA `0x4be808`);
  - `entry+0x15c & 0x7 == 0` → the same bit-`0x8` test when global
    `0x77ee28 >= 0xa` or global `0x768b34 != 0`; otherwise
    `NoAccess(2)` plus the notice VA `0x6481e8` "Not enough PT
    (points)." (VAs `0x4be959`-`0x4be98c`, message box on `0x762b44`).
  The state is written into the local seat through VA `0x541460`, i.e.
  the seat field `+0x217c`, and sent as the 4025 body.
- S→C events write the same seat field: `4037 SyncMemberMusicState`
  handler VA `0x559990` (file `0x159990`, two 1-byte reads, first byte
  compared with the local id `0x77cd04`, second byte → `+0x217c`,
  forward `(0x12, 0xfc5, 0, 0)`); `4035 AcquireMusic` handler just above
  (VA `0x5598f7`-area, counted list, same writer, forward
  `(0x12, 0xfc3, member, state)`); `4039 DownloadProgressChanged`
  handler VA `0x559a10` (two 1-byte reads, forward `(0x12, 0xfc7, pct,
  member)`). Seat reset parks `0` at `+0x217c` (VA `0x53fcc1`).
- **#559/#838 refutation — 4035 is the paint arm (capture-pinned).**
  The matched `p559-badge-ab4` pair (exe OTwo.exe sha256 `83a04541…72c823b`,
  Wine 11.22; manifests `~/scratch/p559-badge-ab4/{exe,rust3}/manifest.json`)
  sends `4026 000004` + `4037 0004` and **no 4039**. The exe screenshot
  `exe/artifacts/screenshots/member-downloading.png` (sha256
  `731c4685…d92f0895`) shows no DOWNLOADING badge or bar; our Rust
  screenshot `rust3/states/member-downloading.png` (sha256
  `77d74909…e463a9449`) paints both. Wire logs: `exe/wire/room-stage.log`
  sha256 `f853a28b…3f1f7b7a`, `rust3/wire/room-stage.log` sha256
  `ed682ad2…8ab8a6bbf616a0b`. The first refutation (state alone ⇒ badge)
  was followed by a `4037 0004` + `4039` 42 %/100 % qualification in
  `~/scratch/boot-wave3/{p559-progress-exe3,p559-progress-rust2}`: exe
  `member-progress-42.png` (`d4edc94d…`) and `member-progress-100.png`
  (`459ad408…`) stay badge-free, while Rust `states/member-progress-42.png`
  (`d2c98e5d…`) painted both. The qualified #773 frame
  `~/scratch/p773-proxy-exe3/after-start-immediate.png` (`3a053d00…`) with
  S2C `4035 0200000104` paints the local seat's green
  `Wait_Downloding.ojs` and black `Wait_Downloding_Bar.ojs`. Statics agree:
  the 4035 case in each Waiting dispatcher (normal `0x4b1d42`, live
  `0x4aba51`; REA `ev_18a485d7…` / `ev_0593bf27…`) calls the seat refresh
  and, for the local member, `StartMusicDownload` (`0x4bb1e0`); the 4037
  case only refreshes/retries. Our painter therefore arms those two bodies
  **only on 4035 with a non-Ready local member**. 4037/4039 stay wire-only
  after #838; the #559 equal-07E8 A/B recapture stays OPEN.

## 3. Notice pin — the select dialog says "not downloaded" (PINNED)

`docs/dialog-strings.md` §3 recorded the message-box table as missing;
this section pins the download subset (CP949, text summarized).

| VA | text (own words) | where used |
|---|---|---|
| `0x645ae0` | "This song is not downloaded. Click the [Download] icon to download it." | select-dialog row flag test, file `0x49542` |
| `0x6459e8` | "The song is downloading. Press OK after it completes." | file `0x3eb83` |
| `0x645858` | "Downloading : [%d%%]" | file `0x3e163` |
| `0x645b20` | "Wait for the current download to finish first." | files `0x4e1c0`, `0x4e1e7` |
| `0x645b9c` | "Not enough disk space; check free space." | file `0xb6b24` |
| `0x6498b0` | "The music file for this purchased/free song is not in the Music folder. Download it from the music shop!" | files `0xadfa1`, `0xaf8c6`, `0xb4929` |
| `0x649738` | "A user left during the song download; start the game again." | file `0xab4d4` |
| `0x649b60` | "Error while receiving the file; retrying the download." | file `0xb6af3` |
| `0x649ba0` | "Could not reach the download server; retrying the download." | file `0xb6ac2` |
| `0x64a1dc` | "There is no playable song. Download a song." | file `0xbf6cc` |
| `0x64a710` | "Cancel the download?" | file `0xc8257` |
| `0x644eb0` | "Cannot create a room: no playable %d-Key song. Download a song." | file `0x36d3d` |

The select (선곡) dialog keeps rows of stride `0x238` (vector
`[obj+4]`..`[obj+8]`; whole-row copy at file `0x49527`, `rep movsl` of
`0x8e` dwords). Row field `+0x218` is the downloaded flag: the confirm
path reads it at VA `0x449534` and, when it is zero, sets the caption
`0x645ae0` on the shared notice dialog object `0x77acc8` through VA
`0x50bfb0` (thiscall, args `(text, 0, 0xffff)`; the same helper carries
the download-failure notices). A second read sits at VA `0x448d11`.
The two select handlers that consume `entry+0x15c` (VAs `0x4ad940`,
`0x4b42d0`) show `0x6498b0` when the chosen song's file is missing and
send `4000 SetRoomMusic` only for a row whose difficulty bit is set.

## 4. Post-START download flow (PINNED)

StartMusicDownload, VA `0x4bb1e0` (waiting-room state method, file
`0xbb1e0`):

1. `0x4f7a10` on the download-manager singleton (global `0x77a868`)
   answers "transfer active?"; if yes, `0x4f9490` runs first.
2. `0x4f87f0(id, 0)` with `id = 0x7789d8 & 0x0fffffff` (files
   `0xbb205`-`0xbb216`) enqueues the room's song.
3. Queue = manager vector at `+0x64`, element stride 8. Empty → return;
   else `0x4f9410` pumps the engine (file `0xbb263`) and
   `state+0x4fc = 1` marks "download running" (file `0xbb26b`).

The only two callers are waiting-room dispatcher arms (VA `0x4abaf7`,
VA `0x4b1dd7`), each entered when the event's flag argument is non-zero
and its member argument is the local member id `0x77cd04`; that arm sets
`state+0x504 = 1` and `state+0x538 = 1`. The opposite arm sends `0xfaa`
= `4010 StartGame`. So the START exchange is what kicks the download for
the member who lacks the song — the owner's observation, in exe terms.

Engine: WinInet FTP. Imports (IAT VA) `InternetOpenA 0x63f4b8`,
`InternetConnectA 0x63f4b4`, `FtpSetCurrentDirectoryA 0x63f4a4`,
`FtpFindFirstFileA 0x63f4a8`, `FtpOpenFileA 0x63f4c0`,
`InternetReadFile 0x63f4bc`, `InternetCloseHandle 0x63f4a0`,
`FtpDeleteFileA 0x63f4d0`, `FtpRenameFileA 0x63f4b0`. Client district VA
`0x4f7a30` (building `o2ma<id>.ojn` / `o2ma<id>_<variant>.ojn` names),
VA `0x4f94b0` (state machine), VA `0x4fa5b0` (temp→final promote),
VA `0x4f87f0` (enqueue), VA `0x4f9410` (pump). Endpoints and staging
names are the ones already pinned in `docs/net-protocol.md` §2
(`O2JAM_MUSIC_FTP`, `121.125.76.136`/`.135`, `hione@nowcom.co.kr` /
`anonymous`, `O2JamMusic/`, `MUSIC\Temp\`, `FileStatus%d`…).

Retry/error dispatcher (cases at files `0xb6abb`, `0xb6aec`, `0xb6b1d`):
unreachable → notice `0x649ba0`, `Sleep(1000)`, pump again; transfer
error → notice `0x649b60`, `Sleep(1000)`, pump again; disk full → notice
`0x645b9c` then walk the queue. A finished transfer lands in
`MUSIC\Temp\`, VA `0x4fa5b0` promotes it into `MUSIC\`
(`FindFirstFileA`/`MoveFileA`/`DeleteFileA`, files `0xfa788`/`0xfa7e8`/
`0xfa811`), and `5012 SyncMusicDownload` (u16 music id) announces
completion on game TCP. Progress to peers: `4038 SetDownloadProgress`
(`progress, member`, 2 send sites: files `0xba58f`, `0xba5b3`).

Server side (corroboration, Memoryer `Controllers/WaitingController.cs`):
`StartGame` with more than one member broadcasts `AcquireMusicEventData`
(4035) with `Downloading` for members whose state is `NoMusic`, answers
4011 `StartResult.None`, and treats a member as `Ready` once
`SetDownloadProgress` reports 100 %.

### 4a. The START gate and the kick arm (PINNED, slice 3)

The `CNormalWaiting` START click handler (VA `0x4b22d0`, file
`0xb22d0`, control word `0x16202012`) runs this order:

1. a transfer already stands (`scene+0x504 != 0`) → only the button face
   is re-armed, no packet, no notice;
2. gate helper VA `0x4b5560` answers a verdict word:
   - `1` → send `4010 StartGame` (`0xfaa`);
   - `-2` (`0xfffffffe`) → send **`4034 AcquireMusicRequest`**
     (`0xfc2`, empty body; send push at file `0xb23d2`), no notice;
   - `-1` (`0xffffffff`) → notice VA `0x6497ec` "some players are not
     ready" on the notice box (call VA `0x478c90`, text push file
     `0xb23b1`);
   - `-5` (`0xfffffffb`) → notice VA `0x6497bc` "the teams do not match"
     (text push file `0xb23ee`);
   - `-3`/`-4` → no packet and no notice (the player sees the download
     bar).

Gate helper VA `0x4b5560` (file `0xb5560`) verdicts in test order:

1. any occupied peer seat (index other than the local id byte
   `0x77cd04`) with a zero ready byte (`seat+0x32`) → `-1`;
2. with more than one member inset (`0x7789ec`), a team histogram over
   the self team (`0x77cd31`) and the peer team bytes (`seat+0x31`):
   every used team must hold the same number of players and at least two
   teams must be used, else `-5` (the same rule Memoryer's
   `TeamUnbalanced` test applies, and its join path gives every arriving
   member the first unused team, so a real room always spans teams);
3. each occupied peer seat's music-state byte (`seat+0x217c`): `1` →
   `-2`, `2` → `-3`, `4` → `-4`;
4. else `1`.

The loop skips the local seat, so our own missing song never blocks the
click: the server's refusal is what kicks our download. `CLiveWaiting`
carries a simpler twin at VA `0x4aefe0` that also tests the local seat's
byte (global `0x77ee7c` = self seat `+0x217c`) and has no team arm; both
twins answer the same `-1`..`-4` codes.

The 4035 answer then runs the per-member arms (CNormalWaiting
`0x4b1d42`, CLiveWaiting `0x4aba51`):

- state non-zero → `scene+0x504 = 1` (transfer running) and
  `scene+0x538 = 1` (start pending), refresh, then, when the member is
  the local id, **VA `0x4bb1e0` StartMusicDownload** — the arm taken
  instead of sending 4010;
- state zero → only with the master byte (`0x77cd30`, the self seat's
  master marker) set and the gate verdict `> 0`: send 4010 and clear
  `scene+0x538`.

The retry arms are the same master-and-gate pair: 4037 SyncMemberMusicState
(`0x4b1de1`), 3007 UserLeaveWaiting and 3009 RoomSlotUpdate. A member
leaving during a transfer adds notice VA `0x649738` "a user left during
the song download; start the game again" (file `0xab4d3`).

4011 itself maps to the waiting classes' default arm — neither class
reads its result word; the refusal still has to keep the client in
Waiting, because the server starts nothing while a member's state is not
Ready.

## 5. OPEN (next evidence step each)

1. Writer of `entry+0x15c` bit `0x8`. The scan packs the installed bit
   as `0x8` into `seat+0x1038`; follow the `-0x494` compare pair (VAs
   `0x47cc61`, `0x47cccb`) and the `0x540150` call (VA `0x47cbfe`) to
   see whether a merge pass copies it into the song table or whether the
   dialogs read the seat array instead.
2. `row+0x218` writer: the only write in the image is the row-to-row
   copy at VA `0x442455`, so rows arrive pre-built; find that record's
   source (the `imul $0x238` sites between VAs `0x43f000`-`0x445000`).
3. `5012` send site: scan for the `SetID` ctor store of `0x1394` rather
   than a `push imm32` (that scan found nothing).
4. Which scene class owns each twin select handler (VAs `0x4ad940`,
   `0x4b42d0`) — vtable/RTTI read. **Partly closed in slice 3:** the
   same twins are called from the two waiting classes' own event arms
   (`0x4ad940` from `CLiveWaiting`, `0x4b42d0` from `CNormalWaiting`),
   so each one belongs to its class; which *select* dialog instance
   uses them stays OPEN.
5. Meaning of globals `0x77ee28` (compared `>= 0xa`) and `0x768b34` in
   the `NoAccess` branch of VA `0x4be690`.

Closed by slice 3: the 4025 send moments (§2's table above) and the
`-2` acquire arm of the START gate (§4a).

## 6. Our client today

Slices 1-3 landed (1+2 merged with the #273/#274 song-dialog filter
columns, `docs/select-filters.md`). The promote half of slice 4 landed:
the queue is pumped in Waiting, a fetched chart is staged in `Music\Temp\`
and renamed into `Music\`, the staged catalog takes the rescan, and the
session sends `4038` (100 %) then `5012`. The real WinInet FTP transfer,
partial progress and the download bar are still OPEN; without a fetch
source the engine stays idle.

- `crates/o2jam-file/src/music.rs`: pure availability model.
  `installed_music(names)` packs a `Music\` listing into the seat shape
  of §1 (id `packed & 0x0fff`, variant nibble `(packed & 0xf000) >> 12`
  with the installed mark `0x8`, one entry per file, duplicates kept);
  `ChartMask` is the `entry+0x15c` chart triple.
- `crates/o2jam-shell/src/music_install.rs`: the thin `read_dir`
  adapter at the shell edge. `scan_installed_music()` answers `None`
  when the install has no `Music` folder (CI ships no game data), and
  `None` means **no local knowledge**: `SelectMusicSong.installed`
  stays `true` for every catalog row, which keeps the seam tests' world
  coherent and paints no invented notice. A real install always has
  the folder, so the live client always has the scan.
- `crates/o2jam-shell/src/screens/side.rs`: `SelectMusicSong` carries
  `charts`/`installed`; `song_installed` is the one availability
  answer, `SelectMusicRow::playable` the per-chart gate.
- `crates/o2jam-shell/src/screens/game/mod.rs`:
  `set_song_catalog_with_labels` stages the scan's answer
  (order-independent with `set_installed_music`); a pick of a row
  whose chart file is missing raises `SELECT_NOT_DOWNLOADED_NOTICE`
  and queues no `4000` (§3). The dialog's ONE row list feeds both
  features: `select_music_rows` narrows by the live filter columns
  (#273) and every surviving row carries its availability, so a row
  click resolves its index against the filtered rows and then refuses
  an uninstalled one. The caption inks over the dialog's own bottom
  caption line (`IDC_TEXT_DSMUSIC_SERACHRESHLT`) — the exe sets it on
  the shared notice box object `0x77acc8`, whose sheet we do not hold,
  so the box geometry stays OPEN.
  `set_member_music_state`/`member_music_state` are the seat `+0x217c`
  byte.
- `crates/o2jam-shell/src/screens/game/mod.rs`: `can_start_game` keeps
  the readiness rule, and `start_gate_verdict` is the exe's gate (§4a):
  readiness, then team balance, then every occupied peer's music state,
  in the exe's order. `press_start` runs the click arms and answers
  `StartRequest::{StartGame, AcquireMusic, Nothing}`; the two gate arms
  with a box stage a pinned notice (`WAITING_START_NOT_READY_NOTICE`,
  `WAITING_START_TEAMS_NOTICE`) that the waiting painter inks over the
  top-bar `ROOMNAME` line — the notice box sheet is not pinned, the same
  stand-in shape the song dialog uses. `music_state_for(song_id)` is our
  own byte (§2's two-way answer) and `room_music_id` the staged room
  song.
- `crates/o2jam-shell/src/music_download.rs`: the download queue behind
  the kick (§4). `MusicDownload::enqueue` keeps one request per song and
  `is_running` is the scene's `+0x4fc`; `pump` reads a named chart from
  the configured source, parks it in `Music\Temp\`, renames it into
  `Music\` (VA `0x4fa5b0`) and reports the completion. `chart_file_name`
  is the exe's `o2ma<id>.ojn` name (VA `0x4f7a30`).
- `crates/o2jam-shell/src/screens/game/mod.rs` `begin_music_download`
  is the kick the session calls; `pump_music_downloads` advances the
  queue and restages the scanned availability into every catalog row,
  so a promoted song becomes selectable again. A `music-state=4` seat
  now arms the staged `Wait_Downloding.ojs` badge and its bar at the
  Waiting seat-template `.bnd` rects (matched-ab-339 `4037 0004`;
  ticket #559). `4039` percentages stage per seat, but the bar's
  frame/fill mapping stays OPEN: no synthetic fill is painted. The
  separate `WAITING_START_DOWNLOAD_NOTICE` (VA `0x6459e8`) still
  covers the local START refusal.
- `crates/recomp2jam/src/connect.rs`: `run_song_flow` sends the honest
  `UpdateMusicState` 4025 (`Ready 0` when the install has the chart
  file, `NoMusic 1` when not) instead of the old blind
  `SyncMemberMusicState` plus `SetDownloadProgress(100)` pair. The CLI
  derives the byte with `main::installed_music_state` (§1 scan); a
  download that never ran has no percentage to report, so `4038` no
  longer rides the select flow.
- `crates/recomp2jam/src/session.rs`: the Waiting drain folds 4037
  (one seat) and 4035 (every listed seat) into the room mirror through
  `ScreenGame::set_member_music_state`, announces 4025 at the pinned
  moments (§2: the 4001 song change, 4022 UserLeaveGame, 4024
  WaitingSkillChanged, 4058 WaitingSkillExChanged), runs the START
  gate's arms (`4034` on a missing song, `4010` when startable,
  the pinned notice arms otherwise), kicks the room song's download when
  the 4035 answer marks our own state non-Ready, arms the local seat's
  DOWNLOADING badge and bar through `ScreenGame::arm_local_download_badge`,
  retries `4010` from the 4035/4037 ready arms, and keeps Waiting when a
  4011 arrives with a non-zero result. 4039 stages its percentage in the
  seat's progress mirror and 4037 Ready clears that readout; neither
  event changes the paint arm (#838). The bar's frame/fill mapping stays
  OPEN, so no synthetic fill is painted.
  `configure_music_source` points the engine at the optional
  `O2JAM_MUSIC_SOURCE` local mirror (our adapter, not an exe setting;
  the WinInet FTP client stays OPEN). Each Waiting paint runs
  `poll_music_downloads` -> `ScreenGame::pump_music_downloads` and reports
  a promoted chart as `SetDownloadProgress` 4038 (100 %) then
  `SyncMusicDownload` 5012 (§4's announce). Partial progress percentages,
  retry notices and the bar remain OPEN.
- `crates/o2jam-net`: 4025 `UpdateMusicState`, 4026/4022/4024/4058 event
  ids, 4034 `AcquireMusicRequest`, 4035 `AcquireMusic`, 4036
  `SyncMemberMusicState`, 4037 `SyncMemberMusicStateEvent`, 4039
  `DownloadProgressChanged`, 4001 `WaitingMusicChanged`, 5012
  `SyncMusicDownload` plus the `MusicState` enum. The `4030`/`4031` list
  family is already in `o2jam-net`.

## 7. Implementation slicing (proposal)

Slice 1 — availability model (pure, `o2jam-file` + `o2jam-shell`).
Add a pure `installed_music(dir names) -> InstalledMusic` that turns a
list of file names into the seat shape above (id, variant nibble,
installed flag) with scenario tests, plus a thin `read_dir` adapter at
the shell edge. Extend `SelectMusicSong` with `charts: ChartMask` and
`installed: bool`; rows for an uninstalled song render the reference's
"[Download]" state and refuse confirm. Acceptance: selecting an
uninstalled song shows the notice, never a 4000; a test names the live
caller chain from catalog load to the row.

Slice 2 — wire state (`o2jam-net` + session). Add 4025
`UpdateMusicState`, 4035 `AcquireMusic`, 4037 `SyncMemberMusicState`
event, 4039 `DownloadProgressChanged`, 5012 `SyncMusicDownload`
(fixtures from Memoryer field orders; exe read orders in §2 as the
decoder knob). Replace `send_sync`'s blind 100 % with the real state
from slice 1. Acceptance: a fake gateway e2e proves a NoMusic member
refuses 4011 and reports NoMusic; payload guards cite §2 offsets.

Slice 3 — START gate + download kick (`o2jam-shell` + binary).
**Landed.** Gate `can_start_game`/`confirm_waiting` on the pinned gate
verdicts (§4a) and wire the acquire arm: a member without the room song
answers `4034 AcquireMusicRequest`, the 4035 answer kicks that member's
download instead of starting, and the 4035/4037 ready arms retry `4010`.
A non-zero 4011 result keeps Waiting.

Acceptance, corrected against the exe: the START with an uninstalled
room song does **not** show a "user left / start again" notice there —
that notice (VA `0x649738`) belongs to the leave-during-download arm.
The `-2` arm sends `4034` and shows nothing; the player's visible
feedback is the download bar. Because slice 4 owns that bar, slice 3
paints the pinned download-in-progress notice class (VA `0x6459e8`)
until then. The gate's own notices are the not-ready (`0x6497ec`) and
team (`0x6497bc`) arms. Guards name the caller chain from
`main.rs::run_play`.

Slice 4 — downloader (`o2jam-file` naming + new OS adapter). Pure part:
id → `o2ma<id>.ojn`/`o2ma<id>_<v>.ojn` names, `MUSIC\Temp\` staging,
promote plan, retry policy (§4). OS edge: plain FTP client (config key
`O2JAM_MUSIC_FTP`, hosts and credentials from §4) writing to the stage
dir, then promote + 4038 progress + 5012 announce. Acceptance: a local
FTP fixture server fetches a synthetic `o2ma999.ojn`, promotes it, sends
progress then 5012, and the song becomes selectable and startable.

Slice 5 — owner confirm (playtest both branches: shop-style notice and
post-START download), then close per `docs/agents/triage-labels.md`.

## 8. Guards (slices 1+3)

Live chain or seam reason per `docs/live-path-reachability.md`; every
guard below carries a red-proof (PASS / FAIL / PASS / FAIL / PASS) in
the closing report.

- `scan_pattern_names_pack_the_installed_seat`,
  `non_chart_names_stay_out_of_the_seat`,
  `variant_suffix_and_directory_prefix_keep_the_same_id`,
  `seat_holds_one_entry_per_file_and_masks_the_id_to_twelve_bits`,
  `seat_lookup_answers_by_id_not_by_position`,
  `chart_mask_reads_the_three_difficulty_bits` — seam: pure model, no
  runtime caller beyond `installed_music`/`ChartMask`.
- `absent_music_folder_answers_no_local_knowledge`,
  `music_folder_listing_packs_the_chart_files_only` — seam: the
  `read_dir` adapter is the only OS touch; a synthetic temp folder
  proves it without game data.
- `select_songs_take_their_availability_from_the_install_scan` — seam:
  the catalog mapper is pure. Live chain proven by
  `uninstalled_song_row_shows_the_notice_and_refuses_the_pick`.
- `uninstalled_song_row_shows_the_notice_and_refuses_the_pick` — live
  chain: `main.rs::run_play` → `stage_session_catalog`
  (`music_install::scan_installed_music` →
  `session::OnlineSession::set_installed_music` →
  `ScreenGame::set_installed_music`) /
  `stage_offline_windowed` → `stage_offline_music_boards` →
  `ScreenGame::set_song_catalog` → `click_waiting_button` (MUSELECT) →
  `click_select_music` → `pick_select_music_row`.
- `select_notice_caption_inks_over_the_dialog_status_line` — live
  chain: the staging above → `ScreenGame::paint_at` → `paint_waiting`
  → `paint_select_music_text` → `select_paint::paint_select_text`.
- `update_music_state_carries_one_state_byte`,
  `music_state_for_installed_follows_the_exe_two_way_answer`,
  `sync_member_music_state_event_reads_member_then_state`,
  `acquire_music_counts_member_state_pairs`,
  `download_progress_changed_reads_percentage_then_member`,
  `sync_music_download_carries_the_song_id`,
  `resolves_music_state_family_ids` — seam: pure codecs and the id
  table, §2/§4 offsets cited per test.
- `no_music_member_reports_no_music_state_and_no_blind_progress` —
  live chain: `main.rs::run_command` (`connect --music-id`) →
  `SongFlowParams::into_song_flow` → `main::installed_music_state` →
  `connect::connect_and_login_with_transport` →
  `connect::run_song_flow` → `connect::send_music_state`.
- `waiting_seats_take_the_music_state_family` — live chain:
  `main.rs::run_play` → `assemble_online_session` →
  `session::OnlineSession::paint` (Waiting) → `poll_waiting_chat` →
  `drain_waiting_chat` → `apply_member_music_state` /
  `apply_acquire_music` → `ScreenGame::set_member_music_state`.

### Slice 3 guards

Red-proofs ran as green → break → red → restore → green; the break
listed after each name is the one that turned it red.

- `no_music_member_blocks_start_and_kicks_the_room_song_download` — live
  chain: `main.rs::run_play` → `assemble_online_session` →
  `OnlineSession::paint` (Waiting) → `poll_waiting_chat` →
  `drain_waiting_chat` → `apply_acquire_music` →
  `answer_acquire_music` → `kick_room_song_download` →
  `ScreenGame::begin_music_download`. Break: the kick arm removed, or
  the 4037 retry arm removed.
- `a_refused_start_broadcast_keeps_the_room_in_waiting` — live chain:
  the same paint → `drain_waiting_chat` → `apply_start_broadcast`. Break:
  the result word ignored.
- `the_room_song_change_announces_the_music_state` — live chain: the
  same paint → `drain_waiting_chat` → `apply_waiting_music_changed` →
  `announce_music_state`. Break: the 4001 arm removed.
- `a_member_leaving_the_game_and_a_skill_change_reannounce_the_music_state`
  — same chain → `announces_room_music_state` → `announce_music_state`.
  Break: the three event arms removed, or only 4022 kept.
- `start_gate_asks_for_the_music_when_a_peer_lacks_the_room_song` —
  chain: `session::confirm_master_start` → `ScreenGame::press_start` →
  `start_gate_verdict`. Break: the `NoMusic` peer arm claims startable.
- `start_gate_tests_readiness_before_the_team_and_music_arms` — same
  chain. Break: readiness checked only for a Ready-music peer.
- `start_gate_reads_the_team_shape_like_the_exe` — same chain. Break:
  `teams_startable` always true.
- `start_click_during_a_download_only_rearms_the_face` — same chain plus
  `ScreenGame::begin_music_download`. Break: the running-download arm
  removed.
- `the_waiting_notice_takes_the_room_title_line` — chain:
  `ScreenGame::paint_at` → `paint_waiting` → `paint_waiting_scene` →
  `paint_waiting_staged` → `paint_waiting_top_line`. Break: the notice
  branch falls back to the title.
- `enqueuing_a_room_song_marks_the_download_running`,
  `the_same_room_song_is_queued_once`,
  `an_unstaged_room_song_queues_nothing`,
  `finishing_the_last_transfer_clears_the_running_flag` — seam: the pure
  queue, reached from `begin_music_download` and slice 4's pump. Break:
  the duplicate check removed.
- `acquire_music_request_carries_no_body` and
  `waiting_music_changed_reads_id_then_difficulty_then_speed` — seam: wire
  codecs, §2/§4a offsets cited per test. Break: the 4001 difficulty and
  speed bytes swapped.
- OPEN left for slice 4/5: the download bar geometry (`Wait_Downloding`
  art), the notice-box sheet, and the real transport.

## 9. V13 music-select pair — chat and role are wire-scenario deltas

The v13 music-select pair (`~/scratch/paired-shots-v3/states/music-select/exe.png`
vs `~/scratch/paired-shots-v6/legs/music-select/rust/states/music-select-p2.png`)
shows exe chat lines and a master START face where ours shows an empty
chat box and a guest READY face. Per the owner A/B rule this is not a
scenario dismissal: the wire proves it. The exe leg's `room-stage.log`
carries `3007 body=0001000000000000` (member 0 leaves, member 1 promoted)
and then `2007`; the Rust leg carries neither. Both legs logged in as
user 1 and both room snapshots carry `tester`/`t207probe` (§2's seat
family). The exe's Korean leave/skill notices appear in neither wire
log, so they are locally generated chat on that 3007/2007 path (handler
evidence `ev_50242b6c183543f49ce6cbb43eccad2e`); no 4025/4035/4039
message-state difference is involved. No paint change is warranted:
replaying the exe's 3007 into our client already promotes the local
seat and flips the role-owned controls (guards in
`docs/parity/waiting-chrome-dossier.md` §6).
