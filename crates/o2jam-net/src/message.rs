//! Phase-1 message layouts (Memoryer field orders).
//!
//! Fixed-size bodies only for now: `Authorize` (1000) carries
//! strings whose encoding is not yet exe-verified, so it stays
//! out until the string codec is pinned down.

use std::net::{Ipv4Addr, SocketAddrV4};

use super::command::Request;
use super::error::Error;
use super::frame::{encode_frame, Frame};
use super::reader::Reader;
use super::rows::rows_reject_negative_exact;
use super::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};

/// Room-list and room-row codecs live in the split `rooms/list.rs`.
/// The module is compiled here until the `rooms/` split is wired, so its
/// codec guards run with the rest of the net suite (lane room-wire).
#[path = "rooms/list.rs"]
pub mod room_wire;
pub use room_wire::{
    RoomDescriptorChanged, RoomParameterChanged, RoomSkillChanged, RoomStateChanged,
    RoomUserCountChanged, RoomUserList, RoomUserListEntry,
};

/// Encode/decode contract for one command's body.
pub trait WireMessage: Sized {
    /// Command id this body travels under.
    const COMMAND: u16;
    /// Body bytes after the 4-byte header.
    fn encode_body(&self) -> Vec<u8>;
    /// Parse exactly the body bytes (no header).
    ///
    /// # Errors
    ///
    /// Whatever the command layout reports for malformed bodies.
    fn decode_body(body: &[u8]) -> Result<Self, Error>;

    /// Full frame bytes ready to send.
    fn encode(&self) -> Vec<u8> {
        encode_frame(Self::COMMAND, &self.encode_body())
    }

    /// Parse a received frame, checking its command first.
    ///
    /// # Errors
    ///
    /// `Error::UnknownCommand` on a foreign id, plus body errors.
    fn decode_frame(frame: &Frame) -> Result<Self, Error> {
        if frame.id != Self::COMMAND {
            return Err(Error::UnknownCommand(frame.id));
        }
        let message = Self::decode_body(&frame.payload)?;
        Ok(message)
    }
}

/// Empty body shared by id-only requests.
pub(crate) fn decode_empty(body: &[u8]) -> Result<(), Error> {
    if body.is_empty() {
        Ok(())
    } else {
        Err(Error::TrailingBytes(body.len()))
    }
}

/// Marker for empty-body messages; each unit struct below names
/// the command it travels under.
pub trait EmptyMessage {
    /// Command id this empty body travels under.
    const COMMAND: u16;
}

/// Encode an empty body under the type's command.
pub(crate) fn encode_empty<T: EmptyMessage>() -> Vec<u8> {
    encode_frame(T::COMMAND, &[])
}

/// Decode an empty body, rejecting any payload bytes.
pub(crate) fn decode_empty_frame<T: EmptyMessage>(frame: &Frame) -> Result<(), Error> {
    if frame.id != T::COMMAND {
        return Err(Error::UnknownCommand(frame.id));
    }
    decode_empty(&frame.payload)
}

macro_rules! empty_message {
    ($name:ident, $command:expr) => {
        #[doc = concat!("Empty-body message, command `", stringify!($command), "`")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;

        impl EmptyMessage for $name {
            const COMMAND: u16 = $command;
        }

        impl $name {
            /// Full frame bytes ready to send.
            #[must_use]
            pub fn encode(&self) -> Vec<u8> {
                encode_empty::<Self>()
            }

            /// Parse a received frame, checking command and emptiness.
            ///
            /// # Errors
            ///
            /// `Error::UnknownCommand` on a foreign id,
            /// `Error::TrailingBytes` on any payload.
            pub fn decode_frame(frame: &Frame) -> Result<Self, Error> {
                decode_empty_frame::<Self>(frame)?;
                Ok(Self)
            }
        }
    };
}

empty_message!(GetChannelList, 1002);
empty_message!(SessionKeysRequest, 1007);
empty_message!(Terminate, 65520);
empty_message!(LegacyPing, 6000);
empty_message!(Ping, 6001);
// Session alerts: Memoryer defines 51/52 as empty classes and has no
// message class at all for 50 (bare opcode, never sent by its server
// code); the exe handler for all three is still open, so empty bodies
// are Memoryer corroboration, not exe-verified behavior.
empty_message!(SubscriptionAlert, 50);
empty_message!(ForceReauthorize, 51);
empty_message!(MultiSessionError, 52);
// `GetCharacterInfo` 2000 takes no parameters (its Memoryer handler
// reads nothing but the session), so the request is an empty body.
empty_message!(GetCharacterInfo, 2000);
empty_message!(GetChannelInfo, 2002);
empty_message!(GetUserList, 2010);

/// Client's installed catalog ids (`SendMusicList` 2024, #341).
///
/// Memoryer `MusicListRequest` carries an `i32` count then one `u16`
/// per id; the exe sets the high nibble `0xf000` as the "New" label
/// (`Source/Memoryer/Memoryer/Controllers/MainRoomController.cs:144-167`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMusicList {
    /// Installed catalog ids, high nibble already flagged.
    pub music_ids: Vec<u16>,
}

impl WireMessage for SendMusicList {
    const COMMAND: u16 = Request::SendMusicList as u16;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + self.music_ids.len() * 2);
        body.extend_from_slice(
            &i32::try_from(self.music_ids.len())
                .unwrap_or(i32::MAX)
                .to_le_bytes(),
        );
        for id in &self.music_ids {
            body.extend_from_slice(&id.to_le_bytes());
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let count = reader.read_count("music ids", true)?;
        let mut music_ids = Vec::with_capacity(count);
        for _ in 0..count {
            music_ids.push(reader.read_u16()?);
        }
        reader.finish()?;
        Ok(Self { music_ids })
    }
}

/// Client identifier every 7.55 client sends (Memoryer `AuthRequest.Id`,
/// byte-identical to the in-exe constant at file `0x2472c8`).
pub const MEMORYER_CLIENT_ID: &str = "726PNA-B1YW-28ZG9M-I5SC-Q0OPM6";

/// Gender as the wire carries it: a null-terminated string (Memoryer
/// `AuthGenderCodec` is decode-only and reads `"m"`/`"1"`, `"f"`/`"0"`,
/// else `Any`). A tapped 1000 from the exe sends the launch blob's
/// gender word verbatim (`"1"` for the male tester account, 2026-11-18),
/// so both spellings decode and we send the numeric one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gender {
    /// `"0"` on the wire.
    Female,
    /// `"1"` on the wire.
    Male,
    /// Empty string on the wire (decodes back to `Any`).
    #[default]
    Any,
}

impl Gender {
    /// Wire spelling we send.
    #[must_use]
    pub fn as_code(self) -> &'static str {
        match self {
            Self::Female => "0",
            Self::Male => "1",
            Self::Any => "",
        }
    }

    /// Byte form (`Encore.Metadata.Gender`: Female 0, Male 1, Any 2).
    #[must_use]
    pub fn as_byte(self) -> u8 {
        match self {
            Self::Female => 0,
            Self::Male => 1,
            Self::Any => 2,
        }
    }

    /// Interpret a gender byte. `None` means out of range.
    #[must_use]
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Female),
            1 => Some(Self::Male),
            2 => Some(Self::Any),
            _ => None,
        }
    }

    /// Wire spelling Memoryer accepts (case-insensitive).
    #[must_use]
    pub fn from_code(code: &str) -> Self {
        if code.eq_ignore_ascii_case("m") || code == "1" {
            Self::Male
        } else if code.eq_ignore_ascii_case("f") || code == "0" {
            Self::Female
        } else {
            Self::Any
        }
    }
}

/// One IPv4 endpoint as Memoryer frames it: 4 address bytes then the
/// port as little-endian u16 (6 bytes total — same as the exe's two
/// inline 6-byte blocks at the `Authorize` builder).
fn encode_endpoint(endpoint: SocketAddrV4, body: &mut Vec<u8>) {
    body.extend_from_slice(&endpoint.ip().octets());
    body.extend_from_slice(&endpoint.port().to_le_bytes());
}

/// Split 6 endpoint bytes off the front of `body`.
fn decode_endpoint(body: &[u8]) -> Result<(SocketAddrV4, &[u8]), Error> {
    if body.len() < 6 {
        return Err(Error::TruncatedMessage {
            need: 6,
            have: body.len(),
        });
    }
    let ip = Ipv4Addr::new(body[0], body[1], body[2], body[3]);
    let port = u16::from_le_bytes([body[4], body[5]]);
    Ok((SocketAddrV4::new(ip, port), &body[6..]))
}

/// `Authorize` 1000 request in Memoryer's field order: marker, relay
/// keys, public + local endpoints, user id, token, gender, marker,
/// client id. This is the order our client sends to Memoryer; the
/// exe's builder emits the same shape (its second runtime string now
/// reads as the gender string), with byte-level proof still open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizeRequest {
    /// Relay session key 1.
    pub relay_key1: i32,
    /// Relay session key 2.
    pub relay_key2: i32,
    /// Client's public address.
    pub public_endpoint: SocketAddrV4,
    /// Client's LAN address.
    pub local_endpoint: SocketAddrV4,
    /// Account id.
    pub user_id: i32,
    /// Username.
    pub token: String,
    /// Account gender.
    pub gender: Gender,
    /// Client id; defaults to [`MEMORYER_CLIENT_ID`].
    pub client_id: String,
}

