//! Room list messages: the 2002 empty request and the
//! 2003 room-list response (Memoryer `RoomListResponse`), plus the
//! room-row mode enums (state, difficulty, speed).

use crate::error::Error;
use crate::frame::encode_frame;
use crate::macros::wire_enum;
use crate::message::WireMessage;
use crate::reader::Reader;
use crate::strings::{encode_null_terminated, MAX_STRING_LEN};
/// Title cap (`StringMessageField(maxLength: 21)`).
pub const ROOM_TITLE_LEN: usize = 21;

/// Room lifecycle state (`Encore.Metadata.RoomState`: 0 defaults to
/// unavailable).
///
/// The 7.55 client keeps the state byte at row `+0x04` of its 0xb4-stride
/// room table. Consumers: the row fill passes the raw byte as the
/// Room_State.ojs image frame (file `0x1268ae`, set at `0x1268b6` — frames
/// 0/1/2 are distinct icons), the builder gates on 1 (files `0x92405`,
/// `0x924b2`, `0x92568` — cite prefix unverified, see
/// `docs/room-enums.md` OPEN), and the row bind zero-tests it (file
/// `0x1251cc`); see `docs/room-enums.md`.
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
///
/// The 7.55 client reads the row `+0x8c` byte as an index into the music
/// record's `u16` level table at `+0x14` (stride 2, files `0x125c52`,
/// `0x125db9`, `0x1261bd`, `0x1265ae`) and into the Room_Speed.ojs icon
/// grid (`byte*12`, file `0x126751`); value 0 has a zero-test with an
/// extra `+0x30` icon-frame term (files `0x126610`, `0x12664e`, reset at
/// `0x12566c`); see `docs/room-enums.md`.
///
/// The selected level word then bands the Room_Om.ojs icon through
/// [`om_icon_frame`].
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

/// Room-list Om icon frame for a song level: the 1-based
/// `Room_Om.ojs` frame the lobby row paints, whose *n*-th frame
/// depicts *n* pudding mascots.
///
/// The 7.55 client's row fill (file `0x125240`) reads the selected
/// song's level word — the `u16` at `music record + 2*difficulty +
/// 0x14` — and passes it to the band mapper (file `0x126cb0`): a level
/// above 12 defaults to frame 4 (compare at `0x126cc7`), otherwise a
/// 13-entry byte table (file `0x126d18`) walks a jump table (file
/// `0x126d0c`) to frames 1–3. The sprite setter (file `0x2f870`)
/// subtracts 1 before its image lookup, and `Room_Om.ojs`
/// (Interface1.opi, 6168 B) carries exactly four 40×19 frames of 1,
/// 2, 3, and 4 puddings — so frame value `n` paints `n` puddings.
/// See `docs/room-enums.md`.
#[must_use]
pub fn om_icon_frame(level: u16) -> u8 {
    match level {
        0..=4 => 1,
        5..=8 => 2,
        9..=12 => 3,
        13.. => 4,
    }
}

