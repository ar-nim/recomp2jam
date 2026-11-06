//! Room list messages: `GetChannelInfo` 2002 (empty) and the
//! `RoomList` 2003 response (Memoryer `RoomListResponse`, sent with
//! one entry per channel slot, `Number` 0 when empty).
//!
//! All mode enums travel as one byte; structs keep raw codes with
//! [`from_code`](RoomState::from_code)-style converters so unknown
//! values never break decoding.

use super::error::Error;
use super::frame::encode_frame;
use super::message::WireMessage;
use super::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};

/// Title cap (`StringMessageField(maxLength: 21)`).
pub const ROOM_TITLE_LEN: usize = 21;

/// Room lifecycle state (`Encore.Metadata.RoomState`: 0 defaults to
/// unavailable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RoomState {
    /// Slot empty / unavailable.
    Unavailable = 0,
    /// Waiting for players.
    Waiting = 1,
    /// Game in progress.
    Playing = 2,
}

impl RoomState {
    /// Interpret a raw state byte. `None` means unknown.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Unavailable),
            1 => Some(Self::Waiting),
            2 => Some(Self::Playing),
            _ => None,
        }
    }
}

/// Chart difficulty (`Encore.Metadata.Difficulty`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Difficulty {
    /// Easy.
    Ex = 0,
    /// Normal.
    Nx = 1,
    /// Hard.
    Hx = 2,
    /// Remix.
    Rx = 3,
}

impl Difficulty {
    /// Interpret a raw difficulty byte. `None` means unknown.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Ex),
            1 => Some(Self::Nx),
            2 => Some(Self::Hx),
            3 => Some(Self::Rx),
            _ => None,
        }
    }
}

/// Scroll speed (`Mozart.Metadata.GameSpeed` as forked by
/// Memoryer.Data — ordinal, unlike the Mozart_Client build's
/// shuffled mapping).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameSpeed {
    /// Half speed.
    X05 = 0,
    /// Normal speed.
    X10 = 1,
    /// 1.5x.
    X15 = 2,
    /// Double.
    X20 = 3,
    /// 2.5x.
    X25 = 4,
    /// Triple.
    X30 = 5,
    /// 3.5x.
    X35 = 6,
    /// 4x.
    X40 = 7,
    /// 4.5x.
    X45 = 8,
    /// 5x.
    X50 = 9,
    /// 6x.
    X60 = 10,
    /// Random.
    Xr = 11,
}

impl GameSpeed {
    /// Interpret a raw speed byte. `None` means unknown.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::X05),
            1 => Some(Self::X10),
            2 => Some(Self::X15),
            3 => Some(Self::X20),
            4 => Some(Self::X25),
            5 => Some(Self::X30),
            6 => Some(Self::X35),
            7 => Some(Self::X40),
            8 => Some(Self::X45),
            9 => Some(Self::X50),
            10 => Some(Self::X60),
            11 => Some(Self::Xr),
            _ => None,
        }
    }
}

/// One room row: fixed stats, capped title, counted skill list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomListEntry {
    /// Room number (0 = empty slot).
    pub number: i32,
    /// Lifecycle code; see [`RoomState::from_code`].
    pub state_code: u8,
    /// Room title (capped at 21 chars on the wire).
    pub title: String,
    /// Password required.
    pub has_password: bool,
    /// Selected song id.
    pub music_id: u16,
    /// Difficulty code; see [`Difficulty::from_code`].
    pub difficulty: u8,
    /// Key-mode code (`KeyMode::from_u8`).
    pub key_mode: u8,
    /// Game-mode code (`GameMode::from_u8`; fork mapping).
    pub game_mode: u8,
    /// Speed code; see [`GameSpeed::from_code`].
    pub speed: u8,
    /// Seat capacity.
    pub capacity: u8,
    /// Seated players.
    pub user_count: u8,
    /// Minimum level gate.
    pub min_level: u8,
    /// Maximum level gate.
    pub max_level: u8,
    /// Active skill-ring ids.
    pub skills: Vec<i32>,
    /// Premium-only room.
    pub premium: bool,
    /// Room type byte.
    pub room_type: u8,
}

/// Decode one `0`/`1` byte as bool.
fn decode_flag(body: &[u8], field: &'static str) -> Result<bool, Error> {
    match body {
        [0] => Ok(false),
        [1] => Ok(true),
        _ => Err(Error::InvalidField {
            field,
            reason: "not 0 or 1",
        }),
    }
}

/// Cursor over a room-list body; every read bounds-checks.
struct Reader<'body> {
    body: &'body [u8],
    pos: usize,
}