impl AuthorizeRequest {
    /// Request with the standard client id filled in.
    #[must_use]
    pub fn new(
        relay_key1: i32,
        relay_key2: i32,
        public_endpoint: SocketAddrV4,
        local_endpoint: SocketAddrV4,
        user_id: i32,
        token: String,
        gender: Gender,
    ) -> Self {
        Self {
            relay_key1,
            relay_key2,
            public_endpoint,
            local_endpoint,
            user_id,
            token,
            gender,
            client_id: MEMORYER_CLIENT_ID.to_owned(),
        }
    }
}

impl WireMessage for AuthorizeRequest {
    const COMMAND: u16 = 1000;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::new();
        body.push(1);
        body.extend_from_slice(&self.relay_key1.to_le_bytes());
        body.extend_from_slice(&self.relay_key2.to_le_bytes());
        encode_endpoint(self.public_endpoint, &mut body);
        encode_endpoint(self.local_endpoint, &mut body);
        body.extend_from_slice(&self.user_id.to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.token, MAX_STRING_LEN));
        body.extend_from_slice(&encode_null_terminated(
            self.gender.as_code(),
            MAX_STRING_LEN,
        ));
        body.push(1);
        body.extend_from_slice(&encode_null_terminated(&self.client_id, MAX_STRING_LEN));
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 25 {
            return Err(Error::TruncatedMessage {
                need: 25,
                have: body.len(),
            });
        }
        let rest = &body[1..];
        let relay_key1 = i32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
        let relay_key2 = i32::from_le_bytes([rest[4], rest[5], rest[6], rest[7]]);
        let (public_endpoint, tail) = decode_endpoint(&rest[8..])?;
        let (local_endpoint, tail) = decode_endpoint(tail)?;
        let user_id = i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
        let (token, token_used) = decode_null_terminated(&tail[4..], MAX_STRING_LEN)?;
        let after_token = &tail[4 + token_used..];
        let (gender_code, gender_used) = decode_null_terminated(after_token, MAX_STRING_LEN)?;
        let after_gender = &after_token[gender_used..];
        if after_gender.len() < 2 {
            return Err(Error::TruncatedMessage {
                need: 2,
                have: after_gender.len(),
            });
        }
        let (client_id, client_used) = decode_null_terminated(&after_gender[1..], MAX_STRING_LEN)?;
        let rest = &after_gender[1 + client_used..];
        if !rest.is_empty() {
            return Err(Error::TrailingBytes(rest.len()));
        }
        Ok(Self {
            relay_key1,
            relay_key2,
            public_endpoint,
            local_endpoint,
            user_id,
            token,
            gender: Gender::from_code(&gender_code),
            client_id,
        })
    }
}

/// `ChannelLogin` 1004: two little-endian shorts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelLogin {
    /// Gateway from the channel list (`short`).
    pub gateway_id: i16,
    /// Channel from the channel list (`short`).
    pub channel_id: i16,
}

impl WireMessage for ChannelLogin {
    const COMMAND: u16 = 1004;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4);
        body.extend_from_slice(&self.gateway_id.to_le_bytes());
        body.extend_from_slice(&self.channel_id.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        if body.len() > 4 {
            return Err(Error::TrailingBytes(body.len() - 4));
        }
        Ok(Self {
            gateway_id: i16::from_le_bytes([body[0], body[1]]),
            channel_id: i16::from_le_bytes([body[2], body[3]]),
        })
    }
}

/// `ChannelLogin` result codes (Memoryer `ChannelLoginResult`, `uint`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ChannelLoginResult {
    /// Channel entered.
    Success = 0,
    /// Channel needs a premium subscription.
    PremiumNotAuthorized = 1,
    /// Channel info lookup failed.
    ChannelInfoUnavailable = 6,
    /// Channel is closed or full.
    ChannelUnavailable = 7,
}

impl ChannelLoginResult {
    /// Interpret a raw result code. `None` means a code the 7.55
    /// server never sends (kept extensible, never guessed).
    #[must_use]
    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            1 => Some(Self::PremiumNotAuthorized),
            6 => Some(Self::ChannelInfoUnavailable),
            7 => Some(Self::ChannelUnavailable),
            _ => None,
        }
    }
}

/// `ChannelLogin` 1005 response: `u32` result plus an optional
/// trailing `i32` restricted flag (Memoryer `bool? Restricted` —
/// Encore skips trailing-nullable members when the stream is
/// exhausted, so a 4-byte body means "no restriction info").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelLoginResponse {
    /// Raw result code; see [`ChannelLoginResult::from_code`].
    pub result_code: u32,
    /// Account restriction flag; `None` when the server sent none.
    pub restricted: Option<bool>,
}

/// Body length without the optional flag.
const CHANNEL_LOGIN_RESPONSE_MIN_LEN: usize = 4;
/// Body length with the optional flag.
const CHANNEL_LOGIN_RESPONSE_MAX_LEN: usize = 8;

impl WireMessage for ChannelLoginResponse {
    const COMMAND: u16 = 1005;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(CHANNEL_LOGIN_RESPONSE_MAX_LEN);
        body.extend_from_slice(&self.result_code.to_le_bytes());
        if let Some(restricted) = self.restricted {
            body.extend_from_slice(&i32::from(restricted).to_le_bytes());
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < CHANNEL_LOGIN_RESPONSE_MIN_LEN {
            return Err(Error::TruncatedMessage {
                need: CHANNEL_LOGIN_RESPONSE_MIN_LEN,
                have: body.len(),
            });
        }
        if body.len() != CHANNEL_LOGIN_RESPONSE_MIN_LEN
            && body.len() != CHANNEL_LOGIN_RESPONSE_MAX_LEN
        {
            return Err(Error::TrailingBytes(
                body.len() - CHANNEL_LOGIN_RESPONSE_MIN_LEN,
            ));
        }
        let restricted = if body.len() == CHANNEL_LOGIN_RESPONSE_MAX_LEN {
            match i32::from_le_bytes([body[4], body[5], body[6], body[7]]) {
                0 => Some(false),
                1 => Some(true),
                _ => {
                    return Err(Error::InvalidField {
                        field: "channel restriction flag",
                        reason: "not 0 or 1",
                    });
                }
            }
        } else {
            None
        };
        Ok(Self {
            result_code: u32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            restricted,
        })
    }
}

/// `Authorize` result codes (Memoryer `AuthResult`, `uint`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AuthResult {
    /// Login accepted.
    Success = 0,
    /// Bad credentials.
    InvalidLogin = 0xFFFF_FFFF,
    /// Account already online.
    AlreadyConnected = 0xFFFF_FFFE,
    /// Region mismatch.
    RegionError = 0xFFFF_FFFD,
    /// PC-room authorization missing.
    UnauthorizedPc = 0xFFFF_FFFC,
    /// Account banned.
    Banned = 0xFFFF_FFFB,
    /// Play time exhausted.
    TimeExceeded = 0xFFFF_FFFA,
    /// Server database failure.
    DatabaseError = 0xFFFF_FF9B,
}

impl AuthResult {
    /// Interpret a raw result code. `None` means a code the 7.55
    /// server never sends (kept extensible, never guessed).
    #[must_use]
    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            0xFFFF_FFFF => Some(Self::InvalidLogin),
            0xFFFF_FFFE => Some(Self::AlreadyConnected),
            0xFFFF_FFFD => Some(Self::RegionError),
            0xFFFF_FFFC => Some(Self::UnauthorizedPc),
            0xFFFF_FFFB => Some(Self::Banned),
            0xFFFF_FFFA => Some(Self::TimeExceeded),
            0xFFFF_FF9B => Some(Self::DatabaseError),
            _ => None,
        }
    }
}

/// `Authorize` 1001 response: `u32` result plus the pass-info tail
/// kept opaque (Memoryer `PassesInfo?` — nested submessages with
/// formatted durations; full decode deferred until the login flow
/// needs subscription display. Encore skips trailing-nullable
/// members when the stream is exhausted, so a 4-byte body is a
/// complete result-only response).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthResponse {
    /// Raw result code; see [`AuthResult::from_code`].
    pub result_code: u32,
    /// Bytes after the result (pass-info when present); untouched.
    pub tail: Vec<u8>,
}

/// Body length of a result-only [`AuthResponse`].
const AUTH_RESPONSE_MIN_LEN: usize = 4;

impl WireMessage for AuthResponse {
    const COMMAND: u16 = 1001;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(AUTH_RESPONSE_MIN_LEN + self.tail.len());
        body.extend_from_slice(&self.result_code.to_le_bytes());
        body.extend_from_slice(&self.tail);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < AUTH_RESPONSE_MIN_LEN {
            return Err(Error::TruncatedMessage {
                need: AUTH_RESPONSE_MIN_LEN,
                have: body.len(),
            });
        }
        Ok(Self {
            result_code: u32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            tail: body[AUTH_RESPONSE_MIN_LEN..].to_vec(),
        })
    }
}

/// Fixed 51-byte `SessionKeys` 1008 body: seed, prefix, 32 random
/// bytes, 16 random bytes (Memoryer `SessionKeysResponse`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionKeys {
    /// Seed for key generation (not the key itself).
    pub seed: i16,
    /// First byte of the primary XOR table (always 0).
    pub prefix: u8,
    /// 32-byte primary XOR key.
    pub primary_key: [u8; 32],
    /// 16-byte secondary XOR key.
    pub secondary_key: [u8; 16],
}

/// Body length of [`SessionKeys`]: 2 + 1 + 32 + 16.
const SESSION_KEYS_BODY_LEN: usize = 51;

