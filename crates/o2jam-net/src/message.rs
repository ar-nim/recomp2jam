//! Phase-1 message layouts (Memoryer field orders).
//!
//! Fixed-size bodies only for now: `Authorize` (1000) carries
//! strings whose encoding is not yet exe-verified, so it stays
//! out until the string codec is pinned down.

use std::net::{Ipv4Addr, SocketAddrV4};

use super::error::Error;
use super::frame::{encode_frame, Frame};
use super::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};

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
fn decode_empty(body: &[u8]) -> Result<(), Error> {
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
fn encode_empty<T: EmptyMessage>() -> Vec<u8> {
    encode_frame(T::COMMAND, &[])
}

/// Decode an empty body, rejecting any payload bytes.
fn decode_empty_frame<T: EmptyMessage>(frame: &Frame) -> Result<(), Error> {
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

/// Client identifier every 7.55 client sends (Memoryer `AuthRequest.Id`,
/// byte-identical to the in-exe constant at file `0x2472c8`).
pub const MEMORYER_CLIENT_ID: &str = "726PNA-B1YW-28ZG9M-I5SC-Q0OPM6";

/// Gender as the wire carries it: a null-terminated string (Memoryer
/// `AuthGenderCodec` is decode-only and reads `"m"`/`"1"`, `"f"`/`"0"`,
/// else `Any`). The exe's exact byte for this field is unverified, so
/// both spellings decode and we send the numeric one.
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
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        let count = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        if count < 0 {
            return Err(Error::InvalidField {
                field: "channel count",
                reason: "negative",
            });
        }
        let count = count as usize;
        let need = 4 + count * CHANNEL_ROW_LEN;
        if body.len() < need {
            return Err(Error::TruncatedMessage {
                need,
                have: body.len(),
            });
        }
        if body.len() > need {
            return Err(Error::TrailingBytes(body.len() - need));
        }
        let mut channels = Vec::with_capacity(count);
        for row in 0..count {
            let base = 4 + row * CHANNEL_ROW_LEN;
            let active = match body[base + 12] {
                0 => false,
                1 => true,
                _ => {
                    return Err(Error::InvalidField {
                        field: "channel active flag",
                        reason: "not 0 or 1",
                    })
                }
            };
            channels.push(ChannelState {
                gateway_id: u16::from_le_bytes([body[base], body[base + 1]]),
                channel_id: u16::from_le_bytes([body[base + 2], body[base + 3]]),
                capacity: i32::from_le_bytes([
                    body[base + 4],
                    body[base + 5],
                    body[base + 6],
                    body[base + 7],
                ]),
                population: i32::from_le_bytes([
                    body[base + 8],
                    body[base + 9],
                    body[base + 10],
                    body[base + 11],
                ]),
                active,
            });
        }
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
fn read_flag<'body>(body: &'body [u8], field: &'static str) -> Result<(bool, &'body [u8]), Error> {
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

/// `RoomCreated` 2007 broadcast: the new room's full descriptor.
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
    const COMMAND: u16 = 2007;

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

/// `EquipItem` 5004 request: wear `item_id` in equipment `slot`.
///
/// Body is two little-endian `i32` words, slot first. The send
/// builder stores the opcode as `u16`, stamps length `4`, then
/// appends both args as 4-byte words (VA `0x4a4ec8`, file
/// `0xa4ec8`); both C→S call sites pass slot then item (VAs
/// `0x4a4af4`/`0x4a4bcb`, files `0xa4af4`/`0xa4bcb`). The server
/// echoes the result as [`EquipItemResponse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipItem {
    /// Equipment slot index (`0..EQUIPMENT_SLOTS`;
    /// `crates/o2jam-net/src/character.rs`). The client derives it
    /// from the item record and range-checks `0 <= slot < 0x11`
    /// (VA `0x4a4baa`, file `0xa4baa`).
    pub slot: i32,
    /// Item id from `Itemdata_Korea.dat` (`1..=1541`); `0` is the
    /// empty-slot value (see `CharacterInfo::equipment`).
    pub item_id: i32,
}

/// Body length of [`EquipItem`]: two `i32` words.
const EQUIP_ITEM_BODY_LEN: usize = 8;

impl WireMessage for EquipItem {
    const COMMAND: u16 = 5004;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(EQUIP_ITEM_BODY_LEN);
        body.extend_from_slice(&self.slot.to_le_bytes());
        body.extend_from_slice(&self.item_id.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < EQUIP_ITEM_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: EQUIP_ITEM_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > EQUIP_ITEM_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - EQUIP_ITEM_BODY_LEN));
        }
        Ok(Self {
            slot: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            item_id: i32::from_le_bytes([body[4], body[5], body[6], body[7]]),
        })
    }
}

/// `EquipItem` 5005 response: the server's equip verdict.
///
/// Body is one little-endian `i32`. The S→C handler (dispatched by
/// `cmpl $0x138d` / `je 0x4a5fe8`, VAs `0x4a5570`/`0x4a5586`,
/// files `0xa5570`/`0xa5586`) reads a single `i32` (VA `0x4a5fe8`,
/// file `0xa5fe8`): nonzero routes to the equip-apply path
/// (`0x4a3c80`, file `0xa3c80`), zero to the remove path
/// (`0x4a3dc0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipItemResponse {
    /// Echoed item id; `0` means the remove/failure path.
    pub item_id: i32,
}

/// Body length of [`EquipItemResponse`]: one `i32` word.
const EQUIP_ITEM_RESPONSE_BODY_LEN: usize = 4;

impl WireMessage for EquipItemResponse {
    const COMMAND: u16 = 5005;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(EQUIP_ITEM_RESPONSE_BODY_LEN);
        body.extend_from_slice(&self.item_id.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < EQUIP_ITEM_RESPONSE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: EQUIP_ITEM_RESPONSE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > EQUIP_ITEM_RESPONSE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - EQUIP_ITEM_RESPONSE_BODY_LEN,
            ));
        }
        Ok(Self {
            item_id: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
        })
    }
}

/// Gift box entry kind on the wire: one byte.
///
/// Memoryer `Encore.Metadata.GiftType` (`Item = 0`, `Music = 1`);
/// the `ClaimGift` 5023 send case reads the entry's head byte as
/// this value (VA `0x4a5038`, file `0xa5038`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GiftType {
    /// Gifted avatar/consumable item.
    Item = 0,
    /// Gifted song.
    Music = 1,
}

impl GiftType {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse a wire byte; unknown kinds are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside 0–1.
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Item),
            1 => Ok(Self::Music),
            _ => Err(Error::InvalidField {
                field: "gift type",
                reason: "not 0 or 1",
            }),
        }
    }
}