impl<'body> Reader<'body> {
    fn new(body: &'body [u8]) -> Self {
        Self { body, pos: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'body [u8], Error> {
        if self.body.len() - self.pos < count {
            return Err(Error::TruncatedMessage {
                need: self.pos + count,
                have: self.body.len(),
            });
        }
        let bytes = &self.body[self.pos..self.pos + count];
        self.pos += count;
        Ok(bytes)
    }

    fn read_u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, Error> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_i32(&mut self) -> Result<i32, Error> {
        let bytes = self.take(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u32(&mut self) -> Result<u32, Error> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_string(&mut self) -> Result<String, Error> {
        let (text, used) = decode_null_terminated(&self.body[self.pos..], MAX_STRING_LEN)?;
        self.pos += used;
        Ok(text)
    }

    fn read_flag(&mut self, field: &'static str) -> Result<bool, Error> {
        decode_flag(self.take(1)?, field)
    }

    fn read_title(&mut self) -> Result<String, Error> {
        let (text, used) = decode_null_terminated(&self.body[self.pos..], ROOM_TITLE_LEN)?;
        self.pos += used;
        Ok(text)
    }

    fn finish(&self) -> Result<(), Error> {
        if self.pos != self.body.len() {
            return Err(Error::TrailingBytes(self.body.len() - self.pos));
        }
        Ok(())
    }

    fn remaining(&self) -> usize {
        self.body.len() - self.pos
    }
}

/// `i32` 0/1 as bool.
fn read_int_flag(reader: &mut Reader<'_>, field: &'static str) -> Result<bool, Error> {
    match reader.read_i32()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::InvalidField {
            field,
            reason: "not 0 or 1",
        }),
    }
}

/// `i16` 0/1 as bool.
fn read_short_flag(reader: &mut Reader<'_>, field: &'static str) -> Result<bool, Error> {
    let bytes = reader.take(2)?;
    match i16::from_le_bytes([bytes[0], bytes[1]]) {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::InvalidField {
            field,
            reason: "not 0 or 1",
        }),
    }
}

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
    let music_count = reader.read_i32()?;
    if music_count < 0 {
        return Err(Error::InvalidField {
            field: "member music ids",
            reason: "negative count",
        });
    }
    let mut music_ids = Vec::with_capacity(music_count as usize);
    for _ in 0..music_count {
        music_ids.push(reader.read_u16()?);
    }
    let cash_point = reader.read_i32()?;
    let free_pass = reader.read_u32()?;
    let playing_state = reader.read_u8()?;
    let administrator = reader.read_flag("member administrator flag")?;
    let super_manager = read_int_flag(reader, "member super-manager flag")?;
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
    let has_super_manager = read_int_flag(&mut reader, "room super-manager flag")?;
    let user_count = reader.read_i32()?;
    let mut slots = Vec::with_capacity(7);
    for _ in 0..7 {
        slots.push(decode_room_slot(&mut reader)?);
    }
    let skill_count = reader.read_i32()?;
    if skill_count < 0 {
        return Err(Error::InvalidField {
            field: "joined-room skills",
            reason: "negative count",
        });
    }
    let mut skills = Vec::with_capacity(skill_count as usize);
    for _ in 0..skill_count {
        skills.push(reader.read_i32()?);
    }
    let premium = read_short_flag(&mut reader, "joined-room premium flag")?;
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

fn read_room_entry(reader: &mut Reader<'_>) -> Result<RoomListEntry, Error> {
    let number = reader.read_i32()?;
    let state_code = reader.read_u8()?;
    let title = reader.read_title()?;
    let has_password = reader.read_flag("room password flag")?;
    let music_id = reader.read_u16()?;
    let difficulty = reader.read_u8()?;
    let key_mode = reader.read_u8()?;
    let game_mode = reader.read_u8()?;
    let speed = reader.read_u8()?;
    let capacity = reader.read_u8()?;
    let user_count = reader.read_u8()?;
    let min_level = reader.read_u8()?;
    let max_level = reader.read_u8()?;
    let skill_count = reader.read_i32()?;
    if skill_count < 0 {
        return Err(Error::InvalidField {
            field: "room skills",
            reason: "negative count",
        });
    }
    let mut skills = Vec::with_capacity(skill_count as usize);
    for _ in 0..skill_count {
        skills.push(reader.read_i32()?);
    }
    let premium = match reader.read_u16()? {
        0 => false,
        1 => true,
        _ => {
            return Err(Error::InvalidField {
                field: "room premium flag",
                reason: "not 0 or 1",
            });
        }
    };
    let room_type = reader.read_u8()?;
    Ok(RoomListEntry {
        number,
        state_code,
        title,
        has_password,
        music_id,
        difficulty,
        key_mode,
        game_mode,
        speed,
        capacity,
        user_count,
        min_level,
        max_level,
        skills,
        premium,
        room_type,
    })
}