wire_enum! {
    /// Scroll speed (`Mozart.Metadata.GameSpeed` as forked by
    /// Memoryer.Data — ordinal, unlike the Mozart_Client build's
    /// shuffled mapping).
    ///
    /// The 7.55 client maps each value 0–11 to an `X …` label in one
    /// function and to consecutive Room_Speed.ojs icon cells (frame =
    /// `speed + 12*difficulty + extra + 1`, files `0x12674a`–`0x12676f`);
    /// see `docs/room-enums.md`.
    pub enum GameSpeed: u8, resolve from_code, {
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

/// Decode one member slot of a joined-room body.
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
    let skill_count = reader.read_count("room skills", true)?;
    let mut skills = Vec::with_capacity(skill_count);
    for _ in 0..skill_count {
        skills.push(reader.read_i32()?);
    }
    let premium = reader.read_short_flag("room premium flag")?;
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

/// 2008 `RoomDescriptorChanged`: rewrite an existing 0xb4-byte row.
///
/// Exe handler at file `0x15bcb0` (VA `0x55bcb0`; REA analysis
/// `fffc.d837…42f9` in `docs/pins/net-b.md`) reads the row index, two
/// null-terminated names, five descriptor bytes, then a 32-bit word.
/// The field names below are neutral because the exe pin names no
/// meaning for the descriptor bytes; their order is the evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDescriptorChanged {
    /// Index of the staged row to rewrite.
    pub number: i32,
    /// First name on the row.
    pub title: String,
    /// First descriptor byte (row `+0x09` in the handler).
    pub descriptor_a: u8,
    /// Second descriptor byte (row `+0x0a`).
    pub descriptor_b: u8,
    /// Third descriptor byte (row `+0x01`).
    pub descriptor_c: u8,
    /// Second name on the row.
    pub secondary_title: String,
    /// Fourth descriptor byte (row `+0x0e`).
    pub descriptor_d: u8,
    /// Fifth descriptor byte (row `+0x0f`).
    pub descriptor_e: u8,
    /// 32-bit descriptor word (row `+0x04`).
    pub descriptor_word: i32,
}

impl WireMessage for RoomDescriptorChanged {
    const COMMAND: u16 = 2008;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = self.number.to_le_bytes().to_vec();
        body.extend_from_slice(&encode_null_terminated(&self.title, MAX_STRING_LEN));
        body.extend_from_slice(&[self.descriptor_a, self.descriptor_b, self.descriptor_c]);
        body.extend_from_slice(&encode_null_terminated(
            &self.secondary_title,
            MAX_STRING_LEN,
        ));
        body.extend_from_slice(&[self.descriptor_d, self.descriptor_e]);
        body.extend_from_slice(&self.descriptor_word.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let number = reader.read_i32()?;
        let title = reader.read_string()?;
        let descriptor_a = reader.read_u8()?;
        let descriptor_b = reader.read_u8()?;
        let descriptor_c = reader.read_u8()?;
        let secondary_title = reader.read_string()?;
        let descriptor_d = reader.read_u8()?;
        let descriptor_e = reader.read_u8()?;
        let descriptor_word = reader.read_i32()?;
        reader.finish()?;
        Ok(Self {
            number,
            title,
            descriptor_a,
            descriptor_b,
            descriptor_c,
            secondary_title,
            descriptor_d,
            descriptor_e,
            descriptor_word,
        })
    }
}

/// 2009 `RoomUserCountChanged`: rewrite the occupancy words of one row.
/// Exe handler file `0x15bdf0`, REA `e766.2b4b…b6d9`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomUserCountChanged {
    /// Index of the staged row.
    pub number: i32,
    /// Seat capacity byte.
    pub capacity: u8,
    /// Seated-player byte.
    pub user_count: u8,
    /// 16-bit occupancy descriptor.
    pub descriptor_word: u16,
    /// Trailing occupancy byte.
    pub descriptor_byte: u8,
}

impl WireMessage for RoomUserCountChanged {
    const COMMAND: u16 = 2009;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = self.number.to_le_bytes().to_vec();
        body.extend_from_slice(&[self.capacity, self.user_count]);
        body.extend_from_slice(&self.descriptor_word.to_le_bytes());
        body.push(self.descriptor_byte);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let number = reader.read_i32()?;
        let capacity = reader.read_u8()?;
        let user_count = reader.read_u8()?;
        let descriptor_word = reader.read_u16()?;
        let descriptor_byte = reader.read_u8()?;
        reader.finish()?;
        Ok(Self {
            number,
            capacity,
            user_count,
            descriptor_word,
            descriptor_byte,
        })
    }
}

/// One named row of the 2011 user list. Exe handler file `0x15bec0`,
/// REA `5b92.288a…6478`: id, two strings, then a 32-bit level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomUserListEntry {
    /// Numeric user id.
    pub id: i32,
    /// Account name.
    pub username: String,
    /// Display name.
    pub nickname: String,
    /// Level word.
    pub level: i32,
}

/// 2011 `RoomUserList`: counted user rows for the Room user sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomUserList {
    /// Users in wire order.
    pub users: Vec<RoomUserListEntry>,
}

impl WireMessage for RoomUserList {
    const COMMAND: u16 = 2011;

    fn encode_body(&self) -> Vec<u8> {
        let count = i32::try_from(self.users.len()).unwrap_or(i32::MAX);
        let mut body = count.to_le_bytes().to_vec();
        for user in &self.users {
            body.extend_from_slice(&user.id.to_le_bytes());
            body.extend_from_slice(&encode_null_terminated(&user.username, MAX_STRING_LEN));
            body.extend_from_slice(&encode_null_terminated(&user.nickname, MAX_STRING_LEN));
            body.extend_from_slice(&user.level.to_le_bytes());
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let count = reader.read_count("room users", true)?;
        let mut users = Vec::with_capacity(count);
        for _ in 0..count {
            users.push(RoomUserListEntry {
                id: reader.read_i32()?,
                username: reader.read_string()?,
                nickname: reader.read_string()?,
                level: reader.read_i32()?,
            });
        }
        reader.finish()?;
        Ok(Self { users })
    }
}

/// 2020 `RoomStateChanged`: one row's state byte. Exe handler file
/// `0x15c600`, REA `deab.f4a7…5704`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomStateChanged {
    /// Index of the staged row.
    pub number: i32,
    /// Raw state code (see [`RoomState::from_code`]).
    pub state_code: u8,
}

