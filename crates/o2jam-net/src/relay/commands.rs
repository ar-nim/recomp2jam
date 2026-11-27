//! Relay command bodies: the op 1-7 messages (routing
//! preface plus per-op fields), the Live-handshake pair, and the
//! Live-state / peer-list answers (commands 3003-3007, 4043, 4045).

use std::net::SocketAddrV4;

use super::routing::{decode_socket, encode_socket, word_at, ConnectionEndpoint, RelayRouting};
use crate::error::Error;
use crate::macros::wire_enum;
use crate::message::WireMessage;
use crate::rows::rows_zero_on_negative_exact;
use crate::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};

wire_enum! {
    /// Relay opcodes (`Memoryer.Relay` `RelayCommand`, matched in-exe by
    /// the `SetID(1..7)` ctors; see `docs/relay.md`).
    pub enum RelayCommand: u16, resolve from_id, encode id, {
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
}

/// Room seat role, carried as `u16` on relay op 5/6/7.
///
/// Value meanings are exe-pinned, not just Memoryer-corroborated
/// (`Encore.Metadata.MemberRole` agrees: None 0, Champion 1, Challenger 2,
/// Spectator 3). The role word lives at VA `0x77ee86`: seven sites load it
/// with `66 A1 86 EE 77 00` (files `0xaec29`/`0xc33b4`/`0xdbb1c`/`0xe6f81`/
/// `0xe7a75`/`0xe840a`/`0xea26e`); six branch on the value while the op-5
/// packer (`0xea26e`) only copies it into the tally. Spectator gates on
/// `83 F9 03` at files `0xaec3d`/`0xc33c2`/`0xe7a89`/`0xe841e`; the
/// dispatcher at file `0xdbb1c` splits Champion/Challenger with
/// `83 7D 8C 01`/`83 7D 8C 02` plus an else fallthrough, and file `0xe6f81`
/// checks `83 F9 01` then `83 F8 02`. None (0) is the residual value taken
/// when none of the 1/2/3 compares hit. See `docs/relay.md` §11
/// (`ev_acacd596017f22ba805e419c18cfe686`,
/// `ev_cbe189ba7fb55898e42e98c6175b8e75`,
/// `ev_6f91b9283627171b33801a2c0e653100`,
/// `ev_e5879e64727f6c4c5ab0759b929433c8`, observed).
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
/// Sibling lead-flag member-update command (room dispatcher id 3003).
///
/// Command binding (7.55, local-static, read-only, VA = file + `0x400000`):
/// the room dispatcher subtracts base `0x0BBB` (3003), range-checks the
/// bound `0x0B`, and jumps through the dword table at file `0x1589c4`
/// (`ff 24 8d c4 89 55 00`); index 0 routes through the stub at file
/// `0x1585ab` into the lead-flag handler at entry file `0x15cad0` (sole
/// caller at file `0x1585af`). That handler reads the room-master flag
/// (`+0x30`, helper file `0x140d20`) at site file `0x15cf0d`, ahead of
/// the seat-byte readers (files `0x15cf26`/`0x15cf3f`/`0x15cf58`).
/// (`ev_a83aada10d256cb689110409dc89c067`,
/// `ev_f1ed2d82f90e372666e2a01a66831d6f`, observed.)
/// (ERRORS-06: plain constant, no `unwrap`; PERF-07: `u16` literal.)
pub const MEMBER_LEAD_FLAG_COMMAND: u16 = 3003;
/// Sibling trailing-flag member-update command (room dispatcher id 3007).
///
/// Same dispatcher as [`MEMBER_LEAD_FLAG_COMMAND`] (base `0x0BBB`, bound
/// `0x0B`, table file `0x1589c4`); index 4 routes through the stub at file
/// `0x1585de` into the trailing-flag handler at entry file `0x15d820`
/// (sole caller at file `0x1585e2`). That handler reads the room-master
/// flag at site file `0x15dec3`, after the `playing_state`/`live_extra`
/// pair like the 4043 path, then reads the win-streak word.
/// (`ev_50242b6c183543f49ce6cbb43eccad2e`, observed.)
/// (ERRORS-06: plain constant, no `unwrap`; PERF-07: `u16` literal.)
pub const MEMBER_TRAIL_FLAG_COMMAND: u16 = 3007;
/// Fixed equipment words per active [`MemberInfo`]: one `i32` plus a
/// 16-read loop (`0x11` bound at file `0x140eb1`).
const LIVE_EQUIPMENT_WORDS: usize = 17;
/// Serialized size of one [`PeerInfo`]: `i32` member + two 6-byte endpoints.
const PEER_ENTRY_LEN: usize = 16;

/// Member-slot table base (VA) the 4043 handler stores into: every active
/// slot address is `MEMBER_SLOT_BASE + slot * MEMBER_SLOT_STRIDE`
/// (files `0x159d38`–`0x159f82` repeat `imul $0x2190` + `add $0x77ee90`).
/// Static only over the local install-root copy (read-only, VA = file +
/// `0x400000`); behavior described in own words with file offsets plus
/// local-static record ids, no disassembly reproduced.
/// (`ev_5a25bf3fec517df32dde42e275128074`,
/// `ev_7737f29fa5a4df1f2270ce72df7b6d54`,
/// `ev_54701cf711eebb93adbcbd632629542c`,
/// `ev_20f7c72d0d3f156c51ba3c3a0417f774`, observed.)
/// (ERRORS-06: plain constants, no `unwrap`; PERF-07: `u32` literals,
/// no narrowing casts.)
pub const MEMBER_SLOT_BASE: u32 = 0x77ee90;
/// Stride of one member slot (8592 bytes): slot `i` starts at
/// `MEMBER_SLOT_BASE + i * MEMBER_SLOT_STRIDE` (8 slots).
pub const MEMBER_SLOT_STRIDE: u32 = 0x2190;
/// Store offset of [`MemberInfo::member_id`] (1 B, helper file `0x140bd0`,
/// call site file `0x159d44`).
pub const MEMBER_OFF_ID: u32 = 0x04;
/// Store offset of [`MemberInfo::nickname`] (NUL-terminated copy, helper
/// file `0x140bf0` via the string reader at file `0x155ea0`, call site
/// file `0x159d5c`).
pub const MEMBER_OFF_NICKNAME: u32 = 0x05;
/// Store offset of [`MemberInfo::level`] (`i32`, helper file `0x140c50`,
/// call site file `0x159d74`).
pub const MEMBER_OFF_LEVEL: u32 = 0x20;
/// Store offset of [`MemberInfo::gender`] (1 B raw; reader helper file
/// `0x140c90` at call site file `0x159d8c`, then setter file `0x13fe50`
/// stores `+0x28` and picks the avatar sub-object by zero/nonzero).
pub const MEMBER_OFF_GENDER: u32 = 0x28;
/// Store offset of [`MemberInfo::gem`] (`i32`, helper file `0x140cc0`,
/// call site file `0x159da4`).
pub const MEMBER_OFF_GEM: u32 = 0x2c;
/// Store offset of the first seat byte ([`MemberInfo::team`]) (1 B, helper
/// file `0x140d40`, call site file `0x159dbc`).
pub const MEMBER_OFF_TEAM: u32 = 0x31;
/// Wire store of the second seat byte ([`MemberInfo::ready`]) (1 B, helper
/// file `0x140d70`, call site file `0x159dd4`). The byte is consumed from
/// the wire, then the slot-index branch overwrites `+0x32` from position
/// (setter file `0x1400f0`: slot 0 pushes 1 at file `0x159f00`, slot 3
/// pushes 0 at file `0x159f2e`, any other slot pushes 1 at file
/// `0x159f5c`; see [`slot_position_marks`]), so the decoded `ready` keeps
/// the wire value while the live slot shows the derived one.
pub const MEMBER_OFF_READY: u32 = 0x32;
/// Store offset of the third seat byte ([`MemberInfo::music_state`])
/// (1 B, helper file `0x141480`, call site file `0x159dec`). Storage is
/// pinned but value semantics stay OPEN: no compare or branch on
/// `+0x217c` was located image-wide (only the direct setter file
/// `0x141460` with 27 callers, incl. the UI-forward paths for codes
/// `0x0FC3`/`0x0FC5` at files `0x159943`/`0x15995e`/`0x1599d2`/`0x1599ed`,
/// the slot-ctor default `+0x217c = 0` beside `+0x217d = 0xFF` at files
/// `0x13fcc1`/`0x13fcc8`, and the bulk copy at files `0xe961d`/`0xe9623`;
/// `ev_f11710744bbbb8279df245ae2e46aeac`, observed). The Memoryer
/// `MusicState` position matches; its value set does not transfer.
pub const MEMBER_OFF_MUSIC_STATE: u32 = 0x217c;
/// Store offset of the wire-order match index (1 B, direct setter file
/// `0x1414a0`). Storage role is pinned: the slot ctor defaults `+0x217d`
/// to `0xFF` beside `+0x217c = 0` (files `0x13fcc1`/`0x13fcc8`), the reset
/// routine entered at file `0xdb6e0` restores `0xFF` to all eight slots
/// (call site file `0xdb8c6`) plus the local object, and two 8-by-8
/// id-match routines record the wire position `0..7` there — the S→C
/// routine at file `0x159a60` (stub file `0x15879c`; local store file
/// `0x159acf`, slot store file `0x159b1a`) and the room-dispatcher
/// routine entered at file `0x15fa20` (stub file `0x1586bf`; local store
/// file `0x15fb34`, slot store file `0x15fb7e`) — each comparing the
/// just-read id byte against the requesting id at `0x77cd04` or the slot
/// id at `+0x04` (`0x77ee94`) and storing the outer index on match. The
/// direct setter has exactly six image-wide callers (the two stores in
/// each of the three routines above); the only other `0x217d` sites are
/// the ctor default and the bulk-copy pair (files `0xe962f`/`0xe9635`).
/// No compare or branch on `+0x217d` was located image-wide, so what the
/// index selects stays OPEN; `0xFF` means unmatched.
/// (`ev_ef517890c33d30e9f73e4d968f15c740`,
/// `ev_81b04a61265048cdf391244aeff6beac`,
/// `ev_15e8653ce8cffef7c50fec80b16e4b14`, observed.)
/// (ERRORS-06: plain constant, no `unwrap`; PERF-07: `u32` literal.)
pub const MEMBER_OFF_MATCH_INDEX: u32 = 0x217d;
/// Sentinel stored at [`MEMBER_OFF_MATCH_INDEX`] when no wire position
/// matches the slot (slot-ctor default and reset-loop value).
/// (ERRORS-06: plain constant, no `unwrap`; PERF-07: no narrowing casts.)
pub const MEMBER_MATCH_INDEX_UNSET: u8 = 0xFF;
/// Equipment count word the music/equipment helper reads (`i32` at
/// `+0x2038`, helper file `0x140e40`, call site file `0x159e30`; the
/// `0x11` loop bound at file `0x140eb1` stages 17 words at
/// [`MEMBER_OFF_EQUIPMENTS`]).
pub const MEMBER_OFF_EQUIP_COUNT: u32 = 0x2038;
/// Store offset of [`MemberInfo::equipments`] (17 `i32` words, helper file
/// `0x140e40`, call site file `0x159e30`).
pub const MEMBER_OFF_EQUIPMENTS: u32 = 0x2040;
/// Store offset of the music-id count (`i32`, helper file `0x140db0`,
/// call site file `0x159e48`; the loop bound is the stored count, and
/// each `u16` is split into `+0x38` low bits plus `+0x1038` high nibble).
pub const MEMBER_OFF_MUSIC_COUNT: u32 = 0x34;
/// Store offset of [`MemberInfo::music_ids`] entries (helper file
/// `0x140db0`, call site file `0x159e48`).
pub const MEMBER_OFF_MUSIC_IDS: u32 = 0x38;
/// Store offset of [`MemberInfo::cash_point`] (`i32`, helper file
/// `0x140ce0`, call site file `0x159e60`).
pub const MEMBER_OFF_CASH_POINT: u32 = 0x2128;
/// Store offset of [`MemberInfo::free_pass`] (`i32`, helper file
/// `0x1415a0`, call site file `0x159e78`).
pub const MEMBER_OFF_FREE_PASS: u32 = 0x212c;
/// Store offset of [`MemberInfo::playing_state`] (1 B, helper file
/// `0x141670`, call site file `0x159e90`).
pub const MEMBER_OFF_PLAYING_STATE: u32 = 0x2130;
/// Store offset of [`MemberInfo::live_extra`] (1 B wire copy, helper file
/// `0x141690`, call site file `0x159ea8`; the second `+0x2131` read —
/// files `0x141670`/`0x141690` back to back — keeps the wire byte whose
/// in-memory first copy is discarded, semantics OPEN).
pub const MEMBER_OFF_LIVE_EXTRA: u32 = 0x2131;
/// Store offset of the sibling extra dword (4 B, slot `+0x2134`).
///
/// Byte-pinned in the lead-flag sibling handler (entry file `0x15cad0`,
/// command [`MEMBER_LEAD_FLAG_COMMAND`]): the shared fixed-size reader
/// (file `0x155d20`) reads 4 bytes at site file `0x15d009` (pushed 4)
/// right after the `playing_state`/`live_extra` pair (sites files
/// `0x15cfd5`/`0x15cfee`), then the handler stores that dword into the
/// member slot at file `0x15d03c` (`89 88 34 21 00 00`).
/// (`ev_bf2aa97d2356819f76df5ea05e24cfc4`, observed.) Value semantics
/// stay OPEN: no compare or branch consumer is pinned yet.
/// (ERRORS-06: plain constant, no `unwrap`; PERF-07: `u32` literal.)
pub const MEMBER_OFF_EXTRA_DWORD: u32 = 0x2134;
/// Store offset of the room-master flag ([`MemberInfo::is_room_master`])
/// (1 B, helper file `0x140d20`). This is the only trailing flag the wire
/// carries: the 4043 path reads it after `live_extra` (call site file
/// `0x159ec0`), while the sibling parse path reads it ahead of the seat
/// bytes (file `0x15cf0d`) and a third path trails it the same way 4043
/// does (file `0x15dec3`). Identification as room-master (not
/// administrator): the 4043 header derives the local object
/// (`0x77cd00`) from the requesting member id at `0x77cd04`, storing
/// `+0x30 = 1` exactly when the id is 0 (pushes at files
/// `0x159bdf`/`0x159c1b`/`0x159c4d`) alongside the `0x77ee3c = 1` mark at
/// file `0x159be4`; and the sibling handler compares the member id
/// against the announced master id byte at `0x778a14`, marking the local
/// `+0x30` when equal and the announced slot otherwise (files
/// `0x15d888`–`0x15d8d3`; `ev_bc4842b1a6fa649bb1a62f21200a3a30`,
/// observed). No administrator flag exists on this wire: helper file
/// `0x140d90` (the `+0x33` reader) has no image-wide caller.
pub const MEMBER_OFF_ROOM_MASTER: u32 = 0x30;
/// Store offset of the spare low flag byte (1 B). Never populated from
/// the 4043 wire: the handler zeroes `+0x33` through the direct setter
/// file `0x140110` (call site file `0x159e18`, pushed 0) right after the
/// third seat byte, and no wire reader targets it in any of the three
/// member-parse paths (helper file `0x140d90` has zero image-wide
/// callers). Elsewhere `+0x33` is a local-only effect selector holding
/// `0..8`: the direct setter has 49 image-wide callers, mostly pushing
/// immediates `0..8` to the local object at `0x77cd00` (one call targets
/// `0x78fb10`); the lone consumer is the ticker helper file `0x141620`
/// (six callers in the score region: files
/// `0x16e5c1`/`0x16e73e`/`0x176140`/`0x176d86`/`0x177230`/`0x178200`),
/// which skips the `+0x2158 += 10` bonus when `+0x33 == 6` yet always
/// applies `+0x2152 += 1`. `ev_c030b2551aa535f98e016858e180ae73`,
/// `ev_c18b6a071288740ad1166134e7fc92a6`,
/// `ev_261849e6804de5f8d1b476fbe9d0fb55`,
/// `ev_d6fa2e234044d3517f0c887799e19eda`, observed.
pub const MEMBER_OFF_SPARE_FLAG: u32 = 0x33;
/// Store offset of the slot-group word the 4043 handler derives from slot
/// position (2 B, setter file `0x141520`; call sites files `0x159eea` /
/// `0x159f18` / `0x159f46`). Never wire-read: slot 0 stores 1, slot 3
/// stores 2, any other slot stores 3 (branch files `0x159ec5`–`0x159f5c`;
/// see [`slot_position_marks`]). The slot ctor defaults it to 3 (file
/// `0x13fa96`) and the bulk copy preserves it (files `0xe9665`/`0xe966f`);
/// `ev_14fd3f2dae86c90bf45f505c7c7a70ef`, observed.
pub const MEMBER_OFF_SLOT_GROUP: u32 = 0x2186;
/// Store offset of [`MemberInfo::win_streak`] (`i32`, inline size-4 read
/// at file `0x159f61` then direct setter file `0x141560` at call site
/// file `0x159f82`; `0x218c + 4` ends exactly at the slot end `0x2190`).
pub const MEMBER_OFF_WIN_STREAK: u32 = 0x218c;

/// Marks the 4043 handler derives from slot position, not the wire.
///
/// The slot-index branch (files `0x159ec5`–`0x159f5c`) stores a group word
/// at [`MEMBER_OFF_SLOT_GROUP`] (setter file `0x141520`) and a ready byte
/// at [`MEMBER_OFF_READY`] (setter file `0x1400f0`) without reading packet
/// bytes: slot 0 stores `(1, true)`, slot 3 stores `(2, false)`, and any
/// other slot stores `(3, true)`. The 4043 header runs the same 0/3/other
/// mapping over the requesting member id for the local object at
/// `0x77cd00` (files `0x159baf`–`0x159c5e`, plus `+0x2184` via file
/// `0x141500` and `+0x30` via file `0x1400b0`).
/// (`ev_14fd3f2dae86c90bf45f505c7c7a70ef`, observed.)
/// (ERRORS-06: total function, no `unwrap`; PERF-07: no narrowing casts.)
#[must_use]
pub fn slot_position_marks(slot: u8) -> (u16, bool) {
    match slot {
        0 => (1, true),
        3 => (2, false),
        _ => (3, true),
    }
}

/// The two control ids the gender setter chooses between (file
/// `0x13fe50`, called from the gender wire reader at file `0x140c90`).
/// Element 0 is the zero-gender id, element 1 the nonzero-gender id.
pub const MEMBER_GENDER_CONTROL_IDS: [u32; 2] = [0x24, 0x23];

/// Pick the seat control id named by one raw gender byte.
///
/// The exe branches only zero against nonzero; no range gate exists
/// on the wire byte (pin `ev b85e.5113...`).
#[must_use]
pub fn gender_control_id(gender: u8) -> u32 {
    if gender == 0 {
        MEMBER_GENDER_CONTROL_IDS[0]
    } else {
        MEMBER_GENDER_CONTROL_IDS[1]
    }
}

/// Amount the team reader adds to the byte it stored at `+0x31`
/// (helper file `0x140d40`, pin `ev ca8b.21e6...`).
pub const MEMBER_TEAM_STORE_INCREMENT: u8 = 1;

/// Team value left in the member slot after the wire reader's own
/// increment. Values above seven have no pinned seat meaning.
#[must_use]
pub fn stored_team_from_wire(wire: u8) -> u8 {
    wire.wrapping_add(MEMBER_TEAM_STORE_INCREMENT)
}

/// The five equipment control ids the member helper stages at
/// `+0x2040`, `+0x2048`, `+0x2050`, `+0x2058`, `+0x2060` (file
/// `0x140e40`). The first four differ for the two channel records
/// `0x2fd`/`0x2fe`; the fifth is shared.
#[must_use]
pub fn default_equipment_control_ids(channel_id: u32) -> [u32; 5] {
    if channel_id == 0x2fd || channel_id == 0x2fe {
        [0x2f9, 0x2fa, 0x2fc, 0x2fb, 0x22]
    } else {
        [0x1e, 0x1f, 0x21, 0x20, 0x22]
    }
}

/// Opcode of the 2026 premium-time row and description packet.
pub const PREMIUM_TIME_COMMAND: u16 = 2026;
/// Wire width of one premium-time row: `u16` song id plus five status bytes.
pub const PREMIUM_TIME_ROW_LEN: usize = 7;
/// Ink the 2026 handler passes to the text-queue helper (file `0x15a9a0`).
pub const PREMIUM_TIME_TEXT_INK: u8 = 2;

/// One staged premium-time row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PremiumTimeRow {
    /// Song id read as a little-endian word.
    pub song_id: u16,
    /// The five status bytes that follow the song id.
    pub status: [u8; 5],
}