// `GetGiftMessageList` 5017 carries no parameters (its send case
// stamps the header and sends without appending a field: VA
// `0x491974`, file `0x91974`).
empty_message!(GetGiftMessageList, 5017);
// `SyncFreePass` 5025 likewise sends a bare header (send case VA
// `0x4f400c`, file `0xf400c`; the three virtual-send sites pass a
// placeholder zero: VAs `0x4f4184`/`0x4f41c6`/`0x4f51b4`).
empty_message!(SyncFreePass, 5025);
// `SyncItemPurchase` 5000 (cart sync request) sends a bare header:
// the opcode-arg builder case for `0x1388` (VA `0x4f3fa8`, file
// `0xf3fa8`, dispatched on the id word at VA `0x4f3f7f`, file `0xf3f7f`)
// writes the opcode word plus a length word of 4 with no appends, and
// the common tail (VA `0x4f4051`, file `0xf4051`) transmits it unchanged.
empty_message!(SyncItemPurchase, 5000);
// `StartPayment` 5015 (web-coupon charge trigger) sends a bare
// header (send case VA `0x491a27`, file `0x91a27`).
empty_message!(StartPayment, 5015);
// `SyncCashPoint` 5028 (post-charge wallet refresh) sends a bare
// header (send case VA `0x491436`, file `0x91436`; virtual-send
// site VA `0x48ee81`, file `0x8ee81`).
empty_message!(SyncCashPoint, 5028);
// `StartPayment` 5016 response: dispatched (`cmpl $0x1398`, VA
// `0x490a09`, file `0x90a09`) to a bare UI notify that reads no
// body fields (VA `0x4910c1`, file `0x910c1`), so the body is
// modeled empty.
empty_message!(StartPaymentResponse, 5016);

/// `ReadGiftMessage` 5019 request: mark gift message
/// `gift_message_id` as read. No server reply follows (Memoryer's
/// handler returns nothing).
///
/// Body is one little-endian `i32`. The send case appends a
/// single 4-byte word from its arg (VA `0x4919c2`, file `0x919c2`);
/// the gift-list UI path pushes one message id with the opcode (VA
/// `0x4954b4`, file `0x954b4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadGiftMessage {
    /// Gift message id from [`GetGiftMessageListResponse`].
    pub gift_message_id: i32,
}

/// Body length of [`ReadGiftMessage`]: one `i32` word.
const READ_GIFT_MESSAGE_BODY_LEN: usize = 4;

impl WireMessage for ReadGiftMessage {
    const COMMAND: u16 = 5019;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(READ_GIFT_MESSAGE_BODY_LEN);
        body.extend_from_slice(&self.gift_message_id.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < READ_GIFT_MESSAGE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: READ_GIFT_MESSAGE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > READ_GIFT_MESSAGE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - READ_GIFT_MESSAGE_BODY_LEN,
            ));
        }
        Ok(Self {
            gift_message_id: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
        })
    }
}

/// `ClaimGift` 5023 request: move a waiting gift into the
/// inventory (item) or song list (music).
///
/// Body is 10 bytes: gift-type byte, gift id `i32`, resource
/// (item/music) id `i32`, attributive-flag byte. The send case (VA
/// `0x4a4f98`, file `0xa4f98`) appends exactly those widths — a
/// byte from the entry head (VA `0x4a5038`), 4-byte words from
/// entry `+4`/`+8` (VAs `0x4a505a`/`0x4a507d`), then a final byte:
/// a lookup result for item entries, `0` for music entries (VAs
/// `0x4a50aa`/`0x4a50d6`). Field names follow Memoryer's
/// `ClaimGiftRequest` (corroboration); the call site passes the
/// gift-box index plus a zero word (VA `0x4a4622`, file `0xa4622`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimGift {
    /// Item or music gift.
    pub gift_type: GiftType,
    /// Gift id (`GiftItemInfo`/`GiftMusicInfo` in `character.rs`).
    pub gift_id: i32,
    /// Item id or song id being claimed.
    pub resource_id: i32,
    /// True when an item gift is a countable (attributive) stack.
    pub is_attributive: bool,
}

/// Body length of [`ClaimGift`]: 1 + 4 + 4 + 1.
const CLAIM_GIFT_BODY_LEN: usize = 10;

impl WireMessage for ClaimGift {
    const COMMAND: u16 = 5023;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(CLAIM_GIFT_BODY_LEN);
        body.push(self.gift_type.as_u8());
        body.extend_from_slice(&self.gift_id.to_le_bytes());
        body.extend_from_slice(&self.resource_id.to_le_bytes());
        body.push(u8::from(self.is_attributive));
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < CLAIM_GIFT_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: CLAIM_GIFT_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > CLAIM_GIFT_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - CLAIM_GIFT_BODY_LEN));
        }
        let gift_type = GiftType::from_u8(body[0])?;
        let (is_attributive, _) = read_flag(&body[9..], "attributive flag")?;
        Ok(Self {
            gift_type,
            gift_id: i32::from_le_bytes([body[1], body[2], body[3], body[4]]),
            resource_id: i32::from_le_bytes([body[5], body[6], body[7], body[8]]),
            is_attributive,
        })
    }
}

/// `CheckGiftFreePass` 5040 request: look up `name` before gifting
/// a free pass.
///
/// Body is one null-terminated nickname (Memoryer's
/// `CheckGiftFreePassRequest`; corroboration only — no `0x13b0`
/// send site exists in `.text`, so the exe's trigger stays OPEN).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckGiftFreePass {
    /// Recipient nickname.
    pub name: String,
}

impl WireMessage for CheckGiftFreePass {
    const COMMAND: u16 = 5040;

    fn encode_body(&self) -> Vec<u8> {
        encode_null_terminated(&self.name, MAX_STRING_LEN)
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (name, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
        if used != body.len() {
            return Err(Error::TrailingBytes(body.len() - used));
        }
        Ok(Self { name })
    }
}

/// One gift message inside [`GetGiftMessageListResponse`] 5018
/// (Memoryer's `GiftEntry`; order and widths are corroboration —
/// exe side the handler walks a `0x168`-stride message table, VA
/// `0x490e86`, file `0x90e86`, with the exact field split OPEN).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GiftMessage {
    /// Message id (passed back to [`ReadGiftMessage`]).
    pub message_id: i32,
    /// Item or music gift.
    pub gift_type: GiftType,
    /// Send date as displayed.
    pub write_date: String,
    /// Gifter nickname.
    pub sender: String,
    /// Message title.
    pub title: String,
    /// Message body.
    pub content: String,
}

