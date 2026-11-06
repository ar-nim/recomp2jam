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

use std::net::{Ipv4Addr, SocketAddrV4};

use super::error::Error;
use super::inflate::{compress_zlib_stored, decompress_zlib};
use super::message::WireMessage;
use super::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};

/// Relay opcodes (`Memoryer.Relay` `RelayCommand`, matched in-exe by
/// the `SetID(1..7)` ctors; see `docs/relay.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum RelayCommand {
    /// Session keys issued (`CO2Jam`-side `SetID(1)`).
    PeerConnected = 1,
    /// Client answers with its session keys (`SetID(2)`).
    PeerConfirm = 2,
    /// Relay path assigned; forces TCP fallback (`SetID(3)`).
    PeerEndpointAssigned = 3,
    /// Keepalive (`SetID(4)`).
    Ping = 4,
    /// Full score tally, fanned out to room peers (`SetID(5)`).
    UpdateGameStats = 5,
    /// Combo broke (`CO2JamMissPack`, `SetID(6)`).
    ComboBroken = 6,
    /// Combo (re)started (`CO2JamCoolPack`, `SetID(7)`).
    ComboStarted = 7,
    /// Bind relay sessions into a game (`u8`-counted key pairs).
    CreateSession = 10,
    /// Unbind relay sessions.
    DeleteSession = 11,
}

impl RelayCommand {
    /// Numeric wire value.
    #[must_use]
    pub fn id(self) -> u16 {
        self as u16
    }

    /// Resolve a received id. `None` means not a relay command.
    #[must_use]
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            1 => Some(Self::PeerConnected),
            2 => Some(Self::PeerConfirm),
            3 => Some(Self::PeerEndpointAssigned),
            4 => Some(Self::Ping),
            5 => Some(Self::UpdateGameStats),
            6 => Some(Self::ComboBroken),
            7 => Some(Self::ComboStarted),
            10 => Some(Self::CreateSession),
            11 => Some(Self::DeleteSession),
            _ => None,
        }
    }
}

/// Room seat role, carried as `u16` on relay op 5/6/7
/// (`Encore.Metadata.MemberRole`: None 0, Champion 1, Challenger 2,
/// Spectator 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MemberRole {
    /// No seat role.
    #[default]
    None = 0,
    /// Room champion (Live rooms).
    Champion = 1,
    /// Challenger (Live rooms).
    Challenger = 2,
    /// Spectator.
    Spectator = 3,
}

impl MemberRole {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u16(self) -> u16 {
        u16::from(self as u8)
    }

    /// Parse a wire word; unknown roles are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside 0–3.
    pub fn from_u16(value: u16) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Champion),
            2 => Ok(Self::Challenger),
            3 => Ok(Self::Spectator),
            _ => Err(Error::InvalidField {
                field: "member role",
                reason: "not 0, 1, 2, or 3",
            }),
        }
    }
}

/// Hardcoded relay endpoint ports picked by the in-exe relay selector
/// alongside the three relay IPs (`docs/relay.md` §9). These are the
/// only hardcoded relay ports: a full-image `u16` scan finds `0x998f`
/// at exactly the three selector-leaf stores (files
/// `0x18c73b`–`0x18c77f`) and `0x9990` port-word stores only at the
/// two matching leaves; the five other `0x9990` hits are non-port
/// bytes (table address, relative call, jump table, data). Relay TCP
/// peer-leg ports are server-provided (`GetLiveState`/`GetP2PList`),
/// never immediates, so no `RELAY_TCP_PORTS` constant exists by design.
///
/// Byte-verified immediates: `0x998f`/`0x9990` stored as the port word by
/// the selector leaves at file `0x18c73b`–`0x18c77f`; transport role is
/// inferred from the UHP/UDP context, not proven by those bytes alone.
/// (ERRORS-06: plain constants, no `unwrap`; PERF-07: `u16` literals,
/// no narrowing casts.)
pub const RELAY_UDP_PORTS: [u16; 2] = [39311, 39312];

/// Adjunct host configured by the shared setup helper entered at file
/// `0x193650` (`docs/relay.md` §9).
///
/// Byte-verified references: the `203.238.136.52` string (file `0x24e368`)
/// is pushed as a call argument at exactly two image-wide sites (files
/// `0x1936e2`, `0x1936fc`), both inside the same setup block that stores
/// [`ADJUNCT_SETUP_NUMBER`]; neither the relay selector (file `0x18c6c0`)
/// nor the UHP controller (file `0x18d580`) references this host. It is
/// therefore a separately configured adjunct endpoint — not a fourth
/// relay-pool member, a UHP helper, or the `CUDPFailTCPConnect` fallback
/// path — whose exact service role stays open.
/// (ERRORS-06: plain constant, no `unwrap`; PERF-07: no narrowing casts.)
pub const ADJUNCT_RELAY_HOST: Ipv4Addr = Ipv4Addr::new(203, 238, 136, 52);

/// Numeric value co-located with the adjunct-host setup (file `0x19370b`).
///
/// Byte-verified immediate: the only `0x1e6c` dword image-wide, stored into
/// a setup-block slot just after the second adjunct-host call. Recorded as
/// a number, not claimed as a port: no socket-connect sequence ties it to
/// the host. (ERRORS-06/PERF-07: plain `u16` literal.)
pub const ADJUNCT_SETUP_NUMBER: u16 = 7788;

/// Serialized size of one relay endpoint: 4 address bytes + `u16` port.
const ENDPOINT_LEN: usize = 6;

/// Serialized size of one `ConnectionEndpoint`: two `u16` ports +
/// 4 address bytes.
const CONNECTION_ENDPOINT_LEN: usize = 8;

/// Serialized size of the routing preface shared by relay ops 1–7:
/// `i32` code + 3 endpoints + `u16` unused.
const ROUTING_LEN: usize = 4 + 3 * ENDPOINT_LEN + 2;

/// Append one relay endpoint (4 address bytes, then port as `u16`).
fn encode_socket(endpoint: SocketAddrV4, body: &mut Vec<u8>) {
    body.extend_from_slice(&endpoint.ip().octets());
    body.extend_from_slice(&endpoint.port().to_le_bytes());
}

/// Split 6 endpoint bytes off the front of `body`.
fn decode_socket(body: &[u8]) -> Result<(SocketAddrV4, &[u8]), Error> {
    if body.len() < ENDPOINT_LEN {
        return Err(Error::TruncatedMessage {
            need: ENDPOINT_LEN,
            have: body.len(),
        });
    }
    let address = Ipv4Addr::new(body[0], body[1], body[2], body[3]);
    let port = u16::from_le_bytes([body[4], body[5]]);
    Ok((SocketAddrV4::new(address, port), &body[ENDPOINT_LEN..]))
}

/// Dual-port endpoint of relay ops 2–3 (`ConnectionEndpoint`:
/// `Port1`, `Port2`, then 4 address bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionEndpoint {
    /// First port (the `IPEndPoint` view of this endpoint).
    pub port1: u16,
    /// Second port.
    pub port2: u16,
    /// IPv4 address.
    pub address: Ipv4Addr,
}

impl ConnectionEndpoint {
    /// Append the 8-byte form to `body`.
    fn encode_into(&self, body: &mut Vec<u8>) {
        body.extend_from_slice(&self.port1.to_le_bytes());
        body.extend_from_slice(&self.port2.to_le_bytes());
        body.extend_from_slice(&self.address.octets());
    }

    /// Split 8 endpoint bytes off the front of `body`.
    fn decode_from(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < CONNECTION_ENDPOINT_LEN {
            return Err(Error::TruncatedMessage {
                need: CONNECTION_ENDPOINT_LEN,
                have: body.len(),
            });
        }
        Ok((
            Self {
                port1: u16::from_le_bytes([body[0], body[1]]),
                port2: u16::from_le_bytes([body[2], body[3]]),
                address: Ipv4Addr::new(body[4], body[5], body[6], body[7]),
            },
            &body[CONNECTION_ENDPOINT_LEN..],
        ))
    }
}

/// Routing preface shared by relay ops 1–7 (Memoryer field orders
/// 0–2): session `code`, the 3 relay-server endpoints, `unused`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayRouting {
    /// Session code (private `Code` field; zero in server events).
    pub code: i32,
    /// The 3 relay-server endpoints (no count prefix; always 3).
    pub relays: [SocketAddrV4; 3],
    /// Reserved word (private `Unused`; zero observed).
    pub unused: u16,
}

impl RelayRouting {
    /// Append the 24-byte preface to `body`.
    fn encode_into(&self, body: &mut Vec<u8>) {
        body.extend_from_slice(&self.code.to_le_bytes());
        for relay in &self.relays {
            encode_socket(*relay, body);
        }
        body.extend_from_slice(&self.unused.to_le_bytes());
    }

    /// Split the 24-byte preface off the front of `body`.
    fn decode_from(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < ROUTING_LEN {
            return Err(Error::TruncatedMessage {
                need: ROUTING_LEN,
                have: body.len(),
            });
        }
        let code = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let (first, tail) = decode_socket(&body[4..])?;
        let (second, tail) = decode_socket(tail)?;
        let (third, tail) = decode_socket(tail)?;
        let unused = u16::from_le_bytes([tail[0], tail[1]]);
        Ok((
            Self {
                code,
                relays: [first, second, third],
                unused,
            },
            &tail[2..],
        ))
    }
}

/// Read one little-endian `u16` at `at`.
fn word_at(body: &[u8], at: usize) -> Result<u16, Error> {
    if body.len() < at + 2 {
        return Err(Error::TruncatedMessage {
            need: at + 2,
            have: body.len(),
        });
    }
    Ok(u16::from_le_bytes([body[at], body[at + 1]]))
}

/// `PeerConnected` 1: server issues session keys (S→C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerConnected {
    /// Routing preface (code, relay endpoints, unused).
    pub routing: RelayRouting,
    /// First relay session key.
    pub session_key1: i32,
    /// Second relay session key.
    pub session_key2: i32,
}

/// Body length of [`PeerConnected`]: 24 + 4 + 4.
const PEER_CONNECTED_BODY_LEN: usize = 32;

impl WireMessage for PeerConnected {
    const COMMAND: u16 = RelayCommand::PeerConnected as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(PEER_CONNECTED_BODY_LEN);
        self.routing.encode_into(&mut body);
        body.extend_from_slice(&self.session_key1.to_le_bytes());
        body.extend_from_slice(&self.session_key2.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < PEER_CONNECTED_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: PEER_CONNECTED_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > PEER_CONNECTED_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - PEER_CONNECTED_BODY_LEN));
        }
        let (routing, tail) = RelayRouting::decode_from(body)?;
        Ok(Self {
            routing,
            session_key1: i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]),
            session_key2: i32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]),
        })
    }
}

/// `PeerConfirm` 2: client answers with session keys plus its local
/// and public endpoints (C→S).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerConfirm {
    /// Routing preface (code, relay endpoints, unused).
    pub routing: RelayRouting,
    /// First relay session key.
    pub session_key1: i32,
    /// Second relay session key.
    pub session_key2: i32,
    /// Client LAN endpoint.
    pub local: ConnectionEndpoint,
    /// Client public endpoint.
    pub public: ConnectionEndpoint,
}

/// Body length of [`PeerConfirm`]: 24 + 4 + 4 + 8 + 8.
const PEER_CONFIRM_BODY_LEN: usize = 48;

impl WireMessage for PeerConfirm {
    const COMMAND: u16 = RelayCommand::PeerConfirm as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(PEER_CONFIRM_BODY_LEN);
        self.routing.encode_into(&mut body);
        body.extend_from_slice(&self.session_key1.to_le_bytes());
        body.extend_from_slice(&self.session_key2.to_le_bytes());
        self.local.encode_into(&mut body);
        self.public.encode_into(&mut body);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < PEER_CONFIRM_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: PEER_CONFIRM_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > PEER_CONFIRM_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - PEER_CONFIRM_BODY_LEN));
        }
        let (routing, tail) = RelayRouting::decode_from(body)?;
        let session_key1 = i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
        let session_key2 = i32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]);
        let (local, tail) = ConnectionEndpoint::decode_from(&tail[8..])?;
        let (public, _) = ConnectionEndpoint::decode_from(tail)?;
        Ok(Self {
            routing,
            session_key1,
            session_key2,
            local,
            public,
        })
    }
}