impl WireMessage for SessionKeys {
    const COMMAND: u16 = 1008;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SESSION_KEYS_BODY_LEN);
        body.extend_from_slice(&self.seed.to_le_bytes());
        body.push(self.prefix);
        body.extend_from_slice(&self.primary_key);
        body.extend_from_slice(&self.secondary_key);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SESSION_KEYS_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SESSION_KEYS_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SESSION_KEYS_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - SESSION_KEYS_BODY_LEN));
        }
        let mut primary_key = [0u8; 32];
        primary_key.copy_from_slice(&body[3..35]);
        let mut secondary_key = [0u8; 16];
        secondary_key.copy_from_slice(&body[35..51]);
        Ok(Self {
            seed: i16::from_le_bytes([body[0], body[1]]),
            prefix: body[2],
            primary_key,
            secondary_key,
        })
    }
}

/// One row of the `GetChannelList` 1003 response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelState {
    /// Gateway id (`ushort`).
    pub gateway_id: u16,
    /// Channel id (`ushort`).
    pub channel_id: u16,
    /// Slot capacity (`int`).
    pub capacity: i32,
    /// Current population (`int`).
    pub population: i32,
    /// Listed as joinable (`bool` byte).
    pub active: bool,
}

/// Serialized size of one [`ChannelState`] row: 2+2+4+4+1.
const CHANNEL_ROW_LEN: usize = 13;

/// `GetChannelList` 1003 response: `i32` count + rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelList {
    /// Channels in list order.
    pub channels: Vec<ChannelState>,
}

impl WireMessage for ChannelList {
    const COMMAND: u16 = 1003;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + self.channels.len() * CHANNEL_ROW_LEN);
        body.extend_from_slice(&(self.channels.len() as i32).to_le_bytes());
        for channel in &self.channels {
            body.extend_from_slice(&channel.gateway_id.to_le_bytes());
            body.extend_from_slice(&channel.channel_id.to_le_bytes());
            body.extend_from_slice(&channel.capacity.to_le_bytes());
            body.extend_from_slice(&channel.population.to_le_bytes());
            body.push(u8::from(channel.active));
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let channels = rows_reject_negative_exact(
            body,
            4,
            0,
            4,
            CHANNEL_ROW_LEN,
            0,
            "channel count",
            "negative",
            "row bytes overflow address space",
            |row| {
                let active = match row[12] {
                    0 => false,
                    1 => true,
                    _ => {
                        return Err(Error::InvalidField {
                            field: "channel active flag",
                            reason: "not 0 or 1",
                        })
                    }
                };
                Ok(ChannelState {
                    gateway_id: u16::from_le_bytes([row[0], row[1]]),
                    channel_id: u16::from_le_bytes([row[2], row[3]]),
                    capacity: i32::from_le_bytes([row[4], row[5], row[6], row[7]]),
                    population: i32::from_le_bytes([row[8], row[9], row[10], row[11]]),
                    active,
                })
            },
        )?;
        Ok(Self { channels })
    }
}

/// Key mode: 3/5/7-key play (one byte on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyMode {
    ThreeKeys = 0,
    FiveKeys = 1,
    SevenKeys = 2,
}

impl KeyMode {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse a wire byte; unknown values are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside 0–2.
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::ThreeKeys),
            1 => Ok(Self::FiveKeys),
            2 => Ok(Self::SevenKeys),
            _ => Err(Error::InvalidField {
                field: "key mode",
                reason: "not 0, 1, or 2",
            }),
        }
    }
}

/// Game mode: normal play or Live PvP (one byte on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameMode {
    Normal = 0,
    Live = 1,
}

impl GameMode {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse a wire byte; unknown values are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside 0–1.
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Live),
            _ => Err(Error::InvalidField {
                field: "game mode",
                reason: "not 0 or 1",
            }),
        }
    }
}

/// One `0`/`1` byte as bool.
pub(crate) fn read_flag<'body>(
    body: &'body [u8],
    field: &'static str,
) -> Result<(bool, &'body [u8]), Error> {
    let (&first, rest) = body
        .split_first()
        .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
    match first {
        0 => Ok((false, rest)),
        1 => Ok((true, rest)),
        _ => Err(Error::InvalidField {
            field,
            reason: "not 0 or 1",
        }),
    }
}

/// `bool` as little-endian `i16` (`MessageFieldCodec<short>`).
fn encode_short_flag(value: bool, body: &mut Vec<u8>) {
    body.extend_from_slice(&i16::from(value).to_le_bytes());
}

/// Parse an `i16` `0`/`1` as bool.
fn decode_short_flag<'body>(
    body: &'body [u8],
    field: &'static str,
) -> Result<(bool, &'body [u8]), Error> {
    if body.len() < 2 {
        return Err(Error::TruncatedMessage {
            need: 2,
            have: body.len(),
        });
    }
    match i16::from_le_bytes([body[0], body[1]]) {
        0 => Ok((false, &body[2..])),
        1 => Ok((true, &body[2..])),
        _ => Err(Error::InvalidField {
            field,
            reason: "not 0 or 1",
        }),
    }
}

/// Room title cap, bytes including terminator (Memoryer
/// `maxLength: 22`).
pub const ROOM_TITLE_CAP: usize = 22;
/// Room password cap, bytes including terminator (Memoryer
/// `maxLength: 12`).
pub const ROOM_PASSWORD_CAP: usize = 12;

/// `CreateRoom` 2004: title, modes, lock, level band, premium flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRoom {
    /// Room title (max 22 bytes).
    pub title: String,
    /// 3/5/7 keys.
    pub key_mode: KeyMode,
    /// Normal or Live.
    pub game_mode: GameMode,
    /// Whether `password` applies.
    pub has_password: bool,
    /// Room password (max 12 bytes).
    pub password: String,
    /// Minimum level limit.
    pub min_level: u8,
    /// Maximum level limit.
    pub max_level: u8,
    /// Premium-only room.
    pub premium: bool,
}

impl WireMessage for CreateRoom {
    const COMMAND: u16 = 2004;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&encode_null_terminated(&self.title, ROOM_TITLE_CAP));
        body.push(self.key_mode.as_u8());
        body.push(self.game_mode.as_u8());
        body.push(u8::from(self.has_password));
        body.extend_from_slice(&encode_null_terminated(&self.password, ROOM_PASSWORD_CAP));
        body.push(self.min_level);
        body.push(self.max_level);
        encode_short_flag(self.premium, &mut body);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (title, used) = decode_null_terminated(body, ROOM_TITLE_CAP)?;
        let mut tail = &body[used..];
        let (&key_mode, rest) = tail
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        let key_mode = KeyMode::from_u8(key_mode)?;
        tail = rest;
        let (&game_mode, rest) = tail
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        let game_mode = GameMode::from_u8(game_mode)?;
        tail = rest;
        let (has_password, rest) = read_flag(tail, "has password")?;
        let (password, used) = decode_null_terminated(rest, ROOM_PASSWORD_CAP)?;
        tail = &rest[used..];
        let (&min_level, rest) = tail
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        let (&max_level, rest) = rest
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        let (premium, rest) = decode_short_flag(rest, "premium")?;
        if !rest.is_empty() {
            return Err(Error::TrailingBytes(rest.len()));
        }
        Ok(Self {
            title,
            key_mode,
            game_mode,
            has_password,
            password,
            min_level,
            max_level,
            premium,
        })
    }
}

/// `CreateRoom` result: success or channel full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum CreateResult {
    Success = 0,
    ChannelFull = 1,
}

impl CreateResult {
    /// Numeric wire value.
    #[must_use]
    pub fn as_i32(self) -> i32 {
        self as i32
    }

    /// Parse a wire word; unknown values are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside the known codes.
    pub fn from_i32(value: i32) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Success),
            1 => Ok(Self::ChannelFull),
            _ => Err(Error::InvalidField {
                field: "create result",
                reason: "not 0 or 1",
            }),
        }
    }
}

/// Created-room snapshot inside `CreateRoomResponse` 2006 (Memoryer
/// nests it as `RoomInfo`; renamed here since rooms share one namespace).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedRoom {
    /// Room number.
    pub number: i32,
    /// Premium-only room.
    pub premium: bool,
    /// Room password (may be empty).
    pub password: String,
    /// Whether `password` applies.
    pub has_password: bool,
    /// Minimum level limit.
    pub min_level: u8,
    /// Maximum level limit.
    pub max_level: u8,
}

impl CreatedRoom {
    /// Parse from the front of `body`, returning the value and rest.
    fn decode_prefix(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        let number = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let (premium, tail) = decode_short_flag(&body[4..], "premium")?;
        let (password, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
        let tail = &tail[used..];
        let (has_password, tail) = read_flag(tail, "has password")?;
        let (&min_level, tail) = tail
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        let (&max_level, tail) = tail
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        Ok((
            Self {
                number,
                premium,
                password,
                has_password,
                min_level,
                max_level,
            },
            tail,
        ))
    }

    /// Full body for sending (tests and future use).
    fn encode_inner(&self, body: &mut Vec<u8>) {
        body.extend_from_slice(&self.number.to_le_bytes());
        encode_short_flag(self.premium, body);
        body.extend_from_slice(&encode_null_terminated(&self.password, MAX_STRING_LEN));
        body.push(u8::from(self.has_password));
        body.push(self.min_level);
        body.push(self.max_level);
    }
}

/// `CreateRoom` 2006 response: result plus the room slot when
/// creation succeeded (present iff bytes follow the result).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRoomResponse {
    /// Success or channel-full.
    pub result: CreateResult,
    /// Slot snapshot; `None` when the body ends after `result`.
    pub info: Option<CreatedRoom>,
}

