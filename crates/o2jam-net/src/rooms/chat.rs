//! Room and waiting-room chat messages: 2012/2013/2014
//! main-room chat, 2017/2018/2019 whispers, 2015/2016 announce,
//! and the 3011/3012/3013/3014 waiting-room chat family.

use std::borrow::Cow;

use crate::error::Error;
use crate::message::WireMessage;
use crate::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};
/// `SendMainRoomMessage` 2012 request: raw chat bytes fill the whole
/// body — no length prefix, no terminator, and an empty body sends an
/// empty line. Same layout as [`SendWaitingMessage`], one opcode apart.
/// Memoryer (`Messages/Requests/Messaging/
/// `SendMainRoomMessageRequest.cs`) carries one
/// `CollectionMessageField(order: 0)` byte array, broadcast back as
/// 2013/2014 by `Controllers/MessagingController.cs` (admins take the
/// 2014 fan-out, everyone else 2013). Exe send site: file `0x8fa1e`
/// (`push 0; push 0x7dc`, indirect send call), in the lobby
/// slash-command parser beside the 2017 whisper send at file `0x8fa02`
/// (`/w`, `/r`, `/h`, `/help` literals at VAs `0x647d60`+; observed with
/// GNU objdump 2.47 slices over the read-only 7.55 copy, no new REA
/// dossier filed — Memoryer shape is corroboration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMainRoomMessage {
    /// Chat text bytes (client encoding; opaque on the wire).
    pub content: Vec<u8>,
}

impl SendMainRoomMessage {
    /// Chat text as UTF-8 with invalid sequences replaced.
    #[must_use]
    pub fn content_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }
}

impl WireMessage for SendMainRoomMessage {
    const COMMAND: u16 = 2012;

    fn encode_body(&self) -> Vec<u8> {
        self.content.clone()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            content: body.to_vec(),
        })
    }
}

/// One main-room chat line: null-terminated sender plus raw content
/// bytes running to end of body. Unlike [`WaitingChatLine`] there is no
/// trailing seat id — the content length rides out-of-band
/// (`body.len - sender_used`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainRoomChatLine {
    /// Chatter nickname (UTF-8, null-terminated on the wire).
    pub sender: String,
    /// Chat text bytes (client encoding; opaque on the wire).
    pub content: Vec<u8>,
}

impl MainRoomChatLine {
    /// Chat text as UTF-8 with invalid sequences replaced.
    #[must_use]
    pub fn content_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }
}

fn encode_main_room_line(line: &MainRoomChatLine) -> Vec<u8> {
    let mut body = encode_null_terminated(&line.sender, MAX_STRING_LEN);
    body.extend_from_slice(&line.content);
    body
}

fn decode_main_room_line_body(body: &[u8]) -> Result<MainRoomChatLine, Error> {
    let (sender, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
    Ok(MainRoomChatLine {
        sender,
        content: body[used..].to_vec(),
    })
}

/// `MainRoomUserMessage` 2013 broadcast: a channel line from a player.
/// Memoryer (`Messages/Responses/Messaging/
/// `MainRoomUserMessageResponse.cs`): string sender (order 0), byte-array
/// content (order 1, trailing, so raw to end of stream). Exe receive
/// handler: file `0x15bfa0` reads sender + content through the
/// packet-reader helper, formats `[%12s] %s` (VA `0x64d8d4`), and feeds
/// the ControlList parent global `0x762B44` (opcode push at file
/// `0x15bff5`; observed with GNU objdump 2.47, no new REA dossier
/// filed — Memoryer shape is corroboration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainRoomUserMessage {
    /// Chatter nickname.
    pub sender: String,
    /// Chat text bytes.
    pub content: Vec<u8>,
}

impl MainRoomUserMessage {
    /// Chat text as UTF-8 with invalid sequences replaced.
    #[must_use]
    pub fn content_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }

    fn line(&self) -> MainRoomChatLine {
        MainRoomChatLine {
            sender: self.sender.clone(),
            content: self.content.clone(),
        }
    }

    fn from_line(line: MainRoomChatLine) -> Self {
        Self {
            sender: line.sender,
            content: line.content,
        }
    }
}

impl WireMessage for MainRoomUserMessage {
    const COMMAND: u16 = 2013;

    fn encode_body(&self) -> Vec<u8> {
        encode_main_room_line(&self.line())
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        decode_main_room_line_body(body).map(Self::from_line)
    }
}

/// `MainRoomAdminMessage` 2014 broadcast: a channel line from an
/// administrator. Same layout as 2013 (`Messages/Responses/Messaging/
/// `MainRoomAdminMessageResponse.cs`). Exe receive sites push 2014
/// beside the ControlList parent global `0x762B44` at files `0x15c075`,
/// `0x15c11a`, `0x15c151` (observed with GNU objdump 2.47, no new REA
/// dossier filed — Memoryer shape is corroboration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainRoomAdminMessage {
    /// Chatter nickname.
    pub sender: String,
    /// Chat text bytes.
    pub content: Vec<u8>,
}

impl MainRoomAdminMessage {
    /// Chat text as UTF-8 with invalid sequences replaced.
    #[must_use]
    pub fn content_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }

    fn line(&self) -> MainRoomChatLine {
        MainRoomChatLine {
            sender: self.sender.clone(),
            content: self.content.clone(),
        }
    }

    fn from_line(line: MainRoomChatLine) -> Self {
        Self {
            sender: line.sender,
            content: line.content,
        }
    }
}