/// `PeerEndpointAssigned` 3: relay path handout (S→C); receiving it
/// forces the game onto TCP relay instead of peer-to-peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerEndpointAssigned {
    /// Routing preface (code, relay endpoints, unused).
    pub routing: RelayRouting,
    /// Public endpoint the peer must use.
    pub public: ConnectionEndpoint,
}

/// Body length of [`PeerEndpointAssigned`]: 24 + 8.
const PEER_ENDPOINT_ASSIGNED_BODY_LEN: usize = 32;

impl WireMessage for PeerEndpointAssigned {
    const COMMAND: u16 = RelayCommand::PeerEndpointAssigned as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(PEER_ENDPOINT_ASSIGNED_BODY_LEN);
        self.routing.encode_into(&mut body);
        self.public.encode_into(&mut body);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < PEER_ENDPOINT_ASSIGNED_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: PEER_ENDPOINT_ASSIGNED_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > PEER_ENDPOINT_ASSIGNED_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - PEER_ENDPOINT_ASSIGNED_BODY_LEN,
            ));
        }
        let (routing, tail) = RelayRouting::decode_from(body)?;
        let (public, _) = ConnectionEndpoint::decode_from(tail)?;
        Ok(Self { routing, public })
    }
}

/// `Ping` 4: relay keepalive (C→S).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayPing {
    /// Routing preface (code, relay endpoints, unused).
    pub routing: RelayRouting,
    /// Ping start tick.
    pub start: u32,
    /// Ping reply tick.
    pub tick: u32,
}

/// Body length of [`RelayPing`]: 24 + 4 + 4.
const RELAY_PING_BODY_LEN: usize = 32;

impl WireMessage for RelayPing {
    const COMMAND: u16 = RelayCommand::Ping as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(RELAY_PING_BODY_LEN);
        self.routing.encode_into(&mut body);
        body.extend_from_slice(&self.start.to_le_bytes());
        body.extend_from_slice(&self.tick.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < RELAY_PING_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: RELAY_PING_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > RELAY_PING_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - RELAY_PING_BODY_LEN));
        }
        let (routing, tail) = RelayRouting::decode_from(body)?;
        Ok(Self {
            routing,
            start: u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]),
            tick: u32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]),
        })
    }
}

/// `UpdateGameStats` 5: full score tally mirrored to room peers.
///
/// This is the relay stats mirror: unlike game TCP 4014 (a 16-byte
/// life/jam tick), op 5 carries the whole tally the relay fans out
/// to every other member of the game session.
///
/// In-exe writer (7.55, local-static, read-only): tally packer at file
/// `0xea230` (VA `0x4ea230`), called per tick from `0xeaec0` at site
/// `0xeb0c9`; ctor `SetID(5)` at file `0xea4e0` (vtable entry file
/// `0x24b334`); send helper at file `0x187870`. See
/// `docs/op5-writer.md` (`ev_deb10e8fcc5d56f0435fc92c3afc57bc`,
/// `ev_ccded3a418039e0962f34de24a6a669f`,
/// `ev_3f1b719e9d41cdc2eef7b7b7584396b3`,
/// `ev_8f8df9d42c5cfbc0073f6a1c13f91cb9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayStats {
    /// Routing preface (code, relay endpoints, unused).
    pub routing: RelayRouting,
    /// Seat role of the sender.
    pub role: MemberRole,
    /// Cool hits.
    pub cool: u16,
    /// Good hits.
    pub good: u16,
    /// Bad hits.
    pub bad: u16,
    /// Misses.
    pub miss: u16,
    /// Current combo.
    pub combo: u16,
    /// Best combo.
    pub max_combo: u16,
    /// Current jam level.
    pub jam_combo: u16,
    /// Best jam level.
    pub total_jam_combo: u16,
    /// Long-note hold ticks.
    pub long_note_combo: u16,
    /// Long-note bonus score.
    pub long_note_score: u32,
    /// Remaining life.
    pub life: u16,
    /// Pill progress toward the next pill.
    pub pill_buildup: u32,
    /// Pills held.
    pub pill_count: u32,
    /// Running score.
    pub score: u32,
}

/// Body length of [`RelayStats`]: 24 + 2 + 9×2 + 4 + 2 + 4 + 4 + 4.
const RELAY_STATS_BODY_LEN: usize = 62;

impl WireMessage for RelayStats {
    const COMMAND: u16 = RelayCommand::UpdateGameStats as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(RELAY_STATS_BODY_LEN);
        self.routing.encode_into(&mut body);
        body.extend_from_slice(&self.role.as_u16().to_le_bytes());
        for tally in [
            self.cool,
            self.good,
            self.bad,
            self.miss,
            self.combo,
            self.max_combo,
            self.jam_combo,
            self.total_jam_combo,
            self.long_note_combo,
        ] {
            body.extend_from_slice(&tally.to_le_bytes());
        }
        body.extend_from_slice(&self.long_note_score.to_le_bytes());
        body.extend_from_slice(&self.life.to_le_bytes());
        body.extend_from_slice(&self.pill_buildup.to_le_bytes());
        body.extend_from_slice(&self.pill_count.to_le_bytes());
        body.extend_from_slice(&self.score.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < RELAY_STATS_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: RELAY_STATS_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > RELAY_STATS_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - RELAY_STATS_BODY_LEN));
        }
        let (routing, tail) = RelayRouting::decode_from(body)?;
        Ok(Self {
            routing,
            role: MemberRole::from_u16(u16::from_le_bytes([tail[0], tail[1]]))?,
            cool: word_at(tail, 2)?,
            good: word_at(tail, 4)?,
            bad: word_at(tail, 6)?,
            miss: word_at(tail, 8)?,
            combo: word_at(tail, 10)?,
            max_combo: word_at(tail, 12)?,
            jam_combo: word_at(tail, 14)?,
            total_jam_combo: word_at(tail, 16)?,
            long_note_combo: word_at(tail, 18)?,
            long_note_score: u32::from_le_bytes([tail[20], tail[21], tail[22], tail[23]]),
            life: word_at(tail, 24)?,
            pill_buildup: u32::from_le_bytes([tail[26], tail[27], tail[28], tail[29]]),
            pill_count: u32::from_le_bytes([tail[30], tail[31], tail[32], tail[33]]),
            score: u32::from_le_bytes([tail[34], tail[35], tail[36], tail[37]]),
        })
    }
}

/// Append the combo tail: autoplay flag as `u32`, then role as `u16`.
fn encode_combo_tail(autoplay: bool, role: MemberRole, body: &mut Vec<u8>) {
    body.extend_from_slice(&u32::from(autoplay).to_le_bytes());
    body.extend_from_slice(&role.as_u16().to_le_bytes());
}

/// Split the combo tail off the front of `body`.
fn decode_combo_tail(body: &[u8]) -> Result<(bool, MemberRole), Error> {
    if body.len() < 6 {
        return Err(Error::TruncatedMessage {
            need: 6,
            have: body.len(),
        });
    }
    let autoplay = match u32::from_le_bytes([body[0], body[1], body[2], body[3]]) {
        0 => false,
        1 => true,
        _ => {
            return Err(Error::InvalidField {
                field: "combo autoplay flag",
                reason: "not 0 or 1",
            });
        }
    };
    let role = MemberRole::from_u16(u16::from_le_bytes([body[4], body[5]]))?;
    Ok((autoplay, role))
}

/// Body length of [`ComboBroken`] and [`ComboStarted`]: 24 + 4 + 2.
const COMBO_BODY_LEN: usize = 30;

/// `ComboBroken` 6 (`CO2JamMissPack`): combo broke, disable play.
///
/// In-exe senders (7.55, local-static, read-only): the score-region
/// dispatcher at file `0x16ddf0` (miss branch, state field `+0x39b0 == 1`,
/// autoplay dword-1 store at file `0x16dec0`) and the relay-table fanout
/// through the wrapper at file `0x16df90` (autoplay dword-1 store at file
/// `0x16dfa8`, sole caller at file `0x18eafa`). No direct call into the
/// `SetID(6)` ctor at file `0x16dfe0` exists image-wide. See `docs/relay.md`
/// (`ev_09e854f508b2e4040ee438ec6356d8f8`,
/// `ev_881b47397192e387e32beb24c313ba1b`,
/// `ev_55d43b289edf1ab13813a2a3530c03f1`,
/// `ev_fae5559777b535cb3878108d9b0484df`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboBroken {
    /// Routing preface (code, relay endpoints, unused).
    pub routing: RelayRouting,
    /// Autoplay flag (7.55 always sends 1; no zero-store exists on any
    /// combo send path).
    pub autoplay: bool,
    /// Seat role of the sender.
    pub role: MemberRole,
}

impl WireMessage for ComboBroken {
    const COMMAND: u16 = RelayCommand::ComboBroken as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(COMBO_BODY_LEN);
        self.routing.encode_into(&mut body);
        encode_combo_tail(self.autoplay, self.role, &mut body);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() != COMBO_BODY_LEN {
            return Err(if body.len() < COMBO_BODY_LEN {
                Error::TruncatedMessage {
                    need: COMBO_BODY_LEN,
                    have: body.len(),
                }
            } else {
                Error::TrailingBytes(body.len() - COMBO_BODY_LEN)
            });
        }
        let (routing, tail) = RelayRouting::decode_from(body)?;
        let (autoplay, role) = decode_combo_tail(tail)?;
        Ok(Self {
            routing,
            autoplay,
            role,
        })
    }
}

/// `ComboStarted` 7 (`CO2JamCoolPack`): combo (re)started, enable play.
///
/// In-exe senders (7.55, local-static, read-only): the score-region
/// dispatcher at file `0x16ddf0` (cool branch, state field `+0x39b0 == 2`,
/// autoplay dword-1 stores at files `0x16df28`/`0x16df2f`) and the
/// relay-table fanout through the wrapper at file `0x16e030` (autoplay
/// dword-1 store at file `0x16e048`; no direct caller, reached through the
/// relay table). No direct call into the `SetID(7)` ctor at file `0x16e080`
/// exists image-wide. See `docs/relay.md` (same evidence ids as
/// [`ComboBroken`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboStarted {
    /// Routing preface (code, relay endpoints, unused).
    pub routing: RelayRouting,
    /// Autoplay flag (7.55 always sends 1; no zero-store exists on any
    /// combo send path).
    pub autoplay: bool,
    /// Seat role of the sender.
    pub role: MemberRole,
}

impl WireMessage for ComboStarted {
    const COMMAND: u16 = RelayCommand::ComboStarted as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(COMBO_BODY_LEN);
        self.routing.encode_into(&mut body);
        encode_combo_tail(self.autoplay, self.role, &mut body);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() != COMBO_BODY_LEN {
            return Err(if body.len() < COMBO_BODY_LEN {
                Error::TruncatedMessage {
                    need: COMBO_BODY_LEN,
                    have: body.len(),
                }
            } else {
                Error::TrailingBytes(body.len() - COMBO_BODY_LEN)
            });
        }
        let (routing, tail) = RelayRouting::decode_from(body)?;
        let (autoplay, role) = decode_combo_tail(tail)?;
        Ok(Self {
            routing,
            autoplay,
            role,
        })
    }
}

/// One relay session member: the key pair binding a TCP session to
/// a game (relay ops 10–11 carry a `u8`-counted list of these).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayMember {
    /// First relay session key.
    pub session_key1: i32,
    /// Second relay session key.
    pub session_key2: i32,
}

/// Serialized size of one [`RelayMember`].
const RELAY_MEMBER_LEN: usize = 8;

/// Append a `u8`-counted member list to `body`.
fn encode_members(members: &[RelayMember], body: &mut Vec<u8>) {
    let count = u8::try_from(members.len()).unwrap_or(u8::MAX);
    body.push(count);
    for member in members.iter().take(usize::from(count)) {
        body.extend_from_slice(&member.session_key1.to_le_bytes());
        body.extend_from_slice(&member.session_key2.to_le_bytes());
    }
}

/// Split a `u8`-counted member list off the front of `body`.
fn decode_members(body: &[u8]) -> Result<(Vec<RelayMember>, &[u8]), Error> {
    if body.is_empty() {
        return Err(Error::TruncatedMessage { need: 1, have: 0 });
    }
    let count = usize::from(body[0]);
    let need = 1 + count * RELAY_MEMBER_LEN;
    if body.len() < need {
        return Err(Error::TruncatedMessage {
            need,
            have: body.len(),
        });
    }
    let mut members = Vec::with_capacity(count);
    for index in 0..count {
        let at = 1 + index * RELAY_MEMBER_LEN;
        members.push(RelayMember {
            session_key1: i32::from_le_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]]),
            session_key2: i32::from_le_bytes([
                body[at + 4],
                body[at + 5],
                body[at + 6],
                body[at + 7],
            ]),
        });
    }
    Ok((members, &body[need..]))
}

