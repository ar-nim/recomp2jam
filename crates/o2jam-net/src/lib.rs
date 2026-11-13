//! Game TCP wire contract for a Memoryer-compatible client.
//!
//! Phase 1 of `docs/client-server-plan.md`: framing plus the
//! login/channel messages, with the Phase 3/4 host song-flow
//! messages alongside. Layouts mirror Mozart.Encore
//! Memoryer's `Source/Memoryer` field orders (clean-room C#,
//! v7.55); the exe confirms the framing anchor (command id as
//! little-endian `u16` at buffer offset 2, `FUN_005a0550`
//! file `0x1a0550`) and the opcodes it sends.
//!
//! Length semantics (`len` = total frame length including the length
//! field) are server-verified against Encore's `SizePrefixedMessageFramer`
//! and a live Memoryer gateway (`docs/net-protocol.md` §3).

pub mod character;
pub mod client;
pub mod command;
pub mod error;
pub mod frame;
mod inflate;
// `message` precedes `gift`/`shop`/`wallet`: its `empty_message!`
// macro expands there (`macro_rules!` travels by textual scope).
#[macro_use]
pub mod message;
pub mod gift;
mod macros;
pub mod play;
pub mod reader;
pub mod relay;
pub mod rooms;
mod rows;
pub mod shop;
pub mod strings;
pub mod testing;
pub mod wallet;

pub use client::{GatewayConnection, DEFAULT_GATEWAY_PORT};

pub use character::{
    AttributiveItemInfo, CharacterInfo, GiftItemInfo, GiftMusicInfo, EQUIPMENT_SLOTS,
};
pub use command::{Event, Generic, Request, Response};
pub use error::Error;
pub use frame::{Frame, FrameSplitter, HEADER_LEN, ID_OFFSET};
pub use message::{
    AuthResponse, AuthResult, AuthorizeRequest, ChannelList, ChannelLogin, ChannelLoginResponse,
    ChannelLoginResult, ChannelState, CreateResult, CreateRoom, CreateRoomResponse, CreatedRoom,
    EmptyMessage, ForceReauthorize, GameMode, Gender, GetChannelList, GetCharacterInfo, KeyMode,
    LegacyPing, MultiSessionError, Ping, RoomCreated, SessionKeys, SessionKeysRequest,
    SubscriptionAlert, Terminate, WireMessage, MEMORYER_CLIENT_ID,
};
pub use play::{
    ConfirmMusicLoaded, ExitPlaying, FinalizeRank, GameRankUpdate, GameStatsUpdate, GetMusicList,
    MatchResult, MemberReadyStateChanged, Ready, RoomTeamChanged, ScoreCompleted, ScoreEntry,
    ScoreSubmission, SetDownloadProgress, SetRoomArena, SetRoomMusic, SetRoomTeam, StartGame,
    StatKind, SubmitScore, SyncMemberMusicState, UpdateGameStats,
};
pub use relay::{
    ComboBroken, ComboStarted, ConnectionEndpoint, CreateRelaySession, DeleteRelaySession,
    MemberRole, PeerConfirm, PeerConnected, PeerEndpointAssigned, RelayCommand, RelayMember,
    RelayPing, RelayRouting, RelayStats, UdpPacketType, UdpRelayFrame,
};
pub use rooms::{
    Announce, Announcement, Difficulty, GameSpeed, GetMusicMaxScore, GetMusicPlayRanking,
    GetMusicScoreList, JoinResult, JoinRoom, JoinRoomResponse, JoinedRoom, MainRoomAdminMessage,
    MainRoomChatLine, MainRoomUserMessage, MusicMaxScoreAnswer, MusicPlayRankingAnswer,
    MusicScoreListAnswer, MusicScoreRow, RankDelta, RankEntry, RankNotification, ReceiveWhisper,
    RoomList, RoomListEntry, RoomMember, RoomSlot, RoomSlotAction, RoomSlotState, RoomSlotUpdate,
    RoomState, SendMainRoomMessage, SendWhisper, SendWhisperResponse, ROOM_TITLE_LEN,
};
pub use strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};