impl WireMessage for CreateRoomResponse {
    const COMMAND: u16 = 2006;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4);
        body.extend_from_slice(&self.result.as_i32().to_le_bytes());
        if let Some(info) = &self.info {
            info.encode_inner(&mut body);
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
        let result =
            CreateResult::from_i32(i32::from_le_bytes([body[0], body[1], body[2], body[3]]))?;
        let tail = &body[4..];
        let info = if tail.is_empty() {
            None
        } else {
            let (info, rest) = CreatedRoom::decode_prefix(tail)?;
            if !rest.is_empty() {
                return Err(Error::TrailingBytes(rest.len()));
            }
            Some(info)
        };
        Ok(Self { result, info })
    }
}

/// `RoomCreated` 2005 broadcast: the new room's full descriptor
/// (docs/net-protocol.md §4b; 2007 is `RoomRemoved`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomCreated {
    /// Room number.
    pub number: i32,
    /// Room title.
    pub title: String,
    /// 3/5/7 keys.
    pub key_mode: KeyMode,
    /// Normal or Live.
    pub game_mode: GameMode,
    /// Whether a password is set.
    pub has_password: bool,
    /// Minimum level limit.
    pub min_level: u8,
    /// Maximum level limit.
    pub max_level: u8,
    /// Premium-only room.
    pub premium: bool,
    /// Room type tag.
    pub room_type: u8,
}

impl WireMessage for RoomCreated {
    const COMMAND: u16 = 2005;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4);
        body.extend_from_slice(&self.number.to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.title, MAX_STRING_LEN));
        body.push(self.key_mode.as_u8());
        body.push(self.game_mode.as_u8());
        body.push(u8::from(self.has_password));
        body.push(self.min_level);
        body.push(self.max_level);
        encode_short_flag(self.premium, &mut body);
        body.push(self.room_type);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        let number = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let (title, used) = decode_null_terminated(&body[4..], MAX_STRING_LEN)?;
        let mut tail = &body[4 + used..];
        let (&key_mode, rest) = tail
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        let key_mode = KeyMode::from_u8(key_mode)?;
        tail = rest;
        let (&game_mode, rest) = tail
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        let game_mode = GameMode::from_u8(game_mode)?;
        tail = rest;
        let (has_password, rest) = read_flag(tail, "has password")?;
        tail = rest;
        let (&min_level, rest) = tail
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        let (&max_level, rest) = rest
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        let (premium, rest) = decode_short_flag(rest, "premium")?;
        let (&room_type, rest) = rest
            .split_first()
            .ok_or(Error::TruncatedMessage { need: 1, have: 0 })?;
        if !rest.is_empty() {
            return Err(Error::TrailingBytes(rest.len()));
        }
        Ok(Self {
            number,
            title,
            key_mode,
            game_mode,
            has_password,
            min_level,
            max_level,
            premium,
            room_type,
        })
    }
}

/// `RoomRemoved` 2007 broadcast: the disbanded room's number.
///
/// Memoryer `Messages/Events/MainRoom/RoomRemovedEventData.cs` carries
/// one `i32 Number`. The server sends it to the whole channel after
/// `Room.Remove` finds no member slots left
/// (`~/scratch/p380-wire-probe.log`, opcode 0x07D7, 4-byte body
/// `00000000`; #380).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomRemoved {
    /// Room number that no longer exists.
    pub number: i32,
}

impl WireMessage for RoomRemoved {
    const COMMAND: u16 = 2007;

    fn encode_body(&self) -> Vec<u8> {
        self.number.to_le_bytes().to_vec()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        if body.len() > 4 {
            return Err(Error::TrailingBytes(body.len() - 4));
        }
        Ok(Self {
            number: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
        })
    }
}

/// Waiting wire bodies pinned at 7.55 VA `0x55b6d0` (opcode 2035).
/// The result word gates the one-byte level cap: a non-zero result
/// carries no cap byte. `docs/pins/net-b.md`, REA ev 2a4a.9653…
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreeMusicStatusAnswer {
    /// Zero means the cap byte follows; any other code omits it.
    pub result: i32,
    /// Byte staged as the Waiting auto-pick level ceiling.
    pub level_cap: Option<u8>,
}

impl WireMessage for FreeMusicStatusAnswer {
    const COMMAND: u16 = 2035;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = self.result.to_le_bytes().to_vec();
        if let Some(cap) = self.level_cap {
            body.push(cap);
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let result = reader.read_i32()?;
        let level_cap = if result == 0 {
            Some(reader.read_u8()?)
        } else {
            None
        };
        reader.finish()?;
        Ok(Self { result, level_cap })
    }
}

/// One result word from a Waiting answer whose 7.55 handler clears or
/// keeps the room state: 3006 (`0x55d700`) and 4011 (`0x55f630`).
macro_rules! result_answer {
    ($name:ident, $command:expr, $about:expr) => {
        #[doc = $about]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name {
            /// Zero is the success code in both handlers.
            pub result: i32,
        }

        impl WireMessage for $name {
            const COMMAND: u16 = $command;

            fn encode_body(&self) -> Vec<u8> {
                self.result.to_le_bytes().to_vec()
            }

            fn decode_body(body: &[u8]) -> Result<Self, Error> {
                let mut reader = Reader::new(body);
                let result = reader.read_i32()?;
                reader.finish()?;
                Ok(Self { result })
            }
        }
    };
}

result_answer!(
    ExitWaitingAnswer,
    3006,
    "ExitWaiting answer 3006: a result word, zero clears the Waiting room."
);
result_answer!(
    StartGameAnswer,
    4011,
    "StartGame answer 4011: zero enters state 10; 1..4 and a state mismatch refuse."
);

/// One opcode-only Waiting notice: 3010 (`0x55e2f0`) and 3014
/// (`0x55f210`). Both handlers read no field before posting a caption.
macro_rules! notice_event {
    ($name:ident, $command:expr, $about:expr) => {
        #[doc = $about]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;

        impl WireMessage for $name {
            const COMMAND: u16 = $command;

            fn encode_body(&self) -> Vec<u8> {
                Vec::new()
            }

            fn decode_body(body: &[u8]) -> Result<Self, Error> {
                decode_empty(body)?;
                Ok(Self)
            }
        }
    };
}

notice_event!(
    RoomClosedNotice,
    3010,
    "Room-closed event 3010: no fields, the caption choice is local."
);
notice_event!(
    RoomDisappearedNotice,
    3014,
    "Room-disappeared event 3014: no fields, one fixed caption."
);

/// A seat byte the Waiting handler guards below eight. 4005, 4009,
/// 4013 and 4026 all read one member byte; 4013 carries a second state
/// byte after it.
macro_rules! member_byte_event {
    ($name:ident, $command:expr, $about:expr) => {
        #[doc = $about]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name {
            /// Seat index, 0..8.
            pub member_id: u8,
        }

        impl WireMessage for $name {
            const COMMAND: u16 = $command;

            fn encode_body(&self) -> Vec<u8> {
                vec![self.member_id]
            }

            fn decode_body(body: &[u8]) -> Result<Self, Error> {
                let mut reader = Reader::new(body);
                let member_id = reader.read_u8()?;
                reader.finish()?;
                if member_id >= 8 {
                    return Err(Error::InvalidField {
                        field: "member_id",
                        reason: "seat index is not below eight",
                    });
                }
                Ok(Self { member_id })
            }
        }
    };
}

member_byte_event!(
    SetRoomMusicEcho,
    4005,
    "SetRoomMusicEcho 4005: one seat id names the peer whose pick is echoed."
);

/// MemberStateChanged 4026: one seat plus the two state bytes the
/// handler writes through `0x541670` (`+0x2130`, playing) and `0x541480`
/// (`+0x217c`, music). The body is `[member][playing][music]`
/// (`docs/pins/net-b.md`, 4026 addendum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberStateChanged {
    /// Seat index, 0..8.
    pub member_id: u8,
    /// Playing-state byte stored at the member record `+0x2130`.
    pub playing_state: u8,
    /// Music-state byte stored at the member record `+0x217c`.
    pub music_state: u8,
}

impl WireMessage for MemberStateChanged {
    const COMMAND: u16 = 4026;

    fn encode_body(&self) -> Vec<u8> {
        vec![self.member_id, self.playing_state, self.music_state]
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let member_id = reader.read_u8()?;
        let playing_state = reader.read_u8()?;
        let music_state = reader.read_u8()?;
        reader.finish()?;
        if member_id >= 8 {
            return Err(Error::InvalidField {
                field: "member_id",
                reason: "seat index is not below eight",
            });
        }
        Ok(Self {
            member_id,
            playing_state,
            music_state,
        })
    }
}

/// One word of the 5027 peer music list: the low twelve bits name the
/// song, the high four are split out by reader `0x540db0`
/// (`docs/pins/net-c1.md`, 5027 pin). The high nibble's meaning stays
/// OPEN; both halves are preserved here without interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberMusicListEntry {
    /// Song id in the word's low twelve bits.
    pub music_id: u16,
    /// Word's high four bits.
    pub high_nibble: u8,
}

/// SyncMemberMusicList 5027: one member byte, a signed count and that
/// many 16-bit music words (`0x563880`, reader `0x540db0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncMemberMusicList {
    /// Seat index, 0..8.
    pub member_id: u8,
    /// One split word per owned song.
    pub entries: Vec<MemberMusicListEntry>,
}