/// `CreateSession` 10: bind relay sessions into one game (C→S).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRelaySession {
    /// Members to bind (key pairs locate their TCP sessions).
    pub members: Vec<RelayMember>,
}

impl WireMessage for CreateRelaySession {
    const COMMAND: u16 = RelayCommand::CreateSession as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(1 + self.members.len() * RELAY_MEMBER_LEN);
        encode_members(&self.members, &mut body);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (members, rest) = decode_members(body)?;
        if !rest.is_empty() {
            return Err(Error::TrailingBytes(rest.len()));
        }
        Ok(Self { members })
    }
}

/// `DeleteSession` 11: unbind relay sessions from their game (C→S).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteRelaySession {
    /// Members to unbind.
    pub members: Vec<RelayMember>,
}

impl WireMessage for DeleteRelaySession {
    const COMMAND: u16 = RelayCommand::DeleteSession as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(1 + self.members.len() * RELAY_MEMBER_LEN);
        encode_members(&self.members, &mut body);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (members, rest) = decode_members(body)?;
        if !rest.is_empty() {
            return Err(Error::TrailingBytes(rest.len()));
        }
        Ok(Self { members })
    }
}

/// Live handshake on the game gateway: UHP punch report + relay confirm.
///
/// Unlike [`RelayCommand`] frames these ride the game TCP gateway
/// (`[u16 len][u16 id]`, `len` counts the whole frame, no routing
/// preface). Memoryer implements both pairs as fieldless log-only
/// requests with empty echo responses (`PlayingController.ConfirmRelay`
/// and `ReportPunchHole` take no request object); the exe byte proof
/// behind each shape is quoted on the type itself.
///
/// `ReportUdpPunchHole` 4046 (C→S): client reports its UDP hole-punch
/// attempt.
///
/// Byte-verified empty: the UHP controller (function entry file
/// `0x18d580`, the `UHP Start` / `UDP Hole Punching Success~!!` +
/// `Sleep(300)` poller) writes the id as `u16` `0x0FCE` at file
/// `0x18d8a1` and the length as `u16` 4 at file `0x18d8ac`, forming
/// the stack frame `04 00 CE 0F` (empty `CPacket_Str` body) ahead of
/// the session send call. The id immediate is unique image-wide, and
/// Memoryer agrees (no request fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportUdpPunchHole;

impl WireMessage for ReportUdpPunchHole {
    /// Game-gateway command id (Memoryer `RequestCommand`, `0x0FCE`).
    const COMMAND: u16 = 4046;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.is_empty() {
            Ok(Self)
        } else {
            Err(Error::TrailingBytes(body.len()))
        }
    }
}

/// `ReportUdpPunchHoleAnswer` 4047 (S→C): hole-punch report echo
/// (Memoryer `ResponseCommand.ReportUdpPunchHole`).
///
/// Byte-verified body-ignored: the S→C dispatcher (id minus base
/// `0xFA3`, bound `0x37`, byte table file `0x158a68`, address table
/// file `0x1589f4`) routes 4047 (index 44) through slot 21 (case
/// file `0x1587cb`) to the sub-handler at file `0x163d90`, which
/// sets a session flag and returns without reading any packet bytes.
/// The flag is the same word the 4048 send path checks, and Memoryer
/// sends no response fields — empty is the only consistent shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportUdpPunchHoleAnswer;

impl WireMessage for ReportUdpPunchHoleAnswer {
    /// Game-gateway command id (Memoryer `ResponseCommand`, `0x0FCF`).
    const COMMAND: u16 = 4047;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.is_empty() {
            Ok(Self)
        } else {
            Err(Error::TrailingBytes(body.len()))
        }
    }
}

/// `ConfirmRelay` 4055 (C→S): relay-path confirmation.
///
/// Byte-verified empty: the send at file `0xf0439` (`push 0` +
/// `push 0x0FD7` into the session send through vtable offset `0x84`)
/// is the sole `0x0FD7` dword site image-wide (immediate at file
/// `0xf043a`; no `66 C7 .. D7 0F` word-store, `66 3D D7 0F`, or
/// `66 81 .. D7 0F` compare site exists, so no helper-internal
/// field branch can key on the id). It fires 25 bytes after the
/// 4012 send at file `0xf0420` with only the next send's `6A 00`
/// opener between them — none of the field-staging immediates that
/// precede every 4012/4014/4016 send — while the packet-builder
/// switch (dword cases for 4016 at files `0xdd4c0`/`0xdd4cc`, 4012
/// at file `0xdd51d`, 4014 at file `0xdd529`) holds no `0x0FD7`
/// case. Encoded empty per Memoryer (`PlayingController.ConfirmRelay`
/// takes no request fields, log-only) — see `docs/relay.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmRelay;

impl WireMessage for ConfirmRelay {
    /// Game-gateway command id (Memoryer `RequestCommand`, `0x0FD7`).
    const COMMAND: u16 = 4055;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.is_empty() {
            Ok(Self)
        } else {
            Err(Error::TrailingBytes(body.len()))
        }
    }
}

/// `ConfirmRelayAnswer` 4056 (S→C): relay-confirm echo (Memoryer
/// `ResponseCommand.ConfirmRelay`).
///
/// Byte-verified body-ignored: the S→C dispatcher routes 4056
/// (index 53) through slot 26 (case file `0x158820`) to the
/// sub-handler at file `0x163d50`, which forwards only immediates
/// (`0, 0, 0x0FD8, 0x12`) to the UI event queue and returns — the
/// same body-ignoring shape as the 4052/4054 forwarders at files
/// `0x163d10`/`0x163d30`. A game-state compare against `0x0FD8`
/// also sits at file `0xde4db`. Memoryer sends no response fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmRelayAnswer;

impl WireMessage for ConfirmRelayAnswer {
    /// Game-gateway command id (Memoryer `ResponseCommand`, `0x0FD8`).
    const COMMAND: u16 = 4056;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.is_empty() {
            Ok(Self)
        } else {
            Err(Error::TrailingBytes(body.len()))
        }
    }
}

/// Live-state setup on the game gateway: `GetLiveState` 4042 (C→S),
/// `LiveStateResponse` 4043 (S→C), `GetP2PList` 4044 (C→S),
/// `P2PListResponse` 4045 (S→C).
///
/// The two requests are empty: each send site pushes only `0` plus the
/// id ahead of the session send (`GetLiveState` at file `0xc365c`, gated
/// on the global byte at `0x7789e6` equal to 1, vtable slot `+0x5c`;
/// `GetP2PList` at file `0xead18`, gated on the Live-mode byte at
/// `0x77cd33` equal to 1 plus a once-flag at session `+0x89c`, vtable
/// slot `+0x84`). Each id's `push`-immediate is the sole image-wide site
/// (`ev_dc7171af6c958b25a6b37cc34a1cb0ad`, observed); `0x0FCD` has no
/// `push`-immediate or word-compare site at all, so 4045 is S→C-only.
///
/// The responses arrive through the S→C dispatcher (id minus base `0xFA3`):
/// 4043 takes slot 19 (case file `0x1587a9`, handler entry file `0x159b50`),
/// 4045 takes slot 20 (case file `0x1587ba`, handler entry file `0x163d70`,
/// which forwards the packet into the relay-side parser at file `0x18cf50`).
/// The sibling C→S ids 4042/4044 map to the default slot 28 (no S→C
/// handler), as do the other C→S-only live ids. Layouts below mirror
/// Memoryer's `LiveStateResponse` / `P2PListResponse` field orders (clean-room
/// C#, v7.2.0); every width, bound, and read order is exe-pinned (see
/// `docs/relay.md` §10), except the enum semantics, which stay OPEN.
/// (ERRORS-06: no `unwrap`; PERF-07: `try_from` for narrowing conversions.)
///
/// `GetLiveState` 4042 (C→S): ask the server for the room's live snapshot.
///
/// Byte-verified empty: the send at file `0xc365c` (`push 0` +
/// `push 0x0FCA` into the session send through vtable offset `+0x5c`)
/// is the sole `0x0FCA` `push`-immediate image-wide, fires only when the
/// global byte at `0x7789e6` equals 1, and stages no fields.
/// (`ev_f133f3bf30eb77f74ddc57f2c81d8783`, observed).
/// See `docs/relay.md` §10.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetLiveState;

impl WireMessage for GetLiveState {
    /// Game-gateway command id (Memoryer `RequestCommand`, `0x0FCA`).
    const COMMAND: u16 = 4042;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.is_empty() {
            Ok(Self)
        } else {
            Err(Error::TrailingBytes(body.len()))
        }
    }
}

/// `GetP2PList` 4044 (C→S): ask the server for the room's peer endpoints.
///
/// Byte-verified empty: the send at file `0xead18` (`push 0` +
/// `push 0x0FCC` into the session send through vtable offset `+0x84`)
/// is the sole `0x0FCC` `push`-immediate image-wide, fires only when the
/// Live-mode byte at `0x77cd33` equals 1 and the session once-flag at
/// `+0x89c` is still 0 (then sets it to 1), and stages no fields — the
/// same empty shape as the 4042 send (`ev_70b81e37e5e8d6f15ec316c1b93b3947`,
/// observed). See `docs/relay.md` §10.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetP2PList;

impl WireMessage for GetP2PList {
    /// Game-gateway command id (Memoryer `RequestCommand`, `0x0FCC`).
    const COMMAND: u16 = 4044;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.is_empty() {
            Ok(Self)
        } else {
            Err(Error::TrailingBytes(body.len()))
        }
    }
}

/// Fixed member-slot count of [`LiveStateResponse`]: the handler loops
/// slot indices `0..8` (bound at file `0x159cf9`).
const LIVE_STATE_SLOTS: usize = 8;
/// Fixed equipment words per active [`MemberInfo`]: one `i32` plus a
/// 16-read loop (`0x11` bound at file `0x140eb1`).
const LIVE_EQUIPMENT_WORDS: usize = 17;
/// Serialized size of one [`PeerInfo`]: `i32` member + two 6-byte endpoints.
const PEER_ENTRY_LEN: usize = 16;

/// Split one byte off the front of `body`.
fn take_byte(body: &mut &[u8]) -> Result<u8, Error> {
    let (&first, rest) = body
        .split_first()
        .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
    *body = rest;
    Ok(first)
}

/// Split one little-endian `u16` off the front of `body`.
fn take_le_u16(body: &mut &[u8]) -> Result<u16, Error> {
    if body.len() < 2 {
        return Err(Error::TruncatedMessage {
            need: 2,
            have: body.len(),
        });
    }
    let (word, rest) = body.split_at(2);
    *body = rest;
    Ok(u16::from_le_bytes([word[0], word[1]]))
}

/// Split one little-endian `i32` off the front of `body`.
fn take_le_i32(body: &mut &[u8]) -> Result<i32, Error> {
    if body.len() < 4 {
        return Err(Error::TruncatedMessage {
            need: 4,
            have: body.len(),
        });
    }
    let (word, rest) = body.split_at(4);
    *body = rest;
    Ok(i32::from_le_bytes([word[0], word[1], word[2], word[3]]))
}

/// Split one `0`/`1` byte off the front of `body` as bool.
fn take_flag(body: &mut &[u8], field: &'static str) -> Result<bool, Error> {
    match take_byte(body)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::InvalidField {
            field,
            reason: "not 0 or 1",
        }),
    }
}

/// Clamp a wire count to a length: negative bounds skip the exe's copy
/// loops (signed exit test), so they decode as empty on the client.
fn nonnegative_count(raw: i32) -> usize {
    usize::try_from(raw.max(0)).unwrap_or(0)
}

/// One member slot of [`LiveStateResponse`]: the per-slot `active` byte
/// gates the [`MemberInfo`] tail (only `1` continues; any other byte takes
/// the skip path, file `0x159d24`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemberLiveState {
    /// True when the slot carries a [`MemberInfo`] tail.
    pub active: bool,
    /// Member detail; `Some` exactly when `active`.
    pub info: Option<MemberInfo>,
}