/// Decode the row list of an opcode-2026 body: a little-endian `u32`
/// count followed by fixed-width rows.
///
/// # Errors
///
/// `Error::TruncatedMessage` when the count or a row is incomplete.
pub fn decode_premium_time_rows(body: &[u8]) -> Result<Vec<PremiumTimeRow>, Error> {
    let mut cursor = body;
    let count = take_le_u32(&mut cursor)?;
    let count = usize::try_from(count).map_err(|_| Error::InvalidField {
        field: "premium time row count",
        reason: "count exceeds usize",
    })?;
    let mut rows = Vec::new();
    for _ in 0..count {
        if cursor.len() < PREMIUM_TIME_ROW_LEN {
            return Err(Error::TruncatedMessage {
                need: PREMIUM_TIME_ROW_LEN,
                have: cursor.len(),
            });
        }
        let song_id = take_le_u16(&mut cursor)?;
        let status = [
            take_byte(&mut cursor)?,
            take_byte(&mut cursor)?,
            take_byte(&mut cursor)?,
            take_byte(&mut cursor)?,
            take_byte(&mut cursor)?,
        ];
        rows.push(PremiumTimeRow { song_id, status });
    }
    Ok(rows)
}

/// Number of description lines the 2026 handler posts for one mode
/// bitmask and premium flag (file `0x15a9a0`). The exact text of each
/// line stays OPEN: the format strings are executable resources, not
/// bytes pinned in this repository.
#[must_use]
pub fn premium_description_line_count(mode_bits: u32, premium_active: bool) -> usize {
    match mode_bits {
        0 => usize::from(premium_active) * 3,
        0x200 => 1,
        0x20 | 0x800 => {
            if premium_active {
                5
            } else {
                2
            }
        }
        0x001 | 0x002 | 0x004 | 0x008 | 0x010 | 0x040 | 0x080 | 0x100 => 2,
        _ => 0,
    }
}

