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
    fn decode_body(body: &[u8]) -> Result<Self, Error>;

    /// Full frame bytes ready to send.
    fn encode(&self) -> Vec<u8> {
        encode_frame(Self::COMMAND, &self.encode_body())
    }

    /// Parse a received frame, checking its command first.
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
fn encode_endpoint(endpoint: &SocketAddrV4, body: &mut Vec<u8>) {
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
        encode_endpoint(&self.public_endpoint, &mut body);
        encode_endpoint(&self.local_endpoint, &mut body);
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

/// Keep [`Request`]/[`Response`]/[`Generic`] ids consistent with
/// the message types above.
#[cfg(test)]
mod command_consistency_tests {
    use super::*;
    use crate::command::{Generic, Request, Response};

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
            SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 15010),
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
            vec![0x06, 0x00, 0xEC, 0x03, 0x01, 0x00, 0x02, 0x00]
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
        assert_eq!(GetChannelList.encode(), vec![0x02, 0x00, 0xEA, 0x03]);
        assert_eq!(Ping.encode(), vec![0x02, 0x00, 0x71, 0x17]);
        assert_eq!(Terminate.encode(), vec![0x02, 0x00, 0xF0, 0xFF]);
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
        assert_eq!(SubscriptionAlert.encode(), vec![0x02, 0x00, 0x32, 0x00]);
        assert_eq!(ForceReauthorize.encode(), vec![0x02, 0x00, 0x33, 0x00]);
        assert_eq!(MultiSessionError.encode(), vec![0x02, 0x00, 0x34, 0x00]);
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
}