impl WireMessage for RoomStateChanged {
    const COMMAND: u16 = 2020;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = self.number.to_le_bytes().to_vec();
        body.push(self.state_code);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let number = reader.read_i32()?;
        let state_code = reader.read_u8()?;
        reader.finish()?;
        Ok(Self { number, state_code })
    }
}

/// 2023 `RoomParameterChanged`: one 16-bit parameter word plus two
/// descriptor bytes. Exe handler file `0x15c790`, REA `dce9.1ea8…7848`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomParameterChanged {
    /// Index of the staged member.
    pub number: i32,
    /// Parameter word: low twelve bits plus a shifted high nibble.
    pub parameter: u16,
    /// First descriptor byte.
    pub descriptor_a: u8,
    /// Second descriptor byte.
    pub descriptor_b: u8,
}

impl WireMessage for RoomParameterChanged {
    const COMMAND: u16 = 2023;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = self.number.to_le_bytes().to_vec();
        body.extend_from_slice(&self.parameter.to_le_bytes());
        body.extend_from_slice(&[self.descriptor_a, self.descriptor_b]);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let number = reader.read_i32()?;
        let parameter = reader.read_u16()?;
        let descriptor_a = reader.read_u8()?;
        let descriptor_b = reader.read_u8()?;
        reader.finish()?;
        Ok(Self {
            number,
            parameter,
            descriptor_a,
            descriptor_b,
        })
    }
}

/// 2025 `RoomSkillChanged`: replace one row's counted skill ids. Exe
/// handler file `0x15c940`, REA `64aa.4e91…7dab`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSkillChanged {
    /// Index of the staged row.
    pub number: i32,
    /// Replacement skill ids.
    pub skills: Vec<i32>,
}

impl WireMessage for RoomSkillChanged {
    const COMMAND: u16 = 2025;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = self.number.to_le_bytes().to_vec();
        let count = i32::try_from(self.skills.len()).unwrap_or(i32::MAX);
        body.extend_from_slice(&count.to_le_bytes());
        for skill in &self.skills {
            body.extend_from_slice(&skill.to_le_bytes());
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(body);
        let number = reader.read_i32()?;
        let count = reader.read_count("room skills", true)?;
        let mut skills = Vec::with_capacity(count);
        for _ in 0..count {
            skills.push(reader.read_i32()?);
        }
        reader.finish()?;
        Ok(Self { number, skills })
    }
}

#[cfg(test)]
mod room_list_tests {
    use super::*;
    use crate::frame::decode_frame;
    use crate::message::{GameMode, KeyMode};
    use crate::rooms::{
        JoinResult, JoinRoom, JoinRoomResponse, JoinedRoom, RoomMember, RoomSlot, RoomSlotState,
    };

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
    fn room_state_names_each_wire_value() {
        // Icon frames render the raw byte (file 0x968ae), so the
        // discriminants must equal the wire values one to one.
        assert_eq!(RoomState::Unavailable as u8, 0);
        assert_eq!(RoomState::Waiting as u8, 1);
        assert_eq!(RoomState::Playing as u8, 2);
        assert_eq!(RoomState::from_code(0), Some(RoomState::Unavailable));
        assert_eq!(RoomState::from_code(1), Some(RoomState::Waiting));
        assert_eq!(RoomState::from_code(2), Some(RoomState::Playing));
        assert_eq!(RoomState::from_code(3), None);
        assert_eq!(RoomState::from_code(0xFF), None);
    }

    #[test]
    fn difficulty_names_each_wire_value() {
        // The exe indexes the level table with the raw byte (files
        // 0x125c52 et al.), so ordinals 0..3 must stay contiguous.
        for (code, name) in [
            (0, Difficulty::Ex),
            (1, Difficulty::Nx),
            (2, Difficulty::Hx),
            (3, Difficulty::Rx),
        ] {
            assert_eq!(name as u8, code);
            assert_eq!(Difficulty::from_code(code), Some(name));
        }
        assert_eq!(Difficulty::from_code(4), None);
        assert_eq!(Difficulty::from_code(0xFF), None);
    }

