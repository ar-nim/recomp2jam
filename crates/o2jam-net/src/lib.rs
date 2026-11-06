//! Game TCP wire contract for a Memoryer-compatible client.
//!
//! Phase 1 of `docs/client-server-plan.md`: framing plus the
//! login/channel messages. Layouts mirror Mozart.Encore
//! Memoryer's `Source/Memoryer` field orders (clean-room C#,
//! v7.55); the exe confirms the framing anchor (command id as
//! little-endian `u16` at buffer offset 2, `FUN_005a0550`
//! file `0x1a0550`) and the opcodes it sends.
//!
//! Length semantics (`len` = bytes after the length field) follow
//! Memoryer convention and the in-repo UDP precedent — the exe
//! only pins id-at-offset-2, so live interop is still the final
//! check (`docs/net-protocol.md` records it OPEN).

pub mod client;
pub mod command;
pub mod error;
pub mod frame;
pub mod message;

pub use client::{GatewayConnection, DEFAULT_GATEWAY_PORT};

pub use command::{Generic, Request, Response};
pub use error::Error;
pub use frame::{Frame, FrameSplitter, HEADER_LEN, ID_OFFSET};
pub use message::{
    ChannelList, ChannelLogin, ChannelLoginResponse, ChannelLoginResult, ChannelState,
    EmptyMessage, ForceReauthorize, GetChannelList, LegacyPing, MultiSessionError, Ping,
    SessionKeys, SessionKeysRequest, SubscriptionAlert, Terminate,
};
