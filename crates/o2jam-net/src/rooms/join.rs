//! Room join messages: 2004/2005 create, 3002/3003 join,
//! the joined-room member/slot layout, and the 3009 slot-update
//! event.

use crate::error::Error;
use crate::message::WireMessage;
use crate::reader::Reader;
use crate::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};
fn decode_room_member(reader: &mut Reader<'_>) -> Result<RoomMember, Error> {
    let nickname = reader.read_string()?;
    let level = reader.read_i32()?;
    let gender = reader.read_u8()?;
    let gem = reader.read_i32()?;
    let master = reader.read_flag("member master flag")?;
    let team = reader.read_u8()?;
    let ready = reader.read_flag("member ready flag")?;
    let music_state = reader.read_u8()?;
    let mut equipment = [0i32; 17];
    for slot in &mut equipment {
        *slot = reader.read_i32()?;
    }
    let music_count = reader.read_count("member music ids", true)?;
    let mut music_ids = Vec::with_capacity(music_count);
    for _ in 0..music_count {
        music_ids.push(reader.read_u16()?);
    }
    let cash_point = reader.read_i32()?;
    let free_pass = reader.read_u32()?;
    let playing_state = reader.read_u8()?;
    let administrator = reader.read_flag("member administrator flag")?;
    let super_manager = reader.read_int_flag("member super-manager flag")?;
    Ok(RoomMember {
        nickname,
        level,
        gender,
        gem,
        master,
        team,
        ready,
        music_state,
        equipment,
        music_ids,
        cash_point,
        free_pass,
        playing_state,
        administrator,
        super_manager,
    })
}

fn encode_room_member(body: &mut Vec<u8>, member: &RoomMember) {
    body.extend_from_slice(&encode_null_terminated(&member.nickname, MAX_STRING_LEN));
    body.extend_from_slice(&member.level.to_le_bytes());
    body.push(member.gender);
    body.extend_from_slice(&member.gem.to_le_bytes());
    body.push(u8::from(member.master));
    body.push(member.team);
    body.push(u8::from(member.ready));
    body.push(member.music_state);
    for slot in member.equipment {
        body.extend_from_slice(&slot.to_le_bytes());
    }
    body.extend_from_slice(&(member.music_ids.len() as i32).to_le_bytes());
    for id in &member.music_ids {
        body.extend_from_slice(&id.to_le_bytes());
    }
    body.extend_from_slice(&member.cash_point.to_le_bytes());
    body.extend_from_slice(&member.free_pass.to_le_bytes());
    body.push(member.playing_state);
    body.push(u8::from(member.administrator));
    body.extend_from_slice(&i32::from(member.super_manager).to_le_bytes());
}

fn decode_room_slot(reader: &mut Reader<'_>) -> Result<RoomSlot, Error> {
    let index = reader.read_u8()?;
    let state_code = reader.read_i32()?;
    let member = if state_code == RoomSlotState::Occupied as i32 {
        Some(decode_room_member(reader)?)
    } else {
        None
    };
    Ok(RoomSlot {
        index,
        state_code,
        member,
    })
}

fn encode_room_slot(body: &mut Vec<u8>, slot: &RoomSlot) {
    body.push(slot.index);
    body.extend_from_slice(&slot.state_code.to_le_bytes());
    if let Some(member) = &slot.member {
        encode_room_member(body, member);
    }
}

fn decode_joined_room(body: &[u8]) -> Result<JoinedRoom, Error> {
    let mut reader = Reader::new(body);
    let member_id = reader.read_u8()?;
    let team = reader.read_u8()?;
    let title = reader.read_string()?;
    let music_id = reader.read_u16()?;
    let arena = reader.read_i32()?;
    let key_mode = reader.read_u8()?;
    let game_mode = reader.read_u8()?;
    let difficulty = reader.read_u8()?;
    let speed = reader.read_u8()?;
    let has_super_manager = reader.read_int_flag("room super-manager flag")?;
    let user_count = reader.read_i32()?;
    let mut slots = Vec::with_capacity(7);
    for _ in 0..7 {
        slots.push(decode_room_slot(&mut reader)?);
    }
    let skill_count = reader.read_count("joined-room skills", true)?;
    let mut skills = Vec::with_capacity(skill_count);
    for _ in 0..skill_count {
        skills.push(reader.read_i32()?);
    }
    let premium = reader.read_short_flag("joined-room premium flag")?;
    let champion = if reader.remaining() == 0 {
        None
    } else {
        Some(reader.read_u8()?)
    };
    let champion_streak = if reader.remaining() == 0 {
        None
    } else {
        Some(reader.read_i32()?)
    };
    reader.finish()?;
    Ok(JoinedRoom {
        member_id,
        team,
        title,
        music_id,
        arena,
        key_mode,
        game_mode,
        difficulty,
        speed,
        has_super_manager,
        user_count,
        slots,
        skills,
        premium,
        champion,
        champion_streak,
    })
}

