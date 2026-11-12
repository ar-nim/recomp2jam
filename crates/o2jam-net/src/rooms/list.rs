//! Room list messages: the 2002 empty request and the
//! 2003 room-list response (Memoryer `RoomListResponse`), plus the
//! room-row mode enums (state, difficulty, speed).

use crate::error::Error;
use crate::frame::encode_frame;
use crate::message::WireMessage;
use crate::reader::Reader;
use crate::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};
use crate::macros::wire_enum;
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