    #[test]
    fn game_speed_names_each_wire_value() {
        // Icon cells follow label order (file 0x96751: twelve cells per
        // difficulty row), so each discriminant must equal its code.
        for (code, name) in [
            (0, GameSpeed::X05),
            (1, GameSpeed::X10),
            (2, GameSpeed::X15),
            (3, GameSpeed::X20),
            (4, GameSpeed::X25),
            (5, GameSpeed::X30),
            (6, GameSpeed::X35),
            (7, GameSpeed::X40),
            (8, GameSpeed::X45),
            (9, GameSpeed::X50),
            (10, GameSpeed::X60),
            (11, GameSpeed::Xr),
        ] {
            assert_eq!(name as u8, code);
            assert_eq!(GameSpeed::from_code(code), Some(name));
        }
        assert_eq!(GameSpeed::from_code(12), None);
        assert_eq!(GameSpeed::from_code(0xFF), None);
    }

    #[test]
    fn game_speed_codes_round_trip() {
        let all = [
            GameSpeed::X05,
            GameSpeed::X10,
            GameSpeed::X15,
            GameSpeed::X20,
            GameSpeed::X25,
            GameSpeed::X30,
            GameSpeed::X35,
            GameSpeed::X40,
            GameSpeed::X45,
            GameSpeed::X50,
            GameSpeed::X60,
            GameSpeed::Xr,
        ];
        for speed in all {
            assert_eq!(GameSpeed::from_code(speed as u8), Some(speed));
        }
    }

    #[test]
    fn om_icon_frame_one_covers_levels_0_to_4() {
        // Band table entries for levels 0..4 are all 0 (file
        // 0x126d18), and jump-table slot 0 returns frame 1.
        for level in 0..=4_u16 {
            assert_eq!(om_icon_frame(level), 1);
        }
    }

    #[test]
    fn om_icon_frame_two_starts_at_level_5() {
        // The 4→5 cut and the whole 5..8 run (table bytes 0x126d1d..).
        assert_eq!(om_icon_frame(4), 1);
        for level in 5..=8_u16 {
            assert_eq!(om_icon_frame(level), 2);
        }
    }

    #[test]
    fn om_icon_frame_three_starts_at_level_9() {
        // The 8→9 cut and the whole 9..12 run; 12 is the last table
        // index before the mapper's >12 default.
        assert_eq!(om_icon_frame(8), 2);
        for level in 9..=12_u16 {
            assert_eq!(om_icon_frame(level), 3);
        }
    }

