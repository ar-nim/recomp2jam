//! Live relay packets (room PvP lane share).
//!
//! Relay TCP carries [`RelayCommand`] frames with the same
//! length-prefixed `CPacket_Str` framing as the game gateway
//! (`[u16 len][u16 id][body]`, `len` counts the whole frame; id at
//! buffer offset 2). The shared `SetID` writer is byte-verified:
//! the op-5 ctor at file `0xea4e0` pushes 5 and calls the same
//! `SetID` function at file `0x1a0550` as the gateway path
//! (call site file `0xea4f4`; id-at-offset-2 pinned by
//! `FUN_005a0550`, length-includes-prefix server-verified in
//! `crate::frame`). Relay op 5 on TCP is therefore the 66-byte
//! frame `42 00 05 00` + 62-byte tally, not the 20-byte game-
//! gateway 4014 tick (`14 00 AE 0F` + 16-byte body); see
//! `docs/relay.md` §2. Relay TCP peer-leg ports have no hardcoded
//! immediate in-exe; they arrive via the server (`GetLiveState`
//! 4042/4043, `GetP2PList` 4044/4045). The only hardcoded relay
//! ports are [`RELAY_UDP_PORTS`]. UDP is the primary Live path: each datagram is
//! `[u16 uncompressedLen][zlib(...)]` holding one RUDP inner frame,
//! decoded by [`UdpRelayFrame`].
//!
//! Layouts mirror Memoryer's clean-room `Memoryer.Relay` messages
//! (field order = wire order): `UpdateGameStatsEventData`,
//! `ComboBrokenEventData`, `ComboStartedEventData`,
//! `PeerConnectedEventData`, `PeerEndpointAssignedEventData`,
//! `PeerConfirmRequest`, `PingRequest`, `CreateRelaySessionRequest`,
//! `DeleteRelaySessionRequest`, `UdpRelayFrameCodec`,
//! `UdpRelayPacketType`. The exe pins the opcodes (`SetID(1..7)`
//! ctors), the class names (`CO2JamCoolPack`/`CO2JamMissPack`
//! RTTI), and the endpoints — not the field bytes, so the relay
//! bodies below are Memoryer-corroborated; see `docs/relay.md` for
//! the anchor table and the open exe-verification items. The
//! trailing Live-handshake section (game-gateway 4046/4047/4055/
//! 4056) is the exception: all four are byte-verified empty
//! in-exe (per-type offsets below; 4055 separated from the
//! field-carrying send sites, see `docs/relay.md`).

mod commands;
mod routing;
mod udp;

pub use commands::*;
pub use routing::*;
pub use udp::*;