impl WireMessage for SyncMemberMusicList {
    const COMMAND: u16 = 5027;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = vec![self.member_id];
        let count = i32::try_from(self.entries.len()).unwrap_or(i32::MAX);
        body.extend_from_slice(&count.to_le_bytes());
        for entry in &self.entries {
            let word = (u16::from(entry.high_nibble & 0x0F) << 12) | (entry.music_id & 0x0FFF);
            body.extend_from_slice(&word.to_le_bytes());
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let member_id = reader.read_u8()?;
        let raw_count = reader.read_i32()?;
        if member_id >= 8 {
            return Err(Error::InvalidField {
                field: "member_id",
                reason: "seat index is not below eight",
            });
        }
        let count = usize::try_from(raw_count.max(0)).map_err(|_| Error::InvalidField {
            field: "music list count",
            reason: "count exceeds usize",
        })?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let word = reader.read_u16()?;
            entries.push(MemberMusicListEntry {
                music_id: word & 0x0FFF,
                high_nibble: u8::try_from(word >> 12).unwrap_or(0),
            });
        }
        reader.finish()?;
        Ok(Self { member_id, entries })
    }
}

/// MusicLoaded 4013: the named seat's loaded byte (`0x55f950`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicLoaded {
    /// Seat index, 0..8.
    pub member_id: u8,
    /// State byte: local seats stage it to the global, peers to the array.
    pub state: u8,
}

impl WireMessage for MusicLoaded {
    const COMMAND: u16 = 4013;

    fn encode_body(&self) -> Vec<u8> {
        vec![self.member_id, self.state]
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let member_id = reader.read_u8()?;
        let state = reader.read_u8()?;
        reader.finish()?;
        if member_id >= 8 {
            return Err(Error::InvalidField {
                field: "member_id",
                reason: "seat index is not below eight",
            });
        }
        Ok(Self { member_id, state })
    }
}

/// One counted member/value row from WaitingEvent 4029 (`0x55ff00`).
/// A value of `1` carries one more state byte; other values do not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingMemberRow {
    /// Seat index named by the row.
    pub member_id: u8,
    /// Row value; `1` selects the local-record copy arm.
    pub value: u8,
    /// Extra byte read only by the value-1 arm.
    pub state: Option<u8>,
}

/// WaitingEvent 4029: a wide count followed by member/value rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingMemberRows {
    /// Rows in wire order.
    pub rows: Vec<WaitingMemberRow>,
}

impl WireMessage for WaitingMemberRows {
    const COMMAND: u16 = 4029;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = u32::try_from(self.rows.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes()
            .to_vec();
        for row in &self.rows {
            body.extend_from_slice(&[row.member_id, row.value]);
            if let Some(state) = row.state {
                body.push(state);
            }
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let count = reader.read_u32()?;
        let mut rows = Vec::with_capacity(usize::try_from(count).unwrap_or(usize::MAX).min(64));
        for _ in 0..count {
            let member_id = reader.read_u8()?;
            let value = reader.read_u8()?;
            let state = if value == 1 {
                Some(reader.read_u8()?)
            } else {
                None
            };
            rows.push(WaitingMemberRow {
                member_id,
                value,
                state,
            });
        }
        reader.finish()?;
        Ok(Self { rows })
    }
}

/// WaitingEvent 4028: a wide value and one member byte, no seat check
/// in the 7.55 body (`0x560a00`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingValueMemberEvent {
    /// Value staged as the Waiting chrome word.
    pub value: u32,
    /// Member byte carried with the value.
    pub member_id: u8,
}

impl WireMessage for WaitingValueMemberEvent {
    const COMMAND: u16 = 4028;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = self.value.to_le_bytes().to_vec();
        body.push(self.member_id);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let value = reader.read_u32()?;
        let member_id = reader.read_u8()?;
        reader.finish()?;
        Ok(Self { value, member_id })
    }
}

/// Waiting skill list from 4024 (`0x5602a0`) or 4058 (`0x560600`):
/// a wide item-id count, the ids, then a wide seat value and member
/// byte. The catalog byte lookup behind each id stays OPEN; the raw
/// ids are kept so a caller never invents catalog bytes.
macro_rules! skill_list_event {
    ($name:ident, $command:expr, $about:expr) => {
        #[doc = $about]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            /// Raw item ids in wire order.
            pub item_ids: Vec<u32>,
            /// Wide value written to the seat field.
            pub value: u32,
            /// Seat index named by the event.
            pub member_id: u8,
        }

        impl WireMessage for $name {
            const COMMAND: u16 = $command;

            fn encode_body(&self) -> Vec<u8> {
                let mut body = u32::try_from(self.item_ids.len())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes()
                    .to_vec();
                for id in &self.item_ids {
                    body.extend_from_slice(&id.to_le_bytes());
                }
                body.extend_from_slice(&self.value.to_le_bytes());
                body.push(self.member_id);
                body
            }

            fn decode_body(body: &[u8]) -> Result<Self, Error> {
                let mut reader = Reader::new(body);
                let count = reader.read_u32()?;
                let mut item_ids =
                    Vec::with_capacity(usize::try_from(count).unwrap_or(usize::MAX).min(64));
                for _ in 0..count {
                    item_ids.push(reader.read_u32()?);
                }
                let value = reader.read_u32()?;
                let member_id = reader.read_u8()?;
                reader.finish()?;
                Ok(Self {
                    item_ids,
                    value,
                    member_id,
                })
            }
        }
    };
}

skill_list_event!(
    WaitingSkillChanged,
    4024,
    "WaitingSkillChanged 4024: first skill list plus a seat value."
);
skill_list_event!(
    WaitingSkillExChanged,
    4058,
    "WaitingSkillExChanged 4058: extended skill list plus a seat value."
);

/// Gift, shop and wallet packets live in topical modules now;
/// re-exported here so `crate::message::…` paths keep working.
pub use super::gift::{
    CheckGiftFreePass, CheckGiftFreePassResponse, ClaimGift, ClaimGiftResponse, ClaimGiftResult,
    GetGiftMessageList, GetGiftMessageListResponse, GiftMessage, GiftType, ReadGiftMessage,
};
pub use super::shop::{
    ChangeName, ChangeNameResponse, CheckName, CheckNameResponse, EquipItem, EquipItemResponse,
    EquipTail, MusicEntry, RefreshBag, SellItem, SellItemResponse, ShopRestockPair,
    SyncItemPurchase, SyncItemPurchasePrefix, SyncItemPurchaseTail, SyncItemPurchaseWallets,
    SyncMusicPurchase, SyncMusicPurchaseResponse,
};
pub use super::wallet::{
    FreePassType, StartPayment, StartPaymentResponse, SyncCashPoint, SyncCashPointResponse,
    SyncFreePass, SyncFreePassResponse, SyncFreePassResult, UserFreePassInfo,
};

/// Keep [`Request`]/[`Response`]/[`Generic`] ids consistent with
/// the message types above.
#[cfg(test)]
mod command_consistency_tests {
    use super::*;
    use crate::command::{Event, Generic, Request, Response};

    #[test]
    fn room_stage_messages_match_their_opcodes() {
        assert_eq!(GetUserList::COMMAND, Request::GetUserList as u16);
        assert_eq!(SendMusicList::COMMAND, Request::SendMusicList as u16);
    }