/// Split one little-endian `u32` off the front of `body`.
fn take_le_u32(body: &mut &[u8]) -> Result<u32, Error> {
    if body.len() < 4 {
        return Err(Error::TruncatedMessage {
            need: 4,
            have: body.len(),
        });
    }
    let (word, rest) = body.split_at(4);
    *body = rest;
    Ok(u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
}

/// Address (VA) of a member-slot field: `base + slot * stride + offset`.
///
/// # Errors
///
/// `Error::InvalidField` when `slot` is outside `0..8` or `offset` falls
/// outside the `0x2190`-byte slot.
/// (ERRORS-06: checked arithmetic, no `unwrap`; PERF-07: widening `u32`
/// conversions only.)
pub fn member_slot_address(slot: u8, field_offset: u32) -> Result<u32, Error> {
    if usize::from(slot) >= LIVE_STATE_SLOTS {
        return Err(Error::InvalidField {
            field: "live member id",
            reason: "not 0-7",
        });
    }
    if field_offset >= MEMBER_SLOT_STRIDE {
        return Err(Error::InvalidField {
            field: "member field offset",
            reason: "beyond slot",
        });
    }
    let stepped = u32::from(slot)
        .checked_mul(MEMBER_SLOT_STRIDE)
        .and_then(|scaled| scaled.checked_add(field_offset))
        .and_then(|inner| MEMBER_SLOT_BASE.checked_add(inner))
        .ok_or(Error::InvalidField {
            field: "member slot address",
            reason: "offset overflow",
        })?;
    Ok(stepped)
}

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
/// plus `u16` music ids, two `i32` counters, three trailing `u8` bytes,
/// `i32` win streak. The trailing run holds (`playing_state`,
/// `live_extra`, `is_room_master`): the two helpers at files
/// `0x141670`/`0x141690` read one wire byte each into `+0x2130`/`+0x2131`,
/// then file `0x140d20` reads the room-master flag into `+0x30` (call
/// sites files `0x159e90`/`0x159ea8`/`0x159ec0`). There is no fourth
/// trailing flag on this wire: helper file `0x140d90` (the `+0x33`
/// reader) has zero image-wide callers, and `+0x33` is zeroed up front
/// (direct setter file `0x140110`, call site file `0x159e18`); the
/// pre-Memoryer-style administrator flag is not carried here.
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
    /// Room-master flag (the only trailing flag on the wire).
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
        let peers = rows_zero_on_negative_exact(
            body,
            4,
            0,
            4,
            PEER_ENTRY_LEN,
            0,
            "peer count",
            "count too large",
            |row| {
                let member_id = i32::from_le_bytes([row[0], row[1], row[2], row[3]]);
                let (public_endpoint, rest) = decode_socket(&row[4..])?;
                let (local_endpoint, _) = decode_socket(rest)?;
                Ok(PeerInfo {
                    member_id,
                    public_endpoint,
                    local_endpoint,
                })
            },
        )?;
        Ok(Self { peers })
    }
}