/// One room member's live detail (S→C 4043 tail, per active slot).
///
/// Wire order is the handler's read order (shared fixed-size reader at
/// file `0x155d20`, sizes below; `ev_e781f2560f769dc9425a997e25f263dd`,
/// observed): `u8` id, null-terminated nickname, `i32` level, `u8` gender
/// (stored raw; the handler only branches zero/nonzero, file `0x13fe50`),
/// `i32` gem, three `u8` seat bytes, 17 `i32` equipment words, `i32` count
/// plus `u16` music ids, two `i32` counters, four trailing `u8` bytes,
/// `i32` win streak. The trailing run holds
/// (`playing_state`, `live_extra`, `is_administrator`, `is_room_master`):
/// the exe reads slot `+0x2131` twice in a row (files `0x141670`/`0x141690`),
/// so the first copy is discarded in memory — `live_extra` preserves that
/// wire byte, which has no Memoryer v7.2.0 counterpart (likely 7.55 drift).
/// Enum semantics (gender/team/music/playing/free-pass values) stay OPEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    /// Slot member id (`u8`).
    pub member_id: u8,
    /// Nickname, null-terminated UTF-8 (reader at file `0x155ea0`
    /// measures to the terminator, then consumes it).
    pub nickname: String,
    /// Level (`i32`).
    pub level: i32,
    /// Gender byte, raw (no range gate in-exe).
    pub gender: u8,
    /// Gem count (`i32`).
    pub gem: i32,
    /// Team byte, raw.
    pub team: u8,
    /// Ready flag.
    pub ready: bool,
    /// Music-state byte, raw.
    pub music_state: u8,
    /// Equipment words (17 `i32`, `0x11`-bounded loop).
    pub equipments: [i32; LIVE_EQUIPMENT_WORDS],
    /// Owned music ids (`i32` count plus `u16` entries).
    pub music_ids: Vec<u16>,
    /// Cash points (`i32`).
    pub cash_point: i32,
    /// Free-pass counter (`i32`).
    pub free_pass: i32,
    /// Playing-state byte, raw.
    pub playing_state: u8,
    /// Extra wire byte of the trailing run (see above; semantics OPEN).
    pub live_extra: u8,
    /// Administrator flag.
    pub is_administrator: bool,
    /// Room-master flag.
    pub is_room_master: bool,
    /// Win streak (`i32`, read inline at file `0x159f61`).
    pub win_streak: i32,
}

impl MemberInfo {
    /// Parse from the front of `body`, advancing the cursor past the tail.
    fn decode_prefix(body: &mut &[u8]) -> Result<Self, Error> {
        let member_id = take_byte(body)?;
        let (nickname, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
        *body = &body[used..];
        let level = take_le_i32(body)?;
        let gender = take_byte(body)?;
        let gem = take_le_i32(body)?;
        let team = take_byte(body)?;
        let ready = take_flag(body, "member ready flag")?;
        let music_state = take_byte(body)?;
        let mut equipments = [0i32; LIVE_EQUIPMENT_WORDS];
        for slot in equipments.iter_mut() {
            *slot = take_le_i32(body)?;
        }
        let music_raw = take_le_i32(body)?;
        let music_count = nonnegative_count(music_raw);
        let music_need = music_count.checked_mul(2).ok_or(Error::InvalidField {
            field: "member music ids",
            reason: "count too large",
        })?;
        if body.len() < music_need {
            return Err(Error::TruncatedMessage {
                need: music_need,
                have: body.len(),
            });
        }
        let mut music_ids = Vec::with_capacity(music_count);
        for _ in 0..music_count {
            music_ids.push(take_le_u16(body)?);
        }
        let cash_point = take_le_i32(body)?;
        let free_pass = take_le_i32(body)?;
        let playing_state = take_byte(body)?;
        let live_extra = take_byte(body)?;
        let is_administrator = take_flag(body, "member administrator flag")?;
        let is_room_master = take_flag(body, "member room-master flag")?;
        let win_streak = take_le_i32(body)?;
        Ok(Self {
            member_id,
            nickname,
            level,
            gender,
            gem,
            team,
            ready,
            music_state,
            equipments,
            music_ids,
            cash_point,
            free_pass,
            playing_state,
            live_extra,
            is_administrator,
            is_room_master,
            win_streak,
        })
    }

    /// Append the full tail for sending (tests and future use).
    fn encode_inner(&self, body: &mut Vec<u8>) {
        body.push(self.member_id);
        body.extend_from_slice(&encode_null_terminated(&self.nickname, MAX_STRING_LEN));
        body.extend_from_slice(&self.level.to_le_bytes());
        body.push(self.gender);
        body.extend_from_slice(&self.gem.to_le_bytes());
        body.push(self.team);
        body.push(u8::from(self.ready));
        body.push(self.music_state);
        for word in self.equipments {
            body.extend_from_slice(&word.to_le_bytes());
        }
        let music_count = i32::try_from(self.music_ids.len()).unwrap_or(i32::MAX);
        body.extend_from_slice(&music_count.to_le_bytes());
        for id in &self.music_ids {
            body.extend_from_slice(&id.to_le_bytes());
        }
        body.extend_from_slice(&self.cash_point.to_le_bytes());
        body.extend_from_slice(&self.free_pass.to_le_bytes());
        body.push(self.playing_state);
        body.push(self.live_extra);
        body.push(u8::from(self.is_administrator));
        body.push(u8::from(self.is_room_master));
        body.extend_from_slice(&self.win_streak.to_le_bytes());
    }
}

/// `LiveStateResponse` 4043 (S→C): room live snapshot — header plus eight
/// fixed member slots, each starting with the `active` byte.
///
/// Byte-verified structure: the handler at file `0x159b50` reads `u8`
/// member id, `i32` user count, `i32` win streak through the shared reader
/// (files `0x159b5d`/`0x159b6b`/`0x159b79`), rejects member ids `>= 8`
/// (check at file `0x159b87`, error path at file `0x159fc2` returns without
/// forwarding), then loops the eight slots (bound at file `0x159cf9`):
/// each slot's `u8` active byte (read at file `0x159d0a`) gates some
/// fifteen field reads, and completion forwards `(0, 0, 0x0FCB, 0x12)` to
/// the UI queue (file `0x159fb0`; `ev_55248e1f2c72c6ef9d26603f94925bf2`,
/// observed — same shape as the 4056 forwarder).
/// (`ev_83bc05cdd0c1b7b5fb9313f9a1f485e6`,
/// `ev_5396edb7c26f6ac2aa78c743fff3905f`, observed.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveStateResponse {
    /// Requesting member id (`u8`, must be below 8).
    pub member_id: u8,
    /// Room user count (`i32`).
    pub user_count: i32,
    /// Room win streak (`i32`).
    pub win_streak: i32,
    /// Eight fixed member slots in wire order.
    pub members: [MemberLiveState; LIVE_STATE_SLOTS],
}

impl WireMessage for LiveStateResponse {
    const COMMAND: u16 = 4043;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::new();
        body.push(self.member_id);
        body.extend_from_slice(&self.user_count.to_le_bytes());
        body.extend_from_slice(&self.win_streak.to_le_bytes());
        for slot in &self.members {
            body.push(u8::from(slot.active));
            if let Some(info) = &slot.info {
                info.encode_inner(&mut body);
            }
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut tail = body;
        let member_id = take_byte(&mut tail)?;
        if usize::from(member_id) >= LIVE_STATE_SLOTS {
            return Err(Error::InvalidField {
                field: "live member id",
                reason: "not 0-7",
            });
        }
        let user_count = take_le_i32(&mut tail)?;
        let win_streak = take_le_i32(&mut tail)?;
        let mut members: [MemberLiveState; LIVE_STATE_SLOTS] = Default::default();
        for slot in members.iter_mut() {
            let active_byte = take_byte(&mut tail)?;
            if active_byte == 1 {
                slot.active = true;
                slot.info = Some(MemberInfo::decode_prefix(&mut tail)?);
            }
        }
        if !tail.is_empty() {
            return Err(Error::TrailingBytes(tail.len()));
        }
        Ok(Self {
            member_id,
            user_count,
            win_streak,
            members,
        })
    }
}

/// One peer of [`P2PListResponse`]: member key plus the two 6-byte
/// endpoints (public, local) the TCP peer leg dials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerInfo {
    /// Peer member key (`i32` on the wire: Memoryer's `MessageFieldCodec`
    /// for this `Byte` property is `Int32`, and the exe reads 4 bytes).
    pub member_id: i32,
    /// Public endpoint (4-byte IP + `u16` port).
    pub public_endpoint: SocketAddrV4,
    /// Local endpoint (4-byte IP + `u16` port).
    pub local_endpoint: SocketAddrV4,
}

/// `P2PListResponse` 4045 (S→C): the room's peer list carrying the TCP
/// peer-leg ports.
///
/// Byte-verified shape: the relay-side parser at file `0x18cf50` reads an
/// `i32` peer count through the shared reader (file `0x18cf9d`), loops
/// while the index stays below it (signed bound, file `0x18cfb7` — a
/// negative count yields zero peers, as on the client), and per peer
/// reads `i32` member, 6-byte public, 6-byte local (files
/// `0x18cfc5`/`0x18cfd3`/`0x18cfe1`), staging each endpoint as a dword IP
/// plus a word port (files `0x18cfeb`/`0x18cff0`, `0x18cffd`/`0x18d002`)
/// into the per-peer record call (file `0x18d00a`).
/// (`ev_0a144946a7ac801729031065ad78008d`, observed.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P2PListResponse {
    /// Peers in server order.
    pub peers: Vec<PeerInfo>,
}

impl WireMessage for P2PListResponse {
    const COMMAND: u16 = 4045;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + self.peers.len() * PEER_ENTRY_LEN);
        let count = i32::try_from(self.peers.len()).unwrap_or(i32::MAX);
        body.extend_from_slice(&count.to_le_bytes());
        for peer in &self.peers {
            body.extend_from_slice(&peer.member_id.to_le_bytes());
            encode_socket(peer.public_endpoint, &mut body);
            encode_socket(peer.local_endpoint, &mut body);
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        let raw = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let count = nonnegative_count(raw);
        let need = count
            .checked_mul(PEER_ENTRY_LEN)
            .and_then(|entries| entries.checked_add(4))
            .ok_or(Error::InvalidField {
                field: "peer count",
                reason: "count too large",
            })?;
        if body.len() < need {
            return Err(Error::TruncatedMessage {
                need,
                have: body.len(),
            });
        }
        if body.len() > need {
            return Err(Error::TrailingBytes(body.len() - need));
        }
        let mut peers = Vec::with_capacity(count);
        let mut tail = &body[4..];
        for _ in 0..count {
            let member_id = i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
            let (public_endpoint, rest) = decode_socket(&tail[4..])?;
            let (local_endpoint, rest) = decode_socket(rest)?;
            peers.push(PeerInfo {
                member_id,
                public_endpoint,
                local_endpoint,
            });
            tail = rest;
        }
        Ok(Self { peers })
    }
}

/// RUDP inner packet types (`Memoryer.Relay` `UdpRelayPacketType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UdpPacketType {
    /// Fire-and-forget (Live lane traffic).
    Unreliable = 1,
    /// Retransmitted until acked (stats, combo packs).
    Reliable = 2,
    /// Bare 4-byte ack, never zlib-wrapped.
    Ack = 3,
}

impl UdpPacketType {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse a wire byte; unknown types are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside 1–3.
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            1 => Ok(Self::Unreliable),
            2 => Ok(Self::Reliable),
            3 => Ok(Self::Ack),
            _ => Err(Error::InvalidField {
                field: "udp packet type",
                reason: "not 1, 2, or 3",
            }),
        }
    }
}

/// One decoded RUDP datagram.
///
/// `payload` is the relay command frame the inner envelope wraps:
/// `[u16 command][body]` (the envelope's `u32` command word and
/// `innerSize` framing are validated, then stripped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpRelayFrame {
    /// Inner packet type.
    pub packet_type: UdpPacketType,
    /// Inner sequence number (acked by [`UdpRelayFrame::encode_ack`]).
    pub sequence: u8,
    /// Relay command frame: `[u16 command][body]`.
    pub payload: Vec<u8>,
}

/// Bare ack length: `[0x03][seq][0x00][0x00]`, no zlib, no prefix.
const ACK_FRAME_LEN: usize = 4;
/// Datagram length prefix: `u16` uncompressed length.
const DATAGRAM_LEN_LEN: usize = 2;
/// Inner envelope: type + seq + size + `u32` command.
const INNER_HEADER_LEN: usize = 8;