impl WireMessage for MainRoomAdminMessage {
    const COMMAND: u16 = 2014;

    fn encode_body(&self) -> Vec<u8> {
        encode_main_room_line(&self.line())
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        decode_main_room_line_body(body).map(Self::from_line)
    }
}

/// `SendWhisper` 2017 request: null-terminated recipient plus
/// null-terminated content. Memoryer (`Messages/Requests/Messaging/
/// `SendWhisperMessageRequest.cs`): string recipient (order 0), string
/// content (order 1). The server delivers a 2019 event to the named
/// recipient and answers 2018 (`Controllers/MessagingController.cs`).
/// Exe send sites (`push 0; push 0x7e1`, indirect send call): files
/// `0x8f757`/`0x8fa02` (lobby `/w` branch), `0xb8ffc`/`0xb9369`
/// (waiting room), `0xe1a85`/`0xe1fd3` (beside the 3011 room-chat sends;
/// observed with GNU objdump 2.47, no new REA dossier filed — Memoryer
/// shape is corroboration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendWhisper {
    /// Recipient nickname (UTF-8, null-terminated on the wire).
    pub recipient: String,
    /// Whisper text (UTF-8, null-terminated on the wire).
    pub content: String,
}

impl WireMessage for SendWhisper {
    const COMMAND: u16 = 2017;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = encode_null_terminated(&self.recipient, MAX_STRING_LEN);
        body.extend_from_slice(&encode_null_terminated(&self.content, MAX_STRING_LEN));
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (recipient, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
        let tail = &body[used..];
        let (content, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
        if !tail[used..].is_empty() {
            return Err(Error::TrailingBytes(tail.len() - used));
        }
        Ok(Self { recipient, content })
    }
}

/// `SendWhisper` 2018 response: delivery report for a 2017 request.
/// Memoryer (`Messages/Responses/Messaging/SendWhisperResponse.cs`):
/// bool `Invalid` as little-endian `i32` (order 0, non-zero reads true),
/// string recipient (order 1), trailing-nullable string content
/// (order 2 — absent when nobody heard the whisper). Exe receive site:
/// file `0x15c20b` feeds the ControlList parent global `0x762B44`
/// (observed with GNU objdump 2.47, no new REA dossier filed —
/// Memoryer shape is corroboration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendWhisperResponse {
    /// True when no session carried the recipient nickname.
    pub invalid: bool,
    /// Recipient nickname the request named.
    pub recipient: String,
    /// Echoed whisper text (`None` when undelivered).
    pub content: Option<String>,
}

impl WireMessage for SendWhisperResponse {
    const COMMAND: u16 = 2018;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4);
        body.extend_from_slice(&i32::from(self.invalid).to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.recipient, MAX_STRING_LEN));
        if let Some(content) = &self.content {
            body.extend_from_slice(&encode_null_terminated(content, MAX_STRING_LEN));
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
        let invalid = i32::from_le_bytes([body[0], body[1], body[2], body[3]]) != 0;
        let (recipient, used) = decode_null_terminated(&body[4..], MAX_STRING_LEN)?;
        let tail = &body[4 + used..];
        let content = if tail.is_empty() {
            None
        } else {
            let (content, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
            if !tail[used..].is_empty() {
                return Err(Error::TrailingBytes(tail.len() - used));
            }
            Some(content)
        };
        Ok(Self {
            invalid,
            recipient,
            content,
        })
    }
}

/// `ReceiveWhisper` 2019 event: an incoming whisper. Memoryer
/// (`Messages/Events/Common/WhisperEventData.cs`): string sender
/// (order 0), string content (order 1, default empty). The content
/// always rides the wire (even empty content writes its terminator);
/// a body ending right after the sender terminator still decodes to
/// empty content, mirroring the trailing-default. Exe receive site:
/// file `0x15c5b0` feeds the ControlList parent global `0x762B44`
/// (observed with GNU objdump 2.47, no new REA dossier filed —
/// Memoryer shape is corroboration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiveWhisper {
    /// Sender nickname.
    pub sender: String,
    /// Whisper text.
    pub content: String,
}

impl WireMessage for ReceiveWhisper {
    const COMMAND: u16 = 2019;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = encode_null_terminated(&self.sender, MAX_STRING_LEN);
        body.extend_from_slice(&encode_null_terminated(&self.content, MAX_STRING_LEN));
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (sender, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
        let tail = &body[used..];
        let content = if tail.is_empty() {
            String::new()
        } else {
            let (content, used) = decode_null_terminated(tail, MAX_STRING_LEN)?;
            if !tail[used..].is_empty() {
                return Err(Error::TrailingBytes(tail.len() - used));
            }
            content
        };
        Ok(Self { sender, content })
    }
}

/// Lobby notice text: one null-terminated UTF-8 string, no prefix.
/// Shared layout of [`Announce`] and [`Announcement`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct NoticeText {
    /// Notice text (UTF-8, null-terminated on the wire).
    content: String,
}

fn encode_notice_text(notice: &NoticeText) -> Vec<u8> {
    encode_null_terminated(&notice.content, MAX_STRING_LEN)
}