fn encode_joined_room(body: &mut Vec<u8>, room: &JoinedRoom) {
    body.push(room.member_id);
    body.push(room.team);
    body.extend_from_slice(&encode_null_terminated(&room.title, MAX_STRING_LEN));
    body.extend_from_slice(&room.music_id.to_le_bytes());
    body.extend_from_slice(&room.arena.to_le_bytes());
    for byte in [room.key_mode, room.game_mode, room.difficulty, room.speed] {
        body.push(byte);
    }
    body.extend_from_slice(&i32::from(room.has_super_manager).to_le_bytes());
    body.extend_from_slice(&room.user_count.to_le_bytes());
    for slot in &room.slots {
        encode_room_slot(body, slot);
    }
    body.extend_from_slice(&(room.skills.len() as i32).to_le_bytes());
    for skill in &room.skills {
        body.extend_from_slice(&skill.to_le_bytes());
    }
    body.extend_from_slice(&i16::from(room.premium).to_le_bytes());
    if let Some(champion) = room.champion {
        body.push(champion);
    }
    if let Some(streak) = room.champion_streak {
        body.extend_from_slice(&streak.to_le_bytes());
    }
}

/// Join result codes (Memoryer `JoinResult`, `uint`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum JoinResult {
    /// Seated.
    Success = 0,
    /// Unspecified failure.
    GenericError = 0xFFFF_FFFF,
    /// Bad game mode for the room.
    InvalidMode = 0xFFFF_FFFE,
    /// Wrong password.
    InvalidPassword = 0xFFFF_FFFD,
    /// Game already started.
    InProgress = 0xFFFF_FFFB,
    /// Room full.
    Full = 0xFFFF_FFFA,
    /// Password required but none given.
    NoPass = 0xFFFF_FFF9,
    /// No such room number.
    InvalidNumber = 0xFFFF_FFF8,
}

impl JoinResult {
    /// Interpret a raw result code. `None` means unknown.
    #[must_use]
    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            0xFFFF_FFFF => Some(Self::GenericError),
            0xFFFF_FFFE => Some(Self::InvalidMode),
            0xFFFF_FFFD => Some(Self::InvalidPassword),
            0xFFFF_FFFB => Some(Self::InProgress),
            0xFFFF_FFFA => Some(Self::Full),
            0xFFFF_FFF9 => Some(Self::NoPass),
            0xFFFF_FFF8 => Some(Self::InvalidNumber),
            _ => None,
        }
    }
}

/// Seat state (`JoinResult.RoomSlotState`, `int`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum RoomSlotState {
    /// Free seat.
    Unoccupied = 0,
    /// Taken seat.
    Occupied = 1,
    /// Locked seat.
    Locked = 2,
}

impl RoomSlotState {
    /// Interpret a raw state word. `None` means unknown.
    #[must_use]
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Unoccupied),
            1 => Some(Self::Occupied),
            2 => Some(Self::Locked),
            _ => None,
        }
    }
}

/// One seated member (`RoomMemberInfo` submessage).
// Four flags mirror the wire layout one-to-one; grouping them would
// obscure the field order.
#[allow(clippy::struct_excessive_bools, reason = "wire-mandated flag fields")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMember {
    /// Display name.
    pub nickname: String,
    /// Level.
    pub level: i32,
    /// Gender byte (`crate::message::Gender` reads 0/1/2).
    pub gender: u8,
    /// Gem balance.
    pub gem: i32,
    /// Room master.
    pub master: bool,
    /// Team slot (A–H as 0–7).
    pub team: u8,
    /// Ready flag.
    pub ready: bool,
    /// Song state byte.
    pub music_state: u8,
    /// Equipped item ids by `ItemType` order.
    pub equipment: [i32; 17],
    /// Installed song ids.
    pub music_ids: Vec<u16>,
    /// Cash points.
    pub cash_point: i32,
    /// Free-pass type word.
    pub free_pass: u32,
    /// Playing-state byte.
    pub playing_state: u8,
    /// Administrator account.
    pub administrator: bool,
    /// Super room manager (ring pass holder, not master).
    pub super_manager: bool,
}