fn push_room_entry(body: &mut Vec<u8>, room: &RoomListEntry) {
    body.extend_from_slice(&room.number.to_le_bytes());
    body.push(room.state_code);
    body.extend_from_slice(&encode_null_terminated(&room.title, ROOM_TITLE_LEN));
    body.push(u8::from(room.has_password));
    body.extend_from_slice(&room.music_id.to_le_bytes());
    for byte in [
        room.difficulty,
        room.key_mode,
        room.game_mode,
        room.speed,
        room.capacity,
        room.user_count,
        room.min_level,
        room.max_level,
    ] {
        body.push(byte);
    }
    body.extend_from_slice(&(room.skills.len() as i32).to_le_bytes());
    for skill in &room.skills {
        body.extend_from_slice(&skill.to_le_bytes());
    }
    body.extend_from_slice(&u16::from(room.premium).to_le_bytes());
    body.push(room.room_type);
}

/// `RoomList` 2003 response: `i32` count + room rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomList {
    /// Rooms in slot order.
    pub rooms: Vec<RoomListEntry>,
}

impl WireMessage for RoomList {
    const COMMAND: u16 = 2003;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(self.rooms.len() as i32).to_le_bytes());
        for room in &self.rooms {
            push_room_entry(&mut body, room);
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let count = reader.read_i32()?;
        if count < 0 {
            return Err(Error::InvalidField {
                field: "room count",
                reason: "negative",
            });
        }
        let mut rooms = Vec::with_capacity(count as usize);
        for _ in 0..count {
            rooms.push(read_room_entry(&mut reader)?);
        }
        reader.finish()?;
        Ok(Self { rooms })
    }
}