impl GiftMessage {
    /// Parse from the front of `body`, returning the value and rest.
    fn decode_prefix(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < 5 {
            return Err(Error::TruncatedMessage {
                need: 5,
                have: body.len(),
            });
        }
        let gift_type = GiftType::from_u8(body[4])?;
        let mut tail = &body[5..];
        let (write_date, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
        tail = &tail[used..];
        let (sender, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
        tail = &tail[used..];
        let (title, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
        tail = &tail[used..];
        let (content, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
        tail = &tail[used..];
        Ok((
            Self {
                message_id: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
                gift_type,
                write_date,
                sender,
                title,
                content,
            },
            tail,
        ))
    }

    /// Full body for sending (tests and future use).
    fn encode_inner(&self, body: &mut Vec<u8>) {
        body.extend_from_slice(&self.message_id.to_le_bytes());
        body.push(self.gift_type.as_u8());
        body.extend_from_slice(&encode_null_terminated(&self.write_date, MAX_STRING_LEN));
        body.extend_from_slice(&encode_null_terminated(&self.sender, MAX_STRING_LEN));
        body.extend_from_slice(&encode_null_terminated(&self.title, MAX_STRING_LEN));
        body.extend_from_slice(&encode_null_terminated(&self.content, MAX_STRING_LEN));
    }
}

/// Saturating `i16` collection prefix (Encore `Int16` counts).
fn count_prefix(len: usize) -> i16 {
    i16::try_from(len.min(i16::MAX as usize)).unwrap_or(i16::MAX)
}

/// `GetGiftMessageList` 5018 response: validity word plus an
/// `i16`-prefixed message list.
///
/// Dispatched by `cmpl $0x139a` (VA `0x490a4a`, file `0x90a4a`);
/// Memoryer shapes the body as int-flag + `Int16` collection
/// (corroboration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetGiftMessageListResponse {
    /// False when `messages` is valid.
    pub invalid: bool,
    /// Gift messages in server order.
    pub messages: Vec<GiftMessage>,
}

/// Body length of an empty [`GetGiftMessageListResponse`]: `i32`
/// flag + `i16` count.
const GIFT_LIST_EMPTY_BODY_LEN: usize = 6;

impl WireMessage for GetGiftMessageListResponse {
    const COMMAND: u16 = 5018;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(GIFT_LIST_EMPTY_BODY_LEN);
        body.extend_from_slice(&i32::from(self.invalid).to_le_bytes());
        body.extend_from_slice(&count_prefix(self.messages.len()).to_le_bytes());
        for message in &self.messages {
            message.encode_inner(&mut body);
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < GIFT_LIST_EMPTY_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: GIFT_LIST_EMPTY_BODY_LEN,
                have: body.len(),
            });
        }
        let invalid = i32::from_le_bytes([body[0], body[1], body[2], body[3]]) != 0;
        let raw_count = i16::from_le_bytes([body[4], body[5]]);
        let count = usize::try_from(i32::from(raw_count)).map_err(|_| Error::InvalidField {
            field: "gift message count",
            reason: "negative count",
        })?;
        let mut messages = Vec::with_capacity(count);
        let mut tail = &body[GIFT_LIST_EMPTY_BODY_LEN..];
        for _ in 0..count {
            let (message, rest) = GiftMessage::decode_prefix(tail)?;
            tail = rest;
            messages.push(message);
        }
        if !tail.is_empty() {
            return Err(Error::TrailingBytes(tail.len()));
        }
        Ok(Self { invalid, messages })
    }
}

/// Claim verdict codes (Memoryer `ClaimGiftResult`, one byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ClaimGiftResult {
    /// Gift moved into the account.
    Success = 0,
    /// Gift id or resource unknown (codes 1–2).
    NotDefined = 1,
    /// Server database failure.
    DbError = 3,
    /// Anything else.
    UnknownError = 4,
}

impl ClaimGiftResult {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Interpret a raw verdict. `None` means a code the 7.55
    /// server never sends (kept extensible, never guessed).
    #[must_use]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Success),
            1 | 2 => Some(Self::NotDefined),
            3 => Some(Self::DbError),
            4 => Some(Self::UnknownError),
            _ => None,
        }
    }
}

/// `ClaimGift` 5024 response: validity word plus claim verdict.
///
/// Dispatched by `cmpl $0x13a0` (VA `0x4a55b1`, file `0xa55b1`);
/// the handler takes the error path unless the first word is zero
/// (VA `0x4a6135`, file `0xa6135`), matching the validity flag;
/// the verdict byte itself is Memoryer corroboration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimGiftResponse {
    /// False when `result_code` applies.
    pub invalid: bool,
    /// Raw verdict; see [`ClaimGiftResult::from_u8`].
    pub result_code: u8,
}

/// Body length of [`ClaimGiftResponse`]: `i32` flag + verdict byte.
const CLAIM_GIFT_RESPONSE_BODY_LEN: usize = 5;

impl WireMessage for ClaimGiftResponse {
    const COMMAND: u16 = 5024;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(CLAIM_GIFT_RESPONSE_BODY_LEN);
        body.extend_from_slice(&i32::from(self.invalid).to_le_bytes());
        body.push(self.result_code);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < CLAIM_GIFT_RESPONSE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: CLAIM_GIFT_RESPONSE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > CLAIM_GIFT_RESPONSE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - CLAIM_GIFT_RESPONSE_BODY_LEN,
            ));
        }
        Ok(Self {
            invalid: i32::from_le_bytes([body[0], body[1], body[2], body[3]]) != 0,
            result_code: body[4],
        })
    }
}

/// Free-pass kinds (Memoryer `FreePassType`, `uint` bit flags;
/// the server sends one value per sync).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FreePassType {
    /// No pass.
    None = 0,
    /// Every song, no expiry.
    AllMusic = 1,
    /// Display-only slot.
    UnusedFreePlay1 = 2,
    /// New songs while the pass lasts.
    NewMusic = 4,
    /// Acquired songs while the pass lasts.
    FreePlay = 8,
    /// Acquired songs while the pass lasts.
    FreePlayAlt1 = 16,
    /// TOP 100 while the pass lasts.
    Top100 = 32,
    /// Every song while the pass lasts.
    PremiumTime = 64,
    /// Display-only slot.
    UnusedFreePlay2 = 128,
    /// TOP 100 (extended) while the pass lasts.
    Top100Extended = 256,
    /// Acquired songs while the pass lasts.
    FreePlayAlt2 = 512,
    /// Display-only slot.
    UnusedFreePlay3 = 1024,
    /// Acquired songs while the pass lasts.
    FreePlayAlt3 = 1056,
}

impl FreePassType {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// Parse a wire word; unknown or combined flags are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside the known flag values.
    pub fn from_u32(value: u32) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::AllMusic),
            2 => Ok(Self::UnusedFreePlay1),
            4 => Ok(Self::NewMusic),
            8 => Ok(Self::FreePlay),
            16 => Ok(Self::FreePlayAlt1),
            32 => Ok(Self::Top100),
            64 => Ok(Self::PremiumTime),
            128 => Ok(Self::UnusedFreePlay2),
            256 => Ok(Self::Top100Extended),
            512 => Ok(Self::FreePlayAlt2),
            1024 => Ok(Self::UnusedFreePlay3),
            1056 => Ok(Self::FreePlayAlt3),
            _ => Err(Error::InvalidField {
                field: "free-pass type",
                reason: "unknown flag value",
            }),
        }
    }
}

/// Free-pass sync verdicts (Memoryer `SyncResult`, `uint`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SyncFreePassResult {
    /// Wallet + pass state follow (or nothing on a short body).
    Success = 0,
    /// Pass state unavailable.
    Unknown = 0xFFFF_FFA2,
}

impl SyncFreePassResult {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// Interpret a raw verdict. `None` means a code the 7.55
    /// server never sends (kept extensible, never guessed).
    #[must_use]
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Success),
            0xFFFF_FFA2 => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// Wallet + pass snapshot inside [`SyncFreePassResponse`] 5026
/// (Memoryer's `UserFreePassInfo`; corroboration — exe side only
/// the first body word reaches the pass applier, VA `0x4f424f`,
/// file `0xf424f`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFreePassInfo {
    /// Gem balance.
    pub gem: i32,
    /// Premium currency.
    pub point: i32,
    /// Global premium currency.
    pub o2_cash: i32,
    /// Song-shop cash.
    pub music_cash: i32,
    /// Item-shop cash.
    pub item_cash: i32,
    /// Active pass kind.
    pub free_pass_type: FreePassType,
    /// Cash points.
    pub cash_point: i32,
    /// Formatted pass duration (empty when no pass runs).
    pub duration: String,
    /// Pass seconds remaining (`SubscriptionTimeExpiryCodec`
    /// writes total seconds as `i32`).
    pub expiry_secs: i32,
}

