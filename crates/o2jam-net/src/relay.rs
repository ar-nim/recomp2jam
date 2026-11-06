//! Live relay packets (room PvP lane share).
//!
//! Relay TCP carries [`RelayCommand`] frames with the same
//! length-prefixed `CPacket_Str` framing as the game gateway
//! (`[u16 len][u16 id][body]`, `len` counts the whole frame; id at
//! buffer offset 2). UDP is the primary Live path: each datagram is
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
//! RTTI), and the endpoints — not the field bytes, so every body
//! below is Memoryer-corroborated; see `docs/relay.md` for the
//! anchor table and the open exe-verification items.

use std::net::{Ipv4Addr, SocketAddrV4};

use super::error::Error;
use super::inflate::{compress_zlib_stored, decompress_zlib};
use super::message::WireMessage;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboBroken {
    /// Routing preface (code, relay endpoints, unused).
    pub routing: RelayRouting,
    /// Autoplay flag (hardcoded to 1 when sent).
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboStarted {
    /// Routing preface (code, relay endpoints, unused).
    pub routing: RelayRouting,
    /// Autoplay flag (hardcoded to 1 when sent).
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
        datagram.extend_from_slice(&(inner.len() as u16).to_le_bytes());
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