    #[test]
    fn command_ids_match_message_types() {
        assert_eq!(AuthorizeRequest::COMMAND, Request::Authorize as u16);
        assert_eq!(ChannelLogin::COMMAND, Request::ChannelLogin as u16);
        assert_eq!(GetChannelList::COMMAND, Request::GetChannelList as u16);
        assert_eq!(SessionKeysRequest::COMMAND, Request::SessionKeys as u16);
        assert_eq!(Terminate::COMMAND, Request::Terminate as u16);
        assert_eq!(LegacyPing::COMMAND, Generic::LegacyPing as u16);
        assert_eq!(Ping::COMMAND, Generic::Ping as u16);
        assert_eq!(SessionKeys::COMMAND, Response::SessionKeys as u16);
        assert_eq!(AuthResponse::COMMAND, Response::Authorize as u16);
        assert_eq!(ChannelList::COMMAND, Response::GetChannelList as u16);
        assert_eq!(ChannelLoginResponse::COMMAND, Response::ChannelLogin as u16);
        assert_eq!(
            SubscriptionAlert::COMMAND,
            Response::SubscriptionAlert as u16
        );
        assert_eq!(ForceReauthorize::COMMAND, Response::ForceReauthorize as u16);
        assert_eq!(
            MultiSessionError::COMMAND,
            Response::MultiSessionError as u16
        );
        assert_eq!(GetCharacterInfo::COMMAND, Request::GetCharacterInfo as u16);
        assert_eq!(crate::rooms::JoinRoom::COMMAND, Request::JoinWaiting as u16);
        assert_eq!(
            crate::rooms::JoinRoomResponse::COMMAND,
            Response::JoinRoom as u16
        );
        assert_eq!(GetChannelInfo::COMMAND, Request::GetChannelInfo as u16);
        assert_eq!(crate::rooms::RoomList::COMMAND, Response::RoomList as u16);
        assert_eq!(CreateRoom::COMMAND, Request::CreateRoom as u16);
        assert_eq!(CreateRoomResponse::COMMAND, Response::CreateRoom as u16);
        assert_eq!(RoomCreated::COMMAND, Event::RoomCreated as u16);
        assert_eq!(
            crate::rooms::SendMainRoomMessage::COMMAND,
            Request::SendMainRoomMessage as u16
        );
        assert_eq!(
            crate::rooms::MainRoomUserMessage::COMMAND,
            Response::MainRoomUserMessage as u16
        );
        assert_eq!(
            crate::rooms::MainRoomAdminMessage::COMMAND,
            Response::MainRoomAdminMessage as u16
        );
        assert_eq!(
            crate::rooms::SendWhisper::COMMAND,
            Request::SendWhisper as u16
        );
        assert_eq!(crate::rooms::Announce::COMMAND, Request::Announce as u16);
        assert_eq!(
            crate::rooms::Announcement::COMMAND,
            Response::Announcement as u16
        );
        assert_eq!(
            crate::rooms::RankNotification::COMMAND,
            Event::RankNotification as u16
        );
        assert_eq!(
            crate::rooms::SendWhisperResponse::COMMAND,
            Response::SendWhisper as u16
        );
        assert_eq!(
            crate::rooms::ReceiveWhisper::COMMAND,
            Event::ReceiveWhisper as u16
        );
        assert_eq!(
            crate::character::CharacterInfo::COMMAND,
            Response::GetCharacterInfo as u16
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::decode_frame;

    #[test]
    fn send_music_list_encodes_the_exe_thirteen_id_body() {
        // Live exe tap (#339): `C2S 2024 len=30` with a 13-word list.
        let body = hex_body("0d00000061f360f35df35cf3fdf202f301f300f3fff2fef2fbf2f6f2f5f2");
        let report = SendMusicList::decode_body(&body).unwrap();
        assert_eq!(report.music_ids.len(), 13);
        assert_eq!(report.music_ids[0], 0xf361);
        assert_eq!(report.music_ids[12], 0xf2f5);
        assert_eq!(report.encode_body(), body);
    }

    fn hex_body(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn authorize_encodes_memoryer_field_order() {
        use std::net::{Ipv4Addr, SocketAddrV4};
        let request = AuthorizeRequest::new(
            0x0102_0304,
            0,
            SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 5678),
            SocketAddrV4::new(Ipv4Addr::new(192, 168, 0, 2), 0),
            42,
            "ab".to_owned(),
            Gender::Male,
        );
        let mut expected = vec![
            0x01, 0x04, 0x03, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x2E,
            0x16, 0xC0, 0xA8, 0x00, 0x02, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x61, 0x62, 0x00,
            0x31, 0x00, 0x01,
        ];
        expected.extend_from_slice(MEMORYER_CLIENT_ID.as_bytes());
        expected.push(0x00);
        assert_eq!(request.encode_body(), expected);
    }

    #[test]
    fn authorize_round_trips_through_frame() {
        use std::net::{Ipv4Addr, SocketAddrV4};
        let request = AuthorizeRequest::new(
            7,
            8,
            SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 15010),
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 15010),
            99,
            "player".to_owned(),
            Gender::Female,
        );
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 1000);
        assert_eq!(AuthorizeRequest::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn gender_accepts_memoryer_spellings() {
        assert_eq!(Gender::from_code("m"), Gender::Male);
        assert_eq!(Gender::from_code("1"), Gender::Male);
        assert_eq!(Gender::from_code("M"), Gender::Male);
        assert_eq!(Gender::from_code("f"), Gender::Female);
        assert_eq!(Gender::from_code("0"), Gender::Female);
        assert_eq!(Gender::from_code("anything-else"), Gender::Any);
        assert_eq!(Gender::from_code(""), Gender::Any);
        assert_eq!(Gender::Male.as_code(), "1");
        assert_eq!(Gender::Female.as_code(), "0");
    }

    #[test]
    fn authorize_rejects_short_body() {
        assert_eq!(
            AuthorizeRequest::decode_body(&[0x01; 10]),
            Err(Error::TruncatedMessage { need: 25, have: 10 })
        );
    }

    #[test]
    fn channel_login_encodes_little_endian_shorts() {
        let message = ChannelLogin {
            gateway_id: 1,
            channel_id: 2,
        };
        assert_eq!(
            message.encode(),
            vec![0x08, 0x00, 0xEC, 0x03, 0x01, 0x00, 0x02, 0x00]
        );
    }

    #[test]
    fn channel_login_round_trips_negative_ids() {
        let message = ChannelLogin {
            gateway_id: -1,
            channel_id: -2,
        };
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(ChannelLogin::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn channel_login_rejects_wrong_command() {
        let frame = decode_frame(&GetChannelList.encode()).unwrap();
        assert_eq!(
            ChannelLogin::decode_frame(&frame),
            Err(Error::UnknownCommand(1002))
        );
    }

    #[test]
    fn channel_login_rejects_short_body() {
        assert_eq!(
            ChannelLogin::decode_body(&[0x01]),
            Err(Error::TruncatedMessage { need: 4, have: 1 })
        );
    }

    #[test]
    fn empty_messages_encode_bare_frames() {
        assert_eq!(GetChannelList.encode(), vec![0x04, 0x00, 0xEA, 0x03]);
        assert_eq!(Ping.encode(), vec![0x04, 0x00, 0x71, 0x17]);
        assert_eq!(Terminate.encode(), vec![0x04, 0x00, 0xF0, 0xFF]);
    }

    #[test]
    fn channel_login_response_decodes_success_without_flag() {
        let response = ChannelLoginResponse::decode_body(&[0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(response.result_code, 0);
        assert_eq!(response.restricted, None);
        assert_eq!(
            ChannelLoginResult::from_code(response.result_code),
            Some(ChannelLoginResult::Success)
        );
    }

    #[test]
    fn channel_login_response_round_trips_restricted_flag() {
        let response = ChannelLoginResponse {
            result_code: 0,
            restricted: Some(true),
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 1005);
        assert_eq!(ChannelLoginResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn channel_login_response_names_all_result_codes() {
        assert_eq!(
            ChannelLoginResult::from_code(1),
            Some(ChannelLoginResult::PremiumNotAuthorized)
        );
        assert_eq!(
            ChannelLoginResult::from_code(6),
            Some(ChannelLoginResult::ChannelInfoUnavailable)
        );
        assert_eq!(
            ChannelLoginResult::from_code(7),
            Some(ChannelLoginResult::ChannelUnavailable)
        );
        assert_eq!(ChannelLoginResult::from_code(2), None);
    }

    #[test]
    fn channel_login_response_rejects_bad_flag_value() {
        assert_eq!(
            ChannelLoginResponse::decode_body(&[0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00]),
            Err(Error::InvalidField {
                field: "channel restriction flag",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn channel_login_response_rejects_short_body() {
        assert_eq!(
            ChannelLoginResponse::decode_body(&[0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 2 })
        );
    }

    #[test]
    fn channel_login_response_rejects_partial_flag() {
        assert_eq!(
            ChannelLoginResponse::decode_body(&[0x00, 0x00, 0x00, 0x00, 0x01]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn session_alerts_encode_bare_frames() {
        assert_eq!(SubscriptionAlert.encode(), vec![0x04, 0x00, 0x32, 0x00]);
        assert_eq!(ForceReauthorize.encode(), vec![0x04, 0x00, 0x33, 0x00]);
        assert_eq!(MultiSessionError.encode(), vec![0x04, 0x00, 0x34, 0x00]);
    }

    #[test]
    fn auth_response_decodes_success_without_tail() {
        let response = AuthResponse::decode_body(&[0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(response.result_code, 0);
        assert!(response.tail.is_empty());
        assert_eq!(
            AuthResult::from_code(response.result_code),
            Some(AuthResult::Success)
        );
    }

    #[test]
    fn auth_response_names_failure_codes() {
        assert_eq!(
            AuthResult::from_code(0xFFFF_FFFF),
            Some(AuthResult::InvalidLogin)
        );
        assert_eq!(
            AuthResult::from_code(0xFFFF_FFFE),
            Some(AuthResult::AlreadyConnected)
        );
        assert_eq!(
            AuthResult::from_code(0xFFFF_FFFD),
            Some(AuthResult::RegionError)
        );
        assert_eq!(
            AuthResult::from_code(0xFFFF_FFFC),
            Some(AuthResult::UnauthorizedPc)
        );
        assert_eq!(AuthResult::from_code(0xFFFF_FFFB), Some(AuthResult::Banned));
        assert_eq!(
            AuthResult::from_code(0xFFFF_FFFA),
            Some(AuthResult::TimeExceeded)
        );
        assert_eq!(
            AuthResult::from_code(0xFFFF_FF9B),
            Some(AuthResult::DatabaseError)
        );
        assert_eq!(AuthResult::from_code(42), None);
    }

    #[test]
    fn auth_response_preserves_opaque_tail() {
        let response = AuthResponse {
            result_code: 0,
            tail: vec![0x01, 0x02, 0x03],
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 1001);
        assert_eq!(AuthResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn auth_response_rejects_short_body() {
        assert_eq!(
            AuthResponse::decode_body(&[0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 2 })
        );
    }

    #[test]
    fn empty_message_rejects_payload() {
        let frame = decode_frame(
            &ChannelLogin {
                gateway_id: 0,
                channel_id: 0,
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(Ping::decode_frame(&frame), Err(Error::UnknownCommand(1004)));
    }

    /// Synthetic 51-byte session-keys body.
    fn synthetic_session_keys_body() -> Vec<u8> {
        let mut body = vec![0x34, 0x12, 0x00];
        body.extend_from_slice(&[0xAA; 32]);
        body.extend_from_slice(&[0xBB; 16]);
        body
    }

    #[test]
    fn session_keys_decodes_fixed_layout() {
        let body = synthetic_session_keys_body();
        let keys = SessionKeys::decode_body(&body).unwrap();
        assert_eq!(keys.seed, 0x1234);
        assert_eq!(keys.prefix, 0x00);
        assert_eq!(keys.primary_key, [0xAA; 32]);
        assert_eq!(keys.secondary_key, [0xBB; 16]);
    }

    #[test]
    fn session_keys_round_trips_through_frame() {
        let keys = SessionKeys {
            seed: -7,
            prefix: 0,
            primary_key: [0x11; 32],
            secondary_key: [0x22; 16],
        };
        let frame = decode_frame(&keys.encode()).unwrap();
        assert_eq!(frame.id, 1008);
        assert_eq!(SessionKeys::decode_frame(&frame), Ok(keys));
    }

    #[test]
    fn session_keys_rejects_short_body() {
        assert_eq!(
            SessionKeys::decode_body(&[0x00; 50]),
            Err(Error::TruncatedMessage { need: 51, have: 50 })
        );
    }

    /// Synthetic two-row channel list body.
    fn synthetic_channel_list_body() -> Vec<u8> {
        let mut body = vec![0x02, 0x00, 0x00, 0x00];
        body.extend_from_slice(&[
            0x01, 0x00, 0x02, 0x00, 0x08, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01,
        ]);
        body.extend_from_slice(&[
            0x03, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        body
    }

    #[test]
    fn channel_list_decodes_count_prefixed_rows() {
        // Named decoder #898: `docs/net-protocol.md` §4c 1003 —
        // dispatcher file `0x1583f1`, handler VA `0x559ff0`
        // (file `0x159ff0`); count then 13-byte rows, no active skip.
        let list = ChannelList::decode_body(&synthetic_channel_list_body()).unwrap();
        assert_eq!(list.channels.len(), 2);
        assert_eq!(
            list.channels[0],
            ChannelState {
                gateway_id: 1,
                channel_id: 2,
                capacity: 8,
                population: 5,
                active: true,
            }
        );
        assert!(!list.channels[1].active);
    }

    #[test]
    fn channel_list_decodes_empty_list() {
        let list = ChannelList::decode_body(&[0x00, 0x00, 0x00, 0x00]).unwrap();
        assert!(list.channels.is_empty());
    }

    #[test]
    fn channel_list_rejects_negative_count() {
        assert_eq!(
            ChannelList::decode_body(&[0xFF, 0xFF, 0xFF, 0xFF]),
            Err(Error::InvalidField {
                field: "channel count",
                reason: "negative"
            })
        );
    }

    #[test]
    fn channel_list_rejects_bad_active_flag() {
        let mut body = synthetic_channel_list_body();
        body[4 + 12] = 0x02;
        assert_eq!(
            ChannelList::decode_body(&body),
            Err(Error::InvalidField {
                field: "channel active flag",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn channel_joinability_keys_on_active_byte_not_capacity() {
        // Wire contract (exe handler VA 0x475260): joinability is the
        // active byte alone; capacity (including 0) never flips it.
        let mut body = vec![0x03, 0x00, 0x00, 0x00];
        // row: gw=1 ch=2 cap=0 pop=0 active=1 (joinable despite zero cap)
        body.extend_from_slice(&[
            0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ]);
        // row: gw=3 ch=4 cap=100 pop=5 active=0 (not joinable despite cap)
        body.extend_from_slice(&[
            0x03, 0x00, 0x04, 0x00, 0x64, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00,
        ]);
        // row: gw=5 ch=6 cap=0 pop=0 active=0 (not joinable)
        body.extend_from_slice(&[
            0x05, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        let list = ChannelList::decode_body(&body).unwrap();
        assert_eq!(list.channels.len(), 3);
        assert!(
            list.channels[0].active,
            "capacity 0 with active byte 1 must stay joinable"
        );
        assert_eq!(list.channels[0].capacity, 0);
        assert!(
            !list.channels[1].active,
            "capacity 100 with active byte 0 must not be joinable"
        );
        assert!(
            !list.channels[2].active,
            "capacity 0 with active byte 0 must not be joinable"
        );
    }

    #[test]
    fn channel_list_rejects_truncated_row() {
        let body = vec![0x01, 0x00, 0x00, 0x00, 0x01, 0x00];
        assert_eq!(
            ChannelList::decode_body(&body),
            Err(Error::TruncatedMessage { need: 17, have: 6 })
        );
    }

    fn room_request() -> CreateRoom {
        CreateRoom {
            title: "Test Room".to_owned(),
            key_mode: KeyMode::SevenKeys,
            game_mode: GameMode::Normal,
            has_password: true,
            password: "secret".to_owned(),
            min_level: 1,
            max_level: 30,
            premium: false,
        }
    }

    /// Exe CREATE wire, ticket #603: `~/scratch/evidence-room/
    /// create-fields-2/exe/wire/room-stage.log` 02:27:01 captured `C2S
    /// opcode=2004 len=17 hex=454d505459505300020000310000000000`
    /// beside `pass-02-empty-password-frame.png` (sha256
    /// `70318790067b14ba91b45cec4ecd47df14ba7aad5bc54fbefef6147db3db671d`),
    /// the qualified CREATE sheet with the password field untouched.
    /// The zero password byte follows the key/game/lock bytes; the other
    /// string field is also variable-length, not a padded slot.
    #[test]
    fn create_room_empty_password_body_matches_the_exe_capture() {
        let exe_body = b"EMPTYPS\x00\x02\x00\x00\x31\x00\x00\x00\x00\x00";
        let request = CreateRoom::decode_body(exe_body).unwrap();
        assert_eq!(request.title, "EMPTYPS");
        assert_eq!(request.key_mode, KeyMode::SevenKeys);
        assert_eq!(request.game_mode, GameMode::Normal);
        assert!(!request.has_password);
        assert_eq!(request.encode_body(), exe_body);
    }

    /// Interop tail pin (ticket #527): the creator socket tap
    /// `~/scratch/close-wr/full01/tap.log` 15:43:30 carries
    /// `C2S 2004 len=17 hex=5332393146554c4c000100000001630000`; the
    /// byte after the lock flag is the empty password's terminator, not
    /// a padded 12-byte field. The exe CREATE frame
    /// `~/scratch/evidence-room/create-fields-2/exe/wire/room-stage.log`
    /// 02:27:01 shows the same one-byte flag and variable string layout.
    #[test]
    fn create_room_zero_password_byte_matches_the_creator_tap() {
        let tapped = b"S291FULL\x00\x01\x00\x00\x00\x01\x63\x00\x00";
        let create = CreateRoom::decode_body(tapped).unwrap();
        assert!(!create.has_password);
        assert_eq!(create.password, "");
        assert_eq!(create.min_level, 1);
        assert_eq!(create.max_level, 99);
        assert_eq!(create.encode_body(), tapped);
    }

    #[test]
    fn create_room_round_trips() {
        let request = room_request();
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 2004);
        assert_eq!(CreateRoom::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn create_room_truncates_long_title() {
        let mut request = room_request();
        request.title = "A".repeat(40);
        let body = request.encode_body();
        // 21 title bytes plus terminator: 22 + 1 + 1 + 1 + 7 + 1 + 1 + 2.
        assert_eq!(body.len(), 22 + 1 + 1 + 1 + 7 + 1 + 1 + 2);
        assert_eq!(&body[..21], &[0x41; 21]);
        assert_eq!(body[21], 0x00);
        let back = CreateRoom::decode_body(&body).unwrap();
        assert_eq!(back.title, "A".repeat(21));
    }

    #[test]
    fn create_room_truncates_long_password() {
        let mut request = room_request();
        request.password = "B".repeat(20);
        let back = CreateRoom::decode_body(&request.encode_body()).unwrap();
        assert_eq!(back.password, "B".repeat(11));
    }

    #[test]
    fn create_room_rejects_bad_key_mode() {
        assert_eq!(
            KeyMode::from_u8(7),
            Err(Error::InvalidField {
                field: "key mode",
                reason: "not 0, 1, or 2"
            })
        );
    }

    #[test]
    fn create_room_rejects_bad_game_mode() {
        assert_eq!(
            GameMode::from_u8(9),
            Err(Error::InvalidField {
                field: "game mode",
                reason: "not 0 or 1"
            })
        );
    }

    fn created_room() -> CreatedRoom {
        CreatedRoom {
            number: 7,
            premium: true,
            password: "pw".to_owned(),
            has_password: true,
            min_level: 1,
            max_level: 30,
        }
    }

    #[test]
    fn create_response_without_info_decodes() {
        let response = CreateRoomResponse {
            result: CreateResult::ChannelFull,
            info: None,
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 2006);
        assert_eq!(CreateRoomResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn create_response_with_info_round_trips() {
        let response = CreateRoomResponse {
            result: CreateResult::Success,
            info: Some(created_room()),
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(CreateRoomResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn create_response_rejects_bad_result() {
        let mut body = vec![0x02, 0x00, 0x00, 0x00];
        assert_eq!(
            CreateRoomResponse::decode_body(&body),
            Err(Error::InvalidField {
                field: "create result",
                reason: "not 0 or 1"
            })
        );
        body.clear();
        assert_eq!(
            CreateRoomResponse::decode_body(&body),
            Err(Error::TruncatedMessage { need: 4, have: 0 })
        );
    }

    #[test]
    fn room_removed_decodes_the_live_disband_broadcast() {
        // Named decoder #898: `docs/net-protocol.md` §4b opcode `0x07D7`.
        // ~/scratch/p380-wire-probe.log: opcode 2007 (0x07D7), body
        // `00000000` for room number 0, sent to both room members.
        let frame = decode_frame(&[8, 0, 0xD7, 0x07, 0, 0, 0, 0]).unwrap();
        assert_eq!(frame.id, 2007);
        assert_eq!(
            RoomRemoved::decode_frame(&frame),
            Ok(RoomRemoved { number: 0 })
        );
        assert_eq!(
            RoomRemoved { number: 3 }.encode(),
            vec![8, 0, 0xD7, 0x07, 3, 0, 0, 0]
        );
    }

    #[test]
    fn room_created_uses_the_server_broadcast_opcode() {
        // docs/net-protocol.md §4b and the live Memoryer push in
        // ~/scratch/p376-wire-probe2.log: 2005 is RoomCreated; 2007 is
        // RoomRemoved. The room-row fix must decode the frame that is
        // actually on the wire.
        assert_eq!(RoomCreated::COMMAND, 2005);
    }

    #[test]
    fn room_created_decodes_the_live_twenty_one_byte_broadcast() {
        // Named decoder #898: `docs/net-protocol.md` §4b opcode `0x07D5`
        // and its 21-byte push; the peer stages the row from this event.
        let body = hex_body("0000000050333736524f4f4d000000000163000000");
        let event = RoomCreated::decode_body(&body).unwrap();
        assert_eq!(event.number, 0);
        assert_eq!(event.title, "P376ROOM");
        assert_eq!(event.min_level, 1);
        assert_eq!(event.max_level, 99);
    }

    #[test]
    fn room_created_round_trips() {
        let event = RoomCreated {
            number: 3,
            title: "Lobby".to_owned(),
            key_mode: KeyMode::FiveKeys,
            game_mode: GameMode::Live,
            has_password: false,
            min_level: 0,
            max_level: 99,
            premium: false,
            room_type: 0,
        };
        let frame = decode_frame(&event.encode()).unwrap();
        assert_eq!(frame.id, 2005);
        assert_eq!(RoomCreated::decode_frame(&frame), Ok(event));
    }

    #[test]
    fn room_created_rejects_trailing_bytes() {
        let mut event = RoomCreated {
            number: 3,
            title: String::new(),
            key_mode: KeyMode::ThreeKeys,
            game_mode: GameMode::Normal,
            has_password: false,
            min_level: 0,
            max_level: 0,
            premium: false,
            room_type: 0,
        };
        let mut body = event.encode_body();
        body.push(0xFF);
        assert_eq!(
            RoomCreated::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
        event.title = "x".to_owned();
    }
}

/// Codec guards for the 16 pinned Waiting handlers (`docs/pins/net-b.md`).
/// Every guard is `seam: pure codec with no caller beyond the decode
/// boundary`; the live caller chains belong to the session guards.
#[cfg(test)]
mod waiting_wire_codec_tests {
    use super::*;
    use crate::frame::decode_frame;

    #[test]
    fn free_music_status_reads_the_level_cap_only_after_a_zero_result() {
        // seam: pure codec; the 2035 handler is pinned at VA 0x55b6d0.
        let frame = decode_frame(
            &FreeMusicStatusAnswer {
                result: 0,
                level_cap: Some(42),
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            FreeMusicStatusAnswer::decode_frame(&frame),
            Ok(FreeMusicStatusAnswer {
                result: 0,
                level_cap: Some(42),
            })
        );
    }

    // seam: pure codec; the handler skips the cap byte when the result
    // word is non-zero (VA 0x55b6d0,
    // ev_2a4a9653505cecfa56fb762db6b71d87fa79a2340ca688f713cca043832fae3d).
    #[test]
    fn free_music_status_with_a_nonzero_result_carries_no_cap_byte() {
        let frame = decode_frame(
            &FreeMusicStatusAnswer {
                result: -1,
                level_cap: None,
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            FreeMusicStatusAnswer::decode_frame(&frame)
                .unwrap()
                .level_cap,
            None
        );
    }

    #[test]
    fn free_music_status_rejects_a_cap_after_a_nonzero_result() {
        // seam: pure codec; an extra byte is not part of the pinned body.
        let mut body = (-1i32).to_le_bytes().to_vec();
        body.push(7);
        assert_eq!(
            FreeMusicStatusAnswer::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn exit_waiting_answer_keeps_a_nonzero_result_without_clearing() {
        // seam: pure codec; the 3006 handler gates its reset on zero.
        let answer = ExitWaitingAnswer { result: 3 };
        let frame = decode_frame(&answer.encode()).unwrap();
        assert_eq!(ExitWaitingAnswer::decode_frame(&frame), Ok(answer));
    }

    #[test]
    fn start_game_answer_decodes_each_pinned_refusal_code() {
        // seam: pure codec; result 1..4 and a state mismatch select
        // captions in the 4011 handler (VA 0x55f630).
        let answer = StartGameAnswer { result: 2 };
        let frame = decode_frame(&answer.encode()).unwrap();
        assert_eq!(StartGameAnswer::decode_frame(&frame), Ok(answer));
    }

    // seam: pure codec; both handlers read no body field (3010 VA
    // 0x55e2f0 ev_e91089f51d6c959161da81f01b5c1d6f51f00bfe10c69998b9629b659941cce8;
    // 3014 VA 0x55f210 ev_5d9c48f9d8fe27887b34e15b4bc11ec313a9527b93b07a15b70db96c178672ee).
    #[test]
    fn room_closed_and_disappeared_events_carry_no_fields() {
        assert_eq!(RoomClosedNotice::decode_body(&[]), Ok(RoomClosedNotice));
        assert_eq!(
            RoomDisappearedNotice::decode_body(&[]),
            Ok(RoomDisappearedNotice)
        );
    }

    #[test]
    fn room_closed_event_rejects_an_invented_body() {
        // seam: pure codec; no field is pinned for 3010.
        assert_eq!(
            RoomClosedNotice::decode_body(&[1]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn music_pick_echo_accepts_only_seats_below_eight() {
        // seam: pure codec; the 4005 handler guards slot < 8.
        let frame = decode_frame(&SetRoomMusicEcho { member_id: 2 }.encode()).unwrap();
        assert_eq!(
            SetRoomMusicEcho::decode_frame(&frame),
            Ok(SetRoomMusicEcho { member_id: 2 })
        );
        assert!(SetRoomMusicEcho::decode_body(&[8]).is_err());
    }

    #[test]
    fn music_loaded_pairs_a_seat_with_its_state_byte() {
        // seam: pure codec; the 4013 handler reads exactly two bytes.
        let frame = decode_frame(
            &MusicLoaded {
                member_id: 3,
                state: 4,
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            MusicLoaded::decode_frame(&frame),
            Ok(MusicLoaded {
                member_id: 3,
                state: 4,
            })
        );
    }

    #[test]
    fn member_state_changed_guards_the_three_byte_state_body() {
        // seam: pure codec; the 4026 handler reads the member plus two
        // state bytes (docs/pins/net-b.md addendum).
        let change = MemberStateChanged {
            member_id: 5,
            playing_state: 1,
            music_state: 4,
        };
        let frame = decode_frame(&change.encode()).unwrap();
        assert_eq!(MemberStateChanged::decode_frame(&frame), Ok(change));
        assert!(MemberStateChanged::decode_body(&[5, 1]).is_err());
        assert!(MemberStateChanged::decode_body(&[5, 1, 4, 0]).is_err());
        assert!(MemberStateChanged::decode_body(&[9, 1, 4]).is_err());
    }

    #[test]
    fn sync_member_music_list_splits_each_word_and_guards_the_seat() {
        // seam: pure codec; the 5027 handler (`0x563880`) reads a seat
        // byte then a counted list of 16-bit words through `0x540db0`.
        let list = SyncMemberMusicList {
            member_id: 2,
            entries: vec![
                MemberMusicListEntry {
                    music_id: 757,
                    high_nibble: 0,
                },
                MemberMusicListEntry {
                    music_id: 0x2F5,
                    high_nibble: 0xF,
                },
            ],
        };
        let frame = decode_frame(&list.encode()).unwrap();
        assert_eq!(SyncMemberMusicList::decode_frame(&frame), Ok(list));
        assert!(SyncMemberMusicList::decode_body(&[8, 0, 0, 0, 0]).is_err());
        assert!(SyncMemberMusicList::decode_body(&[1, 1, 0, 0, 0]).is_err());
    }

    #[test]
    fn counted_waiting_rows_read_an_extra_byte_only_for_value_one() {
        // seam: pure codec; the 4029 loop reads one more byte on the
        // value-1 arm (VA 0x55ff00).
        let rows = WaitingMemberRows {
            rows: vec![
                WaitingMemberRow {
                    member_id: 1,
                    value: 1,
                    state: Some(4),
                },
                WaitingMemberRow {
                    member_id: 2,
                    value: 2,
                    state: None,
                },
            ],
        };
        let frame = decode_frame(&rows.encode()).unwrap();
        assert_eq!(WaitingMemberRows::decode_frame(&frame), Ok(rows));
    }

    #[test]
    fn waiting_value_member_event_keeps_an_unchecked_member_byte() {
        // seam: pure codec; the 4028 handler has no bounds check.
        let event = WaitingValueMemberEvent {
            value: 0x1234_5678,
            member_id: 6,
        };
        let frame = decode_frame(&event.encode()).unwrap();
        assert_eq!(WaitingValueMemberEvent::decode_frame(&frame), Ok(event));
    }

    #[test]
    fn skill_lists_round_trip_their_counted_ids_and_seat_value() {
        // seam: pure codec; 4024 and 4058 share the pinned shape.
        let first = WaitingSkillChanged {
            item_ids: vec![0x11, 0x8000_0012],
            value: 7,
            member_id: 1,
        };
        let extended = WaitingSkillExChanged {
            item_ids: vec![0x21],
            value: 9,
            member_id: 2,
        };
        let first_frame = decode_frame(&first.encode()).unwrap();
        let extended_frame = decode_frame(&extended.encode()).unwrap();
        assert_eq!(
            WaitingSkillChanged::decode_frame(&first_frame)
                .unwrap()
                .item_ids,
            first.item_ids
        );
        assert_eq!(
            WaitingSkillExChanged::decode_frame(&extended_frame)
                .unwrap()
                .value,
            9
        );
    }
}
