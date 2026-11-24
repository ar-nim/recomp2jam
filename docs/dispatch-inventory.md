# Dispatch inventory — opcode to codec or OPEN (7.55)

Version pin: O2Jam 7.55 `AngelO2Jam.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

This is the completeness companion to `docs/net-protocol.md` §4 (that file is
owned by another lane and is not edited here). The id set is the union of
§4a/§4b/§4d opcode lists and the live dispatch surface: the wire enums in
`command.rs` and `relay/commands.rs`, `Generic`, every `WireMessage::COMMAND`
constant and empty-message macro across `crates/o2jam-net/src`, and the
session's named command constants. `crates/o2jam-net/tests/dispatch_inventory.rs`
fails when a dispatched id is missing from this table, when a row maps to
neither a codec nor an OPEN label, or when evidence is absent. No install
data is needed to run it.

A codec cell names the `o2jam-net` type whose `WireMessage::COMMAND` or
`EmptyMessage::COMMAND` equals the id. An `OPEN` cell states what is unknown
and the next evidence step. Existing per-field evidence stays in
`docs/net-protocol.md`; this table tracks coverage, not a second copy of the
wire evidence.

| id | plane | direction | role | codec / status | evidence |
|---:|---|---|---|---|---|
| 1 | relay TCP | both / unknown | PeerConnected | `o2jam-net::PeerConnected` | `docs/net-protocol.md` §4d; `o2jam-net::relay::commands::RelayCommand::PeerConnected` |
| 2 | relay TCP | both / unknown | PeerConfirm | `o2jam-net::PeerConfirm` | `docs/net-protocol.md` §4d; `o2jam-net::relay::commands::RelayCommand::PeerConfirm` |
| 3 | relay TCP | both / unknown | PeerEndpointAssigned | `o2jam-net::PeerEndpointAssigned` | `docs/net-protocol.md` §4d; `o2jam-net::relay::commands::RelayCommand::PeerEndpointAssigned` |
| 4 | relay TCP | both / unknown | Ping | `o2jam-net::RelayPing` | `docs/net-protocol.md` §4d; `o2jam-net::relay::commands::RelayCommand::Ping` |
| 5 | relay TCP | both / unknown | UpdateGameStats | `o2jam-net::RelayStats` | `docs/net-protocol.md` §4d; `o2jam-net::relay::commands::RelayCommand::UpdateGameStats` |
| 6 | relay TCP | both / unknown | ComboBroken | `o2jam-net::ComboBroken` | `docs/net-protocol.md` §4d; `o2jam-net::relay::commands::RelayCommand::ComboBroken` |
| 7 | relay TCP | both / unknown | ComboStarted | `o2jam-net::ComboStarted` | `docs/net-protocol.md` §4d; `o2jam-net::relay::commands::RelayCommand::ComboStarted` |
| 10 | relay TCP | both / unknown | CreateSession | `o2jam-net::CreateRelaySession` | `docs/net-protocol.md` §4d; `o2jam-net::relay::commands::RelayCommand::CreateSession` |
| 11 | relay TCP | both / unknown | DeleteSession | `o2jam-net::DeleteRelaySession` | `docs/net-protocol.md` §4d; `o2jam-net::relay::commands::RelayCommand::DeleteSession` |
| 50 | game TCP | S→C | SubscriptionAlert | `o2jam-net::SubscriptionAlert` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::SubscriptionAlert` |
| 51 | game TCP | S→C | ForceReauthorize | `o2jam-net::ForceReauthorize` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::ForceReauthorize` |
| 52 | game TCP | S→C | MultiSessionError | `o2jam-net::MultiSessionError` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::MultiSessionError` |
| 1000 | game TCP | C→S | Authorize | `o2jam-net::AuthorizeRequest` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::Authorize` |
| 1001 | game TCP | S→C | Authorize | `o2jam-net::AuthResponse` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::Authorize` |
| 1002 | game TCP | C→S | GetChannelList | `o2jam-net::GetChannelList` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::GetChannelList` |
| 1003 | game TCP | S→C | GetChannelList | `o2jam-net::ChannelList` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::GetChannelList` |
| 1004 | game TCP | C→S | ChannelLogin | `o2jam-net::ChannelLogin` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::ChannelLogin` |
| 1005 | game TCP | S→C | ChannelLogin | `o2jam-net::ChannelLoginResponse` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::ChannelLogin` |
| 1007 | game TCP | C→S | SessionKeys | `o2jam-net::SessionKeysRequest` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SessionKeys` |
| 1008 | game TCP | S→C | SessionKeys | `o2jam-net::SessionKeys` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::SessionKeys` |
| 2000 | game TCP | C→S | GetCharacterInfo | `o2jam-net::GetCharacterInfo` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::GetCharacterInfo` |
| 2001 | game TCP | S→C | GetCharacterInfo | `o2jam-net::CharacterInfo` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::GetCharacterInfo` |
| 2002 | game TCP | C→S | GetChannelInfo | `o2jam-net::GetChannelInfo` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::GetChannelInfo` |
| 2003 | game TCP | S→C | RoomList | `o2jam-net::RoomList` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::RoomList` |
| 2004 | game TCP | C→S | CreateRoom | `o2jam-net::CreateRoom` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::CreateRoom` |
| 2005 | game TCP | S→C | RoomCreated | `o2jam-net::RoomCreated` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::RoomCreated` |
| 2006 | game TCP | S→C | CreateRoom | `o2jam-net::CreateRoomResponse` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::CreateRoom` |
| 2007 | game TCP | S→C | RoomRemoved | `o2jam-net::RoomRemoved` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::RoomRemoved` |
| 2008 | game TCP | S→C | RoomParamsChanged | OPEN: body layout of RoomParamsChanged; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 2009 | game TCP | S→C | RoomUserCountChanged | OPEN: body layout of RoomUserCountChanged; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 2010 | game TCP | C→S | GetUserList | `o2jam-net::GetUserList` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::GetUserList` |
| 2011 | game TCP | S→C | GetUserList | OPEN: wire body; next: capture or rewrite a frame on the local gateway | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::GetUserList` |
| 2012 | game TCP | C→S | SendMainRoomMessage | `o2jam-net::SendMainRoomMessage` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SendMainRoomMessage` |
| 2013 | game TCP | S→C | MainRoomUserMessage | `o2jam-net::MainRoomUserMessage` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::MainRoomUserMessage` |
| 2014 | game TCP | S→C | MainRoomAdminMessage | `o2jam-net::MainRoomAdminMessage` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::MainRoomAdminMessage` |
| 2015 | game TCP | C→S | Announce | `o2jam-net::Announce` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::Announce` |
| 2016 | game TCP | S→C | Announcement | `o2jam-net::Announcement` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::Announcement` |
| 2017 | game TCP | C→S | SendWhisper | `o2jam-net::SendWhisper` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SendWhisper` |
| 2018 | game TCP | S→C | SendWhisper | `o2jam-net::SendWhisperResponse` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::SendWhisper` |
| 2019 | game TCP | S→C | ReceiveWhisper | `o2jam-net::ReceiveWhisper` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::ReceiveWhisper` |
| 2020 | game TCP | S→C | RoomStateChanged | OPEN: body layout of RoomStateChanged; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 2021 | game TCP | C→S | ChannelLogout | OPEN: result decoder for the session drain; next: rewrite 2022 body on the local gateway | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::ChannelLogout` |
| 2022 | game TCP | S→C | ChannelLogout | OPEN: result decoder for the session drain; next: rewrite 2022 body on the local gateway | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::ChannelLogout` |
| 2023 | game TCP | S→C | RoomParameterChanged | OPEN: body layout of RoomParameterChanged; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 2024 | game TCP | C→S | SendMusicList | `o2jam-net::SendMusicList` | `docs/net-protocol.md` §4a; codec at `crates/o2jam-net/src/message.rs:132` |
| 2025 | game TCP | S→C | RoomSkillChanged | OPEN: body layout of RoomSkillChanged; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 2026 | game TCP | S→C | MusicPremiumTimeList | OPEN: body layout of MusicPremiumTimeList; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 2027 | game TCP | C→S | GetMusicPlayRanking | `o2jam-net::GetMusicPlayRanking` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::GetMusicPlayRanking` |
| 2028 | game TCP | S→C | GetMusicPlayRanking | `o2jam-net::MusicPlayRankingAnswer` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::GetMusicPlayRanking` |
| 2029 | game TCP | S→C | RankNotification | `o2jam-net::RankNotification` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::RankNotification` |
| 2030 | game TCP | C→S | GetMusicScoreList | `o2jam-net::GetMusicScoreList` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::GetMusicScoreList` |
| 2031 | game TCP | S→C | GetMusicScoreList | `o2jam-net::MusicScoreListAnswer` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::GetMusicScoreList` |
| 2032 | game TCP | C→S | GetMusicMaxScore | `o2jam-net::GetMusicMaxScore` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::GetMusicMaxScore` |
| 2033 | game TCP | S→C | GetMusicMaxScore | `o2jam-net::MusicMaxScoreAnswer` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::GetMusicMaxScore` |
| 2034 | game TCP | C→S | GetFreeMusicStatus | OPEN: request body and answer; next: tap the free-music status leg | `docs/net-protocol.md` §4a |
| 2035 | game TCP | S→C | FreeMusicStatusAnswer | OPEN: body layout of FreeMusicStatusAnswer; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 3000 | game TCP | C→S | SetRoomParams | OPEN: room-parameter body; next: read the exe builder behind the room-settings controls | `docs/net-protocol.md` §4a |
| 3001 | game TCP | S→C | KeyModeChanged | `o2jam-net::KeyModeChanged` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::KeyModeChanged` |
| 3002 | game TCP | C→S | JoinWaiting | `o2jam-net::JoinRoom` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::JoinWaiting` |
| 3003 | game TCP | S→C | JoinRoom | `o2jam-net::JoinRoomResponse` | `docs/net-protocol.md` §4b; `o2jam-net::command::Response::JoinRoom` |
| 3004 | game TCP | S→C | UserJoinWaiting | `o2jam-net::UserJoinedWaiting` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::UserJoinWaiting` |
| 3005 | game TCP | C→S | ExitWaiting | `o2jam-net::ExitWaiting` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::ExitWaiting` |
| 3006 | game TCP | S→C | ExitWaitingAnswer | OPEN: body layout of ExitWaitingAnswer; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 3007 | game TCP | S→C | UserLeaveWaiting | `o2jam-net::UserLeftWaiting` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::UserLeaveWaiting` |
| 3008 | game TCP | C→S | UpdateSlot | OPEN: slot-update body; next: winedbg stopped-state read of the exe send builder | `docs/net-protocol.md` §4a |
| 3009 | game TCP | S→C | RoomSlotUpdate | `o2jam-net::RoomSlotUpdate` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::RoomSlotUpdate` |
| 3010 | game TCP | S→C | WaitingRoomEvent | OPEN: body layout of WaitingRoomEvent; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 3011 | game TCP | C→S | SendWaitingMessage | `o2jam-net::SendWaitingMessage` | `docs/net-protocol.md` §4a |
| 3012 | game TCP | S→C | WaitingUserMessage | `o2jam-net::WaitingUserMessage` | `docs/net-protocol.md` §4b; `o2jam-net` codec `WaitingUserMessage` |
| 3013 | game TCP | S→C | WaitingAdminMessage | `o2jam-net::WaitingAdminMessage` | `docs/net-protocol.md` §4b; `o2jam-net` codec `WaitingAdminMessage` |
| 3014 | game TCP | S→C | WaitingRoomEvent | OPEN: body layout of WaitingRoomEvent; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4000 | game TCP | C→S | SetRoomMusic | `o2jam-net::SetRoomMusic` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SetRoomMusic` |
| 4001 | game TCP | S→C | WaitingMusicChanged | `o2jam-net::WaitingMusicChanged` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::WaitingMusicChanged` |
| 4002 | game TCP | C→S | SetRoomArena | `o2jam-net::SetRoomArena` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SetRoomArena` |
| 4003 | game TCP | S→C | RoomArenaEcho | `o2jam-net::RoomArenaEcho` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::RoomArenaEcho` |
| 4004 | game TCP | C→S | SetRoomTeam | `o2jam-net::SetRoomTeam` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SetRoomTeam` |
| 4005 | game TCP | S→C | SetRoomMusicEcho | OPEN: body layout of SetRoomMusicEcho; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4006 | game TCP | C→S | SetInstrument | OPEN: instrument body; next: proxy rewrite of the 4006 frame | `docs/net-protocol.md` §4a |
| 4007 | game TCP | S→C | RoomTeamChanged | `o2jam-net::RoomTeamChanged` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::RoomTeamChanged` |
| 4008 | game TCP | C→S | Ready | OPEN: wire body; next: capture or rewrite a frame on the local gateway | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::Ready` |
| 4009 | game TCP | S→C | UserReadyStateChanged | `o2jam-net::MemberReadyStateChanged` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::UserReadyStateChanged` |
| 4010 | game TCP | C→S | StartGame | OPEN: wire body; next: capture or rewrite a frame on the local gateway | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::StartGame` |
| 4011 | game TCP | S→C | StartGameAnswer | OPEN: body layout of StartGameAnswer; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4012 | game TCP | C→S | ConfirmMusicLoaded | `o2jam-net::ConfirmMusicLoaded` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::ConfirmMusicLoaded` |
| 4013 | game TCP | S→C | MusicLoaded | OPEN: body layout of MusicLoaded; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4014 | game TCP | C→S | UpdateGameStats | `o2jam-net::UpdateGameStats` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::UpdateGameStats` |
| 4015 | game TCP | S→C | GameStatsUpdate | `o2jam-net::GameStatsUpdate` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::GameStatsUpdate` |
| 4016 | game TCP | C→S | SubmitScore | `o2jam-net::SubmitScore` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SubmitScore` |
| 4017 | game TCP | S→C | ScoreSubmission | `o2jam-net::ScoreSubmission` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::ScoreSubmission` |
| 4018 | game TCP | S→C | ScoreCompleted | `o2jam-net::ScoreCompleted` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::ScoreCompleted` |
| 4021 | game TCP | C→S | ExitPlaying | OPEN: exit-playing body; next: tap the 4021 send at game end | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::ExitPlaying` |
| 4022 | game TCP | S→C | UserLeaveGame | OPEN: wire body; next: capture or rewrite a frame on the local gateway | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::UserLeaveGame` |
| 4023 | game TCP | C→S | SetRoomSkill | OPEN: room-skill body; next: read the exe builder behind the skill controls | `docs/net-protocol.md` §4a |
| 4024 | game TCP | S→C | WaitingSkillChanged | OPEN: wire body; next: capture or rewrite a frame on the local gateway | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::WaitingSkillChanged` |
| 4025 | game TCP | C→S | UpdateMusicState | `o2jam-net::UpdateMusicState` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::UpdateMusicState` |
| 4026 | game TCP | S→C | MemberStateChanged | OPEN: body layout of MemberStateChanged; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4027 | game TCP | C→S | SetRoomAlbum | OPEN: room-album body; next: proxy rewrite of the 4027 frame | `docs/net-protocol.md` §4a |
| 4028 | game TCP | S→C | WaitingEvent | OPEN: body layout of WaitingEvent; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4029 | game TCP | S→C | WaitingEvent | OPEN: body layout of WaitingEvent; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4030 | game TCP | C→S | GetMusicList | OPEN: wire body; next: capture or rewrite a frame on the local gateway | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::GetMusicList` |
| 4031 | game TCP | S→C | MusicListAnswer | `o2jam-net::MusicListAnswer` | `docs/net-protocol.md` §4b; `o2jam-net` codec `MusicListAnswer` |
| 4034 | game TCP | C→S | AcquireMusicRequest | OPEN: codec exists but no exe field-level builder proof; next: capture a master START notice | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::AcquireMusicRequest` |
| 4035 | game TCP | S→C | AcquireMusic | `o2jam-net::AcquireMusic` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::AcquireMusic` |
| 4036 | game TCP | C→S | SyncMemberMusicState | `o2jam-net::SyncMemberMusicState` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SyncMemberMusicState` |
| 4037 | game TCP | S→C | SyncMemberMusicState | `o2jam-net::SyncMemberMusicStateEvent` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::SyncMemberMusicState` |
| 4038 | game TCP | C→S | SetDownloadProgress | `o2jam-net::SetDownloadProgress` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SetDownloadProgress` |
| 4039 | game TCP | S→C | DownloadProgressChanged | `o2jam-net::DownloadProgressChanged` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::DownloadProgressChanged` |
| 4040 | game TCP | S→C | GameRankUpdate | `o2jam-net::GameRankUpdate` | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::GameRankUpdate` |
| 4041 | game TCP | C→S | FinalizeRank | OPEN: rank-finalize body; next: tap the 4041 send at Result entry | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::FinalizeRank` |
| 4042 | game TCP | C→S | GetLiveState | `o2jam-net::GetLiveState` | byte-verified empty send at `crates/o2jam-net/src/relay/commands.rs:801`; moment PINNED in `docs/send-moments.md` §5 |
| 4043 | relay TCP | S→C | LiveStateResponse | `o2jam-net::LiveStateResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `LiveStateResponse` |
| 4044 | game TCP | C→S | GetP2PList | `o2jam-net::GetP2PList` | byte-verified empty send at `crates/o2jam-net/src/relay/commands.rs:820`; moment PINNED in `docs/send-moments.md` §5 |
| 4045 | relay TCP | S→C | P2PListResponse | `o2jam-net::P2PListResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `P2PListResponse` |
| 4046 | game TCP | C→S | ReportUdpPunchHole | `o2jam-net::ReportUdpPunchHole` | `docs/net-protocol.md` §4a |
| 4047 | relay TCP | S→C | ReportUdpPunchHoleAnswer | `o2jam-net::ReportUdpPunchHoleAnswer` | `docs/net-protocol.md` §4b; `o2jam-net` codec `ReportUdpPunchHoleAnswer` |
| 4048 | game TCP | C→S | TestNetworkLatency | OPEN: latency body; next: tap a live-mode latency exchange | `docs/net-protocol.md` §4a |
| 4049 | game TCP | S→C | PlayingEvent | OPEN: body layout of PlayingEvent; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4050 | game TCP | S→C | PlayingEvent | OPEN: body layout of PlayingEvent; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4051 | game TCP | C→S | StartSelectMusic | OPEN: select-music body; next: read the exe send behind the select entry | `docs/net-protocol.md` §4a |
| 4052 | game TCP | S→C | PlayingEvent | OPEN: body layout of PlayingEvent; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4053 | game TCP | C→S | CancelSelectMusic | OPEN: cancel-select body; next: read the exe send behind the select exit | `docs/net-protocol.md` §4a |
| 4054 | game TCP | S→C | PlayingEvent | OPEN: body layout of PlayingEvent; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 4055 | game TCP | C→S | ConfirmRelay | `o2jam-net::ConfirmRelay` | `docs/net-protocol.md` §4a |
| 4056 | relay TCP | S→C | ConfirmRelayAnswer | `o2jam-net::ConfirmRelayAnswer` | `docs/net-protocol.md` §4b; `o2jam-net` codec `ConfirmRelayAnswer` |
| 4057 | game TCP | C→S | SetRoomSkillEx | OPEN: extended-skill body; next: proxy rewrite of the 4057 frame | `docs/net-protocol.md` §4a |
| 4058 | game TCP | S→C | WaitingSkillExChanged | OPEN: wire body; next: capture or rewrite a frame on the local gateway | `docs/net-protocol.md` §4b; `o2jam-net::command::Event::WaitingSkillExChanged` |
| 5000 | game TCP | C→S | SyncItemPurchase | `o2jam-net::SyncItemPurchase` | `docs/net-protocol.md` §4a |
| 5001 | game TCP | S→C | SyncItemPurchaseAnswer | OPEN: only the 152-byte prefix decoder exists; next: finish a full body type from a captured 5001 | `docs/net-protocol.md` §4b opcode list |
| 5002 | game TCP | C→S | SellItem | `o2jam-net::SellItem` | `docs/net-protocol.md` §4a |
| 5003 | game TCP | S→C | SellItemResponse | `o2jam-net::SellItemResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `SellItemResponse` |
| 5004 | game TCP | C→S | EquipItem | `o2jam-net::EquipItem` | `docs/net-protocol.md` §4a |
| 5005 | game TCP | S→C | EquipItemResponse | `o2jam-net::EquipItemResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `EquipItemResponse` |
| 5006 | game TCP | C→S | EnterShop | `o2jam-net::EnterShop` | `docs/net-protocol.md` §4a |
| 5007 | game TCP | C→S | ExitShop | `o2jam-net::ExitShop` | `docs/net-protocol.md` §4a |
| 5010 | game TCP | C→S | SyncMusicPurchase | `o2jam-net::SyncMusicPurchase` | `docs/net-protocol.md` §4a |
| 5011 | game TCP | S→C | SyncMusicPurchaseResponse | `o2jam-net::SyncMusicPurchaseResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `SyncMusicPurchaseResponse` |
| 5012 | game TCP | C→S | SyncMusicDownload | `o2jam-net::SyncMusicDownload` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::SyncMusicDownload` |
| 5013 | game TCP | C→S | SyncPoint | OPEN: point-sync body; next: capture a cash-point change | `docs/net-protocol.md` §4a |
| 5014 | game TCP | S→C | SyncPointAnswer | OPEN: body layout of SyncPointAnswer; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 5015 | game TCP | C→S | StartPayment | `o2jam-net::StartPayment` | `docs/net-protocol.md` §4a |
| 5016 | game TCP | S→C | StartPaymentResponse | `o2jam-net::StartPaymentResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `StartPaymentResponse` |
| 5017 | game TCP | C→S | GetGiftMessageList | `o2jam-net::GetGiftMessageList` | `docs/net-protocol.md` §4a |
| 5018 | game TCP | S→C | GetGiftMessageListResponse | `o2jam-net::GetGiftMessageListResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `GetGiftMessageListResponse` |
| 5019 | game TCP | C→S | ReadGiftMessage | `o2jam-net::ReadGiftMessage` | `docs/net-protocol.md` §4a |
| 5023 | game TCP | C→S | ClaimGift | `o2jam-net::ClaimGift` | `docs/net-protocol.md` §4a |
| 5024 | game TCP | S→C | ClaimGiftResponse | `o2jam-net::ClaimGiftResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `ClaimGiftResponse` |
| 5025 | game TCP | C→S | SyncFreePass | `o2jam-net::SyncFreePass` | `docs/net-protocol.md` §4a |
| 5026 | game TCP | S→C | SyncFreePassResponse | `o2jam-net::SyncFreePassResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `SyncFreePassResponse` |
| 5027 | game TCP | S→C | SyncMemberMusicList | OPEN: body layout of SyncMemberMusicList; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 5028 | game TCP | C→S | SyncCashPoint | `o2jam-net::SyncCashPoint` | `docs/net-protocol.md` §4a |
| 5029 | game TCP | S→C | SyncCashPointResponse | `o2jam-net::SyncCashPointResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `SyncCashPointResponse` |
| 5030 | game TCP | C→S | ChangeName | `o2jam-net::ChangeName` | `docs/net-protocol.md` §4a |
| 5031 | game TCP | S→C | ChangeNameResponse | `o2jam-net::ChangeNameResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `ChangeNameResponse` |
| 5032 | game TCP | C→S | CheckName | `o2jam-net::CheckName` | `docs/net-protocol.md` §4a |
| 5033 | game TCP | S→C | CheckNameResponse | `o2jam-net::CheckNameResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `CheckNameResponse` |
| 5034 | game TCP | C→S | PenaltyReset | OPEN: penalty-reset body; next: capture a penalty-reset click | `docs/net-protocol.md` §4a |
| 5035 | game TCP | S→C | PenaltyResetAnswer | OPEN: body layout of PenaltyResetAnswer; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 5036 | game TCP | C→S | BagExpansion | OPEN: bag-expansion body; next: capture a bag-expansion purchase | `docs/net-protocol.md` §4a |
| 5037 | game TCP | S→C | BagExpansionAnswer | OPEN: body layout of BagExpansionAnswer; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 5038 | game TCP | C→S | RefershBag | `o2jam-net::RefreshBag` | `docs/net-protocol.md` §4a |
| 5039 | game TCP | S→C | RefreshBagAnswer | OPEN: body layout of RefreshBagAnswer; next: capture or rewrite one gateway frame | `docs/net-protocol.md` §4b opcode list |
| 5040 | game TCP | C→S | CheckGiftFreePass | `o2jam-net::CheckGiftFreePass` | `docs/net-protocol.md` §4a |
| 5041 | game TCP | S→C | CheckGiftFreePassResponse | `o2jam-net::CheckGiftFreePassResponse` | `docs/net-protocol.md` §4b; `o2jam-net` codec `CheckGiftFreePassResponse` |
| 6000 | game TCP | echo | LegacyPing | `o2jam-net::LegacyPing` | `docs/net-protocol.md` §4a; `o2jam-net::command::Generic` |
| 6001 | game TCP | echo | Ping | `o2jam-net::Ping` | `docs/net-protocol.md` §4a; `o2jam-net::command::Generic` |
| 65520 | game TCP | C→S | Terminate | `o2jam-net::Terminate` | `docs/net-protocol.md` §4a; `o2jam-net::command::Request::Terminate` |

## Coverage notes

- Requests found only in the enum (for example 4040-adjacent events) inherit the §4b family evidence.
- Relay ids 1–11 use the same framing but a different opcode space; ids 4042–4056 are game-gateway ids with relay semantics.
- OPEN rows are not implemented codecs. Their next step is the evidence action that can close the row.
- Correction record (formerly OPEN, now codec): 2024 has a body codec at `message.rs:132`; 4042/4044 are byte-verified empty sends at `relay/commands.rs:801/820` and their moments are pinned in `docs/send-moments.md` §5. The earlier OPEN labels were inventory errors, not source contradictions; they are recorded here so the correction stays visible.