impl UdpRelayFrame {
    /// Decode one UDP datagram: bare ack or length-prefixed zlib.
    ///
    /// # Errors
    ///
    /// Truncation on short input, `Error::InvalidField` on bad
    /// packet types, sizes, commands, or zlib checksums.
    pub fn decode_datagram(datagram: &[u8]) -> Result<Self, Error> {
        if datagram.len() == ACK_FRAME_LEN && datagram[0] == UdpPacketType::Ack.as_u8() {
            return Ok(Self {
                packet_type: UdpPacketType::Ack,
                sequence: datagram[1],
                payload: Vec::new(),
            });
        }
        if datagram.len() < DATAGRAM_LEN_LEN {
            return Err(Error::TruncatedMessage {
                need: DATAGRAM_LEN_LEN,
                have: datagram.len(),
            });
        }
        let uncompressed_len = u16::from_le_bytes([datagram[0], datagram[1]]) as usize;
        let inflated = decompress_zlib(&datagram[DATAGRAM_LEN_LEN..])?;
        if inflated.len() < uncompressed_len {
            return Err(Error::TruncatedMessage {
                need: uncompressed_len,
                have: inflated.len(),
            });
        }
        if inflated.len() > uncompressed_len {
            return Err(Error::TrailingBytes(inflated.len() - uncompressed_len));
        }
        Self::decode_inner(&inflated)
    }

    /// Parse the inflated inner envelope.
    fn decode_inner(inner: &[u8]) -> Result<Self, Error> {
        if inner.len() < INNER_HEADER_LEN {
            return Err(Error::TruncatedMessage {
                need: INNER_HEADER_LEN,
                have: inner.len(),
            });
        }
        let packet_type = UdpPacketType::from_u8(inner[0])?;
        let sequence = inner[1];
        let inner_size = u16::from_le_bytes([inner[2], inner[3]]) as usize;
        if inner_size < 4 {
            return Err(Error::InvalidField {
                field: "relay inner size",
                reason: "smaller than command word",
            });
        }
        let command_word = u32::from_le_bytes([inner[4], inner[5], inner[6], inner[7]]);
        let command = u16::try_from(command_word).map_err(|_| Error::InvalidField {
            field: "relay command",
            reason: "wider than u16",
        })?;
        let body_len = inner_size - 4;
        if inner.len() < INNER_HEADER_LEN + body_len {
            return Err(Error::TruncatedMessage {
                need: INNER_HEADER_LEN + body_len,
                have: inner.len(),
            });
        }
        if inner.len() > INNER_HEADER_LEN + body_len {
            return Err(Error::TrailingBytes(
                inner.len() - INNER_HEADER_LEN - body_len,
            ));
        }
        let mut payload = Vec::with_capacity(2 + body_len);
        payload.extend_from_slice(&command.to_le_bytes());
        payload.extend_from_slice(&inner[INNER_HEADER_LEN..INNER_HEADER_LEN + body_len]);
        Ok(Self {
            packet_type,
            sequence,
            payload,
        })
    }

    /// Build one UDP datagram: inner envelope + stored-block zlib +
    /// `u16` length prefix. Use [`UdpRelayFrame::encode_ack`] for acks.
    #[must_use]
    pub fn encode_datagram(
        packet_type: UdpPacketType,
        sequence: u8,
        command: u16,
        body: &[u8],
    ) -> Vec<u8> {
        let inner_size = u16::try_from(body.len() + 4).unwrap_or(u16::MAX);
        let mut inner = Vec::with_capacity(INNER_HEADER_LEN + body.len());
        inner.push(packet_type.as_u8());
        inner.push(sequence);
        inner.extend_from_slice(&inner_size.to_le_bytes());
        inner.extend_from_slice(&u32::from(command).to_le_bytes());
        inner.extend_from_slice(body);
        let compressed = compress_zlib_stored(&inner);
        let mut datagram = Vec::with_capacity(DATAGRAM_LEN_LEN + compressed.len());
        let prefix = u16::try_from(inner.len()).unwrap_or(u16::MAX);
        datagram.extend_from_slice(&prefix.to_le_bytes());
        datagram.extend_from_slice(&compressed);
        datagram
    }

    /// Build a bare 4-byte ack for `sequence`.
    #[must_use]
    pub fn encode_ack(sequence: u8) -> [u8; ACK_FRAME_LEN] {
        [UdpPacketType::Ack.as_u8(), sequence, 0x00, 0x00]
    }

    /// Relay command id carried in `payload`, if present.
    #[must_use]
    pub fn command_id(&self) -> Option<u16> {
        if self.payload.len() < 2 {
            return None;
        }
        Some(u16::from_le_bytes([self.payload[0], self.payload[1]]))
    }

    /// Relay command carried in `payload`, if known.
    #[must_use]
    pub fn relay_command(&self) -> Option<RelayCommand> {
        self.command_id().and_then(RelayCommand::from_id)
    }

    /// Relay command body (bytes after the command id).
    #[must_use]
    pub fn body(&self) -> &[u8] {
        if self.payload.len() < 2 {
            return &[];
        }
        &self.payload[2..]
    }
}

#[cfg(test)]
mod relay_command_tests {
    use super::*;
    use crate::frame::decode_frame;

