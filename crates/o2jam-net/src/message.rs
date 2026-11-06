//! Phase-1 message layouts (Memoryer field orders).
//!
//! Fixed-size bodies only for now: `Authorize` (1000) carries
//! strings whose encoding is not yet exe-verified, so it stays
//! out until the string codec is pinned down.

use super::error::Error;
use super::frame::{encode_frame, Frame};

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
        assert_eq!(ChannelLogin::COMMAND, Request::ChannelLogin as u16);
        assert_eq!(GetChannelList::COMMAND, Request::GetChannelList as u16);
        assert_eq!(SessionKeysRequest::COMMAND, Request::SessionKeys as u16);
        assert_eq!(Terminate::COMMAND, Request::Terminate as u16);
        assert_eq!(LegacyPing::COMMAND, Generic::LegacyPing as u16);
        assert_eq!(Ping::COMMAND, Generic::Ping as u16);
        assert_eq!(SessionKeys::COMMAND, Response::SessionKeys as u16);
        assert_eq!(ChannelList::COMMAND, Response::GetChannelList as u16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::decode_frame;

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