/// 4052 (S→C): the Playing-gate event. The dispatcher handler at file
/// `0x163d10` posts `(0x12, 4052, 0, 0)` to the UI queue and reads no
/// wire field (`ev_59fe.f2a7…e2be.4929`). The event carries no body
/// data, so a padded frame must yield the same event as an empty one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayingGateEvent;

impl WireMessage for PlayingGateEvent {
    const COMMAND: u16 = 4052;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(_body: &[u8]) -> Result<Self, Error> {
        Ok(Self)
    }
}

impl PlayingGateEvent {
    /// The UI-queue forward this event stands for (`0x12` category).
    #[must_use]
    pub const fn ui_event() -> (u32, u16, u8, u32) {
        (0x12, Self::COMMAND, 0, 0)
    }
}

/// 4054 (S→C): the body-ignored Playing forward. The handler at file
/// `0x163d30` posts `(0x12, 4054, 0, 0)` and reads no wire field
/// (`ev_1f0c.7727…3ca3.9d5e`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayingForwardEvent;

impl WireMessage for PlayingForwardEvent {
    const COMMAND: u16 = 4054;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(_body: &[u8]) -> Result<Self, Error> {
        Ok(Self)
    }
}

impl PlayingForwardEvent {
    /// The UI-queue forward this event stands for (`0x12` category).
    #[must_use]
    pub const fn ui_event() -> (u32, u16, u8, u32) {
        (0x12, Self::COMMAND, 0, 0)
    }
}