/// One of the 7 seats (`RoomSlotInfo`): member details ride only
/// occupied seats — vacant/locked seats encode index + state alone
/// (the framework writes null submessages as zero bytes, so the
/// exe must branch on state; our decoder does the same).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSlot {
    /// Seat index.
    pub index: u8,
    /// Seat state word; see [`RoomSlotState::from_code`].
    pub state_code: i32,
    /// Member details (`Some` exactly when occupied).
    pub member: Option<RoomMember>,
}

/// The joined room (Memoryer nests it as `RoomInfo` inside 3003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedRoom {
    /// Our seat id.
    pub member_id: u8,
    /// Our team slot.
    pub team: u8,
    /// Room title.
    pub title: String,
    /// Selected song id.
    pub music_id: u16,
    /// Arena info word.
    pub arena: i32,
    /// Key-mode code.
    pub key_mode: u8,
    /// Game-mode code.
    pub game_mode: u8,
    /// Difficulty code.
    pub difficulty: u8,
    /// Speed code.
    pub speed: u8,
    /// Super room manager present.
    pub has_super_manager: bool,
    /// Seated players.
    pub user_count: i32,
    /// Seven seats in index order.
    pub slots: Vec<RoomSlot>,
    /// Active skill-ring ids.
    pub skills: Vec<i32>,
    /// Premium-only room.
    pub premium: bool,
    /// Champion seat (Live rooms only).
    pub champion: Option<u8>,
    /// Champion streak (Live rooms only).
    pub champion_streak: Option<i32>,
}

/// `JoinWaiting` 3002 request: room number, song state, password.
/// An empty password encodes as one terminator (which the server
/// reads as empty either way).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRoom {
    /// Room number.
    pub number: i32,
    /// Song-state byte.
    pub music_state: u8,
    /// Room password (empty when unlocked).
    pub password: String,
}

impl WireMessage for JoinRoom {
    const COMMAND: u16 = 3002;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&self.number.to_le_bytes());
        body.push(self.music_state);
        body.extend_from_slice(&encode_null_terminated(&self.password, MAX_STRING_LEN));
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 5 {
            return Err(Error::TruncatedMessage {
                need: 5,
                have: body.len(),
            });
        }
        let (password, used) = decode_null_terminated(&body[5..], MAX_STRING_LEN)?;
        let rest = &body[5 + used..];
        if !rest.is_empty() {
            return Err(Error::TrailingBytes(rest.len()));
        }
        Ok(Self {
            number: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            music_state: body[4],
            password,
        })
    }
}

/// `JoinRoom` 3003 response: result plus the room on success (the
/// trailing `RoomInfo` is absent on failure — Encore skips
/// trailing-nullable members at stream end).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRoomResponse {
    /// Raw result code; see [`JoinResult::from_code`].
    pub result_code: u32,
    /// Room snapshot (`None` on failure).
    pub room: Option<JoinedRoom>,
}

impl WireMessage for JoinRoomResponse {
    const COMMAND: u16 = 3003;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&self.result_code.to_le_bytes());
        if let Some(room) = &self.room {
            encode_joined_room(&mut body, room);
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
        let room = if body.len() > 4 {
            Some(decode_joined_room(&body[4..])?)
        } else {
            None
        };
        Ok(Self {
            result_code: u32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            room,
        })
    }
}

///
/// The 7.55 client branches exactly these three values (handler entry
/// file `0x15e120`: compare at files `0x15e166`/`0x15e16f`/`0x15e178`);
/// code 1 has no branch and drops like any other unknown value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RoomSlotAction {
    /// Seat opened.
    SlotUnlocked = 0,
    /// Seat locked.
    SlotLocked = 2,
    /// Occupant kicked.
    PlayerKicked = 3,
}

impl RoomSlotAction {
    /// Interpret a raw action byte. `None` means unknown.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::SlotUnlocked),
            2 => Some(Self::SlotLocked),
            3 => Some(Self::PlayerKicked),
            _ => None,
        }
    }
}