    fn routing() -> RelayRouting {
        RelayRouting {
            code: 0x0102_0304,
            relays: [
                SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 5678),
                SocketAddrV4::new(Ipv4Addr::new(5, 6, 7, 8), 9012),
                SocketAddrV4::new(Ipv4Addr::new(9, 10, 11, 12), 3456),
            ],
            unused: 0,
        }
    }

    #[test]
    fn resolves_all_relay_command_ids() {
        assert_eq!(RelayCommand::from_id(1), Some(RelayCommand::PeerConnected));
        assert_eq!(RelayCommand::from_id(2), Some(RelayCommand::PeerConfirm));
        assert_eq!(
            RelayCommand::from_id(3),
            Some(RelayCommand::PeerEndpointAssigned)
        );
        assert_eq!(RelayCommand::from_id(4), Some(RelayCommand::Ping));
        assert_eq!(
            RelayCommand::from_id(5),
            Some(RelayCommand::UpdateGameStats)
        );
        assert_eq!(RelayCommand::from_id(6), Some(RelayCommand::ComboBroken));
        assert_eq!(RelayCommand::from_id(7), Some(RelayCommand::ComboStarted));
        assert_eq!(RelayCommand::from_id(10), Some(RelayCommand::CreateSession));
        assert_eq!(RelayCommand::from_id(11), Some(RelayCommand::DeleteSession));
        assert_eq!(RelayCommand::from_id(8), None);
        assert_eq!(RelayCommand::UpdateGameStats.id(), 5);
    }

    #[test]
    fn relay_udp_ports_match_exe_selector_immediates() {
        assert_eq!(RELAY_UDP_PORTS, [0x998F, 0x9990]);
        assert_eq!(RELAY_UDP_PORTS[0], 39311);
        assert_eq!(RELAY_UDP_PORTS[1], 39312);
        assert_ne!(RELAY_UDP_PORTS[0], RELAY_UDP_PORTS[1]);
    }

    #[test]
    fn member_role_names_all_seats() {
        assert_eq!(MemberRole::from_u16(0), Ok(MemberRole::None));
        assert_eq!(MemberRole::from_u16(1), Ok(MemberRole::Champion));
        assert_eq!(MemberRole::from_u16(2), Ok(MemberRole::Challenger));
        assert_eq!(MemberRole::from_u16(3), Ok(MemberRole::Spectator));
        assert_eq!(MemberRole::Challenger.as_u16(), 2);
    }

    #[test]
    fn member_role_rejects_unknown_seat() {
        assert_eq!(
            MemberRole::from_u16(9),
            Err(Error::InvalidField {
                field: "member role",
                reason: "not 0, 1, 2, or 3"
            })
        );
    }

    #[test]
    fn peer_connected_encodes_session_keys() {
        let connected = PeerConnected {
            routing: routing(),
            session_key1: 0x1111_1111,
            session_key2: 0x2222_2222,
        };
        assert_eq!(
            connected.encode_body(),
            vec![
                0x04, 0x03, 0x02, 0x01, 0x01, 0x02, 0x03, 0x04, 0x2E, 0x16, 0x05, 0x06, 0x07, 0x08,
                0x34, 0x23, 0x09, 0x0A, 0x0B, 0x0C, 0x80, 0x0D, 0x00, 0x00, 0x11, 0x11, 0x11, 0x11,
                0x22, 0x22, 0x22, 0x22,
            ]
        );
        let frame = decode_frame(&connected.encode()).unwrap();
        assert_eq!(frame.id, 1);
        assert_eq!(PeerConnected::decode_frame(&frame), Ok(connected));
    }

    #[test]
    fn peer_connected_rejects_short_body() {
        assert_eq!(
            PeerConnected::decode_body(&[0x01; 31]),
            Err(Error::TruncatedMessage { need: 32, have: 31 })
        );
    }

    #[test]
    fn peer_confirm_encodes_both_endpoints() {
        let confirm = PeerConfirm {
            routing: RelayRouting {
                code: 7,
                relays: routing().relays,
                unused: 0,
            },
            session_key1: 0x0102_0304,
            session_key2: 0x0506_0708,
            local: ConnectionEndpoint {
                port1: 1111,
                port2: 2222,
                address: Ipv4Addr::new(192, 168, 0, 2),
            },
            public: ConnectionEndpoint {
                port1: 3333,
                port2: 4444,
                address: Ipv4Addr::new(10, 0, 0, 1),
            },
        };
        assert_eq!(
            confirm.encode_body(),
            vec![
                0x07, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x2E, 0x16, 0x05, 0x06, 0x07, 0x08,
                0x34, 0x23, 0x09, 0x0A, 0x0B, 0x0C, 0x80, 0x0D, 0x00, 0x00, 0x04, 0x03, 0x02, 0x01,
                0x08, 0x07, 0x06, 0x05, 0x57, 0x04, 0xAE, 0x08, 0xC0, 0xA8, 0x00, 0x02, 0x05, 0x0D,
                0x5C, 0x11, 0x0A, 0x00, 0x00, 0x01,
            ]
        );
        let frame = decode_frame(&confirm.encode()).unwrap();
        assert_eq!(frame.id, 2);
        assert_eq!(PeerConfirm::decode_frame(&frame), Ok(confirm));
    }

    #[test]
    fn peer_endpoint_assigned_encodes_public_endpoint() {
        let assigned = PeerEndpointAssigned {
            routing: RelayRouting {
                code: 0,
                relays: routing().relays,
                unused: 0,
            },
            public: ConnectionEndpoint {
                port1: 5555,
                port2: 6666,
                address: Ipv4Addr::new(203, 238, 136, 184),
            },
        };
        assert_eq!(
            assigned.encode_body(),
            vec![
                0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x2E, 0x16, 0x05, 0x06, 0x07, 0x08,
                0x34, 0x23, 0x09, 0x0A, 0x0B, 0x0C, 0x80, 0x0D, 0x00, 0x00, 0xB3, 0x15, 0x0A, 0x1A,
                0xCB, 0xEE, 0x88, 0xB8,
            ]
        );
        let frame = decode_frame(&assigned.encode()).unwrap();
        assert_eq!(frame.id, 3);
        assert_eq!(PeerEndpointAssigned::decode_frame(&frame), Ok(assigned));
    }

    #[test]
    fn relay_ping_encodes_start_and_tick() {
        let ping = RelayPing {
            routing: RelayRouting {
                code: 0x0A0B_0C0D,
                relays: routing().relays,
                unused: 0,
            },
            start: 0xDEAD_BEEF,
            tick: 0x1234_5678,
        };
        assert_eq!(
            ping.encode_body(),
            vec![
                0x0D, 0x0C, 0x0B, 0x0A, 0x01, 0x02, 0x03, 0x04, 0x2E, 0x16, 0x05, 0x06, 0x07, 0x08,
                0x34, 0x23, 0x09, 0x0A, 0x0B, 0x0C, 0x80, 0x0D, 0x00, 0x00, 0xEF, 0xBE, 0xAD, 0xDE,
                0x78, 0x56, 0x34, 0x12,
            ]
        );
        let frame = decode_frame(&ping.encode()).unwrap();
        assert_eq!(frame.id, 4);
        assert_eq!(RelayPing::decode_frame(&frame), Ok(ping));
    }

    fn full_tally() -> RelayStats {
        RelayStats {
            routing: routing(),
            role: MemberRole::Challenger,
            cool: 10,
            good: 20,
            bad: 30,
            miss: 40,
            combo: 50,
            max_combo: 60,
            jam_combo: 70,
            total_jam_combo: 80,
            long_note_combo: 90,
            long_note_score: 0x1122_3344,
            life: 1000,
            pill_buildup: 5,
            pill_count: 3,
            score: 0x5566_7788,
        }
    }

    #[test]
    fn relay_stats_encodes_full_tally() {
        assert_eq!(
            full_tally().encode_body(),
            vec![
                0x04, 0x03, 0x02, 0x01, 0x01, 0x02, 0x03, 0x04, 0x2E, 0x16, 0x05, 0x06, 0x07, 0x08,
                0x34, 0x23, 0x09, 0x0A, 0x0B, 0x0C, 0x80, 0x0D, 0x00, 0x00, 0x02, 0x00, 0x0A, 0x00,
                0x14, 0x00, 0x1E, 0x00, 0x28, 0x00, 0x32, 0x00, 0x3C, 0x00, 0x46, 0x00, 0x50, 0x00,
                0x5A, 0x00, 0x44, 0x33, 0x22, 0x11, 0xE8, 0x03, 0x05, 0x00, 0x00, 0x00, 0x03, 0x00,
                0x00, 0x00, 0x88, 0x77, 0x66, 0x55,
            ]
        );
    }

    #[test]
    fn relay_stats_round_trips_through_frame() {
        let frame = decode_frame(&full_tally().encode()).unwrap();
        assert_eq!(frame.id, 5);
        assert_eq!(RelayStats::decode_frame(&frame), Ok(full_tally()));
    }

    #[test]
    fn relay_stats_rejects_unknown_role() {
        let mut body = full_tally().encode_body();
        body[24] = 0x09;
        assert_eq!(
            RelayStats::decode_body(&body),
            Err(Error::InvalidField {
                field: "member role",
                reason: "not 0, 1, 2, or 3"
            })
        );
    }

    #[test]
    fn relay_stats_rejects_trailing_bytes() {
        let mut body = full_tally().encode_body();
        body.push(0x00);
        assert_eq!(RelayStats::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn relay_op5_tcp_frame_pins_shared_header_shape() {
        use crate::frame::HEADER_LEN;
        let frame_bytes = full_tally().encode();
        assert_eq!(frame_bytes.len(), HEADER_LEN + 62);
        assert_eq!(&frame_bytes[..HEADER_LEN], &[0x42, 0x00, 0x05, 0x00]);
        let frame = decode_frame(&frame_bytes).unwrap();
        assert_eq!(frame.id, 5);
        assert_eq!(RelayStats::decode_frame(&frame), Ok(full_tally()));
    }

    #[test]
    fn relay_op5_tally_is_not_a_gateway_4014_echo() {
        use crate::play::UpdateGameStats;
        let tally_body = full_tally().encode_body();
        let tick = UpdateGameStats::jam_tick(3, 1, 210, 0);
        let tick_body = tick.encode_body();
        assert_eq!(tally_body.len(), 62);
        assert_eq!(tick_body.len(), 16);
        assert_eq!(full_tally().encode().len(), 66);
        assert_eq!(tick.encode().len(), 20);
        assert_eq!(
            RelayStats::decode_body(&tick_body),
            Err(Error::TruncatedMessage { need: 62, have: 16 })
        );
    }

    #[test]
    fn hardcoded_relay_ports_are_the_udp_pair_only() {
        use crate::client::DEFAULT_GATEWAY_PORT;
        assert_eq!(RELAY_UDP_PORTS, [39311, 39312]);
        for port in RELAY_UDP_PORTS {
            assert_ne!(port, DEFAULT_GATEWAY_PORT);
            assert_ne!(port, ADJUNCT_SETUP_NUMBER);
        }
    }

    #[test]
    fn combo_broken_encodes_miss_pack() {
        let broken = ComboBroken {
            routing: routing(),
            autoplay: true,
            role: MemberRole::Challenger,
        };
        assert_eq!(
            broken.encode_body(),
            vec![
                0x04, 0x03, 0x02, 0x01, 0x01, 0x02, 0x03, 0x04, 0x2E, 0x16, 0x05, 0x06, 0x07, 0x08,
                0x34, 0x23, 0x09, 0x0A, 0x0B, 0x0C, 0x80, 0x0D, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                0x02, 0x00,
            ]
        );
        let frame = decode_frame(&broken.encode()).unwrap();
        assert_eq!(frame.id, 6);
        assert_eq!(ComboBroken::decode_frame(&frame), Ok(broken));
    }

    #[test]
    fn combo_started_encodes_cool_pack() {
        let started = ComboStarted {
            routing: routing(),
            autoplay: true,
            role: MemberRole::Champion,
        };
        assert_eq!(
            started.encode_body(),
            vec![
                0x04, 0x03, 0x02, 0x01, 0x01, 0x02, 0x03, 0x04, 0x2E, 0x16, 0x05, 0x06, 0x07, 0x08,
                0x34, 0x23, 0x09, 0x0A, 0x0B, 0x0C, 0x80, 0x0D, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                0x01, 0x00,
            ]
        );
        let frame = decode_frame(&started.encode()).unwrap();
        assert_eq!(frame.id, 7);
        assert_eq!(ComboStarted::decode_frame(&frame), Ok(started));
    }

    #[test]
    fn combo_packs_reject_bad_autoplay_flag() {
        let mut body = ComboBroken {
            routing: routing(),
            autoplay: true,
            role: MemberRole::None,
        }
        .encode_body();
        body[24] = 0x02;
        assert_eq!(
            ComboBroken::decode_body(&body),
            Err(Error::InvalidField {
                field: "combo autoplay flag",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn combo_packs_reject_short_body() {
        assert_eq!(
            ComboStarted::decode_body(&[0x01; 29]),
            Err(Error::TruncatedMessage { need: 30, have: 29 })
        );
    }

    fn zero_combo_routing() -> RelayRouting {
        RelayRouting {
            code: 0,
            relays: routing().relays,
            unused: 0,
        }
    }

    #[test]
    fn combo_code_zero_sits_at_body_offset_0() {
        let broken = ComboBroken {
            routing: zero_combo_routing(),
            autoplay: true,
            role: MemberRole::None,
        };
        assert_eq!(&broken.encode_body()[0..4], &[0x00, 0x00, 0x00, 0x00]);
        let started = ComboStarted {
            routing: zero_combo_routing(),
            autoplay: true,
            role: MemberRole::None,
        };
        assert_eq!(&started.encode_body()[0..4], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn combo_autoplay_one_sits_at_body_offset_24() {
        let broken = ComboBroken {
            routing: zero_combo_routing(),
            autoplay: true,
            role: MemberRole::None,
        };
        assert_eq!(&broken.encode_body()[24..28], &[0x01, 0x00, 0x00, 0x00]);
        let started = ComboStarted {
            routing: zero_combo_routing(),
            autoplay: true,
            role: MemberRole::None,
        };
        assert_eq!(&started.encode_body()[24..28], &[0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn combo_role_sits_at_body_offset_28() {
        let broken = ComboBroken {
            routing: zero_combo_routing(),
            autoplay: true,
            role: MemberRole::Challenger,
        };
        assert_eq!(&broken.encode_body()[28..30], &[0x02, 0x00]);
        let started = ComboStarted {
            routing: zero_combo_routing(),
            autoplay: true,
            role: MemberRole::Champion,
        };
        assert_eq!(&started.encode_body()[28..30], &[0x01, 0x00]);
    }

    #[test]
    fn combo_packs_reject_unknown_role() {
        let mut body = ComboBroken {
            routing: zero_combo_routing(),
            autoplay: true,
            role: MemberRole::None,
        }
        .encode_body();
        body[28] = 0x09;
        assert_eq!(
            ComboBroken::decode_body(&body),
            Err(Error::InvalidField {
                field: "member role",
                reason: "not 0, 1, 2, or 3"
            })
        );
        let mut body = ComboStarted {
            routing: zero_combo_routing(),
            autoplay: true,
            role: MemberRole::None,
        }
        .encode_body();
        body[28] = 0x09;
        assert_eq!(
            ComboStarted::decode_body(&body),
            Err(Error::InvalidField {
                field: "member role",
                reason: "not 0, 1, 2, or 3"
            })
        );
    }

    #[test]
    fn create_session_encodes_key_pairs() {
        let create = CreateRelaySession {
            members: vec![
                RelayMember {
                    session_key1: 1,
                    session_key2: 2,
                },
                RelayMember {
                    session_key1: 3,
                    session_key2: 4,
                },
            ],
        };
        assert_eq!(
            create.encode_body(),
            vec![
                0x02, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04,
                0x00, 0x00, 0x00,
            ]
        );
        let frame = decode_frame(&create.encode()).unwrap();
        assert_eq!(frame.id, 10);
        assert_eq!(CreateRelaySession::decode_frame(&frame), Ok(create));
    }

    #[test]
    fn delete_session_round_trips_empty_members() {
        let delete = DeleteRelaySession { members: vec![] };
        assert_eq!(delete.encode_body(), vec![0x00]);
        let frame = decode_frame(&delete.encode()).unwrap();
        assert_eq!(frame.id, 11);
        assert_eq!(DeleteRelaySession::decode_frame(&frame), Ok(delete));
    }

    #[test]
    fn session_lists_reject_truncated_pair() {
        assert_eq!(
            CreateRelaySession::decode_body(&[0x01, 0x01, 0x00]),
            Err(Error::TruncatedMessage { need: 9, have: 3 })
        );
    }

    #[test]
    fn relay_frames_reject_foreign_command() {
        let frame = decode_frame(
            &ComboStarted {
                routing: routing(),
                autoplay: true,
                role: MemberRole::Champion,
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            ComboBroken::decode_frame(&frame),
            Err(Error::UnknownCommand(7))
        );
    }

    fn zero_tally() -> RelayStats {
        RelayStats {
            routing: routing(),
            role: MemberRole::None,
            cool: 0,
            good: 0,
            bad: 0,
            miss: 0,
            combo: 0,
            max_combo: 0,
            jam_combo: 0,
            total_jam_combo: 0,
            long_note_combo: 0,
            long_note_score: 0,
            life: 0,
            pill_buildup: 0,
            pill_count: 0,
            score: 0,
        }
    }

    #[test]
    fn role_sits_at_body_offset_24() {
        let mut tally = zero_tally();
        tally.role = MemberRole::Challenger;
        assert_eq!(&tally.encode_body()[24..26], &[0x02, 0x00]);
    }

    #[test]
    fn cool_sits_at_body_offset_26() {
        let mut tally = zero_tally();
        tally.cool = 11;
        assert_eq!(&tally.encode_body()[26..28], &[11, 0]);
    }

    #[test]
    fn good_sits_at_body_offset_28() {
        let mut tally = zero_tally();
        tally.good = 22;
        assert_eq!(&tally.encode_body()[28..30], &[22, 0]);
    }

    #[test]
    fn bad_sits_at_body_offset_30() {
        let mut tally = zero_tally();
        tally.bad = 33;
        assert_eq!(&tally.encode_body()[30..32], &[33, 0]);
    }

    #[test]
    fn miss_sits_at_body_offset_32() {
        let mut tally = zero_tally();
        tally.miss = 44;
        assert_eq!(&tally.encode_body()[32..34], &[44, 0]);
    }

    #[test]
    fn combo_sits_at_body_offset_34() {
        let mut tally = zero_tally();
        tally.combo = 55;
        assert_eq!(&tally.encode_body()[34..36], &[55, 0]);
    }

    #[test]
    fn max_combo_sits_at_body_offset_36() {
        let mut tally = zero_tally();
        tally.max_combo = 66;
        assert_eq!(&tally.encode_body()[36..38], &[66, 0]);
    }

    #[test]
    fn jam_combo_sits_at_body_offset_38() {
        let mut tally = zero_tally();
        tally.jam_combo = 77;
        assert_eq!(&tally.encode_body()[38..40], &[77, 0]);
    }

    #[test]
    fn total_jam_combo_sits_at_body_offset_40() {
        let mut tally = zero_tally();
        tally.total_jam_combo = 88;
        assert_eq!(&tally.encode_body()[40..42], &[88, 0]);
    }

    #[test]
    fn long_note_combo_sits_at_body_offset_42() {
        let mut tally = zero_tally();
        tally.long_note_combo = 99;
        assert_eq!(&tally.encode_body()[42..44], &[99, 0]);
    }

    #[test]
    fn long_note_score_sits_at_body_offset_44() {
        let mut tally = zero_tally();
        tally.long_note_score = 0x1122_3344;
        assert_eq!(&tally.encode_body()[44..48], &[0x44, 0x33, 0x22, 0x11]);
    }

    #[test]
    fn life_sits_at_body_offset_48() {
        let mut tally = zero_tally();
        tally.life = 1000;
        assert_eq!(&tally.encode_body()[48..50], &[0xE8, 0x03]);
    }

    #[test]
    fn pill_buildup_sits_at_body_offset_50() {
        let mut tally = zero_tally();
        tally.pill_buildup = 5;
        assert_eq!(&tally.encode_body()[50..54], &[5, 0, 0, 0]);
    }

    #[test]
    fn pill_count_sits_at_body_offset_54() {
        let mut tally = zero_tally();
        tally.pill_count = 3;
        assert_eq!(&tally.encode_body()[54..58], &[3, 0, 0, 0]);
    }

    #[test]
    fn score_sits_at_body_offset_58() {
        let mut tally = zero_tally();
        tally.score = 0x5566_7788;
        assert_eq!(&tally.encode_body()[58..62], &[0x88, 0x77, 0x66, 0x55]);
    }
}

#[cfg(test)]
mod live_handshake_tests {
    use super::*;
    use crate::frame::decode_frame;

    #[test]
    fn punch_report_frames_exact_uhp_bytes() {
        assert_eq!(ReportUdpPunchHole.encode(), vec![0x04, 0x00, 0xCE, 0x0F]);
        assert_eq!(
            ReportUdpPunchHoleAnswer.encode(),
            vec![0x04, 0x00, 0xCF, 0x0F]
        );
        assert_eq!(ConfirmRelay.encode(), vec![0x04, 0x00, 0xD7, 0x0F]);
        assert_eq!(ConfirmRelayAnswer.encode(), vec![0x04, 0x00, 0xD8, 0x0F]);
    }

    #[test]
    fn punch_and_confirm_round_trip_empty() {
        let punch = decode_frame(&ReportUdpPunchHole.encode()).unwrap();
        assert_eq!(punch.id, 4046);
        assert_eq!(
            ReportUdpPunchHole::decode_frame(&punch),
            Ok(ReportUdpPunchHole)
        );
        let echo = decode_frame(&ReportUdpPunchHoleAnswer.encode()).unwrap();
        assert_eq!(echo.id, 4047);
        assert_eq!(
            ReportUdpPunchHoleAnswer::decode_frame(&echo),
            Ok(ReportUdpPunchHoleAnswer)
        );
        let confirm = decode_frame(&ConfirmRelay.encode()).unwrap();
        assert_eq!(confirm.id, 4055);
        assert_eq!(ConfirmRelay::decode_frame(&confirm), Ok(ConfirmRelay));
        let answer = decode_frame(&ConfirmRelayAnswer.encode()).unwrap();
        assert_eq!(answer.id, 4056);
        assert_eq!(
            ConfirmRelayAnswer::decode_frame(&answer),
            Ok(ConfirmRelayAnswer)
        );
    }

    #[test]
    fn punch_and_confirm_reject_payload() {
        assert_eq!(
            ReportUdpPunchHole::decode_body(&[0x00]),
            Err(Error::TrailingBytes(1))
        );
        assert_eq!(
            ReportUdpPunchHoleAnswer::decode_body(&[0x00]),
            Err(Error::TrailingBytes(1))
        );
        assert_eq!(
            ConfirmRelay::decode_body(&[0x00]),
            Err(Error::TrailingBytes(1))
        );
        assert_eq!(
            ConfirmRelayAnswer::decode_body(&[0x00]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn punch_report_rejects_confirm_command() {
        let answer = decode_frame(&ConfirmRelayAnswer.encode()).unwrap();
        assert_eq!(
            ReportUdpPunchHole::decode_frame(&answer),
            Err(Error::UnknownCommand(4056))
        );
    }

    #[test]
    fn confirm_relay_rejects_shaped_bodies() {
        // Separation from the shared helper's field-carrying callers:
        // 4012 (5 B), 4014 (16 B), 4016 (24 B) lengths never decode as 4055.
        assert_eq!(
            ConfirmRelay::decode_body(&[0x00; 5]),
            Err(Error::TrailingBytes(5))
        );
        assert_eq!(
            ConfirmRelay::decode_body(&[0x00; 16]),
            Err(Error::TrailingBytes(16))
        );
        assert_eq!(
            ConfirmRelay::decode_body(&[0x00; 24]),
            Err(Error::TrailingBytes(24))
        );
    }
}

#[cfg(test)]
mod live_state_tests {
    use super::*;
    use crate::frame::decode_frame;

    #[test]
    fn live_requests_frame_exact_empty_bytes() {
        assert_eq!(GetLiveState.encode(), vec![0x04, 0x00, 0xCA, 0x0F]);
        assert_eq!(GetP2PList.encode(), vec![0x04, 0x00, 0xCC, 0x0F]);
    }

    #[test]
    fn live_requests_round_trip_empty() {
        let live = decode_frame(&GetLiveState.encode()).unwrap();
        assert_eq!(live.id, 4042);
        assert_eq!(GetLiveState::decode_frame(&live), Ok(GetLiveState));
        let peers = decode_frame(&GetP2PList.encode()).unwrap();
        assert_eq!(peers.id, 4044);
        assert_eq!(GetP2PList::decode_frame(&peers), Ok(GetP2PList));
    }

    #[test]
    fn live_requests_reject_payload() {
        assert_eq!(
            GetLiveState::decode_body(&[0x00]),
            Err(Error::TrailingBytes(1))
        );
        assert_eq!(
            GetP2PList::decode_body(&[0x00]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn live_requests_reject_foreign_command() {
        let peers = decode_frame(&GetP2PList.encode()).unwrap();
        assert_eq!(
            GetLiveState::decode_frame(&peers),
            Err(Error::UnknownCommand(4044))
        );
    }

    fn two_peers() -> P2PListResponse {
        P2PListResponse {
            peers: vec![
                PeerInfo {
                    member_id: 3,
                    public_endpoint: SocketAddrV4::new(Ipv4Addr::new(203, 238, 136, 184), 39311),
                    local_endpoint: SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 2), 1111),
                },
                PeerInfo {
                    member_id: 5,
                    public_endpoint: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 3333),
                    local_endpoint: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 2), 4444),
                },
            ],
        }
    }

    #[test]
    fn p2p_list_encodes_two_peers() {
        assert_eq!(
            two_peers().encode_body(),
            vec![
                0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xCB, 0xEE, 0x88, 0xB8, 0x8F, 0x99,
                0xC0, 0xA8, 0x00, 0x02, 0x57, 0x04, 0x05, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x01,
                0x05, 0x0D, 0x0A, 0x00, 0x00, 0x02, 0x5C, 0x11,
            ]
        );
    }

    #[test]
    fn p2p_list_round_trips_through_frame() {
        let frame = decode_frame(&two_peers().encode()).unwrap();
        assert_eq!(frame.id, 4045);
        assert_eq!(P2PListResponse::decode_frame(&frame), Ok(two_peers()));
    }

    #[test]
    fn p2p_list_decodes_empty_count() {
        assert_eq!(
            P2PListResponse::decode_body(&[0x00, 0x00, 0x00, 0x00]),
            Ok(P2PListResponse { peers: vec![] })
        );
    }

    #[test]
    fn p2p_list_negative_count_decodes_empty() {
        // The exe exits its peer loop on a negative bound (signed exit
        // test at file `0x18cfb7`), so the client yields no peers.
        assert_eq!(
            P2PListResponse::decode_body(&[0xFF, 0xFF, 0xFF, 0xFF]),
            Ok(P2PListResponse { peers: vec![] })
        );
    }

    #[test]
    fn p2p_list_rejects_truncated_peer() {
        let mut body = two_peers().encode_body();
        body.truncate(body.len() - 1);
        assert_eq!(
            P2PListResponse::decode_body(&body),
            Err(Error::TruncatedMessage { need: 36, have: 35 })
        );
    }

    #[test]
    fn p2p_list_rejects_trailing_bytes() {
        let mut body = two_peers().encode_body();
        body.push(0x00);
        assert_eq!(
            P2PListResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn p2p_list_rejects_foreign_command() {
        let live = decode_frame(&GetLiveState.encode()).unwrap();
        assert_eq!(
            P2PListResponse::decode_frame(&live),
            Err(Error::UnknownCommand(4042))
        );
    }

    #[test]
    fn p2p_fields_sit_at_expected_offsets() {
        let body = two_peers().encode_body();
        assert_eq!(&body[0..4], &[0x02, 0x00, 0x00, 0x00]);
        assert_eq!(&body[4..8], &[0x03, 0x00, 0x00, 0x00]);
        assert_eq!(&body[8..14], &[0xCB, 0xEE, 0x88, 0xB8, 0x8F, 0x99]);
        assert_eq!(&body[14..20], &[0xC0, 0xA8, 0x00, 0x02, 0x57, 0x04]);
        assert_eq!(&body[20..24], &[0x05, 0x00, 0x00, 0x00]);
    }

    fn sample_info() -> MemberInfo {
        MemberInfo {
            member_id: 0,
            nickname: "AB".to_owned(),
            level: 10,
            gender: 1,
            gem: 100,
            team: 2,
            ready: true,
            music_state: 3,
            equipments: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
            music_ids: vec![7, 440],
            cash_point: 5000,
            free_pass: 1,
            playing_state: 2,
            live_extra: 0,
            is_administrator: false,
            is_room_master: true,
            win_streak: 4,
        }
    }

    fn inactive_slots() -> [MemberLiveState; LIVE_STATE_SLOTS] {
        Default::default()
    }

    #[test]
    fn live_state_inactive_members_exact_bytes() {
        let snapshot = LiveStateResponse {
            member_id: 1,
            user_count: 7,
            win_streak: 2,
            members: inactive_slots(),
        };
        assert_eq!(
            snapshot.encode_body(),
            vec![
                0x01, 0x07, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00,
            ]
        );
        let frame = decode_frame(&snapshot.encode()).unwrap();
        assert_eq!(frame.id, 4043);
        assert_eq!(LiveStateResponse::decode_frame(&frame), Ok(snapshot));
    }

    #[test]
    fn live_state_header_fields_sit_at_expected_offsets() {
        let snapshot = LiveStateResponse {
            member_id: 1,
            user_count: 7,
            win_streak: 2,
            members: inactive_slots(),
        };
        let body = snapshot.encode_body();
        assert_eq!(&body[0..1], &[0x01]);
        assert_eq!(&body[1..5], &[0x07, 0x00, 0x00, 0x00]);
        assert_eq!(&body[5..9], &[0x02, 0x00, 0x00, 0x00]);
        assert_eq!(&body[9..10], &[0x00]);
    }

    #[test]
    fn live_state_active_member_exact_bytes() {
        let mut members = inactive_slots();
        members[0] = MemberLiveState {
            active: true,
            info: Some(sample_info()),
        };
        let snapshot = LiveStateResponse {
            member_id: 0,
            user_count: 1,
            win_streak: 0,
            members,
        };
        assert_eq!(
            snapshot.encode_body(),
            vec![
                0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x41, 0x42, 0x00,
                0x0A, 0x00, 0x00, 0x00, 0x01, 0x64, 0x00, 0x00, 0x00, 0x02, 0x01, 0x03, 0x01, 0x00,
                0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
                0x05, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x08, 0x00,
                0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x0B, 0x00, 0x00, 0x00,
                0x0C, 0x00, 0x00, 0x00, 0x0D, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x0F, 0x00,
                0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
                0x07, 0x00, 0xB8, 0x01, 0x88, 0x13, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00,
                0x00, 0x01, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]
        );
    }

    #[test]
    fn live_state_active_member_round_trips() {
        let mut members = inactive_slots();
        members[0] = MemberLiveState {
            active: true,
            info: Some(sample_info()),
        };
        let snapshot = LiveStateResponse {
            member_id: 0,
            user_count: 1,
            win_streak: 0,
            members,
        };
        let frame = decode_frame(&snapshot.encode()).unwrap();
        assert_eq!(frame.id, 4043);
        let decoded = LiveStateResponse::decode_frame(&frame).unwrap();
        assert_eq!(decoded, snapshot);
        let info = decoded.members[0].info.as_ref().unwrap();
        assert_eq!(info.nickname, "AB");
        assert_eq!(info.equipments[16], 17);
        assert_eq!(info.music_ids, vec![7, 440]);
        assert_eq!(info.win_streak, 4);
        assert!(decoded.members[1..].iter().all(|slot| !slot.active));
    }

    #[test]
    fn live_state_non_one_active_byte_means_inactive() {
        // The handler continues only on `active == 1` (file `0x159d24`);
        // any other byte takes the skip path with no tail.
        let mut body = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        body.push(0x02);
        body.extend_from_slice(&[0x00; 7]);
        let decoded = LiveStateResponse::decode_body(&body).unwrap();
        assert!(!decoded.members[0].active);
        assert_eq!(decoded.members[0].info, None);
    }

    #[test]
    fn live_state_rejects_member_id_at_slot_bound() {
        let mut body = vec![0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        body.extend_from_slice(&[0x00; 8]);
        assert_eq!(
            LiveStateResponse::decode_body(&body),
            Err(Error::InvalidField {
                field: "live member id",
                reason: "not 0-7"
            })
        );
    }

    #[test]
    fn live_state_rejects_short_header() {
        // 8-byte body: member id plus user count parse, then the win
        // streak read finds only 3 bytes left.
        assert_eq!(
            LiveStateResponse::decode_body(&[0x01; 8]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    #[test]
    fn live_state_rejects_trailing_bytes() {
        let mut body = LiveStateResponse {
            member_id: 1,
            user_count: 7,
            win_streak: 2,
            members: inactive_slots(),
        }
        .encode_body();
        body.push(0x00);
        assert_eq!(
            LiveStateResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn live_state_negative_music_count_decodes_empty() {
        let mut info = sample_info();
        info.music_ids.clear();
        let mut members = inactive_slots();
        members[0] = MemberLiveState {
            active: true,
            info: Some(info),
        };
        let snapshot = LiveStateResponse {
            member_id: 0,
            user_count: 1,
            win_streak: 0,
            members,
        };
        let mut body = snapshot.encode_body();
        // Music-id count sits 84 bytes into the active tail (past the
        // 9-byte header, the active byte, and the 84 fixed prefix bytes).
        body[94..98].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        let decoded = LiveStateResponse::decode_body(&body).unwrap();
        assert_eq!(
            decoded.members[0].info.as_ref().unwrap().music_ids,
            Vec::<u16>::new()
        );
    }

    #[test]
    fn live_state_rejects_foreign_command() {
        let peers = decode_frame(&GetP2PList.encode()).unwrap();
        assert_eq!(
            LiveStateResponse::decode_frame(&peers),
            Err(Error::UnknownCommand(4044))
        );
    }
}

#[cfg(test)]
mod adjunct_host_tests {
    use super::*;

    #[test]
    fn adjunct_host_matches_documented_address() {
        assert_eq!(ADJUNCT_RELAY_HOST.octets(), [203, 238, 136, 52]);
    }

    #[test]
    fn adjunct_host_shares_relay_subnet_but_not_pool() {
        let pool = [
            Ipv4Addr::new(203, 238, 136, 184),
            Ipv4Addr::new(203, 238, 136, 182),
            Ipv4Addr::new(203, 238, 136, 183),
        ];
        for member in pool {
            assert_ne!(ADJUNCT_RELAY_HOST, member);
            assert_eq!(ADJUNCT_RELAY_HOST.octets()[..3], member.octets()[..3]);
        }
    }

    #[test]
    fn adjunct_setup_number_matches_exe_immediate() {
        assert_eq!(ADJUNCT_SETUP_NUMBER, 0x1E6C);
        assert_eq!(ADJUNCT_SETUP_NUMBER, 7788);
    }
}

#[cfg(test)]
mod udp_frame_tests {
    use super::*;

    #[test]
    fn udp_packet_type_names_all_types() {
        assert_eq!(UdpPacketType::from_u8(1), Ok(UdpPacketType::Unreliable));
        assert_eq!(UdpPacketType::from_u8(2), Ok(UdpPacketType::Reliable));
        assert_eq!(UdpPacketType::from_u8(3), Ok(UdpPacketType::Ack));
        assert_eq!(UdpPacketType::Reliable.as_u8(), 2);
    }

    #[test]
    fn udp_packet_type_rejects_unknown_type() {
        assert_eq!(
            UdpPacketType::from_u8(9),
            Err(Error::InvalidField {
                field: "udp packet type",
                reason: "not 1, 2, or 3"
            })
        );
    }

    #[test]
    fn ack_encodes_bare_four_bytes() {
        assert_eq!(UdpRelayFrame::encode_ack(0x2A), [0x03, 0x2A, 0x00, 0x00]);
    }

    #[test]
    fn ack_decodes_without_zlib() {
        let frame = UdpRelayFrame::decode_datagram(&[0x03, 0x2A, 0x00, 0x00]).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Ack);
        assert_eq!(frame.sequence, 0x2A);
        assert!(frame.payload.is_empty());
        assert_eq!(frame.command_id(), None);
        assert_eq!(frame.relay_command(), None);
        assert_eq!(frame.body(), &[]);
    }

    /// Fixed-Huffman datagram from Python `zlib.compress` (level 6):
    /// inner `[01][09][08 00][05 00 00 00][AA BB CC DD]`.
    fn fixed_datagram() -> Vec<u8> {
        vec![
            0x0C, 0x00, 0x78, 0x9C, 0x63, 0xE4, 0xE4, 0x60, 0x60, 0x65, 0x60, 0x60, 0x58, 0xB5,
            0xFB, 0xCC, 0x5D, 0x00, 0x08, 0x41, 0x03, 0x26,
        ]
    }

    #[test]
    fn datagram_decodes_fixed_huffman_block() {
        let frame = UdpRelayFrame::decode_datagram(&fixed_datagram()).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Unreliable);
        assert_eq!(frame.sequence, 0x09);
        assert_eq!(frame.payload, vec![0x05, 0x00, 0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(frame.command_id(), Some(5));
        assert_eq!(frame.relay_command(), Some(RelayCommand::UpdateGameStats));
        assert_eq!(frame.body(), &[0xAA, 0xBB, 0xCC, 0xDD]);
    }

    /// Stored-block datagram from Python `zlib.compress` (level 0).
    fn stored_datagram() -> Vec<u8> {
        vec![
            0x0C, 0x00, 0x78, 0x01, 0x01, 0x0C, 0x00, 0xF3, 0xFF, 0x01, 0x09, 0x08, 0x00, 0x05,
            0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD, 0x08, 0x41, 0x03, 0x26,
        ]
    }

    #[test]
    fn datagram_decodes_stored_block() {
        let frame = UdpRelayFrame::decode_datagram(&stored_datagram()).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Unreliable);
        assert_eq!(frame.sequence, 0x09);
        assert_eq!(frame.payload, vec![0x05, 0x00, 0xAA, 0xBB, 0xCC, 0xDD]);
    }

    /// Dynamic-Huffman datagram from Python `zlib.compress` (level 6,
    /// 603-byte text inner): type Unreliable, seq `0x2C`, command 5.
    fn dynamic_datagram() -> Vec<u8> {
        vec![
            0x5B, 0x02, 0x78, 0x9C, 0x55, 0xD1, 0x41, 0x6A, 0x03, 0x31, 0x0C, 0x05, 0xD0, 0x69,
            0xA0, 0xF7, 0xF0, 0x01, 0xBA, 0x90, 0x2C, 0xD9, 0xB2, 0xC8, 0x41, 0xB2, 0x0E, 0x61,
            0x0A, 0x81, 0x29, 0x43, 0x33, 0xAB, 0xDE, 0x3E, 0x06, 0x49, 0x69, 0xE4, 0x8D, 0x36,
            0xFF, 0xF1, 0x0D, 0xFF, 0xE3, 0xEB, 0x72, 0xFA, 0x5C, 0x96, 0xE5, 0xB1, 0x6E, 0xD7,
            0xBF, 0x72, 0xAC, 0xBF, 0x05, 0xCA, 0x6D, 0xDF, 0xB7, 0x79, 0x7E, 0xEE, 0xC7, 0x31,
            0xCF, 0x76, 0xFF, 0x5E, 0x0B, 0x02, 0x40, 0x39, 0x6E, 0xFB, 0x63, 0x2D, 0x70, 0x2E,
            0xFF, 0x61, 0xB4, 0x30, 0x59, 0x58, 0x2C, 0xAC, 0xAA, 0x9E, 0xC5, 0x4A, 0xDC, 0xDE,
            0xF3, 0xD5, 0xF2, 0xDD, 0xF2, 0xC8, 0x01, 0x86, 0x83, 0xCA, 0x5D, 0x53, 0x01, 0x19,
            0x50, 0x03, 0x15, 0x03, 0x88, 0x03, 0x12, 0xA0, 0xD4, 0xC0, 0x06, 0xB0, 0xBA, 0x18,
            0x21, 0xBA, 0x0B, 0x56, 0x1A, 0xA9, 0xA2, 0xB9, 0x68, 0x26, 0xA8, 0x85, 0x68, 0x2E,
            0x3A, 0x4A, 0x4D, 0x1D, 0xDD, 0xC5, 0x30, 0xC1, 0x35, 0x04, 0xBB, 0x10, 0x06, 0x49,
            0x1D, 0x62, 0x62, 0xFE, 0xDF, 0x84, 0x86, 0x20, 0x17, 0xA3, 0x33, 0xA6, 0x8E, 0xE1,
            0x82, 0x4D, 0xB4, 0x1E, 0xA2, 0xBA, 0xD0, 0x21, 0x3D, 0x75, 0xA8, 0x0B, 0x31, 0xD1,
            0x29, 0x04, 0xC6, 0x1A, 0xF3, 0x41, 0x2A, 0x41, 0x1F, 0x9B, 0x7C, 0xED, 0x57, 0x09,
            0xBC, 0x0F, 0x98, 0x17, 0x8F, 0xC9, 0x7D, 0x73, 0x8C, 0x9A, 0xF1, 0x1A, 0x9D, 0x9A,
            0xE8, 0xAC, 0x79, 0x02, 0x5D, 0xDD, 0xB1, 0x42,
        ]
    }

    fn dynamic_text() -> Vec<u8> {
        let mut text = Vec::new();
        for index in 0..12u32 {
            text.extend_from_slice(
                format!(
                    "relay seq {} cool {} miss {} life {} score {}; ",
                    index,
                    index * 3 % 256,
                    index * 7 % 64,
                    1000 - index,
                    index * 12345,
                )
                .as_bytes(),
            );
        }
        text
    }

    #[test]
    fn datagram_decodes_dynamic_huffman_block() {
        let frame = UdpRelayFrame::decode_datagram(&dynamic_datagram()).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Unreliable);
        assert_eq!(frame.sequence, 0x2C);
        assert_eq!(frame.command_id(), Some(5));
        let mut payload = vec![0x05, 0x00];
        payload.extend_from_slice(&dynamic_text());
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn datagram_round_trips_through_stored_encoder() {
        let body = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let datagram = UdpRelayFrame::encode_datagram(UdpPacketType::Reliable, 0x2C, 6, &body);
        let frame = UdpRelayFrame::decode_datagram(&datagram).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Reliable);
        assert_eq!(frame.sequence, 0x2C);
        assert_eq!(frame.relay_command(), Some(RelayCommand::ComboBroken));
        assert_eq!(frame.body(), &body);
    }

    #[test]
    fn datagram_rejects_length_prefix_mismatch() {
        let mut datagram = fixed_datagram();
        datagram[0] = 0x0D;
        assert_eq!(
            UdpRelayFrame::decode_datagram(&datagram),
            Err(Error::TruncatedMessage { need: 13, have: 12 })
        );
    }

    #[test]
    fn datagram_rejects_bad_checksum() {
        let mut datagram = stored_datagram();
        let last = datagram.len() - 1;
        datagram[last] ^= 0xFF;
        assert_eq!(
            UdpRelayFrame::decode_datagram(&datagram),
            Err(Error::InvalidField {
                field: "zlib checksum",
                reason: "adler32 mismatch"
            })
        );
    }

    #[test]
    fn datagram_rejects_tiny_input() {
        assert_eq!(
            UdpRelayFrame::decode_datagram(&[0x01]),
            Err(Error::TruncatedMessage { need: 2, have: 1 })
        );
    }
}