fn decode_notice_text_body(body: &[u8]) -> Result<NoticeText, Error> {
    let (content, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
    if !body[used..].is_empty() {
        return Err(Error::TrailingBytes(body.len() - used));
    }
    Ok(NoticeText { content })
}

/// `Announce` 2015 request: an administrator notice for every channel.
/// Memoryer (`Messages/Requests/Messaging/AnnouncementRequest.cs`):
/// one `StringMessageField(order: 0)`; the server fans it out as 2016
/// (`Controllers/MessagingController.cs`). Exe send sites: files
/// `0x8f585`/`0x8f5dc` (`push 0; push 0x7df`, indirect `+0x5c` send
/// call), in the lobby slash-command parser beside the 2017 whisper
/// send — both take the `/n ` branch (prefix at VA `0x647d68`, length
/// 3 at file `0x8f556` / 4 at file `0x8f5ad`) and both sit behind a
/// nonzero gate on byte global `0x768b6c` (file `0x8f53e`). Observed
/// with GNU objdump 2.47 slices plus Python byte scans over the
/// read-only 7.55 copy; no new REA dossier filed — the single-string
/// shape is Memoryer corroboration, byte-confirmed on the 2016 read
/// side below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announce {
    /// Notice text.
    pub content: String,
}

impl WireMessage for Announce {
    const COMMAND: u16 = 2015;

    fn encode_body(&self) -> Vec<u8> {
        encode_notice_text(&NoticeText {
            content: self.content.clone(),
        })
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        decode_notice_text_body(body).map(|notice| Self {
            content: notice.content,
        })
    }
}

/// `Announcement` 2016 response: the fanned-out notice every client
/// paints on the lobby notice displays. Same single-string layout as
/// [`Announce`] (`Messages/Responses/Messaging/
/// `AnnouncementResponse.cs`). Exe receive: dispatch subtracts base
/// `0x7d3` (file `0x1582ec`), bounds `0x20` (file `0x1582f8`), and
/// indexes the address table at file `0x158940`; index `0x0d` (2016)
/// lands case file `0x1584df`, which calls handler VA `0x55c0a0`
/// (file `0x15c0a0`). The handler reads exactly one string through
/// the string-reader helper (`0x555ea0`) into a `0x408`-byte stack
/// buffer, then feeds the ControlList parent global `0x762B44`:
/// count `0xa` redraws via `0x4793a0`, else the text fans out as
/// `0x7de` (admin-message) rows `(2, text)` / `(5, text)` via
/// `0x479010` (files `0x15c11a`/`0x15c151`). Observed with GNU
/// objdump 2.47 slices plus Python byte scans over the read-only
/// 7.55 copy; no new REA dossier filed. The one-string read
/// byte-verifies the wire shape; the `0x408` buffer hints a practical
/// text cap (Memoryer sets none) — OPEN until a long notice is fed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    /// Notice text.
    pub content: String,
}

impl WireMessage for Announcement {
    const COMMAND: u16 = 2016;

    fn encode_body(&self) -> Vec<u8> {
        encode_notice_text(&NoticeText {
            content: self.content.clone(),
        })
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        decode_notice_text_body(body).map(|notice| Self {
            content: notice.content,
        })
    }
}

/// `SendWaitingMessage` 3011 request: raw chat bytes fill the whole
/// body — no length prefix, no terminator, and an empty body sends an
/// empty line. Memoryer (`Messages/Requests/Messaging/
/// `SendWaitingMessageRequest.cs`) carries one
/// `CollectionMessageField(order: 0)` byte array; Encore 1.0.0
/// `DefaultMessageCodec` encodes `abc` as body `61 62 63` and an empty
/// array as the bare opcode (`C3 0B` + no body), both round-tripping
/// through its decoder. The exe stages five send sites that push opcode
/// 3011 with a zero word then send indirectly (files `0xb9385`,
/// `0xb95b4`, `0xb9742`, `0xe1aa6`, `0xe1ff7`,
/// `ev_130b4726ca4f7075f8526a05fda5d876`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendWaitingMessage {
    /// Chat text bytes (client encoding; opaque on the wire).
    pub content: Vec<u8>,
}

impl SendWaitingMessage {
    /// Chat text as UTF-8 with invalid sequences replaced.
    #[must_use]
    pub fn content_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }
}

impl WireMessage for SendWaitingMessage {
    const COMMAND: u16 = 3011;

    fn encode_body(&self) -> Vec<u8> {
        self.content.clone()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            content: body.to_vec(),
        })
    }
}

/// One waiting-room chat line: null-terminated sender, raw content
/// bytes, trailing seat id. The content length rides out-of-band
/// (`body.len - sender_used - 1`); the sender terminator delimits the
/// front and the fixed seat byte anchors the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingChatLine {
    /// Chatter nickname (UTF-8, null-terminated on the wire).
    pub sender: String,
    /// Chat text bytes (client encoding; opaque on the wire).
    pub content: Vec<u8>,
    /// Sender seat index (matches `MessagingController`, which fills it
    /// from the room slot list).
    pub member_id: u8,
}

impl WaitingChatLine {
    /// Chat text as UTF-8 with invalid sequences replaced.
    #[must_use]
    pub fn content_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }
}

fn encode_chat_line(line: &WaitingChatLine) -> Vec<u8> {
    let mut body = encode_null_terminated(&line.sender, MAX_STRING_LEN);
    body.extend_from_slice(&line.content);
    body.push(line.member_id);
    body
}