/// `RoomSlotUpdate` 3009 event: one waiting-room seat changed.
///
/// Two bytes: `u8` seat index, then the [`RoomSlotAction`] byte.
/// Memoryer (`Messages/Events/Waiting/RoomSlotUpdateEventData.cs`):
/// byte `Index` (order 0), `RoomSlotActionType` (order 1). Exe-verified
/// handler at file `0x15e120`: two size-1 reads through the
/// packet-reader helper (`0x555d20`, sizes pushed at files
/// `0x15e133`/`0x15e141`), index guard `< 8` (compare at file
/// `0x15e153`, drop past file `0x15e158`), then the 0/2/3 action
/// switch — unlock stamps the seat flag `0`, lock stamps `2`
/// (decrementing the open-seat count at `0x7789f4`), kick refreshes
/// the member table — each arm forwarding `(0x12, 3009, index,
/// action)` into the UI event queue (`0x478d50`, pushes at files
/// `0x15e1aa`/`0x15e1e5`/`0x15e2af`). Observed with GNU objdump 2.47
/// slices plus Python byte scans over the read-only 7.55 copy; no new
/// REA dossier filed. Memoryer shapes are corroboration until the
/// per-field exe builder proof lands (the S→C read side above is the
/// byte-verified part).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomSlotUpdate {
    /// Seat index (`< 8`; the exe drops wider values).
    pub index: u8,
    /// Raw action byte; see [`RoomSlotAction::from_code`].
    pub action_code: u8,
}

/// Body length of [`RoomSlotUpdate`]: 1 + 1.
const ROOM_SLOT_UPDATE_BODY_LEN: usize = 2;

/// Highest seat index the exe handler accepts (`cmp 8`, file
/// `0x15e153`).
const MAX_SLOT_INDEX: u8 = 7;

impl RoomSlotUpdate {
    /// Interpret the action byte. `None` means unknown.
    #[must_use]
    pub fn action(self) -> Option<RoomSlotAction> {
        RoomSlotAction::from_code(self.action_code)
    }
}

impl WireMessage for RoomSlotUpdate {
    const COMMAND: u16 = 3009;

    fn encode_body(&self) -> Vec<u8> {
        vec![self.index, self.action_code]
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < ROOM_SLOT_UPDATE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: ROOM_SLOT_UPDATE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > ROOM_SLOT_UPDATE_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - ROOM_SLOT_UPDATE_BODY_LEN));
        }
        if body[0] > MAX_SLOT_INDEX {
            return Err(Error::InvalidField {
                field: "slot index",
                reason: "not below 8",
            });
        }
        if RoomSlotAction::from_code(body[1]).is_none() {
            return Err(Error::InvalidField {
                field: "slot action",
                reason: "not 0, 2, or 3",
            });
        }
        Ok(Self {
            index: body[0],
            action_code: body[1],
        })
    }
}

#[cfg(test)]
mod slot_update_tests {
    use super::*;
    use crate::command::Event;
    use crate::frame::decode_frame;

    #[test]
    fn lock_encodes_index_then_action() {
        let update = RoomSlotUpdate {
            index: 2,
            action_code: 2,
        };
        assert_eq!(update.encode_body(), vec![0x02, 0x02]);
        assert_eq!(update.action(), Some(RoomSlotAction::SlotLocked));
        let frame = decode_frame(&update.encode()).unwrap();
        assert_eq!(frame.id, 3009);
        assert_eq!(frame.id, Event::RoomSlotUpdate as u16);
        assert_eq!(RoomSlotUpdate::decode_frame(&frame), Ok(update));
    }

    #[test]
    fn unlock_and_kick_name_their_actions() {
        assert_eq!(
            RoomSlotUpdate::decode_body(&[0x00, 0x00]),
            Ok(RoomSlotUpdate {
                index: 0,
                action_code: 0,
            })
        );
        let kick = RoomSlotUpdate::decode_body(&[0x07, 0x03]).unwrap();
        assert_eq!(kick.action(), Some(RoomSlotAction::PlayerKicked));
        assert_eq!(
            RoomSlotAction::from_code(0),
            Some(RoomSlotAction::SlotUnlocked)
        );
        assert_eq!(RoomSlotAction::from_code(1), None);
        assert_eq!(RoomSlotAction::from_code(4), None);
    }

    #[test]
    fn rejects_short_body() {
        assert_eq!(
            RoomSlotUpdate::decode_body(&[0x02]),
            Err(Error::TruncatedMessage { need: 2, have: 1 })
        );
    }

    #[test]
    fn rejects_trailing_bytes() {
        assert_eq!(
            RoomSlotUpdate::decode_body(&[0x02, 0x02, 0x00]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn rejects_wide_index() {
        assert_eq!(
            RoomSlotUpdate::decode_body(&[0x08, 0x02]),
            Err(Error::InvalidField {
                field: "slot index",
                reason: "not below 8"
            })
        );
    }

    #[test]
    fn rejects_unknown_action() {
        assert_eq!(
            RoomSlotUpdate::decode_body(&[0x02, 0x01]),
            Err(Error::InvalidField {
                field: "slot action",
                reason: "not 0, 2, or 3"
            })
        );
    }
}