/// 4056 (S→C): the body-ignored relay-confirm echo. The handler at file
/// `0x163d50` posts `(0x12, 4056, 0, 0)` without reading a body word
/// (`ev_cc43.1c11…4263.1af5`). Unlike [`ConfirmRelayAnswer`], which
/// models the empty Memoryer answer, this decoder models the
/// dispatcher forward: padding cannot change the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayConfirmEcho;

impl WireMessage for RelayConfirmEcho {
    const COMMAND: u16 = 4056;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(_body: &[u8]) -> Result<Self, Error> {
        Ok(Self)
    }
}

impl RelayConfirmEcho {
    /// The UI-queue forward this event stands for (`0x12` category).
    #[must_use]
    pub const fn ui_event() -> (u32, u16, u8, u32) {
        (0x12, Self::COMMAND, 0, 0)
    }
}

/// 4050 (S→C): the two speed bytes the dispatcher handler at file
/// `0x163fe0` writes into the self member `+0x2168` slot, followed by
/// the one 4-byte wire word that handler drains (`ev_7689.1c2f…693d`).
/// Both byte reads are pinned; why both target the same slot stays OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayingSpeedEvent {
    /// First byte written to the speed slot.
    pub first_speed: u8,
    /// Second byte written to the same speed slot.
    pub final_speed: u8,
}

impl WireMessage for PlayingSpeedEvent {
    const COMMAND: u16 = 4050;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = vec![self.first_speed, self.final_speed];
        body.extend_from_slice(&0u32.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 2 {
            return Err(Error::TruncatedMessage {
                need: 2,
                have: body.len(),
            });
        }
        Ok(Self {
            first_speed: body[0],
            final_speed: body[1],
        })
    }
}

/// 4049 (S→C): the fixed 20-byte latency answer read by file `0x186470`
/// into `+0x18`, then forwarded as five 4-byte words (`ev_8908.f66c…4572`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencyAnswer {
    /// The five staged words, in wire order.
    pub words: [u32; 5],
}

impl WireMessage for LatencyAnswer {
    const COMMAND: u16 = 4049;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(20);
        for word in self.words {
            body.extend_from_slice(&word.to_le_bytes());
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() != 20 {
            return Err(Error::TruncatedMessage {
                need: 20,
                have: body.len(),
            });
        }
        let mut words = [0u32; 5];
        for (index, word) in words.iter_mut().enumerate() {
            let at = index * 4;
            *word = u32::from_le_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]]);
        }
        Ok(Self { words })
    }
}

