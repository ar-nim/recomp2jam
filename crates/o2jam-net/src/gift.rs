//! Gift-box messages: waiting gifts, claim flow and gift free passes.
//!
//! Covers `GetGiftMessageList` 5017/5018, `ReadGiftMessage` 5019,
//! `ClaimGift` 5023/5024 and `CheckGiftFreePass` 5040/5041. Layouts mirror
//! Memoryer's `Source/Memoryer` field orders (clean-room C#); exe anchors
//! stay on each type, Memoryer-only shapes stay labeled.

use super::error::Error;
use super::frame::Frame;
use super::message::{decode_empty_frame, encode_empty, read_flag, EmptyMessage, WireMessage};
use super::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};

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
/// a free pass. Absent from 7.55 wire: raw-byte scan finds no
/// `0xb0 0x13` / `0xb1 0x13` opcode immediates (the sole `b0 13`
/// bytes are a call rel32 at file `0x162fdc`), and `0x13b0` falls
/// outside the `0x138b`-based receive range (`0x25 > 0x24` →
/// default). Body is one null-terminated nickname (Memoryer's
/// `CheckGiftFreePassRequest`); kept for server interop. The
/// recipient-exists gate lives in the `gift_input.aspx` web view
/// (URL string at file `0x24b65f`).
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
/// (Memoryer's `GiftEntry`; order and widths are Memoryer-only:
/// the exe handler at VA `0x490e86` (file `0x90e86`) never reads the
/// body — it derives a count from the UI table bounds
/// (`0x76fc80` minus `0x76fc7c`, divided by the `0x168` UI stride)
/// and emits UI event `0xc` when nonzero, so the wire field split
/// stays exe-unverified).
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
    i16::try_from(len).unwrap_or(i16::MAX)
}

/// `GetGiftMessageList` 5018 response: validity word plus an
/// `i16`-prefixed message list.
///
/// Dispatched by `cmpl $0x139a` (VA `0x490a4a`, file `0x90a4a`);
/// Memoryer shapes the body as int-flag + `Int16` collection
/// (Memoryer-only: the handler at VA `0x490e86` reads no payload
/// bytes, so neither the count width nor the entry split is
/// exe-verified).
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
/// the success path (`0x4a6156` on) works the `0x24`-stride gift-box
/// table and never reads the verdict slot (`0x14(%ebp)`), so the
/// verdict byte is Memoryer-only (kept for server interop).
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

/// `CheckGiftFreePass` 5041 response: recipient lookup verdict.
/// Absent from 7.55 wire with 5040 (no `0x13b1` immediate or
/// dispatch in `.text`); shape is Memoryer's
/// `CheckGiftFreePassResponse`, kept for server interop.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::decode_frame;
    use crate::message::{StartPayment, StartPaymentResponse, SyncCashPoint, SyncFreePass};

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
    fn gift_list_response_encodes_empty_frame() {
        let response = GetGiftMessageListResponse {
            invalid: false,
            messages: Vec::new(),
        };
        assert_eq!(
            response.encode(),
            vec![0x0A, 0x00, 0x9A, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
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
    fn claim_gift_response_preserves_verdict_bytes() {
        for verdict in [1, 2, 3, 4] {
            let response = ClaimGiftResponse {
                invalid: false,
                result_code: verdict,
            };
            let back = ClaimGiftResponse::decode_body(&response.encode_body()).unwrap();
            assert_eq!(back, response);
        }
        assert_eq!(
            ClaimGiftResult::from_u8(2),
            Some(ClaimGiftResult::NotDefined)
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
}
