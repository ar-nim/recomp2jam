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
Send-fail logging tags each frame with `sendPack` id (`0x401cfe`).
Direction convention (Memoryer): odd/even pairs — request C→S `N`,
response S→C `N+1`; events S→C broadcasts.

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
- `4012 ConfirmMusicLoaded` C→S: i32 powerSkillId, speed; `4014 UpdateGameStats`: type/u16 value/i32 seq/u32 score/i32 longNote; `4016 SubmitScore`: u16 cool/good/bad/miss/maxCombo/jam/maxJam + u32 score + u8 life + speed + i32 longNoteScore.
- `5012 SyncMusicDownload` C→S: u16 musicId = FTP-download-complete notice; server marks installed and broadcasts `4026 MemberStateChanged`. Actual bytes move over FTP, never game TCP.
- `4038 SetDownloadProgress`: u8 progress + u8 memberId; `4034 AcquireMusicRequest`: u8-counted memberId list.

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