    #[test]
    fn om_icon_frame_four_starts_at_level_13() {
        // The 12→13 cut rides the mapper's bound (compare at file
        // 0x126cc7), so every level past the 13-entry table lands on 4.
        assert_eq!(om_icon_frame(12), 3);
        for level in [13, 14, 27, 100, u16::MAX] {
            assert_eq!(om_icon_frame(level), 4);
        }
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
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
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

    /// seam: pure 2003 row codec with no runtime caller beyond the
    /// decode seam (`docs/live-path-reachability.md`).
    #[test]
    fn room_2003_row_stages_occupancy_limits_and_skills_in_exe_field_order() {
        let expected = lobby();
        let decoded = RoomList::decode_body(
            &RoomList {
                rooms: vec![expected.clone()],
            }
            .encode_body(),
        )
        .unwrap();
        let staged = &decoded.rooms[0];
        assert_eq!(staged.number, 3);
        assert_eq!(staged.state_code, 1);
        assert_eq!(staged.title, " Friday Jam ");
        assert!(staged.has_password);
        assert_eq!(staged.music_id, 757);
        assert_eq!(staged.capacity, 8);
        assert_eq!(staged.user_count, 5);
        assert_eq!((staged.min_level, staged.max_level), (0, 10));
        assert_eq!(staged.skills, vec![11, 22]);
        assert_eq!(staged.room_type, 0);
        assert_eq!(*staged, expected);
    }

    /// seam: pure 2008 codec; the field order is the pinned evidence,
    /// and no caller exists beyond this decode seam yet.
    #[test]
    fn room_2008_descriptor_change_decodes_the_exe_field_sequence() {
        assert_eq!(RoomDescriptorChanged::COMMAND, 2008);
        let mut body = 4i32.to_le_bytes().to_vec();
        body.extend_from_slice(&encode_null_terminated("Jam", MAX_STRING_LEN));
        body.extend_from_slice(&[2, 3, 4]);
        body.extend_from_slice(&encode_null_terminated("Stage", MAX_STRING_LEN));
        body.extend_from_slice(&[5, 6]);
        body.extend_from_slice(&0x1234_5678i32.to_le_bytes());
        let decoded = RoomDescriptorChanged::decode_body(&body).unwrap();
        assert_eq!(decoded.number, 4);
        assert_eq!(decoded.title, "Jam");
        assert_eq!(
            (
                decoded.descriptor_a,
                decoded.descriptor_b,
                decoded.descriptor_c
            ),
            (2, 3, 4)
        );
        assert_eq!(decoded.secondary_title, "Stage");
        assert_eq!((decoded.descriptor_d, decoded.descriptor_e), (5, 6));
        assert_eq!(decoded.descriptor_word, 0x1234_5678);
    }

    /// seam: pure 2009 codec with no runtime caller beyond the seam.
    #[test]
    fn room_2009_user_count_decodes_index_then_occupancy_fields() {
        assert_eq!(RoomUserCountChanged::COMMAND, 2009);
        let mut body = 3i32.to_le_bytes().to_vec();
        body.extend_from_slice(&[8, 5]);
        body.extend_from_slice(&0x0102u16.to_le_bytes());
        body.push(7);
        let decoded = RoomUserCountChanged::decode_body(&body).unwrap();
        assert_eq!(decoded.number, 3);
        assert_eq!((decoded.capacity, decoded.user_count), (8, 5));
        assert_eq!(decoded.descriptor_word, 0x0102);
        assert_eq!(decoded.descriptor_byte, 7);
    }

    /// seam: pure 2011 codec with no runtime caller beyond the seam.
    #[test]
    fn room_2011_user_list_decodes_named_counted_rows() {
        assert_eq!(RoomUserList::COMMAND, 2011);
        let mut body = 2i32.to_le_bytes().to_vec();
        for (id, username, nickname, level) in [
            (1i32, "host", "Host", 12i32),
            (2i32, "guest", "Guest", 3i32),
        ] {
            body.extend_from_slice(&id.to_le_bytes());
            body.extend_from_slice(&encode_null_terminated(username, MAX_STRING_LEN));
            body.extend_from_slice(&encode_null_terminated(nickname, MAX_STRING_LEN));
            body.extend_from_slice(&level.to_le_bytes());
        }
        let decoded = RoomUserList::decode_body(&body).unwrap();
        assert_eq!(decoded.users.len(), 2);
        assert_eq!(decoded.users[0].nickname, "Host");
        assert_eq!(decoded.users[1].username, "guest");
        assert_eq!(decoded.users[1].level, 3);
    }

    /// seam: pure 2020 codec with no runtime caller beyond the seam.
    #[test]
    fn room_2020_state_change_decodes_index_then_state_byte() {
        assert_eq!(RoomStateChanged::COMMAND, 2020);
        let mut body = 3i32.to_le_bytes().to_vec();
        body.push(2);
        let decoded = RoomStateChanged::decode_body(&body).unwrap();
        assert_eq!(
            decoded,
            RoomStateChanged {
                number: 3,
                state_code: 2
            }
        );
    }

    /// seam: pure 2023 codec with no runtime caller beyond the seam.
    #[test]
    fn room_2023_parameter_change_decodes_word_and_two_bytes() {
        assert_eq!(RoomParameterChanged::COMMAND, 2023);
        let mut body = 3i32.to_le_bytes().to_vec();
        body.extend_from_slice(&0xF123u16.to_le_bytes());
        body.extend_from_slice(&[4, 5]);
        let decoded = RoomParameterChanged::decode_body(&body).unwrap();
        assert_eq!(decoded.number, 3);
        assert_eq!(decoded.parameter, 0xF123);
        assert_eq!((decoded.descriptor_a, decoded.descriptor_b), (4, 5));
    }

    /// seam: pure 2025 codec with no runtime caller beyond the seam.
    #[test]
    fn room_2025_skill_change_decodes_indexed_skill_ids() {
        assert_eq!(RoomSkillChanged::COMMAND, 2025);
        let mut body = 3i32.to_le_bytes().to_vec();
        body.extend_from_slice(&2i32.to_le_bytes());
        for skill in [11i32, 22i32] {
            body.extend_from_slice(&skill.to_le_bytes());
        }
        let decoded = RoomSkillChanged::decode_body(&body).unwrap();
        assert_eq!(decoded.number, 3);
        assert_eq!(decoded.skills, vec![11, 22]);
    }
}
