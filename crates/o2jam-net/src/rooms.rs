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
use super::strings::{decode_null_terminated, encode_null_terminated};

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
