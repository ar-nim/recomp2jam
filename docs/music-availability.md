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
   `0x4b42d0`) — vtable/RTTI read.
5. Meaning of globals `0x77ee28` (compared `>= 0xa`) and `0x768b34` in
   the `NoAccess` branch of VA `0x4be690`.

## 6. Our client today

Slices 1+2 landed, merged with the #273/#274 song-dialog filter columns
(`docs/select-filters.md`); slices 3-5 still open.

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
- `crates/o2jam-shell/src/screens/game/mod.rs` `can_start_game` still
  gates on peer readiness only — no music-availability gate (slice 3).
- `crates/recomp2jam/src/connect.rs`: `run_song_flow` sends the honest
  `UpdateMusicState` 4025 (`Ready 0` when the install has the chart
  file, `NoMusic 1` when not) instead of the old blind
  `SyncMemberMusicState` plus `SetDownloadProgress(100)` pair. The CLI
  derives the byte with `main::installed_music_state` (§1 scan); a
  download that never ran has no percentage to report, so `4038` no
  longer rides the select flow.
- `crates/recomp2jam/src/session.rs`: the Waiting drain folds 4037
  (one seat) and 4035 (every listed seat) into the room mirror through
  `ScreenGame::set_member_music_state`; 4039 decodes and traces only —
  the percentage has no staged seat field until the download bar lands
  (slice 3/4).
- `crates/o2jam-net`: 4025 `UpdateMusicState`, 4035 `AcquireMusic`,
  4037 `SyncMemberMusicStateEvent`, 4039 `DownloadProgressChanged`,
  5012 `SyncMusicDownload` plus the `MusicState` enum. Still missing:
  4034 `AcquireMusicRequest` (the acquire ping, §4); the `4030`/`4031`
  list family is already in `o2jam-net`.
- OPEN for slice 3: the exe's eight 4025 send sites are unmapped, so
  the live session does not announce our state yet — the headless
  `connect --music-id` flow does. Mapping one site to the waiting
  song-change/START arm is the next evidence step (§5 items 1-2 give
  the code to follow).

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
Gate `can_start_game`/`confirm_waiting` on every member's music state
being Ready (the 4011 refusal arm, §4). Acceptance: START with an
uninstalled room song shows "a user left/start again"-class notice and
starts the download instead of loading; guard names the caller chain.

Slice 4 — downloader (`o2jam-file` naming + new OS adapter). Pure part:
id → `o2ma<id>.ojn`/`o2ma<id>_<v>.ojn` names, `MUSIC\Temp\` staging,
promote plan, retry policy (§4). OS edge: plain FTP client (config key
`O2JAM_MUSIC_FTP`, hosts and credentials from §4) writing to the stage
dir, then promote + 4038 progress + 5012 announce. Acceptance: a local
FTP fixture server fetches a synthetic `o2ma999.ojn`, promotes it, sends
progress then 5012, and the song becomes selectable and startable.

Slice 5 — owner confirm (playtest both branches: shop-style notice and
post-START download), then close per `docs/agents/triage-labels.md`.

## 8. Guards (slices 1+2)

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
