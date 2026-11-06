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
pub mod message;
pub mod play;
pub mod rooms;
pub mod strings;

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
    ConfirmMusicLoaded, ExitPlaying, FinalizeRank, GameStatsUpdate, GetMusicList,
    SetDownloadProgress, SetRoomMusic, StartGame, StatKind, SubmitScore, SyncMemberMusicState,
    UpdateGameStats,
};
pub use rooms::{
    Difficulty, GameSpeed, JoinResult, JoinRoom, JoinRoomResponse, JoinedRoom, RoomList,
    RoomListEntry, RoomMember, RoomSlot, RoomSlotState, RoomState, ROOM_TITLE_LEN,
};
pub use strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};