/// Wire words before the duration string: 7 × `i32`/`u32`.
const FREE_PASS_INFO_WORDS_LEN: usize = 28;

impl UserFreePassInfo {
    /// Parse from the front of `body`, returning the value and rest.
    fn decode_prefix(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < FREE_PASS_INFO_WORDS_LEN {
            return Err(Error::TruncatedMessage {
                need: FREE_PASS_INFO_WORDS_LEN,
                have: body.len(),
            });
        }
        let word =
            |at: usize| i32::from_le_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]]);
        let free_pass_type =
            FreePassType::from_u32(u32::from_le_bytes([body[20], body[21], body[22], body[23]]))?;
        let mut tail = &body[FREE_PASS_INFO_WORDS_LEN..];
        let (duration, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
        tail = &tail[used..];
        if tail.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: tail.len(),
            });
        }
        let expiry_secs = i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
        Ok((
            Self {
                gem: word(0),
                point: word(4),
                o2_cash: word(8),
                music_cash: word(12),
                item_cash: word(16),
                free_pass_type,
                cash_point: word(24),
                duration,
                expiry_secs,
            },
            &tail[4..],
        ))
    }

    /// Full body for sending (tests and future use).
    fn encode_inner(&self, body: &mut Vec<u8>) {
        for word in [
            self.gem,
            self.point,
            self.o2_cash,
            self.music_cash,
            self.item_cash,
        ] {
            body.extend_from_slice(&word.to_le_bytes());
        }
        body.extend_from_slice(&self.free_pass_type.as_u32().to_le_bytes());
        body.extend_from_slice(&self.cash_point.to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.duration, MAX_STRING_LEN));
        body.extend_from_slice(&self.expiry_secs.to_le_bytes());
    }
}

/// `SyncFreePass` 5026 response: `u32` result plus the optional
/// wallet snapshot (absent when the body ends after `result`, like
/// [`ChannelLoginResponse`]'s trailing flag).
///
/// Dispatched by `cmpl $0x13a2` (VAs `0x4f40f2`/`0x4f40fe`, files
/// `0xf40f2`/`0xf40fe`); the info tail is Memoryer corroboration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncFreePassResponse {
    /// Raw result code; see [`SyncFreePassResult::from_u32`].
    pub result_code: u32,
    /// Wallet snapshot; `None` on a 4-byte result-only body.
    pub info: Option<UserFreePassInfo>,
}

/// Body length of a result-only [`SyncFreePassResponse`].
const SYNC_FREE_PASS_RESPONSE_MIN_LEN: usize = 4;

impl WireMessage for SyncFreePassResponse {
    const COMMAND: u16 = 5026;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SYNC_FREE_PASS_RESPONSE_MIN_LEN);
        body.extend_from_slice(&self.result_code.to_le_bytes());
        if let Some(info) = &self.info {
            info.encode_inner(&mut body);
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SYNC_FREE_PASS_RESPONSE_MIN_LEN {
            return Err(Error::TruncatedMessage {
                need: SYNC_FREE_PASS_RESPONSE_MIN_LEN,
                have: body.len(),
            });
        }
        let tail = &body[SYNC_FREE_PASS_RESPONSE_MIN_LEN..];
        let info = if tail.is_empty() {
            None
        } else {
            let (info, rest) = UserFreePassInfo::decode_prefix(tail)?;
            if !rest.is_empty() {
                return Err(Error::TrailingBytes(rest.len()));
            }
            Some(info)
        };
        Ok(Self {
            result_code: u32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            info,
        })
    }
}

/// `SyncCashPoint` 5029 response: wallet refresh after a web
/// coupon/cash charge (Memoryer's `SyncCashPointResponse`;
/// corroboration only — no `0x13a5` site exists in `.text`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncCashPointResponse {
    /// Gem balance.
    pub gem: i32,
    /// Cash points.
    pub cash_point: i32,
}

/// Body length of [`SyncCashPointResponse`]: two `i32` words.
const SYNC_CASH_POINT_RESPONSE_BODY_LEN: usize = 8;

impl WireMessage for SyncCashPointResponse {
    const COMMAND: u16 = 5029;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SYNC_CASH_POINT_RESPONSE_BODY_LEN);
        body.extend_from_slice(&self.gem.to_le_bytes());
        body.extend_from_slice(&self.cash_point.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SYNC_CASH_POINT_RESPONSE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SYNC_CASH_POINT_RESPONSE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SYNC_CASH_POINT_RESPONSE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - SYNC_CASH_POINT_RESPONSE_BODY_LEN,
            ));
        }
        Ok(Self {
            gem: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            cash_point: i32::from_le_bytes([body[4], body[5], body[6], body[7]]),
        })
    }
}

/// `CheckGiftFreePass` 5041 response: recipient lookup verdict
/// (Memoryer's `CheckGiftFreePassResponse`; corroboration only —
/// no `0x13b1` site exists in `.text`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckGiftFreePassResponse {
    /// True when `username` is empty (no such account).
    pub invalid: bool,
    /// Confirmed recipient nickname (empty when invalid).
    pub username: String,
}

impl WireMessage for CheckGiftFreePassResponse {
    const COMMAND: u16 = 5041;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4);
        body.extend_from_slice(&i32::from(self.invalid).to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.username, MAX_STRING_LEN));
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        let invalid = i32::from_le_bytes([body[0], body[1], body[2], body[3]]) != 0;
        let (username, used) = decode_null_terminated(&body[4..], MAX_STRING_LEN)?;
        if 4 + used != body.len() {
            return Err(Error::TrailingBytes(body.len() - 4 - used));
        }
        Ok(Self { invalid, username })
    }
}

/// `SellItem` 5002 request: sell the selected shop entry back.
///
/// Body is two little-endian `i32` words: the validated selection
/// index and the entry's id word. The send case (VA `0x4a510c`,
/// file `0xa510c`; index 0 of the `0x138a`-based send switch, byte
/// table file `0xa54ac`) appends exactly those two words via the
/// fixed-size writer (`0x555d70`): first the `0x77ccc0` selection
/// cell (bounds-checked against the entry count `0x77cce4`, entry
/// fetched via `0x4a3250`), then the entry pair's first word. The
/// two virtual-send call sites pass ignored zeros (VAs `0x49cee2`/
/// `0x4a4c9e`, files `0x9cee2`/`0xa4c9e`); the values come from
/// shop UI state, not the call. Exact game meaning of the id word
/// stays OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellItem {
    /// Selection index snapshot (`0x77ccc0`).
    pub selection: i32,
    /// Entry id word (first word of the fetched entry pair).
    pub entry: i32,
}

/// Body length of [`SellItem`]: two `i32` words.
const SELL_ITEM_BODY_LEN: usize = 8;