/// Full frame bytes ready to send (test fake servers use this).
#[must_use]
pub fn encode_frame_for(list: &RoomList) -> Vec<u8> {
    encode_frame(RoomList::COMMAND, &list.encode_body())
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

#[cfg(test)]
mod room_list_tests {
    use super::*;
    use crate::frame::decode_frame;
    use crate::message::{GameMode, KeyMode};

    fn lobby() -> RoomListEntry {
        RoomListEntry {
            number: 3,
            state_code: 1,
            title: " Friday Jam ".to_owned(),
            has_password: true,
            music_id: 757,
            difficulty: 2,
            key_mode: 2,
            game_mode: 1,
            speed: 0,
            capacity: 8,
            user_count: 5,
            min_level: 0,
            max_level: 10,
            skills: vec![11, 22],
            premium: false,
            room_type: 0,
        }
    }

    #[test]
    fn room_round_trips_through_frame() {
        let list = RoomList {
            rooms: vec![lobby()],
        };
        let frame = decode_frame(&list.encode()).unwrap();
        assert_eq!(frame.id, 2003);
        let back = RoomList::decode_frame(&frame).unwrap();
        assert_eq!(back, list);
        assert_eq!(
            RoomState::from_code(back.rooms[0].state_code),
            Some(RoomState::Waiting)
        );
        assert_eq!(
            Difficulty::from_code(back.rooms[0].difficulty),
            Some(Difficulty::Hx)
        );
        assert_eq!(
            KeyMode::from_u8(back.rooms[0].key_mode),
            Ok(KeyMode::SevenKeys)
        );
        assert_eq!(
            GameMode::from_u8(back.rooms[0].game_mode),
            Ok(GameMode::Live)
        );
        assert_eq!(
            GameSpeed::from_code(back.rooms[0].speed),
            Some(GameSpeed::X05)
        );
    }

    #[test]
    fn room_names_all_mode_codes() {
        assert_eq!(RoomState::from_code(0), Some(RoomState::Unavailable));
        assert_eq!(RoomState::from_code(9), None);
        assert_eq!(Difficulty::from_code(3), Some(Difficulty::Rx));
        assert_eq!(KeyMode::from_u8(0), Ok(KeyMode::ThreeKeys));
        assert_eq!(GameMode::from_u8(1), Ok(GameMode::Live));
        assert_eq!(GameSpeed::from_code(0), Some(GameSpeed::X05));
        assert_eq!(GameSpeed::from_code(1), Some(GameSpeed::X10));
        assert_eq!(GameSpeed::from_code(5), Some(GameSpeed::X30));
        assert_eq!(GameSpeed::from_code(12), None);
    }

    #[test]
    fn title_truncates_to_cap() {
        let mut room = lobby();
        room.title = "x".repeat(40);
        let back = RoomList::decode_body(&RoomList { rooms: vec![room] }.encode_body()).unwrap();
        assert_eq!(back.rooms[0].title.len(), ROOM_TITLE_LEN - 1);
    }

    #[test]
    fn empty_list_decodes() {
        let list = RoomList::decode_body(&[0x00, 0x00, 0x00, 0x00]).unwrap();
        assert!(list.rooms.is_empty());
    }

    #[test]
    fn rejects_negative_room_count() {
        assert_eq!(
            RoomList::decode_body(&[0xFF, 0xFF, 0xFF, 0xFF]),
            Err(Error::InvalidField {
                field: "room count",
                reason: "negative"
            })
        );
    }

    fn member() -> RoomMember {
        RoomMember {
            nickname: "host".to_owned(),
            level: 12,
            gender: 1,
            gem: 30,
            master: true,
            team: 0,
            ready: true,
            music_state: 0,
            equipment: [0; 17],
            music_ids: vec![757],
            cash_point: 40,
            free_pass: 0,
            playing_state: 2,
            administrator: false,
            super_manager: false,
        }
    }

    fn joined() -> JoinedRoom {
        let mut slots = vec![
            RoomSlot {
                index: 0,
                state_code: 1,
                member: Some(member()),
            },
            RoomSlot {
                index: 1,
                state_code: 0,
                member: None,
            },
            RoomSlot {
                index: 2,
                state_code: 2,
                member: None,
            },
        ];
        for index in 3..7u8 {
            slots.push(RoomSlot {
                index,
                state_code: 0,
                member: None,
            });
        }
        JoinedRoom {
            member_id: 1,
            team: 0,
            title: "Evening".to_owned(),
            music_id: 757,
            arena: 0,
            key_mode: 2,
            game_mode: 0,
            difficulty: 2,
            speed: 1,
            has_super_manager: false,
            user_count: 1,
            slots,
            skills: vec![5],
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    #[test]
    fn join_request_round_trips() {
        let request = JoinRoom {
            number: 3,
            music_state: 0,
            password: String::new(),
        };
        let frame = decode_frame(&request.encode()).unwrap();
        assert_eq!(frame.id, 3002);
        assert_eq!(JoinRoom::decode_frame(&frame), Ok(request));
    }

    #[test]
    fn join_names_all_results() {
        assert_eq!(JoinResult::from_code(0), Some(JoinResult::Success));
        assert_eq!(
            JoinResult::from_code(0xFFFF_FFFD),
            Some(JoinResult::InvalidPassword)
        );
        assert_eq!(JoinResult::from_code(0xFFFF_FFFA), Some(JoinResult::Full));
        assert_eq!(
            JoinResult::from_code(0xFFFF_FFF8),
            Some(JoinResult::InvalidNumber)
        );
        assert_eq!(JoinResult::from_code(77), None);
        assert_eq!(RoomSlotState::from_code(1), Some(RoomSlotState::Occupied));
        assert_eq!(RoomSlotState::from_code(9), None);
    }

    #[test]
    fn join_response_round_trips_with_mixed_slots() {
        let response = JoinRoomResponse {
            result_code: 0,
            room: Some(joined()),
        };
        let frame = decode_frame(&response.encode()).unwrap();
        assert_eq!(frame.id, 3003);
        let back = JoinRoomResponse::decode_frame(&frame).unwrap();
        assert_eq!(back, response);
        assert_eq!(back.room.as_ref().unwrap().slots.len(), 7);
        assert_eq!(
            back.room.as_ref().unwrap().slots[0]
                .member
                .as_ref()
                .unwrap()
                .nickname,
            "host"
        );
        assert!(back.room.as_ref().unwrap().slots[1].member.is_none());
    }

    #[test]
    fn join_failure_carries_no_room() {
        let response = JoinRoomResponse::decode_body(&[0xFA, 0xFF, 0xFF, 0xFF]).unwrap();
        assert_eq!(response.result_code, 0xFFFF_FFFA);
        assert_eq!(
            JoinResult::from_code(response.result_code),
            Some(JoinResult::Full)
        );
        assert_eq!(response.room, None);
    }

    #[test]
    fn join_response_rejects_short_body() {
        assert_eq!(
            JoinRoomResponse::decode_body(&[0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 2 })
        );
    }

    #[test]
    fn rejects_bad_password_flag() {
        let room = lobby();
        let mut body = vec![0x01, 0x00, 0x00, 0x00];
        push_room_entry(&mut body, &room);
        // Password flag sits after number + state + title + terminator.
        let at = 4 + 4 + 1 + room.title.len() + 1;
        body[at] = 0x02;
        assert_eq!(
            RoomList::decode_body(&body),
            Err(Error::InvalidField {
                field: "room password flag",
                reason: "not 0 or 1"
            })
        );
    }
}