fn decode_chat_line_body(body: &[u8]) -> Result<WaitingChatLine, Error> {
    let (sender, used) = decode_null_terminated(body, MAX_STRING_LEN)?;
    let tail = body.get(used..).ok_or(Error::TruncatedMessage {
        need: used.saturating_add(1),
        have: body.len(),
    })?;
    if tail.is_empty() {
        return Err(Error::TruncatedMessage {
            need: used.saturating_add(1),
            have: body.len(),
        });
    }
    let member_id = tail[tail.len() - 1];
    Ok(WaitingChatLine {
        sender,
        content: tail[..tail.len() - 1].to_vec(),
        member_id,
    })
}

/// `WaitingUserMessage` 3012 response: a chat line from a player.
/// Memoryer (`Messages/Responses/Messaging/
/// `WaitingUserMessageResponse.cs`): string sender (order 0), byte-array
/// content (order 1), byte seat id (order 2). Encore 1.0.0 encodes
/// (`ab`, `abc`, seat 5) as body `61 62 00 61 62 63 05` and
/// (`""`, empty, seat 0) as `00 00`; its own decoder throws on these
/// vectors (middle-collection plus trailing field), so the encode side
/// is the verified contract and this decoder inverts it. Exe receive
/// cluster: ten sites push 3012 beside the ControlList parent global
/// `0x762B44` from file `0x15e601`
/// (`ev_aba36a3aa42dfa2b7bde94cae4e28a45`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingUserMessage {
    /// Chatter nickname.
    pub sender: String,
    /// Chat text bytes.
    pub content: Vec<u8>,
    /// Sender seat index.
    pub member_id: u8,
}

impl WaitingUserMessage {
    /// Chat text as UTF-8 with invalid sequences replaced.
    #[must_use]
    pub fn content_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }

    fn line(&self) -> WaitingChatLine {
        WaitingChatLine {
            sender: self.sender.clone(),
            content: self.content.clone(),
            member_id: self.member_id,
        }
    }

    fn from_line(line: WaitingChatLine) -> Self {
        Self {
            sender: line.sender,
            content: line.content,
            member_id: line.member_id,
        }
    }
}

impl WireMessage for WaitingUserMessage {
    const COMMAND: u16 = 3012;

    fn encode_body(&self) -> Vec<u8> {
        encode_chat_line(&self.line())
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        decode_chat_line_body(body).map(Self::from_line)
    }
}

/// `WaitingAdminMessage` 3013 response: a chat line from an
/// administrator. Same layout as 3012 (`Messages/Responses/Messaging/
/// `WaitingAdminMessageResponse.cs`); Encore 1.0.0 encodes (`ab`,
/// `a`, seat 7) as body `61 62 00 61 07`. Exe receive cluster: three
/// sites push 3013 beside the ControlList parent global `0x762B44`
/// from file `0x15f051`
/// (`ev_aba36a3aa42dfa2b7bde94cae4e28a45`). The waiting chat view
/// (`EDIT_SWAIT_CHAT 0x16300014`, one ref at file `0xbaf1a`;
/// `LIST_SWAIT_CHAT 0x1680002B`, five refs from file `0xb8e11`;
/// `ev_f8422c8a059247042312e3d7224dbdff`) stages art, but the row
/// template `TEXT_SWAIT_CHAT 0x1640003C` has zero immediate refs — the
/// exact row-feed call stays OPEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingAdminMessage {
    /// Chatter nickname.
    pub sender: String,
    /// Chat text bytes.
    pub content: Vec<u8>,
    /// Sender seat index.
    pub member_id: u8,
}

impl WaitingAdminMessage {
    /// Chat text as UTF-8 with invalid sequences replaced.
    #[must_use]
    pub fn content_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }

    fn line(&self) -> WaitingChatLine {
        WaitingChatLine {
            sender: self.sender.clone(),
            content: self.content.clone(),
            member_id: self.member_id,
        }
    }

    fn from_line(line: WaitingChatLine) -> Self {
        Self {
            sender: line.sender,
            content: line.content,
            member_id: line.member_id,
        }
    }
}

impl WireMessage for WaitingAdminMessage {
    const COMMAND: u16 = 3013;

    fn encode_body(&self) -> Vec<u8> {
        encode_chat_line(&self.line())
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        decode_chat_line_body(body).map(Self::from_line)
    }
}

/// Slot action (`Encore.Metadata.Room.RoomSlotActionType`, byte).
#[cfg(test)]
mod waiting_chat_tests {
    use super::*;
    use crate::frame::decode_frame;

