# Client→Memoryer server — build plan

End goal: `recomp2jam` connects to a Mozart.Encore **Memoryer**
server (v7.55 O2Jam Classic) and behaves exactly like the original
client. Memoryer is the wire-contract reference (clean-room C#,
no copyrighted bytes); the exe is the behavior oracle. Every
client behavior must be exe-verified before implementation;
Memoryer-only claims stay labeled corroboration.

Sources: `docs/net-protocol.md` (opcodes/endpoints/framing),
`docs/gameplay.md` (judgement/score/jam/life), Memoryer
`Source/Memoryer` message layouts (126 `IMessage` types).

## Verified client contract so far (exe)

- Framing: length-framed `CPacket_Str`, little-endian `u16`
  command id at buffer offset 2 (`SetID`/`GetID` file
  `0x1a0550`/`0x1a05d0`). TCP gateway port `15010`.
- Judgement: windows 6/18/25 ms inclusive; flags `0x10/0x20/
  0x40/0x80`; Cool/Good continue combo, Bad/Miss break.
- Score: `Cool 200+10·jam`, `Good 100+5·jam`, Bad-pill→Cool
  path, Bad-no-pill `+4`, Miss `−10` floored.
- Combo: `+0x3972` +1 Cool/Good, reset Bad+Miss, max `+0x3974`.
- Jam: progress `+0x396c` +2 Cool / +1 Good; level `+0x396e`
  (X) +1 when progress beats table `+0xb66[i]=(i+1)*50`
  (400 entries, all difficulties); Bad/Miss reset progress
  and X; X snapshot likely maxJam (`0x5407f0`).
- Pills: cap 5, 15-Cool streak grants, Bad spends 1 to keep
  combo, mode 6 exempt.
- Net during play: `UpdateGameStats` 4014 type 0 Life
  (tick-driven, `0x4eaec0`) / type 1 Jam (per-hit Cool/Good
  tails); `SubmitScore` 4016 layout field-for-field
  (cool/good/bad/miss/maxCombo/jam/maxJam u16, score u32,
  life u8, speed, longNoteScore i32).
- S→C 4015 `GameStatsUpdate`: u8 memberId + type + u16 value
  + 8 rank bytes (peer lifebar + live ranks).
- Life server model (Memoryer, exe handler open): 0–1000,
  dead-freeze at 0, `Clear = Life > 0`.
- Live mode: relay TCP op 5 stats mirror, ops 6/7 combo packs,
  UDP zlib RUDP frames, UHP hole punching (optional phase).

## What Memoryer gives us to build next (client-side work)

All request/response/event layouts live in
`Source/Memoryer/Memoryer/Messages` (`MessageField(order: N)`
= wire order). Minimum viable online session, in order:

### Phase 1 — login + channel (offline-capable shell exists)
- TCP connect to Memoryer gateway (`15010`), framing codec.
- `Authorize` 1000 C→S payload: u8 `1`, relayKey1/2 i32,
  public+local endpoints, i32 userId, str token, gender,
  u8 `1`, str clientId (exe strings + Memoryer
  `AuthorizeRequest`; RSA `user:authorize` params — see
  lifecycle doc). Handle 1001/1003/1005/1008, 50/51/52.
- BLOCKED on string wire format (exe forensics so far):
  in-memory strings are null-terminated (`strlen`-equivalent
  at file `0x15f1bc0`, alignment-optimized scan), packet
  builder appends counted bytes (`0x555d70`/`0x5f2450`);
  room strings carry `maxLength` 22/12 (fixed buffers?),
  auth strings carry none (variable-length?); prefix size
  and charset (presumed CP949 for non-ASCII) unconfirmed.
  Candidate sites: 1000 send wrapper file `0x147451c`,
  CreateRoom builder file `0x1436462` (`SetID` 2004 inline).
  No `Authorize` implementation until the format is pinned —
  guessing the codec would break interop silently.
- `GetChannelList` 1002 → 1003; `ChannelLogin` 1004 (i16
  gatewayId/channelId) → 1005; `SessionKeys` 1007/1008.
- `GetCharacterInfo` 2000 → 2001 (equipment codec:
  `CharacterEquipmentInfoCodec`).

### Phase 2 — main room / channel UI
- Room list (2003), user list (2010/2011), chat
  (2012/2013/2014), whisper (2017/2018/2019), announcements.
- `CreateRoom` 2004 → 2006; room events 2005/2007/2008/2009/
  2020/2023/2025.
- Music metadata: OJNList already parsed locally; ranking
  queries 2027–2035 optional for parity.

### Phase 3 — waiting room
- `JoinWaiting` 3002/3003, `ExitWaiting` 3005/3006,
  `UpdateSlot` 3008 → `RoomSlotUpdate` 3009, kick 3010,
  chat 3011/3012/3013.
- Room params/music/arena/team/instrument/ready:
  3000/3001, 4000/4001, 4002/4003, 4004/4005, 4006/4007,
  4008/4009; skills 4023/4024 + 4057/4058 (ring effects
  already implemented offline).
- Song sync: `GetMusicList` 4030/4031, `UpdateMusicState`
  4025/4026, `AcquireMusicRequest` 4034/4035,
  `SyncMemberMusicState` 4036/4037, `SetDownloadProgress`
  4038/4039, `SyncMusicDownload` 5012 (FTP transfer itself
  out of scope initially; server marks installed on 5012).
- `StartSelectMusic`/`CancelSelectMusic` 4051–4054.

### Phase 4 — play + score loop
- `StartGame` 4010/4011; `ConfirmMusicLoaded` 4012 (i32
  powerSkillId, speed) → `MusicLoaded` 4013; tick sync
  (latency test 4048/4049, `RoomMembersLatencySynced` 4050).
- In-play sends: 4014 Life (tick) + Jam (per Cool/Good);
  consume 4015 for peer bars/ranks (client handler OPEN in
  exe — implement per Memoryer layout, verify against exe
  later).
- `SubmitScore` 4016 → 4017/4018; `FinalizeRank` 4041 →
  4040 GameRankUpdate; `ExitPlaying` 4021/4022.
- Result screen: cool/good/bad/miss/maxCombo/maxJam/score/
  life/clear already computable locally.

### Phase 5 — optional parity
- Live mode: relay TCP ops 1–7 + UDP RUDP + UHP (relay
  endpoint discovery via `GetLiveState` 4042/4043,
  `GetP2PList` 4044/4045, `ReportUdpPunchHole` 4046/4047,
  `ConfirmRelay` 4055/4056).
- Shop/my-room: 5000–5041 family (items, gifts, cash, name
  change) — UI-heavy, low priority.
- Launcher patching (NowSmartLauncher) stays out of scope.

## Engineering notes

- New crate `o2jam-net` (async TCP, framing, message
  encode/decode generated from a single table mirroring
  Memoryer field orders); no copyrighted bytes; synthetic
  packet fixtures only.
- Offline play keeps working: net layer is additive behind
  the existing lifecycle states.
- `cargo fmt`/`clippy -D warnings`/`cargo test` green per
  commit; behavior changes need exe anchors in commit bodies.
