# Net protocol (7.55 client) — endpoints + packet map

Version pinned to 7.55 (`AngelO2Jam.exe` 5,029,888 B, sha256 `83a04541…72c823b`;
`NowSmartLauncher.ocx` 462,848 B, sha256 `ff8868ed…a19ca`).
Static analysis only: string/file-offset survey plus REA/Ghidra dossiers.
Opcode table cross-checked against Memoryer.Encore `Source/Memoryer`
(supported client v7.55 O2Jam Classic; other builds are version-mismatched
and out of scope). No live-server contact.

## 1. Traffic split (launcher vs game)

| Plane | Owner | Transport | Notes |
|---|---|---|---|
| Patch | `NowSmartLauncher.ocx` (MFC ActiveX) | HTTP zip: `http://%s/O2Jam/pub/O2Jam/%s/%s%2.1f.zip` (@`0x4e044`); registry `Software\Nowcom` `Version_Patcher`/`launcher_version`/`patcher_version`/`channel_info` | Patch-time only; never used in-game |
| Game TCP | `AngelO2Jam.exe` + `O2Net32.dll` | TCP to gateway port `15010` (default), length-framed `CPacket_Str` messages | `O2Net32.dll` is a thin Winsock wrapper (exports `O2Net_StartUp`/`Send`/`Recv`; imports `WSASocketA`/`WSAConnect`/`WSASend`/`WSARecv`) |
| Live relay | game exe (`CRelayServerConnect`, RUDP trio, `CHolePunchingCtrl`) | TCP fallback + UDP primary to 3 hardcoded relay IPs; zlib frames over UDP | Live Mode only (room PvP lane share) |
| Music download | game exe (`CCFtp`) | FTP to `O2JAM_MUSIC_FTP` hosts; `5012` announces completion on game TCP | No HTTP for songs |
| Billing/shop web | game exe (`WININET`: `InternetConnectA`/`HttpSendRequestA`, `CCHtmlFile`, `CBillManager`, `CWeb*Ctrl`) | HTTP `.phtml`/`.aspx`/`.php` pages + `:18000` upload | Cash, gifts, coupons, events |
| Telemetry/crash | `SendWebData.dll` (MFC `CHttpConnection`), dump FTP | HTTP + `O2JAM_DUMP_FTP` | No game opcodes; out of scope |

## 2. Endpoint table (in-exe strings, file offsets)

| Endpoint | Value | File offset | Use |
|---|---|---|---|
| Music FTP 1 | `121.125.76.136` | `0x24b9c0` | `CCFtp` song download |
| Music FTP 2 | `121.125.76.135` | `0x24b9d0` | fallback host |
| FTP user / pass | `hione@nowcom.co.kr` / `anonymous` | `0x24b9e0` / `0x24b9f4` | FTP login pair |
| FTP remote dir | `O2JamMusic/` + key `O2JAM_MUSIC_FTP` | `0x24ba14` / `0x24ba20` | remote listing |
| Staging names | `o2ma`, `.ojn`, `MUSIC\Temp\`, `.\Music\TEMP`, `MUSIC\TEMP\`, `FileStatus%d`/`FileName%d`, `FileNum`/`FileInfo` | `0x24ba00`–`0x24bae0` | `o2ma###` fetch + `FileStatus` flow |
| Registry | `SOFTWARE\NOWCOM\O2Jam` | `0x24baf0` | install/config root |
| Patch host | `game1.o2jam.com` + `Patch` + `b.txt` | `0x24bda0`–`0x24bdb8` | version check |
| Channel hosts | `filedown2.o2jam.com` | `0x2513a4` | bootstrap/download |
| Channel tags | `MONAWA`/`GAMEANGEL`/`NATEON`/`PDBOX`/`O2JAM`, `MEMBERSHIP`/`NULL`/`PCROOM_USER` | `0x251378`–`0x2513cc` | per-publisher channel select |
| Game port | `15010` | `0x2513d8` | gateway TCP default |
| Gateway IP slot | `221.132.84.4` + `PORT%d`/`IP%d` keys | `0x2513e0`/`0x2513f0`–`0x2513f8` | `Identifaction.ini` (sic) server list |
| Launch auth | `USER_INDEX_ID`, `O2JamGameVer[ %s ]`, `IEFrame` | `0x251400`/`0x251498`/`0x251490` | `OTwo.exe … \|test\|??\|addr\|port` argv |
| Crash FTP | `203.238.137.58`, `O2JAM_DUMP_FTP`, `user`/`sksnwk` | `0x2514d0`/`0x2514f0`/`0x2514e0` | `.dmp` upload |
| Relay IPs | `203.238.136.184`/`.182`/`.183` | `0x24e240`/`0x24e250`/`0x24e260` | Live TCP+UDP relays (hardcoded ×3) |
| Relay IP 4 | `203.238.136.52` | `0x24e368` | adjunct relay/UHP host |
| UHP strings | `CHolePunchingCtrl::InsertUHPResult`, `Hole punching Result : %d : IP : %d - Port : %d`, `UHP Start`, `UDP Hole Punching Success~!!` | `0x24e1e4`–`0x24e2c0` | UDP hole-punch flow |
| Score upload | `http://upload.nopp.co.kr:18000/O2JamUpload/FileUpload.aspx` | `0x249420` | HTTP replay/score post |
| Web pages | `cash_bill`, `gift_input.aspx`, `payment_input.aspx`, `coupon.phtml`, `jamcard.asp`, `gamefind…asp`, `gamemusic.asp`, `hot_time.php`, `ticket_add.php`, `mtm/ingame_1n1qna01.phtml`, `bingo/index.asp` | `0x2466d0`–`0x24a2d0`, `0x24b638`–`0x24b87c`, `0x248398`, `0x24a9e7` | billing/gifts/events; unreachable `hot_time.php` crashes startup |
| Loopback | `127.0.0.1` | `0x253a34` | local reference only |