impl WireMessage for SellItem {
    const COMMAND: u16 = 5002;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SELL_ITEM_BODY_LEN);
        body.extend_from_slice(&self.selection.to_le_bytes());
        body.extend_from_slice(&self.entry.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SELL_ITEM_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SELL_ITEM_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SELL_ITEM_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - SELL_ITEM_BODY_LEN));
        }
        Ok(Self {
            selection: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            entry: i32::from_le_bytes([body[4], body[5], body[6], body[7]]),
        })
    }
}

/// `SellItem` 5003 response: the server's sell verdict.
///
/// Body is one `i32` result plus an optional second `i32` present
/// exactly when the result is zero. The S→C handler (VA `0x560c80`,
/// file `0x160c80`; index 0 of the `0x138b`-based receive switch,
/// byte table file `0x158ae0`) reads the first word and returns
/// unless it is `0`, then reads the second word into `0x768b4c`
/// and applies it (`0x540090`); the shop-context case
/// (VA `0x4a5c76`, file `0xa5c76`) treats `0` as success and
/// `0xffffffa0` as failure. Code meanings stay OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellItemResponse {
    /// Sell verdict (`0` takes the success path).
    pub result: i32,
    /// Second word; `Some` exactly when `result == 0`.
    pub extra: Option<i32>,
}

/// Body length of a bare [`SellItemResponse`] result.
const SELL_ITEM_RESPONSE_MIN_LEN: usize = 4;
/// Body length of a [`SellItemResponse`] success (result + word).
const SELL_ITEM_RESPONSE_FULL_LEN: usize = 8;

impl WireMessage for SellItemResponse {
    const COMMAND: u16 = 5003;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SELL_ITEM_RESPONSE_FULL_LEN);
        body.extend_from_slice(&self.result.to_le_bytes());
        if let Some(extra) = self.extra {
            body.extend_from_slice(&extra.to_le_bytes());
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SELL_ITEM_RESPONSE_MIN_LEN {
            return Err(Error::TruncatedMessage {
                need: SELL_ITEM_RESPONSE_MIN_LEN,
                have: body.len(),
            });
        }
        let result = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        if result != 0 {
            if body.len() > SELL_ITEM_RESPONSE_MIN_LEN {
                return Err(Error::TrailingBytes(
                    body.len() - SELL_ITEM_RESPONSE_MIN_LEN,
                ));
            }
            return Ok(Self {
                result,
                extra: None,
            });
        }
        if body.len() < SELL_ITEM_RESPONSE_FULL_LEN {
            return Err(Error::TruncatedMessage {
                need: SELL_ITEM_RESPONSE_FULL_LEN,
                have: body.len(),
            });
        }
        if body.len() > SELL_ITEM_RESPONSE_FULL_LEN {
            return Err(Error::TrailingBytes(
                body.len() - SELL_ITEM_RESPONSE_FULL_LEN,
            ));
        }
        Ok(Self {
            result,
            extra: Some(i32::from_le_bytes([body[4], body[5], body[6], body[7]])),
        })
    }
}

/// `ChangeName` 5030 request: spend a rename coupon on `name`.
///
/// Body is a null-terminated name plus two `i32` words. The send
/// case (VA `0x4a520f`, file `0xa520f`; index 28 of the
/// `0x138a`-based send switch, byte table file `0xa54ac`)
/// appends the name from the rename buffer (object `+0xb8`) via
/// the string writer (`0x555e30` = `strlen + 1` bytes), then the
/// selection-cell snapshot (`0x77ccc0`) and one `0x537b80` lookup
/// word via the fixed-size writer (`0x555d70`). Live call sites
/// pass ignored zeros (VAs `0x49ce55`/`0x4a4cba`, files `0x9ce55`/
/// `0xa4cba`); the values come from shop UI state. The words'
/// game meaning stays OPEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeName {
    /// New nickname (null-terminated on the wire).
    pub name: String,
    /// Selection-cell snapshot (`0x77ccc0`).
    pub selection: i32,
    /// Lookup word for the rename entry.
    pub entry: i32,
}

/// Words after the name in a [`ChangeName`] body: two `i32`.
const CHANGE_NAME_WORDS_LEN: usize = 8;

impl WireMessage for ChangeName {
    const COMMAND: u16 = 5030;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = encode_null_terminated(&self.name, MAX_STRING_LEN);
        body.extend_from_slice(&self.selection.to_le_bytes());
        body.extend_from_slice(&self.entry.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (name, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
        let tail = &body[used..];
        if tail.len() < CHANGE_NAME_WORDS_LEN {
            return Err(Error::TruncatedMessage {
                need: CHANGE_NAME_WORDS_LEN,
                have: tail.len(),
            });
        }
        if tail.len() > CHANGE_NAME_WORDS_LEN {
            return Err(Error::TrailingBytes(tail.len() - CHANGE_NAME_WORDS_LEN));
        }
        Ok(Self {
            name,
            selection: i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]),
            entry: i32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]),
        })
    }
}

/// `ChangeName` 5031 response: the rename verdict.
///
/// Body is one little-endian `i32`. The S→C handler (VA `0x563950`,
/// file `0x163950`; index 28 of the `0x138b`-based receive switch,
/// byte table file `0x158ae0`) reads a single word: `0` runs the
/// rename applier (`0x540bf0`), anything else skips it. Code
/// meanings stay OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeNameResponse {
    /// Rename verdict (`0` = applied).
    pub result_code: i32,
}

/// Body length of [`ChangeNameResponse`]: one `i32` word.
const CHANGE_NAME_RESPONSE_BODY_LEN: usize = 4;

impl WireMessage for ChangeNameResponse {
    const COMMAND: u16 = 5031;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(CHANGE_NAME_RESPONSE_BODY_LEN);
        body.extend_from_slice(&self.result_code.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < CHANGE_NAME_RESPONSE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: CHANGE_NAME_RESPONSE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > CHANGE_NAME_RESPONSE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - CHANGE_NAME_RESPONSE_BODY_LEN,
            ));
        }
        Ok(Self {
            result_code: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
        })
    }
}

/// `CheckName` 5032 request: look up `name` before renaming.
///
/// Body is one null-terminated nickname. The send case (VA
/// `0x45c3af`, file `0x5c3af`) writes the header then appends the
/// rename buffer (object `+0x288`) via the string writer
/// (`0x555e30`); the call site (VA `0x45c200`, file `0x5c200`)
/// passes ignored zeros, like the sell/name-change call sites —
/// the value comes from UI state. A length gate precedes it: when
/// the buffer's `strlen` already matches the expected count the
/// client shows a notice instead of sending (VA `0x45c1cb`, file
/// `0x5c1cb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckName {
    /// Nickname to check.
    pub name: String,
}

impl WireMessage for CheckName {
    const COMMAND: u16 = 5032;

    fn encode_body(&self) -> Vec<u8> {
        encode_null_terminated(&self.name, MAX_STRING_LEN)
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (name, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
        if used != body.len() {
            return Err(Error::TrailingBytes(body.len() - used));
        }
        Ok(Self { name })
    }
}

/// `CheckName` 5033 response: nickname-availability answer.
///
/// Body is one `i32` verdict plus a null-terminated string. The
/// S→C handler (VA `0x5639a0`, file `0x1639a0`; index 30 of the
/// `0x138b`-based receive switch) reads the word, then the string
/// via the string reader (`0x555ea0` = `strlen + 1` bytes), and
/// merges the word with `0xff000000`. The string's role
/// (checked-name echo vs notice) stays OPEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckNameResponse {
    /// Availability verdict (merged with `0xff000000` in-exe).
    pub result_code: i32,
    /// Trailing string (null-terminated on the wire).
    pub name: String,
}