    #[test]
    fn send_carries_raw_bytes_verbatim() {
        // Encore vector: `C7 D1 B1 DB` encodes bare (opcode `C3 0B` + body).
        let message = SendWaitingMessage {
            content: vec![0xC7, 0xD1, 0xB1, 0xDB],
        };
        assert_eq!(message.encode_body(), vec![0xC7, 0xD1, 0xB1, 0xDB]);
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 3011);
        assert_eq!(SendWaitingMessage::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn send_empty_body_is_an_empty_line() {
        let message = SendWaitingMessage {
            content: Vec::new(),
        };
        assert_eq!(message.encode(), vec![0x04, 0x00, 0xC3, 0x0B]);
        assert_eq!(SendWaitingMessage::decode_body(&[]), Ok(message));
    }

    #[test]
    fn user_message_matches_encore_vector() {
        // Encore vector: (`ab`, `abc`, seat 5) -> body below.
        let message = WaitingUserMessage {
            sender: "ab".to_owned(),
            content: vec![0x61, 0x62, 0x63],
            member_id: 5,
        };
        assert_eq!(
            message.encode_body(),
            vec![0x61, 0x62, 0x00, 0x61, 0x62, 0x63, 0x05]
        );
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 3012);
        assert_eq!(WaitingUserMessage::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn admin_message_matches_encore_vector() {
        // Encore vector: (`ab`, `a`, seat 7) -> body below.
        let message = WaitingAdminMessage {
            sender: "ab".to_owned(),
            content: vec![0x61],
            member_id: 7,
        };
        assert_eq!(message.encode_body(), vec![0x61, 0x62, 0x00, 0x61, 0x07]);
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 3013);
        assert_eq!(WaitingAdminMessage::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn empty_line_decodes_sender_plus_seat() {
        // Encore vector: empty user message -> body `00 00`.
        let message = WaitingUserMessage::decode_body(&[0x00, 0x00]).unwrap();
        assert_eq!(message.sender, String::new());
        assert!(message.content.is_empty());
        assert_eq!(message.member_id, 0);
    }

    #[test]
    fn rejects_line_without_seat_byte() {
        assert_eq!(
            WaitingUserMessage::decode_body(&[0x61, 0x62, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    #[test]
    fn rejects_line_without_sender_terminator() {
        assert_eq!(
            WaitingAdminMessage::decode_body(&[0x61, 0x62, 0x05]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    #[test]
    fn content_text_replaces_invalid_utf8() {
        let message = SendWaitingMessage {
            content: vec![0xC7, 0xD1],
        };
        assert_eq!(message.content_text(), "\u{FFFD}\u{FFFD}");
    }
}

#[cfg(test)]
mod main_room_chat_tests {
    use super::*;
    use crate::frame::decode_frame;

    #[test]
    fn send_carries_raw_bytes_verbatim() {
        let message = SendMainRoomMessage {
            content: vec![0xC7, 0xD1, 0xB1, 0xDB],
        };
        assert_eq!(message.encode_body(), vec![0xC7, 0xD1, 0xB1, 0xDB]);
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2012);
        assert_eq!(SendMainRoomMessage::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn send_empty_body_is_an_empty_line() {
        let message = SendMainRoomMessage {
            content: Vec::new(),
        };
        assert_eq!(message.encode(), vec![0x04, 0x00, 0xDC, 0x07]);
        assert_eq!(SendMainRoomMessage::decode_body(&[]), Ok(message));
    }

    #[test]
    fn user_message_carries_sender_plus_raw_content() {
        let message = MainRoomUserMessage {
            sender: "ab".to_owned(),
            content: vec![0x61, 0x62, 0x63],
        };
        assert_eq!(
            message.encode_body(),
            vec![0x61, 0x62, 0x00, 0x61, 0x62, 0x63]
        );
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2013);
        assert_eq!(MainRoomUserMessage::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn admin_message_carries_sender_plus_raw_content() {
        let message = MainRoomAdminMessage {
            sender: "ab".to_owned(),
            content: vec![0x61],
        };
        assert_eq!(message.encode_body(), vec![0x61, 0x62, 0x00, 0x61]);
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2014);
        assert_eq!(MainRoomAdminMessage::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn empty_line_decodes_sender_with_empty_content() {
        let message = MainRoomUserMessage::decode_body(&[0x00]).unwrap();
        assert_eq!(message.sender, String::new());
        assert!(message.content.is_empty());
    }

    #[test]
    fn unterminated_sender_decodes_to_end_of_body() {
        // Null-string codec: no terminator in range means the field
        // fills its cap, so the bytes become the sender.
        let message = MainRoomAdminMessage::decode_body(&[0x61, 0x62]).unwrap();
        assert_eq!(message.sender, "ab");
        assert!(message.content.is_empty());
    }

    #[test]
    fn whisper_request_carries_two_terminated_strings() {
        let message = SendWhisper {
            recipient: "ab".to_owned(),
            content: "cd".to_owned(),
        };
        assert_eq!(
            message.encode_body(),
            vec![0x61, 0x62, 0x00, 0x63, 0x64, 0x00]
        );
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2017);
        assert_eq!(SendWhisper::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn unterminated_content_decodes_to_end_of_body() {
        // Null-string codec: no terminator in range means the field
        // fills its cap, so the bytes become the content.
        let message = SendWhisper::decode_body(&[0x61, 0x62, 0x00, 0x63, 0x64]).unwrap();
        assert_eq!(message.recipient, "ab");
        assert_eq!(message.content, "cd");
    }

    #[test]
    fn whisper_request_rejects_trailing_bytes() {
        assert_eq!(
            SendWhisper::decode_body(&[0x61, 0x00, 0x62, 0x00, 0xFF]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn whisper_answer_reports_invalid_without_content() {
        // Undelivered whisper: result word plus recipient only.
        let message = SendWhisperResponse {
            invalid: true,
            recipient: "ab".to_owned(),
            content: None,
        };
        assert_eq!(
            message.encode_body(),
            vec![0x01, 0x00, 0x00, 0x00, 0x61, 0x62, 0x00]
        );
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2018);
        assert_eq!(SendWhisperResponse::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn whisper_answer_echoes_content_when_delivered() {
        let message = SendWhisperResponse {
            invalid: false,
            recipient: "ab".to_owned(),
            content: Some("cd".to_owned()),
        };
        assert_eq!(
            message.encode_body(),
            vec![0x00, 0x00, 0x00, 0x00, 0x61, 0x62, 0x00, 0x63, 0x64, 0x00]
        );
        let back = SendWhisperResponse::decode_body(&message.encode_body()).unwrap();
        assert_eq!(back, message);
    }

    #[test]
    fn whisper_answer_rejects_short_body() {
        assert_eq!(
            SendWhisperResponse::decode_body(&[0x01, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 2 })
        );
    }

    #[test]
    fn receive_whisper_carries_sender_plus_text() {
        let message = ReceiveWhisper {
            sender: "ab".to_owned(),
            content: "cd".to_owned(),
        };
        assert_eq!(
            message.encode_body(),
            vec![0x61, 0x62, 0x00, 0x63, 0x64, 0x00]
        );
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2019);
        assert_eq!(ReceiveWhisper::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn receive_whisper_defaults_missing_content_to_empty() {
        let message = ReceiveWhisper::decode_body(&[0x61, 0x62, 0x00]).unwrap();
        assert_eq!(message.sender, "ab");
        assert_eq!(message.content, String::new());
    }

    #[test]
    fn main_room_content_text_replaces_invalid_utf8() {
        let message = MainRoomUserMessage {
            sender: "ab".to_owned(),
            content: vec![0xC7, 0xD1],
        };
        assert_eq!(message.content_text(), "\u{FFFD}\u{FFFD}");
        let send = SendMainRoomMessage {
            content: vec![0xC7],
        };
        assert_eq!(send.content_text(), "\u{FFFD}");
    }
}

#[cfg(test)]
mod notice_tests {
    use super::*;
    use crate::frame::decode_frame;

    #[test]
    fn announce_carries_one_terminated_string() {
        let message = Announce {
            content: "abc".to_owned(),
        };
        assert_eq!(message.encode_body(), vec![0x61, 0x62, 0x63, 0x00]);
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2015);
        assert_eq!(Announce::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn announce_encodes_full_frame_bytes() {
        let message = Announce {
            content: "abc".to_owned(),
        };
        assert_eq!(
            message.encode(),
            vec![0x08, 0x00, 0xDF, 0x07, 0x61, 0x62, 0x63, 0x00]
        );
    }

    #[test]
    fn announce_empty_body_is_empty_notice() {
        let message = Announce::decode_body(&[]).unwrap();
        assert_eq!(message.content, String::new());
        assert_eq!(message.encode_body(), vec![0x00]);
    }

    #[test]
    fn announce_rejects_trailing_bytes() {
        assert_eq!(
            Announce::decode_body(&[0x61, 0x00, 0xFF]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn announcement_matches_announce_layout() {
        // Same Content string; the server fans 2015 out as 2016.
        let message = Announcement {
            content: "abc".to_owned(),
        };
        assert_eq!(message.encode_body(), vec![0x61, 0x62, 0x63, 0x00]);
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2016);
        assert_eq!(Announcement::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn announcement_decodes_live_capture() {
        // Live bytes 2026-11-08 via the relay.md §12 proxy method
        // (127.0.0.1:15011 -> 15010, local Memoryer Full-mode):
        // C->S 2015 `LIVE-PROBE-2016 hello` echoed as S->C 2016 with the
        // identical NUL-terminated body. Channel enter itself sent no
        // 2016 (1005/2001/2026/2003 only).
        let frame = decode_frame(&[
            0x1a, 0x00, 0xe0, 0x07, 0x4c, 0x49, 0x56, 0x45, 0x2d, 0x50, 0x52, 0x4f, 0x42, 0x45,
            0x2d, 0x32, 0x30, 0x31, 0x36, 0x20, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x00,
        ])
        .unwrap();
        assert_eq!(frame.id, 2016);
        assert_eq!(
            Announcement::decode_frame(&frame),
            Ok(Announcement {
                content: "LIVE-PROBE-2016 hello".to_owned(),
            })
        );
    }

    #[test]
    fn announcement_rejects_foreign_command() {
        let frame = decode_frame(
            &Announce {
                content: "abc".to_owned(),
            }
            .encode(),
        )
        .unwrap();
        assert_eq!(
            Announcement::decode_frame(&frame),
            Err(Error::UnknownCommand(2015))
        );
    }

    #[test]
    fn rank_notification_matches_wire_order() {
        // invalid i32, music id i32, nickname string, ranking i32.
        let message = RankNotification {
            invalid: false,
            music_id: 757,
            nickname: "ab".to_owned(),
            ranking: 3,
        };
        assert_eq!(
            message.encode_body(),
            vec![
                0x00, 0x00, 0x00, 0x00, 0xF5, 0x02, 0x00, 0x00, 0x61, 0x62, 0x00, 0x03, 0x00, 0x00,
                0x00
            ]
        );
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2029);
        assert_eq!(RankNotification::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn rank_notification_reports_invalid_without_rank() {
        // The exe drops nonzero-invalid packets before the UI forward.
        let message = RankNotification {
            invalid: true,
            music_id: 757,
            nickname: "ab".to_owned(),
            ranking: 0,
        };
        assert_eq!(&message.encode_body()[..4], &[0x01, 0x00, 0x00, 0x00]);
        let back = RankNotification::decode_body(&message.encode_body()).unwrap();
        assert_eq!(back, message);
        assert!(back.invalid);
    }

    #[test]
    fn rank_notification_treats_any_nonzero_word_as_invalid() {
        // Mirrors the exe `cmp ..., 0` / `jne skip` gate.
        let mut body = RankNotification {
            invalid: false,
            music_id: 757,
            nickname: "ab".to_owned(),
            ranking: 3,
        }
        .encode_body();
        body[0] = 0x02;
        assert!(RankNotification::decode_body(&body).unwrap().invalid);
    }

    #[test]
    fn rank_notification_rejects_short_body() {
        assert_eq!(
            RankNotification::decode_body(&[0x00, 0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 8, have: 3 })
        );
    }

    #[test]
    fn rank_notification_rejects_missing_rank() {
        // invalid + music id + nickname, no ranking word.
        let body = vec![
            0x00, 0x00, 0x00, 0x00, 0xF5, 0x02, 0x00, 0x00, 0x61, 0x62, 0x00,
        ];
        assert_eq!(
            RankNotification::decode_body(&body),
            Err(Error::TruncatedMessage { need: 15, have: 11 })
        );
    }

    #[test]
    fn rank_notification_rejects_trailing_bytes() {
        let mut body = RankNotification {
            invalid: false,
            music_id: 757,
            nickname: "ab".to_owned(),
            ranking: 3,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            RankNotification::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    fn sample_rank_entry() -> RankEntry {
        RankEntry {
            rank: 1,
            nickname: "ab".to_owned(),
            battles: 10,
            wins: 7,
            win_rate: 0,
            delta: RankDelta::Up,
            rank_delta: 2,
        }
    }

    #[test]
    fn play_ranking_request_encodes_empty_frame() {
        // The exe never sends 2027; the fetch carries no fields.
        let frame = decode_frame(&GetMusicPlayRanking.encode()).unwrap();
        assert_eq!(frame.id, 2027);
        assert!(frame.payload.is_empty());
        assert_eq!(
            GetMusicPlayRanking::decode_frame(&frame),
            Ok(GetMusicPlayRanking)
        );
    }

    #[test]
    fn play_ranking_request_rejects_any_body() {
        assert_eq!(
            GetMusicPlayRanking::decode_body(&[0x00]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn play_ranking_answer_decodes_self_and_entries() {
        // Self line plus two board rows: a named row and a blank
        // (rank-zero) row that carries no further fields.
        let answer = MusicPlayRankingAnswer {
            self_entry: sample_rank_entry(),
            entries: vec![
                RankEntry {
                    rank: 2,
                    nickname: "cd".to_owned(),
                    battles: 5,
                    wins: 5,
                    win_rate: 1,
                    delta: RankDelta::Down,
                    rank_delta: 1,
                },
                RankEntry::default(),
            ],
        };
        let frame = decode_frame(&answer.encode()).unwrap();
        assert_eq!(frame.id, 2028);
        assert_eq!(MusicPlayRankingAnswer::decode_frame(&frame), Ok(answer));
    }

    #[test]
    fn play_ranking_answer_matches_wire_order() {
        // rank i32, nickname string, battles/wins/win-rate i32s,
        // delta u8, rank-delta i32; then count i32 plus rows.
        let body = vec![
            0x01, 0x00, 0x00, 0x00, 0x61, 0x62, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let answer = MusicPlayRankingAnswer::decode_body(&body).unwrap();
        assert_eq!(answer.self_entry, sample_rank_entry());
        assert!(answer.entries.is_empty());
    }

    #[test]
    fn play_ranking_answer_treats_rank_zero_as_default_row() {
        // Rank-zero rows carry no fields: the exe pushes a zeroed row.
        let answer = MusicPlayRankingAnswer::decode_body(&[
            0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ])
        .unwrap();
        assert_eq!(answer.self_entry, RankEntry::default());
        assert_eq!(answer.entries, vec![RankEntry::default()]);
    }

    #[test]
    fn play_ranking_answer_reads_zero_count_as_empty_list() {
        // A zero count paints ten blank rows; the wire carries none.
        let answer =
            MusicPlayRankingAnswer::decode_body(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                .unwrap();
        assert!(answer.entries.is_empty());
    }

    #[test]
    fn play_ranking_answer_reads_negative_count_as_empty() {
        // The exe loop tests the index against the count with a signed
        // compare, so negative counts paint no rows.
        let answer =
            MusicPlayRankingAnswer::decode_body(&[0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF])
                .unwrap();
        assert!(answer.entries.is_empty());
    }

    #[test]
    fn play_ranking_answer_rejects_truncated_row() {
        // Rank word plus an unterminated nickname, no stat words: the
        // codec reads the nickname to end of body, then misses the 13
        // trailing stat bytes (4 + 2 + 13 = 19 needed).
        assert_eq!(
            MusicPlayRankingAnswer::decode_body(&[0x01, 0x00, 0x00, 0x00, 0x61, 0x62]),
            Err(Error::TruncatedMessage { need: 19, have: 6 })
        );
    }

    #[test]
    fn play_ranking_answer_rejects_bad_nickname_encoding() {
        // Rank word, invalid-UTF-8 nickname, terminator, no stats.
        assert_eq!(
            MusicPlayRankingAnswer::decode_body(&[0x01, 0x00, 0x00, 0x00, 0xFF, 0x00]),
            Err(Error::InvalidField {
                field: "string field",
                reason: "not valid UTF-8",
            })
        );
    }

    #[test]
    fn play_ranking_answer_rejects_bad_delta() {
        let mut body = encode_rank_entry(&sample_rank_entry());
        body.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        let delta_at = 4 + 3 + 12;
        body[delta_at] = 0x02;
        assert_eq!(
            MusicPlayRankingAnswer::decode_body(&body),
            Err(Error::InvalidField {
                field: "rank delta type",
                reason: "not 0 (Up) or 1 (Down)",
            })
        );
    }

    #[test]
    fn score_list_request_encodes_empty_frame() {
        // Both exe send sites push only the opcode.
        let frame = decode_frame(&GetMusicScoreList.encode()).unwrap();
        assert_eq!(frame.id, 2030);
        assert!(frame.payload.is_empty());
        assert_eq!(
            GetMusicScoreList::decode_frame(&frame),
            Ok(GetMusicScoreList)
        );
    }

    #[test]
    fn score_list_request_rejects_any_body() {
        assert_eq!(
            GetMusicScoreList::decode_body(&[0x00]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn score_list_answer_decodes_rows() {
        let answer = MusicScoreListAnswer {
            result: 0,
            rows: vec![
                MusicScoreRow {
                    music_id: 757,
                    difficulty_ranks: [8, 7, 6, 0],
                },
                MusicScoreRow {
                    music_id: 758,
                    difficulty_ranks: [0, 0, 0, 0],
                },
            ],
        };
        assert_eq!(
            answer.encode_body(),
            vec![
                0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0xF5, 0x02, 0x08, 0x07, 0x06, 0x00, 0xF6, 0x02,
                0x00, 0x00, 0x00, 0x00,
            ]
        );
        let frame = decode_frame(&answer.encode()).unwrap();
        assert_eq!(frame.id, 2031);
        assert_eq!(MusicScoreListAnswer::decode_frame(&frame), Ok(answer));
    }

    #[test]
    fn score_list_answer_treats_negative_result_as_empty() {
        // Negative results skip the rows but still refresh the UI.
        let answer = MusicScoreListAnswer::decode_body(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
        assert_eq!(answer.result, -1);
        assert!(answer.rows.is_empty());
    }

    #[test]
    fn score_list_answer_rejects_truncated_count() {
        assert_eq!(
            MusicScoreListAnswer::decode_body(&[0x00, 0x00, 0x00, 0x00, 0x02]),
            Err(Error::TruncatedMessage { need: 6, have: 5 })
        );
    }

    #[test]
    fn score_list_answer_rejects_truncated_row() {
        // result 0, count 1, only three of six row bytes.
        assert_eq!(
            MusicScoreListAnswer::decode_body(&[
                0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0xF5, 0x02, 0x08,
            ]),
            Err(Error::TruncatedMessage { need: 12, have: 9 })
        );
    }

    #[test]
    fn max_score_request_carries_music_and_note_count() {
        let message = GetMusicMaxScore {
            music_id: 757,
            note_count: 486,
        };
        assert_eq!(message.encode_body(), vec![0xF5, 0x02, 0xE6, 0x01]);
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2032);
        assert_eq!(GetMusicMaxScore::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn max_score_request_rejects_short_body() {
        assert_eq!(
            GetMusicMaxScore::decode_body(&[0xF5, 0x02]),
            Err(Error::TruncatedMessage { need: 4, have: 2 })
        );
    }

    #[test]
    fn max_score_request_rejects_trailing_bytes() {
        let mut body = GetMusicMaxScore {
            music_id: 757,
            note_count: 486,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            GetMusicMaxScore::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn max_score_answer_decodes_score() {
        let message = MusicMaxScoreAnswer {
            invalid: false,
            max_score: 1_234_567,
        };
        assert_eq!(
            message.encode_body(),
            vec![0x00, 0x00, 0x00, 0x00, 0x87, 0xD6, 0x12, 0x00]
        );
        let frame = decode_frame(&message.encode()).unwrap();
        assert_eq!(frame.id, 2033);
        assert_eq!(MusicMaxScoreAnswer::decode_frame(&frame), Ok(message));
    }

    #[test]
    fn max_score_answer_reports_invalid_without_score() {
        // Nonzero invalid words drop the packet client-side.
        let message = MusicMaxScoreAnswer {
            invalid: true,
            max_score: 0,
        };
        assert_eq!(&message.encode_body()[..4], &[0x01, 0x00, 0x00, 0x00]);
        let back = MusicMaxScoreAnswer::decode_body(&message.encode_body()).unwrap();
        assert_eq!(back, message);
        assert!(back.invalid);
    }

    #[test]
    fn max_score_answer_rejects_short_body() {
        assert_eq!(
            MusicMaxScoreAnswer::decode_body(&[0x00, 0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 8, have: 3 })
        );
    }

    #[test]
    fn max_score_answer_rejects_trailing_bytes() {
        let mut body = MusicMaxScoreAnswer {
            invalid: false,
            max_score: 42,
        }
        .encode_body();
        body.push(0xFF);
        assert_eq!(
            MusicMaxScoreAnswer::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }
}