Wire imports: `WS2_32.dll` `WSASend`/`WSARecv`/`WSASendTo`/`WSARecvFrom` (IAT `0x23f514`/`0x23f590`/`0x23f584`/`0x23f580`);
`NetLib::CClientNet::AppendPacket` + `send() Failed - sendPack : %d` (`0x265678`/`0x26564c`);
RTTI `JNetTCPManager`/`JTCPSocket`/`JUDPSocket`, `CUDPFacade`/`CTCPRelayServClient`/RUDP
receiver/deliver/resender/`CUHPSender`, `CRelayServerConnect`/`CRelayRegisterReq`/
`CRelayConnectCheckReq`/`CConnectTCPConfirmKey`/`CUDPFailTCPConnect`, `CMessengerHandler`/
`MessengerNetTCPMNG`, `CStateRoom`/`CStateWaiting`/`CStateShop`/`CStateMShop`/`CStateResult`,
`CO2JamCoolPack`/`CO2JamMissPack` (`ev_da26a62f`).

## 3. Wire format (in-exe)

`CPacket_Str` carries the id as little-endian u16 at buffer offset 2
(`SetID` writes `[ebp+8]` word at base+2, `FUN_005a0550` file `0x1a0550`,
`ev_861713e2`; `GetID` reads the word at base+2, vaddr `0x5a05d0`).
The length prefix counts the WHOLE frame including itself
(server-verified: Encore `SizePrefixedMessageFramer<short>`
(Memoryer `Program.cs` registers it) writes `payload + prefix`
and reads `size - prefix`, and a live gateway hung waiting for the
2 bytes our old after-header lengths omitted (fixed 2026-11-06).
An empty body thus frames as `04 00 <id>`.
Send-fail logging tags each frame with `sendPack` id (`0x401cfe`).
Direction convention (Memoryer): odd/even pairs — request C→S `N`,
response S→C `N+1`; events S→C broadcasts.

### 3b. String fields (Memoryer/Encore side, pinned)

Reference: Encore framework (`SirusDoma/Encore`, open source),
`Messaging/Codecs/StringMessageFieldCodec.cs` +
`Utilities/StreamExtensions.cs` + `StringMessageFieldCodecTests.cs`.
`[StringMessageField]` defaults: UTF-8, null-terminated (`0x00`), no
length prefix, max 65535 chars. Encode = UTF-8 bytes + terminator,
truncated to `maxLength` *including* the terminator; a length prefix,
when present, counts bytes *including* the terminator. Decode =
consume until the `0x00` unit (encoding-aware) and stop there; empty
stream decodes to empty string. Plain `[MessageField]` on a string
means no terminator and no prefix — it consumes to end of stream.
Test vectors from Encore's own suite: `"abc"` → `61 62 63 00`;
prefixed-u16 unterminated → `03 00 61 62 63`; prefixed-byte
terminated → `04 61 62 63 00`.

Memoryer usage: every Phase-1 string is a bare
`StringMessageField(order: N)` — `AuthRequest` Token (order 6) and
ClientId (order 9) are null-terminated UTF-8 with no prefix.
Room-name fields carry `maxLength`: 22 (`CreateRoomRequest`), 12
(`SetRoomParamsRequest`), 21 elsewhere — as the earlier forensics
note guessed. Still OPEN on the exe side: whether the 7.55 client
emits the `0x00` terminator per string (its builder appends raw
`strlen` bytes), so live interop remains the final check before any
`Authorize` implementation.

## 4. Packet families (opcodes decimal; Memoryer `Source/Memoryer` Commands.cs)

### 4a. Game TCP — requests C→S

| Range | Opcodes | Family |
|---|---|---|
| 1000 | 1000 Authorize, 1002 GetChannelList, 1004 ChannelLogin, 1007 SessionKeys | planet/login |
| 2000 | 2000 GetCharacterInfo, 2002 GetChannelInfo, 2004 CreateRoom, 2010 GetUserList, 2012 SendMainRoomMessage, 2015 Announce, 2017 SendWhisper, 2021 ChannelLogout, 2024 SendMusicList, 2027 GetMusicPlayRanking, 2030 GetMusicScoreList, 2032 GetMusicMaxScore, 2034 GetFreeMusicStatus | channel/main-room/song meta |
| 3000 | 3000 SetRoomParams, 3002 JoinWaiting, 3005 ExitWaiting, 3008 UpdateSlot, 3011 SendWaitingMessage | waiting room |
| 4000 | 4000 SetRoomMusic, 4002 SetRoomArena, 4004 SetRoomTeam, 4006 SetInstrument, 4008 Ready, 4010 StartGame, 4012 ConfirmMusicLoaded, 4014 UpdateGameStats, 4016 SubmitScore, 4021 ExitPlaying, 4023 SetRoomSkill, 4025 UpdateMusicState, 4027 SetRoomAlbum, 4030 GetMusicList, 4034 AcquireMusicRequest, 4036 SyncMemberMusicState, 4038 SetDownloadProgress, 4041 FinalizeRank, 4042 GetLiveState, 4044 GetP2PList, 4046 ReportUdpPunchHole, 4048 TestNetworkLatency, 4051 StartSelectMusic, 4053 CancelSelectMusic, 4055 ConfirmRelay, 4057 SetRoomSkillEx | playing/sync/live setup |
| 5000 | 5000 SyncItemPurchase, 5002 SellItem, 5004 EquipItem, 5006 EnterShop, 5007 ExitShop, 5010 SyncMusicPurchase, **5012 SyncMusicDownload**, 5013 SyncPoint, 5015 StartPayment, 5017 GetGiftMessageList, 5019 ReadGiftMessage, 5023 ClaimGift, 5025 SyncFreePass, 5028 SyncCashPoint, 5030 ChangeName, 5032 CheckName, 5034 PenaltyReset, 5036 BagExpansion, 5038 RefershBag (sic), 5040 CheckGiftFreePass | shop/cash/gifts |
| 6000 | 6000 LegacyPing, 6001 Ping | keepalive |
| misc | 65520 (`0xFFF0`) Terminate | session end |

### 4b. Game TCP — responses S→C (request+1) and events

Responses: 50 SubscriptionAlert, 51 ForceReauthorize, 52 MultiSessionError,
1001/1003/1005/1008, 2001/2003/2006/2011/2013/2014/2016/2018/2022/2028/2031/2033/2035,
3003/3006/3012/3013, 4031/4043/4045/4047/4056, 5001/5003/5005/5011/5014/5016/5018/5024/5026/5029/5031/5033/5035/5037/5039/5041.
Events: 2005 RoomCreated, 2007 RoomRemoved, 2008 RoomParamsChanged, 2009 RoomUserCountChanged,
2019 ReceiveWhisper, 2020 RoomStateChanged, 2023 RoomParameterChanged, 2025 RoomSkillChanged,
2026 MusicPremiumTimeList, 2029 RankNotification, 3001/3004/3007/3009/3010/3014 waiting-room,
4001/4003/4005/4007/4009/4011 StartGame/4013 MusicLoaded/4015 GameStatsUpdate/4017 ScoreSubmission/
4018 ScoreCompleted/4022/4024/4026 MemberStateChanged/4028/4029/4035/4037/4039/4040/4049/4050/4052/4054/4058,
5027 SyncMemberMusicList.

### 4c. Key payload shapes (Memoryer message fields, order = wire order)

- `1000 Authorize` C→S: u8 `1`, i32 relayKey1/2 (from `user:authorize` RSA params), public+local endpoints, i32 userId, str token, gender, u8 `1`, str clientId.
- `1004 ChannelLogin` C→S: i16 gatewayId, i16 channelId.
- `4002 SetRoomArena` C→S: one little-endian `i32` arena word — the room stage with the picker's high-word flag, kept in global `0x7789dc` (send: `0xFA2` pushed before the vtable `+0x5c` call at files `0xbb600`/`0xbb64d`; receive store `arg → 0x7789dc` at file `0x4ad413`, applier `0x4ad350`). Width follows the dword slot and `JoinedRoom::arena` (`i32`); request send is exe-verified, Memoryer corroboration. S→C `4003` is the same one-word `i32` echo (`RoomArenaEcho` in `crates/o2jam-net/src/play.rs`): handler VA `0x55f3c0` (file `0x15f3c0`, 48 B) reads exactly 4 bytes straight into `0x7789dc` through the packet-reader helper (`0x555d20`; size push at file `0x15f3c3`, dest push at file `0x15f3c5`), then forwards (`0x12`, 4003, stored word, 0) into the UI event queue (`0x478d50`; opcode push at file `0x15f3da`) — no member slot, no result code, no slot guard, so the member-broadcast and result-code shapes are ruled out. Dispatcher edge: subtract base `0xFA3` at file `0x15835f`, bound `0x37`, byte/address tables at files `0x158a68`/`0x1589f4` (`ev_c7b26aac…`, `ev_9412e595…` in `extracted/rea-4015-dossier.json`); index 0 holds entry 0 → case at file `0x158655` → the only image-wide relative call into the handler (site file `0x158659`). Observed with GNU objdump 2.47 slices plus Python 3.14.9 byte scans over the read-only 7.55 copy; no new REA dossier filed. The word fans out to the Playing `_` stage resolver (seed files `0xe78db`–`0xe792e`: high word set → low word, else full value) and the waiting map cycle (low word over the table at file `0x249990`, files `0xb3cc6`–`0xb3ce3`; see `docs/stage-bga.md`). Follow-ups: the `Event`/`lib.rs` wiring (touches `command.rs`/`lib.rs`) and a Ghidra function dossier for VA `0x55f3c0` plus case VA `0x558655`.
- `4012 ConfirmMusicLoaded` C→S: i32 powerSkillId, speed; `4014 UpdateGameStats`: type (`0` = Life, `1` = Jam — enum corroborated by Memoryer, usage verified in-exe)/u16 value/i32 seq/u32 score/i32 longNote — sent mid-song from the score path itself (Jam on Cool/Good tails file `0x16e660`/`0x16e7dd`; Life from tick-driven site file `0x4eaec0`, gated on globals byte `!= 6`); Bad/Miss tails send none in this region. Relay TCP op 5 mirrors it C→S in Live mode; S→C echo is `4015 GameStatsUpdate`: u8 memberId + type + u16 value + 8 rank bytes (peer lifebar + live ranks in one packet; exe handler unlocated); `4016 SubmitScore`: u16 cool/good/bad/miss/maxCombo/jam/maxJam + u32 score + u8 life + speed + i32 longNoteScore (layout corroborated field-for-field by Memoryer).
- `4017 ScoreSubmission` S→C: one `u8` member slot naming the submitting member. Exe-verified handler VA `0x55fba0` (file `0x15fba0`, 58 B): one size-1 read through the packet-reader helper (`0x555d20`; size push at file `0x15fba4`, call at file `0x15fbad`), member guard `< 8` (compare at file `0x15fbb6`, drop past it), then forward (`0x12`, 4017, slot, 0) into the UI event queue (`0x478d50`; pushes at files `0x15fbbd`–`0x15fbc9`, call at file `0x15fbd1`). Dispatcher edge: subtract base `0xFA3` index `0x0E` holds byte-table entry 7 → case at file `0x1586cc` → the only image-wide relative call into the handler (site file `0x1586d0`); Ghidra decompilation of the dispatcher recovers the matching `0xFB1` switch arm. Dossier `extracted/rea-score-handlers-dossier.json` (`ev_9ac44f7d…`, `ev_e6cdff98…`, `ev_67fbdde4…`).
- `4018 ScoreCompleted` S→C: `i32` row count, `u8` room-master slot, then that many 45-byte rows. Exe-verified handler VA `0x55fbe0` (file `0x15fbe0`, 797 B): count size-4 read (push file `0x15fc28`, call `0x15fc31`) plus master size-1 read (push file `0x15fc36`, call `0x15fc3f`); per-row member size-1 (push file `0x15fd34`, call `0x15fd3d`) plus active size-4 (push file `0x15fd42`, call `0x15fd4b`), guard member `< 8` (compare file `0x15fd54`) with a scratch-drain path that still consumes the row (row-reader call file `0x15fd75`), admit only active `== 1` (compare file `0x15fd8e`). The row reader VA `0x5411a0` (file `0x1411a0`, 316 B) then performs the remaining 15 sized reads in fixed wire order — six u16 (slot + `0x2142`/`44`/`46`/`48`/`5C`/`4C`), u32 score via a local into slot + `0x2160`, u16 reward (+`0x2164`), two u32 (+`0x20`/`0x24`), u8 result (+`0x2166`), two u32 (+`0x2C`/`0x2128`), u8 speed (+`0x2168`), u32 long-note (+`0x2158`) — committing into the member slot (`0x77ee90` + member × `0x2190`, self base `0x77cd00`, self arm copies `0x2190` bytes into the table slot). Tail counts the 8 place flags at `0x778a1e` into global `0x7789fc` and forwards (`0x12`, 4018, 0, 0). Dispatcher edge: index `0x0F` holds entry 8 → case file `0x1586dd` → single caller (site file `0x1586e1`). Dossier (`ev_476a9085…`, `ev_e5954df4…`, `ev_cbd5ca48…`, `ev_6902875a…`, `ev_5308dcbd…`, `ev_d19ae823…`; open items `ev_bab8aa66…`, closed below except the global labels). Closed since: no count clamp — the only `push 8` is the flag-array memset length (file `0x15fc17`), the only compare against 8 is the member guard (file `0x15fd54`), and all three loop exits test the index against the count word (files `0x15fc80`/`0x15fce2`/`0x15fd28`, signed `jge`, so negative counts trip zero rows); the decoder 8-cap was Memoryer `maxCount: 8`, never exe behavior, and now accepts any funded count. Slot `+0x30` is the room-master marker — the head splits on master == local id (`0x77cd04`, files `0x55fc5b`/`0x55fcc1`): self `+0x30 = 1` (writer VA `0x5400b0`, call file `0x15fc66`) with table-mirror clears (`0x77eec0` read file `0x15fc8e`, call file `0x15fcb6`) when we are master, else self `+0x30 = 0` (file `0x15fcc8`) with clears (file `0x15fcf8`) plus table master slot `+0x30 = 1` (files `0x15fd05`–`0x15fd11`); the row reader never writes `+0x30`. Still OPEN: gameplay labels for the three self-arm globals — sources pinned (`0x768b68` ← self `+0x24`, files `0x15fe01`–`0x15fe13`; `0x768b44` += word `[0x77ee64]`, files `0x15fe19`–`0x15fe33`; `0x768b70` ← `[0x77ee74]`, files `0x15fe39`–`0x15fe4b`; same triple in the sibling handler at files `0x160060`/`0x160081`/`0x160098`; bulk loader VA `0x55a350`, case file `0x158424`, reads eight dwords into `0x768b44`–`0x768b70` plus a byte into `0x768b6c`). Observed with GNU objdump 2.47 slices plus Python 3.14.9 byte scans over the read-only 7.55 copy; no new REA dossier filed.
- `4040 GameRankUpdate` S→C: eight rank bytes, byte `i` holding the member id on leaderboard place `i`. Exe-verified handler VA `0x559a60` (file `0x159a60`, 227 B): an outer 8-loop reading one byte per pass (size push file `0x159a82`, call file `0x159a8b`) plus an inner 8-seat scan delivering each byte through the notify writer (`0x5414a0`, one store at slot + `0x217d`) — the self seat when the gate byte at `0x77cd05` is set and the local id byte at `0x77cd04` matches, else the table seat whose rank byte at slot + 4 matches — then forward (`0x12`, 4040, 0, 0) (call file `0x159b3c`). Each place lands in its seat at slot `+0x217d` via the notify writer (file `0x1414ad`; self arm file `0x159acf`, table arm file `0x159b1a`), and the slot-reset routine parks `0xff` there (file `0x13fccb`), matching the wire empty-place pad — so the stored verdict is the leaderboard place index. Dossier (`ev_4684b913…`, `ev_ecb961cc…`). Still OPEN: gameplay use of the places (result sort/announce) past the stored index.
- `5012 SyncMusicDownload` C→S: u16 musicId = FTP-download-complete notice; server marks installed and broadcasts `4026 MemberStateChanged`. Actual bytes move over FTP, never game TCP.
- `4038 SetDownloadProgress`: u8 progress + u8 memberId; `4034 AcquireMusicRequest`: u8-counted memberId list.
- `3011 SendWaitingMessage` C→S: raw chat bytes fill the whole body (no prefix, no terminator; empty body = empty line). `3012 WaitingUserMessage` / `3013 WaitingAdminMessage` S→C share one layout: null-terminated UTF-8 sender + raw content bytes + trailing u8 seat id; the content length rides out-of-band (`body.len - sender_used - 1`). Memoryer shapes: `Messages/Requests/Messaging/SendWaitingMessageRequest.cs` (one `CollectionMessageField` byte array), `Messages/Responses/Messaging/Waiting{User,Admin}MessageResponse.cs` (string sender, byte-array content, byte seat id), seat filled from the room slot list (`Controllers/MessagingController.cs`). Encore 1.0.0 `DefaultMessageCodec` encode vectors (opcode bytes included): `abc` → `C3 0B 61 62 63`, empty → bare `C3 0B`; user (`ab`, `abc`, seat 5) → `C4 0B 61 62 00 61 62 63 05`; admin (`ab`, `a`, seat 7) → `C5 0B 61 62 00 61 07`; empty user → `C4 0B 00 00`. Caveat: Encore's own decoder throws `ArgumentOutOfRangeException` on the 3012/3013 vectors (middle-collection plus trailing field), so the encode side is the verified contract and `o2jam-net::rooms` inverts it. Exe side: five send sites push opcode 3011 with a zero word then send indirectly (files `0xb9385`, `0xb95b4`, `0xb9742`, `0xe1aa6`, `0xe1ff7`; `ev_130b4726ca4f7075f8526a05fda5d876`); a receive cluster pushes 3012 (ten sites from file `0x15e601`) and 3013 (three sites from file `0x15f051`) beside the ControlList parent global `0x762B44` (`ev_aba36a3aa42dfa2b7bde94cae4e28a45`). The chat view (`EDIT_SWAIT_CHAT 0x16300014`, one ref at file `0xbaf1a`; `LIST_SWAIT_CHAT 0x1680002B`, five refs from file `0xb8e11`; `ev_f8422c8a059247042312e3d7224dbdff`) stages art, but row template `TEXT_SWAIT_CHAT 0x1640003C` has zero immediate refs — the exact row-feed call stays OPEN.

- `2012 SendMainRoomMessage` C→S: raw chat bytes fill the whole body (no prefix, no terminator; empty body = empty line) — the same layout as 3011. Memoryer (`Messages/Requests/Messaging/SendMainRoomMessageRequest.cs`) carries one `CollectionMessageField(order: 0)` byte array; the server re-broadcasts as 2013 (players) or 2014 (admins) via `Controllers/MessagingController.cs`. Exe send site: file `0x8fa1e` (`push 0; push 0x7dc`, indirect send call) in the lobby slash-command parser — `/w`, `/r`, `/h`, `/help` literals at VAs `0x647d60`–`0x647d34` select the 2017 whisper send (file `0x8fa02`) versus this channel send. `2013 MainRoomUserMessage` / `2014 MainRoomAdminMessage` S→C share one layout: null-terminated UTF-8 sender + raw content bytes to end of body (no seat id, unlike 3012/3013). Memoryer shapes: `Messages/Responses/Messaging/MainRoom{User,Admin}MessageResponse.cs` (string sender, trailing byte-array content). Exe receive handler for 2013 (file `0x15bfa0`) reads sender + content through the packet-reader helper, formats `[%12s] %s` (VA `0x64d8d4`), and feeds the ControlList parent global `0x762B44` (opcode push at file `0x15bff5`); three 2014 sites push `0x7de` beside the same parent (files `0x15c075`, `0x15c11a`, `0x15c151`). Observed with GNU objdump 2.47 slices plus Python 3.14.9 byte scans over the read-only 7.55 copy; no new REA dossier filed. Memoryer shapes are corroboration until per-field exe builder proof lands. `o2jam-net::rooms`: `SendMainRoomMessage`, `MainRoom{User,Admin}Message` (+ shared `MainRoomChatLine`).
- `2017 SendWhisper` C→S: null-terminated recipient + null-terminated content. Memoryer (`Messages/Requests/Messaging/SendWhisperMessageRequest.cs`): string recipient (order 0), string content (order 1); the server 2019-events the named recipient and answers 2018. Exe send sites (`push 0; push 0x7e1`, indirect send call): files `0x8f757`/`0x8fa02` (lobby `/w` branch), `0xb8ffc`/`0xb9369` (waiting room), `0xe1a85`/`0xe1fd3` (beside the 3011 room-chat sends). `2018 SendWhisper` S→C: `i32` invalid word (non-zero = undelivered) + null-terminated recipient + trailing-nullable content (absent when undelivered). Memoryer (`Messages/Responses/Messaging/SendWhisperResponse.cs`): bool `Invalid` as `MessageFieldCodec<int>` (order 0), string recipient (order 1), `string?` content (order 2); exe receive site at file `0x15c20b` (ControlList parent `0x762B44`). `2019 ReceiveWhisper` S→C event: null-terminated sender + null-terminated content (empty content still writes its terminator). Memoryer (`Messages/Events/Common/WhisperEventData.cs`): string sender (order 0), string content (order 1, default empty); exe receive site at file `0x15c5b0` (same ControlList feed). Observed with GNU objdump 2.47 slices plus Python 3.14.9 byte scans over the read-only 7.55 copy; no new REA dossier filed. Memoryer shapes are corroboration until per-field exe builder proof lands. `o2jam-net::rooms`: `SendWhisper`, `SendWhisperResponse`, `ReceiveWhisper`.

- `2015 Announce` C→S + `2016 Announcement` S→C + `2029 RankNotification` S→C event: the lobby notice triple. Announce/Announcement carry one null-terminated UTF-8 notice string, no prefix. Memoryer (`Messages/Requests|Responses/Messaging/Announcement{Request,Response}.cs`): one `StringMessageField(order: 0)`; the server fans 2015 out as 2016 to every channel (`Controllers/MessagingController.cs`). Exe sites: two 2015 sends (files `0x8f585`/`0x8f5dc`, `push 0; push 0x7df`, indirect `+0x5c` send call) on the lobby `/n ` branch (prefix VA `0x647d68`, behind the nonzero gate on byte global `0x768b6c` at file `0x8f53e`); the S→C dispatch subtracts base `0x7d3` (file `0x1582ec`), bounds `0x20` (file `0x1582f8`), and indexes the address table at file `0x158940` — 2015 (index `0x0c`) falls into the default case (file `0x158937`, no client handler), while 2016 (index `0x0d`) lands case file `0x1584df`, which calls handler VA `0x55c0a0` (file `0x15c0a0`). That handler reads exactly one string through the string-reader helper (`0x555ea0`) into a `0x408`-byte stack buffer, then feeds the ControlList parent global `0x762B44`: a count-`0xa` redraw via `0x4793a0`, else the text fans out as `0x7de` admin-message rows `(2, text)`/`(5, text)` via `0x479010` (files `0x15c11a`/`0x15c151`). `RankNotification`: `i32` invalid word (nonzero = drop) + `i32` music id + null-terminated nickname + `i32` ranking. Memoryer (`Messages/Events/MainRoom/RankNotificationEventData.cs`): bool `Invalid` as `MessageFieldCodec<int>` (order 0), music id (order 1), nickname (order 2), ranking (order 4 — order 3 dropped upstream). Exe: same dispatch, index `0x1a` → case file `0x158556` → handler VA `0x55b4d0` (file `0x15b4d0`), which reads the invalid word (`0x555d20`), drops the packet when nonzero (`cmp` file `0x15b527`), stages music/nickname/ranking in `.data` slots `0x77969c`/`0x7796a8`/`0x7796a0`, and forwards `(0, 0, 0x7ed, 0x12)` into the ControlList UI queue (`0x478d50`, file `0x15b575`). Observed with GNU objdump 2.47 slices plus Python byte scans over the read-only 7.55 copy; no new REA dossier filed. The one-string 2016 read byte-verifies the announce shape; the 2029 invalid gate matches the exe `cmp`/`jne`, and its field order is Memoryer corroboration against an exe-verified read sequence. OPEN: practical notice cap implied by the `0x408` stack buffer (Memoryer sets none), non-ASCII notice interop. Live capture 2026-11-08 (local Memoryer Full-mode, logging TCP proxy `127.0.0.1:15011` -> `15010` per `docs/relay.md` §12; authorize tester -> 1002/1004/2000/2002, then 2015 `LIVE-PROBE-2016 hello`): channel enter yields no 2016/2029 (1005/2001/2026/2003 only); the 2015 send frames as `1a 00 df 07` + 22-B NUL-terminated body and echoes as S->C 2016 `1a 00 e0 07` + the identical body, matching the one-string decoder exactly with no decoder change (`announcement_decodes_live_capture` in `rooms.rs`). 2029 untriggered live: Memoryer has no broadcast site (the `FinalizeRank` `GameRankUpdate` emit stays commented out) and 2027->2028 / 2030->2031 / score probes returned no 2029, so the music-id / nickname / ranking byte positions stay Memoryer-corroborated (`i32` invalid + `i32` music + NUL name + `i32` rank) against the exe-verified read sequence. `o2jam-net::rooms`: `Announce`, `Announcement`, `RankNotification`.

### 4d. Relay TCP (Live Mode) — `RelayCommand`, matched in-exe

| Op | Name | Dir | In-exe anchor |
|---|---|---|---|
| 1 | PeerConnected | S→C | `SetID(1)` ctor file `0x18e100`, vtable ref `0x24e2ec` |
| 2 | PeerConfirm | C→S | `SetID(2)` ctor file `0x18ca60`, ref `0x24e27c`; payload: i32 code, 3 endpoints, u16, i32 key1/2, local+public `ConnectionEndpoint` (ports+IP) |
| 3 | PeerEndpointAssigned | S→C | `SetID(3)` ctor file `0x18e1e0`, ref `0x24e2fc` |
| 4 | Ping | C→S | `SetID(4)` ctor file `0x18cc50`, ref `0x24e28c`; payload: code, 3 endpoints, u16, u32 start/tick |
| 5 | UpdateGameStats | C→S | `SetID(5)` ctor file `0xea4e0`, ref `0x24b334` |
| 6 | ComboBroken (`CO2JamMissPack`) | C→S | `SetID(6)` ctor file `0x16dfe0`, ref `0x24dc1c`; RTTI `CO2JamMissPack` |
| 7 | ComboStarted (`CO2JamCoolPack`) | C→S | `SetID(7)` ctor file `0x16e080`, ref `0x24dc2c`; RTTI `CO2JamCoolPack` |
| 10/11 | CreateSession/DeleteSession | C→S | u8-counted member list of (key1,key2) pairs |

UDP relay datagram: `[u16 uncompressedLen][zlib([u8 type][u8 seq][u16 innerSize][u32 cmd][body])]`;
type ∈ {1 Unreliable, 2 Reliable, 3 Ack}; Ack = 4 bytes `[0x03][seq][0x00][0x00]`.
UDP is primary for Live, TCP fallback (freeze-prone at game start).

## 5. Offline-playable verdict

Plays offline (no server): chart load/decrypt (`new`-XOR + `ojn` slices), OJM decode,
`o2jam-game` judgement/scoring, `o2jam-shell` window/input, `play --chart/--ojm/--sound`,
`OJNList.dat` song index, `option.ini` display settings.
Needs a server: login/auth (`OTwo.exe` RSA params + gateway 15010), planet/channel/room
lifecycle, any multiplayer incl. Live relay + UHP, `SubmitScore`/rankings,
shop/cash/gifts, FTP song fetch + `5012` sync, all `game1.o2jam.com`/`o2jam.*` web flows.

## 6. Unknowns (not claimed)

- Exact length-prefix semantics of the TCP frame beyond id-at-offset-2; gateway↔channel
  port `15047` is server-side only.
- Per-opcode handler coverage inside the exe (dispatch is virtualized; only relay
  `SetID(1..7)` ctors pinned above).
- `Terminate 65520` trigger; `LegacyPing 6000` vs `Ping 6001` usage split.
- UDP game ports (only TCP `15010` pinned in-exe; relay ports come from server).