impl WireMessage for CheckNameResponse {
    const COMMAND: u16 = 5033;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4);
        body.extend_from_slice(&self.result_code.to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.name, MAX_STRING_LEN));
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        let result_code = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let (name, used) = decode_null_terminated(&body[4..], MAX_STRING_LEN)?;
        if 4 + used != body.len() {
            return Err(Error::TrailingBytes(body.len() - 4 - used));
        }
        Ok(Self { result_code, name })
    }
}

/// Keep [`Request`]/[`Response`]/[`Generic`] ids consistent with
/// the message types above.
#[cfg(test)]
mod command_consistency_tests {
    use super::*;
    use crate::command::{Event, Generic, Request, Response};

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
        assert_eq!(frame.id, 2007);
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

    #[test]
    fn equip_item_encodes_slot_then_item() {
        let request = EquipItem {
            slot: 3,
            item_id: 5,
        };
        assert_eq!(
            request.encode(),
            vec![0x0C, 0x00, 0x8C, 0x13, 0x03, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn equip_item_round_trips_through_frame() {
        let request = EquipItem {
            slot: 11,
            item_id: 23001,
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5004);
        assert_eq!(EquipItem::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn equip_item_rejects_short_body() {
        assert_eq!(
            EquipItem::decode_body(&[0x01; 7]),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn equip_item_rejects_trailing_bytes() {
        let mut body = EquipItem {
            slot: 0,
            item_id: 1,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(EquipItem::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn equip_item_rejects_wrong_command() {
        let frame = decode_frame(&EquipItemResponse { item_id: 5 }.encode()).unwrap();
        assert_eq!(
            EquipItem::decode_frame(&frame),
            Err(Error::UnknownCommand(5005))
        );
    }

    #[test]
    fn equip_item_response_round_trips_through_frame() {
        let response = EquipItemResponse { item_id: 23001 };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5005);
        assert_eq!(EquipItemResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn equip_item_response_zero_item_decodes() {
        let response = EquipItemResponse::decode_body(&[0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(response.item_id, 0);
    }

    #[test]
    fn equip_item_response_rejects_short_body() {
        assert_eq!(
            EquipItemResponse::decode_body(&[0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 2 })
        );
    }

    #[test]
    fn equip_item_response_rejects_trailing_bytes() {
        let mut body = EquipItemResponse { item_id: 1 }.encode_body();
        body.push(0xFF);
        assert_eq!(
            EquipItemResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn gift_type_names_wire_values() {
        assert_eq!(GiftType::from_u8(0), Ok(GiftType::Item));
        assert_eq!(GiftType::from_u8(1), Ok(GiftType::Music));
        assert_eq!(
            GiftType::from_u8(2),
            Err(Error::InvalidField {
                field: "gift type",
                reason: "not 0 or 1"
            })
        );
        assert_eq!(GiftType::Item.as_u8(), 0);
        assert_eq!(GiftType::Music.as_u8(), 1);
    }

    #[test]
    fn gift_requests_encode_bare_frames() {
        assert_eq!(GetGiftMessageList.encode(), vec![0x04, 0x00, 0x99, 0x13]);
        assert_eq!(SyncFreePass.encode(), vec![0x04, 0x00, 0xA1, 0x13]);
        assert_eq!(StartPayment.encode(), vec![0x04, 0x00, 0x97, 0x13]);
        assert_eq!(SyncCashPoint.encode(), vec![0x04, 0x00, 0xA4, 0x13]);
        assert_eq!(StartPaymentResponse.encode(), vec![0x04, 0x00, 0x98, 0x13]);
    }

    #[test]
    fn read_gift_message_encodes_single_word() {
        let request = ReadGiftMessage { gift_message_id: 7 };
        assert_eq!(
            request.encode(),
            vec![0x08, 0x00, 0x9B, 0x13, 0x07, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn read_gift_message_round_trips_through_frame() {
        let request = ReadGiftMessage {
            gift_message_id: -3,
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5019);
        assert_eq!(ReadGiftMessage::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn read_gift_message_rejects_short_body() {
        assert_eq!(
            ReadGiftMessage::decode_body(&[0x01, 0x02, 0x03]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    #[test]
    fn read_gift_message_rejects_trailing_bytes() {
        let mut body = ReadGiftMessage { gift_message_id: 1 }.encode_body();
        body.push(0xFF);
        assert_eq!(
            ReadGiftMessage::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn read_gift_message_rejects_wrong_command() {
        let frame = decode_frame(&GetGiftMessageList.encode()).unwrap();
        assert_eq!(
            ReadGiftMessage::decode_frame(&frame),
            Err(Error::UnknownCommand(5017))
        );
    }

    #[test]
    fn claim_gift_encodes_ten_byte_body() {
        let request = ClaimGift {
            gift_type: GiftType::Music,
            gift_id: 9,
            resource_id: 1001,
            is_attributive: false,
        };
        assert_eq!(
            request.encode(),
            vec![
                0x0E, 0x00, 0x9F, 0x13, 0x01, 0x09, 0x00, 0x00, 0x00, 0xE9, 0x03, 0x00, 0x00, 0x00
            ]
        );
    }

    #[test]
    fn claim_gift_round_trips_attributive_item() {
        let request = ClaimGift {
            gift_type: GiftType::Item,
            gift_id: 11,
            resource_id: 23001,
            is_attributive: true,
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5023);
        assert_eq!(ClaimGift::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn claim_gift_rejects_bad_gift_type() {
        let mut body = ClaimGift {
            gift_type: GiftType::Item,
            gift_id: 1,
            resource_id: 2,
            is_attributive: false,
        }
        .encode_body();
        body[0] = 0x02;
        assert_eq!(
            ClaimGift::decode_body(&body),
            Err(Error::InvalidField {
                field: "gift type",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn claim_gift_rejects_bad_flag_byte() {
        let mut body = ClaimGift {
            gift_type: GiftType::Item,
            gift_id: 1,
            resource_id: 2,
            is_attributive: false,
        }
        .encode_body();
        body[9] = 0x02;
        assert_eq!(
            ClaimGift::decode_body(&body),
            Err(Error::InvalidField {
                field: "attributive flag",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn claim_gift_rejects_short_body() {
        assert_eq!(
            ClaimGift::decode_body(&[0x00; 9]),
            Err(Error::TruncatedMessage { need: 10, have: 9 })
        );
    }

    #[test]
    fn claim_gift_rejects_trailing_bytes() {
        let mut body = ClaimGift {
            gift_type: GiftType::Music,
            gift_id: 1,
            resource_id: 2,
            is_attributive: false,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(ClaimGift::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn check_gift_free_pass_round_trips_name() {
        let request = CheckGiftFreePass {
            name: "tester".to_owned(),
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5040);
        assert_eq!(CheckGiftFreePass::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn check_gift_free_pass_encodes_terminated_name() {
        let request = CheckGiftFreePass {
            name: "ab".to_owned(),
        };
        assert_eq!(
            request.encode(),
            vec![0x07, 0x00, 0xB0, 0x13, 0x61, 0x62, 0x00]
        );
    }

    fn gift_entry() -> GiftMessage {
        GiftMessage {
            message_id: 3,
            gift_type: GiftType::Item,
            write_date: "2026-11-06".to_owned(),
            sender: "giver".to_owned(),
            title: "hi".to_owned(),
            content: "take it".to_owned(),
        }
    }

    #[test]
    fn gift_list_response_decodes_empty_list() {
        let body = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let response = GetGiftMessageListResponse::decode_body(&body).unwrap();
        assert!(!response.invalid);
        assert!(response.messages.is_empty());
    }

    #[test]
    fn gift_list_response_round_trips_one_entry() {
        let response = GetGiftMessageListResponse {
            invalid: false,
            messages: vec![gift_entry()],
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5018);
        assert_eq!(
            GetGiftMessageListResponse::decode_frame(&frame),
            Ok(response)
        );
    }

    #[test]
    fn gift_list_response_marks_invalid() {
        let response = GetGiftMessageListResponse {
            invalid: true,
            messages: Vec::new(),
        };
        let back = GetGiftMessageListResponse::decode_body(&response.encode_body()).unwrap();
        assert!(back.invalid);
    }

    #[test]
    fn gift_list_response_rejects_negative_count() {
        let body = vec![0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF];
        assert_eq!(
            GetGiftMessageListResponse::decode_body(&body),
            Err(Error::InvalidField {
                field: "gift message count",
                reason: "negative count"
            })
        );
    }

    #[test]
    fn gift_list_response_rejects_bad_entry_type() {
        let response = GetGiftMessageListResponse {
            invalid: false,
            messages: vec![gift_entry()],
        };
        let mut body = response.encode_body();
        body[6 + 4] = 0x09;
        assert_eq!(
            GetGiftMessageListResponse::decode_body(&body),
            Err(Error::InvalidField {
                field: "gift type",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn gift_list_response_rejects_short_body() {
        assert_eq!(
            GetGiftMessageListResponse::decode_body(&[0x00; 5]),
            Err(Error::TruncatedMessage { need: 6, have: 5 })
        );
    }

    #[test]
    fn claim_gift_result_names_all_codes() {
        assert_eq!(ClaimGiftResult::from_u8(0), Some(ClaimGiftResult::Success));
        assert_eq!(
            ClaimGiftResult::from_u8(1),
            Some(ClaimGiftResult::NotDefined)
        );
        assert_eq!(
            ClaimGiftResult::from_u8(2),
            Some(ClaimGiftResult::NotDefined)
        );
        assert_eq!(ClaimGiftResult::from_u8(3), Some(ClaimGiftResult::DbError));
        assert_eq!(
            ClaimGiftResult::from_u8(4),
            Some(ClaimGiftResult::UnknownError)
        );
        assert_eq!(ClaimGiftResult::from_u8(9), None);
        assert_eq!(ClaimGiftResult::Success.as_u8(), 0);
    }

    #[test]
    fn claim_gift_response_encodes_flag_plus_verdict() {
        let response = ClaimGiftResponse {
            invalid: false,
            result_code: 0,
        };
        assert_eq!(
            response.encode(),
            vec![0x09, 0x00, 0xA0, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn claim_gift_response_round_trips_through_frame() {
        let response = ClaimGiftResponse {
            invalid: true,
            result_code: 4,
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5024);
        assert_eq!(ClaimGiftResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn claim_gift_response_rejects_short_body() {
        assert_eq!(
            ClaimGiftResponse::decode_body(&[0x00; 4]),
            Err(Error::TruncatedMessage { need: 5, have: 4 })
        );
    }

    #[test]
    fn claim_gift_response_rejects_trailing_bytes() {
        let mut body = ClaimGiftResponse {
            invalid: false,
            result_code: 0,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            ClaimGiftResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn free_pass_type_names_flag_values() {
        assert_eq!(FreePassType::from_u32(0), Ok(FreePassType::None));
        assert_eq!(FreePassType::from_u32(1), Ok(FreePassType::AllMusic));
        assert_eq!(FreePassType::from_u32(64), Ok(FreePassType::PremiumTime));
        assert_eq!(FreePassType::from_u32(1056), Ok(FreePassType::FreePlayAlt3));
        assert_eq!(
            FreePassType::from_u32(3),
            Err(Error::InvalidField {
                field: "free-pass type",
                reason: "unknown flag value"
            })
        );
        assert_eq!(FreePassType::Top100.as_u32(), 32);
    }

    #[test]
    fn sync_free_pass_result_names_codes() {
        assert_eq!(
            SyncFreePassResult::from_u32(0),
            Some(SyncFreePassResult::Success)
        );
        assert_eq!(
            SyncFreePassResult::from_u32(0xFFFF_FFA2),
            Some(SyncFreePassResult::Unknown)
        );
        assert_eq!(SyncFreePassResult::from_u32(7), None);
    }

    #[test]
    fn sync_free_pass_response_decodes_result_only() {
        let response = SyncFreePassResponse::decode_body(&[0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(response.result_code, 0);
        assert_eq!(response.info, None);
        assert_eq!(
            SyncFreePassResult::from_u32(response.result_code),
            Some(SyncFreePassResult::Success)
        );
    }

    fn free_pass_info() -> UserFreePassInfo {
        UserFreePassInfo {
            gem: 1,
            point: 2,
            o2_cash: 3,
            music_cash: 4,
            item_cash: 5,
            free_pass_type: FreePassType::PremiumTime,
            cash_point: 6,
            duration: String::new(),
            expiry_secs: 3600,
        }
    }

    #[test]
    fn sync_free_pass_response_round_trips_full_info() {
        let response = SyncFreePassResponse {
            result_code: 0,
            info: Some(free_pass_info()),
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5026);
        assert_eq!(SyncFreePassResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn sync_free_pass_response_rejects_short_info() {
        let body = vec![0x00, 0x00, 0x00, 0x00, 0x01];
        assert_eq!(
            SyncFreePassResponse::decode_body(&body),
            Err(Error::TruncatedMessage { need: 28, have: 1 })
        );
    }

    #[test]
    fn sync_free_pass_response_rejects_trailing_bytes() {
        let mut body = SyncFreePassResponse {
            result_code: 0,
            info: Some(free_pass_info()),
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            SyncFreePassResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn sync_cash_point_response_encodes_two_words() {
        let response = SyncCashPointResponse {
            gem: 5,
            cash_point: 6,
        };
        assert_eq!(
            response.encode(),
            vec![0x0C, 0x00, 0xA5, 0x13, 0x05, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn sync_cash_point_response_round_trips_through_frame() {
        let response = SyncCashPointResponse {
            gem: -1,
            cash_point: 0,
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5029);
        assert_eq!(SyncCashPointResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn sync_cash_point_response_rejects_short_body() {
        assert_eq!(
            SyncCashPointResponse::decode_body(&[0x00; 7]),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn check_gift_free_pass_response_round_trips() {
        let response = CheckGiftFreePassResponse {
            invalid: false,
            username: "tester".to_owned(),
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5041);
        assert_eq!(
            CheckGiftFreePassResponse::decode_frame(&frame),
            Ok(response)
        );
    }

    #[test]
    fn check_gift_free_pass_response_marks_missing_user() {
        let response = CheckGiftFreePassResponse {
            invalid: true,
            username: String::new(),
        };
        let back = CheckGiftFreePassResponse::decode_body(&response.encode_body()).unwrap();
        assert!(back.invalid);
        assert!(back.username.is_empty());
    }

    #[test]
    fn sync_item_purchase_encodes_bare_frame() {
        assert_eq!(SyncItemPurchase.encode(), vec![0x04, 0x00, 0x88, 0x13]);
    }

    #[test]
    fn sync_item_purchase_round_trips_through_frame() {
        let frame = decode_frame(&SyncItemPurchase.encode()).unwrap();
        assert_eq!(frame.id, 5000);
        assert_eq!(SyncItemPurchase::decode_frame(&frame), Ok(SyncItemPurchase));
    }

    #[test]
    fn sync_item_purchase_rejects_trailing_bytes() {
        let frame = decode_frame(&[0x05, 0x00, 0x88, 0x13, 0x00]).unwrap();
        assert_eq!(
            SyncItemPurchase::decode_frame(&frame),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn sync_item_purchase_rejects_wrong_command() {
        let frame = decode_frame(&SyncCashPoint.encode()).unwrap();
        assert_eq!(
            SyncItemPurchase::decode_frame(&frame),
            Err(Error::UnknownCommand(5028))
        );
    }

    #[test]
    fn sell_item_encodes_two_words() {
        let request = SellItem {
            selection: 3,
            entry: 23001,
        };
        assert_eq!(
            request.encode(),
            vec![0x0C, 0x00, 0x8A, 0x13, 0x03, 0x00, 0x00, 0x00, 0xD9, 0x59, 0x00, 0x00]
        );
    }

    #[test]
    fn sell_item_round_trips_through_frame() {
        let request = SellItem {
            selection: 0,
            entry: -1,
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5002);
        assert_eq!(SellItem::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn sell_item_rejects_short_body() {
        assert_eq!(
            SellItem::decode_body(&[0x01; 7]),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn sell_item_rejects_trailing_bytes() {
        let mut body = SellItem {
            selection: 0,
            entry: 1,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(SellItem::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn sell_item_rejects_wrong_command() {
        let frame = decode_frame(
            &SellItemResponse {
                result: 0,
                extra: None,
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            SellItem::decode_frame(&frame),
            Err(Error::UnknownCommand(5003))
        );
    }

    #[test]
    fn sell_item_response_decodes_result_only() {
        let response = SellItemResponse::decode_body(&[0x05, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(response.result, 5);
        assert_eq!(response.extra, None);
    }

    #[test]
    fn sell_item_response_success_carries_extra_word() {
        let response = SellItemResponse {
            result: 0,
            extra: Some(7),
        };
        assert_eq!(
            response.encode(),
            vec![0x0C, 0x00, 0x8B, 0x13, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00]
        );
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5003);
        assert_eq!(SellItemResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn sell_item_response_rejects_extra_word_on_error() {
        let body = vec![0x01, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00];
        assert_eq!(
            SellItemResponse::decode_body(&body),
            Err(Error::TrailingBytes(4))
        );
    }

    #[test]
    fn sell_item_response_rejects_short_success() {
        assert_eq!(
            SellItemResponse::decode_body(&[0x00, 0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
        assert_eq!(
            SellItemResponse::decode_body(&[0x00, 0x00, 0x00, 0x00, 0x01]),
            Err(Error::TruncatedMessage { need: 8, have: 5 })
        );
    }

    #[test]
    fn change_name_encodes_string_plus_two_words() {
        let request = ChangeName {
            name: "ab".to_owned(),
            selection: 1,
            entry: 2,
        };
        assert_eq!(
            request.encode(),
            vec![
                0x0F, 0x00, 0xA6, 0x13, 0x61, 0x62, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
                0x00
            ]
        );
    }

    #[test]
    fn change_name_round_trips_through_frame() {
        let request = ChangeName {
            name: "newname".to_owned(),
            selection: -1,
            entry: 0,
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5030);
        assert_eq!(ChangeName::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn change_name_rejects_short_tail() {
        let mut body = vec![0x61, 0x00];
        body.extend_from_slice(&[0x01; 7]);
        assert_eq!(
            ChangeName::decode_body(&body),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn change_name_rejects_trailing_bytes() {
        let mut body = ChangeName {
            name: "a".to_owned(),
            selection: 0,
            entry: 0,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(ChangeName::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn change_name_rejects_wrong_command() {
        let frame = decode_frame(&ChangeNameResponse { result_code: 0 }.encode()).unwrap();
        assert_eq!(
            ChangeName::decode_frame(&frame),
            Err(Error::UnknownCommand(5031))
        );
    }

    #[test]
    fn change_name_response_encodes_single_word() {
        let response = ChangeNameResponse { result_code: 0 };
        assert_eq!(
            response.encode(),
            vec![0x08, 0x00, 0xA7, 0x13, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn change_name_response_round_trips_through_frame() {
        let response = ChangeNameResponse { result_code: -96 };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5031);
        assert_eq!(ChangeNameResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn change_name_response_rejects_short_body() {
        assert_eq!(
            ChangeNameResponse::decode_body(&[0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 2 })
        );
    }

    #[test]
    fn change_name_response_rejects_trailing_bytes() {
        let mut body = ChangeNameResponse { result_code: 0 }.encode_body();
        body.push(0xFF);
        assert_eq!(
            ChangeNameResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn check_name_response_encodes_word_plus_string() {
        let response = CheckNameResponse {
            result_code: 0,
            name: "ab".to_owned(),
        };
        assert_eq!(
            response.encode(),
            vec![0x0B, 0x00, 0xA9, 0x13, 0x00, 0x00, 0x00, 0x00, 0x61, 0x62, 0x00]
        );
    }

    #[test]
    fn check_name_response_round_trips_through_frame() {
        let response = CheckNameResponse {
            result_code: -1,
            name: "taken".to_owned(),
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 5033);
        assert_eq!(CheckNameResponse::decode_frame(&frame), Ok(response));
    }

    #[test]
    fn check_name_response_rejects_short_body() {
        assert_eq!(
            CheckNameResponse::decode_body(&[0x00, 0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    #[test]
    fn check_name_response_rejects_trailing_bytes() {
        let mut body = CheckNameResponse {
            result_code: 0,
            name: "a".to_owned(),
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            CheckNameResponse::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn check_name_encodes_terminated_name() {
        let request = CheckName {
            name: "ab".to_owned(),
        };
        assert_eq!(
            request.encode(),
            vec![0x07, 0x00, 0xA8, 0x13, 0x61, 0x62, 0x00]
        );
    }

    #[test]
    fn check_name_round_trips_through_frame() {
        let request = CheckName {
            name: "newname".to_owned(),
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 5032);
        assert_eq!(CheckName::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn check_name_rejects_trailing_bytes() {
        let mut body = CheckName {
            name: "a".to_owned(),
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(CheckName::decode_body(&body), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn check_name_rejects_wrong_command() {
        let frame = decode_frame(
            &CheckNameResponse {
                result_code: 0,
                name: "a".to_owned(),
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            CheckName::decode_frame(&frame),
            Err(Error::UnknownCommand(5033))
        );
    }
}