#[cfg(test)]
mod relay_command_tests {
    use super::*;
    use crate::frame::decode_frame;
    use crate::relay::{ADJUNCT_SETUP_NUMBER, RELAY_UDP_PORTS};
    use std::net::Ipv4Addr;

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
    fn relay_command_ids_round_trip() {
        let all = [
            RelayCommand::PeerConnected,
            RelayCommand::PeerConfirm,
            RelayCommand::PeerEndpointAssigned,
            RelayCommand::Ping,
            RelayCommand::UpdateGameStats,
            RelayCommand::ComboBroken,
            RelayCommand::ComboStarted,
            RelayCommand::CreateSession,
            RelayCommand::DeleteSession,
        ];
        for command in all {
            assert_eq!(RelayCommand::from_id(command.id()), Some(command));
        }
        assert_eq!(RelayCommand::from_id(0), None);
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

/// Value sets of the live/room gateway leg the relay peer leg depends on.
///
/// Widths are exe-pinned (`u16` role, `u8` lifecycle/difficulty/speed,
/// `i32` code: the fanout wrappers stage routing/autoplay/role through
/// helpers with pushed sizes `0x18`/`0x04`/`0x02`, matching the 24/4/2 wire
/// lengths). Value meanings below are Memoryer-corroborated; exe consumer
/// sites stay OPEN except `MemberRole` and the combo client code, both
/// pinned in `docs/relay.md` §11. These tests lock the shared value tables
/// in one place so the relay leg and the room leg cannot drift apart.
/// (ERRORS-06: no `unwrap`; PERF-07: no narrowing casts.)
#[cfg(test)]
mod relay_enum_value_tests {
    use crate::rooms::{Difficulty, GameSpeed, RoomState};

    #[test]
    fn room_state_maps_all_lifecycle_codes() {
        assert_eq!(RoomState::from_code(0), Some(RoomState::Unavailable));
        assert_eq!(RoomState::from_code(1), Some(RoomState::Waiting));
        assert_eq!(RoomState::from_code(2), Some(RoomState::Playing));
        assert_eq!(RoomState::from_code(3), None);
    }

    #[test]
    fn difficulty_maps_all_chart_codes() {
        assert_eq!(Difficulty::from_code(0), Some(Difficulty::Ex));
        assert_eq!(Difficulty::from_code(1), Some(Difficulty::Nx));
        assert_eq!(Difficulty::from_code(2), Some(Difficulty::Hx));
        assert_eq!(Difficulty::from_code(3), Some(Difficulty::Rx));
        assert_eq!(Difficulty::from_code(4), None);
    }

    #[test]
    fn game_speed_maps_all_scroll_codes() {
        assert_eq!(GameSpeed::from_code(0), Some(GameSpeed::X05));
        assert_eq!(GameSpeed::from_code(1), Some(GameSpeed::X10));
        assert_eq!(GameSpeed::from_code(2), Some(GameSpeed::X15));
        assert_eq!(GameSpeed::from_code(3), Some(GameSpeed::X20));
        assert_eq!(GameSpeed::from_code(4), Some(GameSpeed::X25));
        assert_eq!(GameSpeed::from_code(5), Some(GameSpeed::X30));
        assert_eq!(GameSpeed::from_code(6), Some(GameSpeed::X35));
        assert_eq!(GameSpeed::from_code(7), Some(GameSpeed::X40));
        assert_eq!(GameSpeed::from_code(8), Some(GameSpeed::X45));
        assert_eq!(GameSpeed::from_code(9), Some(GameSpeed::X50));
        assert_eq!(GameSpeed::from_code(10), Some(GameSpeed::X60));
        assert_eq!(GameSpeed::from_code(11), Some(GameSpeed::Xr));
        assert_eq!(GameSpeed::from_code(12), None);
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
    use std::net::Ipv4Addr;

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

    // `sample_info()` layout: 9-byte header, one active byte, then 100
    // member-prefix bytes before the trailing pair.
    const PLAYING_STATE_BODY_INDEX: usize = 110;
    const LIVE_EXTRA_BODY_INDEX: usize = 111;

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
            live_extra: 0x5A,
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
        // seam: pure 4043 codec. Helper file `0x141670` writes the
        // reader object at `+0x2130` first; helper file `0x141690` writes
        // `+0x2131` next (`docs/pins/net-a.md:232`, `ev 6aeb.2ada.373c.
        // 103a.9ed5.0f6b.e571.d2e5.d7b5.8599.19f2.f5a5.bbcd.c655.56b3.
        // c245`). The live-extra VALUE is OPEN: find the consumer of the
        // reader object at `+0x2131` as the next evidence step.
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
        let body = snapshot.encode_body();
        assert_eq!(
            body,
            vec![
                0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x41, 0x42, 0x00,
                0x0A, 0x00, 0x00, 0x00, 0x01, 0x64, 0x00, 0x00, 0x00, 0x02, 0x01, 0x03, 0x01, 0x00,
                0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
                0x05, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x08, 0x00,
                0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x0B, 0x00, 0x00, 0x00,
                0x0C, 0x00, 0x00, 0x00, 0x0D, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x0F, 0x00,
                0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
                0x07, 0x00, 0xB8, 0x01, 0x88, 0x13, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x5A,
                0x01, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]
        );
        assert_eq!(
            body[PLAYING_STATE_BODY_INDEX], 0x02,
            "playing-state byte belongs to reader +0x2130 (helper file 0x141670, VA 0x541670)"
        );
        assert_eq!(
            body[LIVE_EXTRA_BODY_INDEX], 0x5A,
            "live-extra byte belongs to reader +0x2131 (helper file 0x141690, VA 0x541690)"
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
    fn seat_bytes_follow_gem_in_wire_order() {
        // member_id(1) + NUL-terminated nickname(3) + level(4) + gender(1) + gem(4).
        let mut tail = Vec::new();
        sample_info().encode_inner(&mut tail);
        assert_eq!(&tail[13..16], &[0x02, 0x01, 0x03]);
    }

    #[test]
    fn trailing_flag_is_room_master_not_administrator() {
        // The wire carries exactly three trailing bytes (playing_state,
        // live_extra, room-master); no administrator byte exists (helper
        // file `0x140d90` has no caller). The tail therefore ends with
        // `02 5A 01` ahead of the win streak.
        let mut tail = Vec::new();
        sample_info().encode_inner(&mut tail);
        let tail_len = tail.len();
        assert_eq!(
            &tail[tail_len - 7..tail_len],
            &[0x02, 0x5A, 0x01, 0x04, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn two_consecutive_active_members_stay_aligned() {
        // Guards the single-trailing-flag shape: with two active tails in
        // a row, a phantom fourth flag byte would eat the next slot's
        // active byte and desynchronize the parse.
        let mut second = sample_info();
        second.nickname = "C".to_owned();
        second.is_room_master = false;
        let mut members = inactive_slots();
        members[0] = MemberLiveState {
            active: true,
            info: Some(sample_info()),
        };
        members[1] = MemberLiveState {
            active: true,
            info: Some(second),
        };
        let snapshot = LiveStateResponse {
            member_id: 0,
            user_count: 2,
            win_streak: 0,
            members,
        };
        let decoded = LiveStateResponse::decode_body(&snapshot.encode_body()).unwrap();
        assert_eq!(decoded, snapshot);
        assert!(decoded.members[0].info.as_ref().unwrap().is_room_master);
        assert!(!decoded.members[1].info.as_ref().unwrap().is_room_master);
        assert_eq!(decoded.members[1].info.as_ref().unwrap().nickname, "C");
        assert!(decoded.members[2..].iter().all(|slot| !slot.active));
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
mod member_slot_tests {
    use super::*;

    #[test]
    fn slot_table_matches_exe_base_and_stride() {
        assert_eq!(MEMBER_SLOT_BASE, 0x77ee90);
        assert_eq!(MEMBER_SLOT_STRIDE, 0x2190);
        assert_eq!(MEMBER_SLOT_STRIDE, 8592);
    }

    #[test]
    fn identity_fields_sit_at_low_offsets() {
        assert_eq!(MEMBER_OFF_ID, 0x04);
        assert_eq!(MEMBER_OFF_NICKNAME, 0x05);
        assert_eq!(MEMBER_OFF_LEVEL, 0x20);
        assert_eq!(MEMBER_OFF_GENDER, 0x28);
        assert_eq!(MEMBER_OFF_GEM, 0x2c);
        assert!(
            member_slot_address(0, MEMBER_OFF_NICKNAME).ok()
                < member_slot_address(0, MEMBER_OFF_LEVEL).ok()
        );
    }

    #[test]
    fn seat_bytes_sit_at_expected_offsets() {
        assert_eq!(MEMBER_OFF_TEAM, 0x31);
        assert_eq!(MEMBER_OFF_READY, 0x32);
        assert_eq!(MEMBER_OFF_MUSIC_STATE, 0x217c);
    }

    #[test]
    fn match_index_follows_music_state_with_unset_sentinel() {
        assert_eq!(MEMBER_OFF_MATCH_INDEX, 0x217d);
        assert_eq!(MEMBER_OFF_MATCH_INDEX, MEMBER_OFF_MUSIC_STATE + 1);
        assert_eq!(MEMBER_MATCH_INDEX_UNSET, 0xFF);
        assert!(
            member_slot_address(0, MEMBER_OFF_MUSIC_STATE).ok()
                < member_slot_address(0, MEMBER_OFF_MATCH_INDEX).ok()
        );
        assert!(
            member_slot_address(0, MEMBER_OFF_MATCH_INDEX).ok()
                < member_slot_address(0, MEMBER_OFF_SLOT_GROUP).ok()
        );
    }

    #[test]
    fn equipment_words_sit_at_high_table() {
        assert_eq!(MEMBER_OFF_EQUIP_COUNT, 0x2038);
        assert_eq!(MEMBER_OFF_EQUIPMENTS, 0x2040);
        assert_eq!(MEMBER_OFF_EQUIPMENTS + 17 * 4, 0x2084);
    }

    #[test]
    fn music_ids_follow_their_count_word() {
        assert_eq!(MEMBER_OFF_MUSIC_COUNT, 0x34);
        assert_eq!(MEMBER_OFF_MUSIC_IDS, 0x38);
        assert!(
            member_slot_address(0, MEMBER_OFF_MUSIC_COUNT).ok()
                < member_slot_address(0, MEMBER_OFF_MUSIC_IDS).ok()
        );
    }

    #[test]
    fn trailing_state_orders_cash_then_flags_then_streak() {
        assert_eq!(MEMBER_OFF_CASH_POINT, 0x2128);
        assert_eq!(MEMBER_OFF_FREE_PASS, 0x212c);
        assert_eq!(MEMBER_OFF_PLAYING_STATE, 0x2130);
        assert_eq!(MEMBER_OFF_LIVE_EXTRA, 0x2131);
        assert_eq!(MEMBER_OFF_ROOM_MASTER, 0x30);
        assert_eq!(MEMBER_OFF_SPARE_FLAG, 0x33);
        assert_eq!(MEMBER_OFF_WIN_STREAK, 0x218c);
        assert!(
            member_slot_address(0, MEMBER_OFF_CASH_POINT).ok()
                < member_slot_address(0, MEMBER_OFF_FREE_PASS).ok()
        );
        assert!(
            member_slot_address(0, MEMBER_OFF_FREE_PASS).ok()
                < member_slot_address(0, MEMBER_OFF_PLAYING_STATE).ok()
        );
        assert!(
            member_slot_address(0, MEMBER_OFF_PLAYING_STATE).ok()
                < member_slot_address(0, MEMBER_OFF_LIVE_EXTRA).ok()
        );
    }

    #[test]
    fn win_streak_ends_exactly_at_slot_end() {
        assert_eq!(MEMBER_OFF_WIN_STREAK + 4, MEMBER_SLOT_STRIDE);
    }

    #[test]
    fn slot_group_word_sits_before_streak() {
        assert_eq!(MEMBER_OFF_SLOT_GROUP, 0x2186);
        assert!(
            member_slot_address(0, MEMBER_OFF_SLOT_GROUP).ok()
                < member_slot_address(0, MEMBER_OFF_WIN_STREAK).ok()
        );
    }

    #[test]
    fn slot_zero_marks_group_one_and_ready() {
        assert_eq!(slot_position_marks(0), (1, true));
    }

    #[test]
    fn slot_three_marks_group_two_and_not_ready() {
        assert_eq!(slot_position_marks(3), (2, false));
    }

    #[test]
    fn other_slots_mark_group_three_and_ready() {
        for slot in [1, 2, 4, 5, 6, 7] {
            assert_eq!(slot_position_marks(slot), (3, true));
        }
    }

    #[test]
    fn slot_address_steps_by_stride_per_slot() {
        assert_eq!(
            member_slot_address(0, MEMBER_OFF_ID),
            Ok(MEMBER_SLOT_BASE + MEMBER_OFF_ID)
        );
        assert_eq!(
            member_slot_address(3, MEMBER_OFF_ID),
            Ok(MEMBER_SLOT_BASE + 3 * MEMBER_SLOT_STRIDE + MEMBER_OFF_ID)
        );
        assert_eq!(
            member_slot_address(7, MEMBER_OFF_WIN_STREAK),
            Ok(MEMBER_SLOT_BASE + 7 * MEMBER_SLOT_STRIDE + MEMBER_OFF_WIN_STREAK)
        );
    }

    #[test]
    fn match_index_address_steps_by_stride_per_slot() {
        assert_eq!(
            member_slot_address(2, MEMBER_OFF_MATCH_INDEX),
            Ok(MEMBER_SLOT_BASE + 2 * MEMBER_SLOT_STRIDE + MEMBER_OFF_MATCH_INDEX)
        );
        assert_eq!(
            member_slot_address(7, MEMBER_OFF_MATCH_INDEX),
            Ok(MEMBER_SLOT_BASE + 7 * MEMBER_SLOT_STRIDE + MEMBER_OFF_MATCH_INDEX)
        );
    }

    #[test]
    fn slot_address_rejects_slot_at_bound() {
        assert_eq!(
            member_slot_address(8, MEMBER_OFF_ID),
            Err(Error::InvalidField {
                field: "live member id",
                reason: "not 0-7"
            })
        );
    }

    #[test]
    fn slot_address_rejects_offset_beyond_slot() {
        assert_eq!(
            member_slot_address(0, MEMBER_SLOT_STRIDE),
            Err(Error::InvalidField {
                field: "member field offset",
                reason: "beyond slot"
            })
        );
    }

    #[test]
    fn sibling_commands_bind_room_dispatcher_ids() {
        assert_eq!(MEMBER_LEAD_FLAG_COMMAND, 3003);
        assert_eq!(MEMBER_TRAIL_FLAG_COMMAND, 3007);
        assert_ne!(MEMBER_LEAD_FLAG_COMMAND, MEMBER_TRAIL_FLAG_COMMAND);
    }

    #[test]
    fn extra_dword_sits_after_live_extra() {
        assert_eq!(MEMBER_OFF_EXTRA_DWORD, 0x2134);
        assert!(
            member_slot_address(0, MEMBER_OFF_LIVE_EXTRA).ok()
                < member_slot_address(0, MEMBER_OFF_EXTRA_DWORD).ok()
        );
        assert!(
            member_slot_address(0, MEMBER_OFF_EXTRA_DWORD).ok()
                < member_slot_address(0, MEMBER_OFF_SLOT_GROUP).ok()
        );
    }

    #[test]
    fn extra_dword_address_steps_by_stride() {
        assert_eq!(
            member_slot_address(2, MEMBER_OFF_EXTRA_DWORD),
            Ok(MEMBER_SLOT_BASE + 2 * MEMBER_SLOT_STRIDE + MEMBER_OFF_EXTRA_DWORD)
        );
    }
}

#[cfg(test)]
mod seat_format_tests {
    use super::*;

    #[test]
    fn gender_byte_selects_the_paired_control_id() {
        // Member helper file `0x140c90`, setter file `0x13fe50`:
        // a zero gender picks 0x24, any nonzero gender picks 0x23.
        assert_eq!(gender_control_id(0), 0x24);
        for gender in [1, 2, 0xFF] {
            assert_eq!(gender_control_id(gender), 0x23);
        }
        assert_eq!(MEMBER_GENDER_CONTROL_IDS, [0x24, 0x23]);
    }

    #[test]
    fn wire_team_byte_lands_with_the_exe_increment() {
        // Helper file `0x140d40`: read one byte into +0x31, then
        // increment that same stored byte (pin ev ca8b.21e6...).
        assert_eq!(MEMBER_TEAM_STORE_INCREMENT, 1);
        for (wire, stored) in [(0, 1), (1, 2), (6, 7)] {
            assert_eq!(stored_team_from_wire(wire), stored);
        }
        assert_ne!(stored_team_from_wire(3), 3);
    }

    #[test]
    fn equipment_default_control_ids_follow_the_channel_id() {
        // Helper file `0x140e40`: two channel records (0x2fd/0x2fe)
        // receive the premium id set; every other record receives the
        // normal set, with 0x22 as the shared fifth id.
        let premium = default_equipment_control_ids(0x2fd);
        let normal = default_equipment_control_ids(0x300);
        assert_eq!(premium, [0x2f9, 0x2fa, 0x2fc, 0x2fb, 0x22]);
        assert_eq!(normal, [0x1e, 0x1f, 0x21, 0x20, 0x22]);
        assert_eq!(default_equipment_control_ids(0x2fe), premium);
        assert_ne!(premium, normal);
    }

    #[test]
    fn premium_time_rows_and_description_lines_keep_their_format() {
        // Opcode 0x7ea (2026), helper file `0x15a9a0`: a u32 row count,
        // a u16 song id plus five status bytes per row, and text posted
        // with ink 2 whose line count follows the mode bitmask.
        assert_eq!(PREMIUM_TIME_COMMAND, 2026);
        assert_eq!(PREMIUM_TIME_ROW_LEN, 7);
        assert_eq!(PREMIUM_TIME_TEXT_INK, 2);
        let body = [
            2u8, 0, 0, 0, 0x34, 0x12, 1, 2, 3, 4, 5, 0x07, 0x08, 8, 9, 10, 0, 0,
        ];
        let rows = decode_premium_time_rows(&body).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].song_id, 0x1234);
        assert_eq!(rows[0].status, [1, 2, 3, 4, 5]);
        assert_eq!(rows[1].song_id, 0x0807);
        assert_eq!(rows[1].status, [8, 9, 10, 0, 0]);
    }

    #[test]
    fn premium_time_bitmask_cases_stage_their_description_lines() {
        // Every mode case observed in the 0x15a9a0 branch tree: the
        // counts are the lines posted before the opcode-tagged message.
        let cases = [
            (0x000, false, 0),
            (0x000, true, 3),
            (0x001, false, 2),
            (0x002, false, 2),
            (0x004, false, 2),
            (0x008, false, 2),
            (0x010, false, 2),
            (0x020, false, 2),
            (0x020, true, 5),
            (0x040, false, 2),
            (0x080, false, 2),
            (0x100, false, 2),
            (0x200, false, 1),
            (0x800, false, 2),
            (0x800, true, 5),
            (0x003, true, 0),
        ];
        for (bits, premium, lines) in cases {
            assert_eq!(
                premium_description_line_count(bits, premium),
                lines,
                "mode {bits:#05x}, premium {premium}"
            );
        }
    }
}

#[cfg(test)]
mod playing_wire_codec_tests {
    use super::*;
    use crate::frame::{decode_frame, encode_frame};
    use std::net::Ipv4Addr;

    #[test]
    fn playing_gate_4052_ignores_empty_and_padded_bodies() {
        // seam: pure codec — the exe handler reads no wire field
        // (VA `0x563d10`, file `0x163d10`, ev `59fe.f2a7…e2be.4929`).
        let empty = decode_frame(&encode_frame(4052, &[])).unwrap();
        let padded = decode_frame(&encode_frame(4052, &[7, 0x33, 0xaa])).unwrap();
        assert_eq!(PlayingGateEvent::decode_frame(&empty), Ok(PlayingGateEvent));
        assert_eq!(
            PlayingGateEvent::decode_frame(&padded),
            Ok(PlayingGateEvent),
            "a padded frame must yield the same one event"
        );
        assert_eq!(PlayingGateEvent::ui_event(), (0x12, 4052, 0, 0));
    }

    // seam: pure codec — the exe handler only forwards immediates
    // (VA `0x563d30`, file `0x163d30`,
    // ev_1f0c7727e2e4abbc318a9d5ee5450ff7528b5a916086a973a4fb0429dbae3ca3).
    #[test]
    fn playing_forward_4054_stays_distinct_from_the_4052_gate_event() {
        let forward = decode_frame(&encode_frame(4054, &[9, 9])).unwrap();
        let gate = decode_frame(&encode_frame(4052, &[9, 9])).unwrap();
        assert_eq!(
            PlayingForwardEvent::decode_frame(&forward),
            Ok(PlayingForwardEvent)
        );
        assert_eq!(
            PlayingGateEvent::decode_frame(&forward),
            Err(Error::UnknownCommand(4054))
        );
        assert_eq!(PlayingGateEvent::decode_frame(&gate), Ok(PlayingGateEvent));
        assert_ne!(
            PlayingForwardEvent::ui_event(),
            PlayingGateEvent::ui_event(),
            "the two body-ignored opcodes must not collapse into one event"
        );
    }

    #[test]
    fn relay_confirm_4056_echo_ignores_empty_and_padded_bodies() {
        // seam: pure codec — the dispatcher forward reads no body word
        // (VA `0x563d50`, file `0x163d50`, ev `cc43.1c11…4263.1af5`).
        let empty = decode_frame(&encode_frame(4056, &[])).unwrap();
        let padded = decode_frame(&encode_frame(4056, &[0xff; 6])).unwrap();
        assert_eq!(RelayConfirmEcho::decode_frame(&empty), Ok(RelayConfirmEcho));
        assert_eq!(
            RelayConfirmEcho::decode_frame(&padded),
            Ok(RelayConfirmEcho)
        );
        assert_eq!(RelayConfirmEcho::ui_event(), (0x12, 4056, 0, 0));
    }

    #[test]
    fn playing_speed_4050_stages_both_byte_writes_and_drains_the_tail() {
        // seam: pure codec — both byte writes and the trailing word are
        // the whole wire body (VA `0x563fe0`, file `0x163fe0`,
        // ev `7689.1c2f…693d`).
        let event = PlayingSpeedEvent {
            first_speed: 3,
            final_speed: 9,
        };
        let frame = decode_frame(&event.encode()).unwrap();
        assert_eq!(frame.id, 4050);
        assert_eq!(frame.payload.len(), 6);
        assert_eq!(PlayingSpeedEvent::decode_frame(&frame), Ok(event));
        assert!(PlayingSpeedEvent::decode_body(&[3]).is_err());
    }

    #[test]
    fn latency_answer_4049_reads_exactly_five_words() {
        // seam: pure codec — the exe reads a fixed 20-byte area
        // (VA `0x586470`, file `0x186470`, ev `8908.f66c…4572`).
        let answer = LatencyAnswer {
            words: [0x11, 0x2233, 0x4455_6677, 0x8899_aabb, 0xccdd_eeff],
        };
        let frame = decode_frame(&answer.encode()).unwrap();
        assert_eq!(frame.id, 4049);
        assert_eq!(frame.payload.len(), 20);
        assert_eq!(LatencyAnswer::decode_frame(&frame), Ok(answer));
        assert!(LatencyAnswer::decode_body(&[0; 16]).is_err());
        assert!(LatencyAnswer::decode_body(&[0; 24]).is_err());
    }

    #[test]
    fn p2p_list_4045_two_peers_carry_member_and_both_six_byte_endpoints() {
        // seam: pure codec — signed count plus 4-byte member and two
        // 6-byte endpoints per peer (VA `0x58cf50`, file `0x18cf50`,
        // ev `41b6.1271…f864.b518`; dispatch VA `0x563d70`).
        let peers = P2PListResponse {
            peers: vec![
                PeerInfo {
                    member_id: -2,
                    public_endpoint: SocketAddrV4::new(Ipv4Addr::new(203, 0, 113, 5), 4000),
                    local_endpoint: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 5), 4001),
                },
                PeerInfo {
                    member_id: 3,
                    public_endpoint: SocketAddrV4::new(Ipv4Addr::new(203, 0, 113, 6), 4002),
                    local_endpoint: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 6), 4003),
                },
            ],
        };
        let body = peers.encode_body();
        assert_eq!(body.len(), 4 + 2 * 16);
        assert_eq!(
            P2PListResponse::decode_body(&body),
            Ok(peers),
            "the signed member word and both endpoints must survive the round trip"
        );
        assert_eq!(
            P2PListResponse::decode_body(&body[..19]),
            Err(Error::TruncatedMessage { need: 36, have: 19 })
        );
    }
}
