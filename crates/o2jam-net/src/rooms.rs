//! Room list messages: `GetChannelInfo` 2002 (empty) and the
//! `RoomList` 2003 response (Memoryer `RoomListResponse`, sent with
//! one entry per channel slot, `Number` 0 when empty).
//!
//! All mode enums travel as one byte; structs keep raw codes with
//! [`from_code`](RoomState::from_code)-style converters so unknown
//! values never break decoding.

use std::borrow::Cow;

use super::error::Error;
use super::frame::encode_frame;
use super::message::WireMessage;
use super::reader::Reader;
use super::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};

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

/// Scroll speed (`Mozart.Metadata.GameSpeed` as forked by
/// Memoryer.Data — ordinal, unlike the Mozart_Client build's
/// shuffled mapping).
///
/// The 7.55 client maps each value 0–11 to an `X …` label in one
/// function and to consecutive Room_Speed.ojs icon cells (frame =
/// `speed + 12*difficulty + extra + 1`, files `0x12674a`–`0x12676f`);
/// see `docs/room-enums.md`.
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

/// Decode one member slot of a joined-room body.
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

/// `RankNotification` 2029 event: a rank-up notice for the lobby
/// displays. Memoryer (`Messages/Events/MainRoom/
/// `RankNotificationEventData.cs`): bool `Invalid` as little-endian
/// `i32` (order 0, nonzero reads true), `i32` music id (order 1),
/// string nickname (order 2), `i32` ranking (order 4 — order 3 was
/// dropped upstream, so the wire order is invalid, music, name,
/// rank). Exe receive: dispatch index `0x1a` (2029) lands case file
/// `0x158556`, which calls handler VA `0x55b4d0` (file `0x15b4d0`).
/// The handler reads the invalid word through the packet-reader
/// helper (`0x555d20`) and drops the packet when nonzero (`cmp` at
/// file `0x15b527`); else it stages music id, nickname, and ranking
/// through `.data` slots `0x77969c`/`0x7796a8`/`0x7796a0` and forwards
/// `(0, 0, 0x7ed, 0x12)` into the ControlList UI queue (`0x478d50`,
/// file `0x15b575`). Observed with GNU objdump 2.47 slices plus
/// Python byte scans over the read-only 7.55 copy; no new REA dossier
/// filed — the field order is Memoryer corroboration against an
/// exe-verified read sequence. (2015 C→S has no client handler: the
/// same table maps index `0x0c` to the default case file `0x158937`.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankNotification {
    /// True when the notice carries no rank (client drops it).
    pub invalid: bool,
    /// Ranked song id.
    pub music_id: i32,
    /// Ranked player nickname.
    pub nickname: String,
    /// New rank value.
    pub ranking: i32,
}

impl WireMessage for RankNotification {
    const COMMAND: u16 = 2029;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + 4 + self.nickname.len() + 1 + 4);
        body.extend_from_slice(&i32::from(self.invalid).to_le_bytes());
        body.extend_from_slice(&self.music_id.to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.nickname, MAX_STRING_LEN));
        body.extend_from_slice(&self.ranking.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 8 {
            return Err(Error::TruncatedMessage {
                need: 8,
                have: body.len(),
            });
        }
        let invalid = i32::from_le_bytes([body[0], body[1], body[2], body[3]]) != 0;
        let music_id = i32::from_le_bytes([body[4], body[5], body[6], body[7]]);
        let (nickname, used) = decode_null_terminated(&body[8..], MAX_STRING_LEN)?;
        let tail = &body[8 + used..];
        if tail.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 8 + used + 4,
                have: body.len(),
            });
        }
        if tail.len() > 4 {
            return Err(Error::TrailingBytes(tail.len() - 4));
        }
        let ranking = i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
        Ok(Self {
            invalid,
            music_id,
            nickname,
            ranking,
        })
    }
}

/// Rank movement arrow on a `MusicPlayRankingAnswer` row
/// (`Mozart.Metadata.RankDeltaType`, byte-backed: `Up = 0x00`,
/// `Down = 0x01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RankDelta {
    /// Rank improved (or unchanged) since last time.
    Up = 0,
    /// Rank fell.
    Down = 1,
}

impl RankDelta {
    /// Interpret a raw delta byte. `None` means unknown.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Up),
            1 => Some(Self::Down),
            _ => None,
        }
    }
}

/// One row of the `MusicPlayRankingAnswer` 2028 answer: a ranked
/// player's lanking line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankEntry {
    /// Leaderboard place (`0` marks a blank row: the exe pushes a
    /// zeroed row and reads no further fields for it).
    pub rank: i32,
    /// Ranked player nickname (UTF-8, null-terminated on the wire).
    pub nickname: String,
    /// Battles fought.
    pub battles: i32,
    /// Battles won.
    pub wins: i32,
    /// Win rate (Memoryer computes `win / battle`, truncated).
    pub win_rate: i32,
    /// Rank movement since last time.
    pub delta: RankDelta,
    /// Rank movement magnitude.
    pub rank_delta: i32,
}

impl Default for RankEntry {
    fn default() -> Self {
        Self {
            rank: 0,
            nickname: String::new(),
            battles: 0,
            wins: 0,
            win_rate: 0,
            delta: RankDelta::Up,
            rank_delta: 0,
        }
    }
}

/// Wire width of one [`RankEntry`] past a nonzero rank word:
/// null-terminated nickname + three `i32` + `u8` delta + `i32`.
fn encode_rank_entry(entry: &RankEntry) -> Vec<u8> {
    let mut body = entry.rank.to_le_bytes().to_vec();
    if entry.rank != 0 {
        body.extend_from_slice(&encode_null_terminated(&entry.nickname, MAX_STRING_LEN));
        body.extend_from_slice(&entry.battles.to_le_bytes());
        body.extend_from_slice(&entry.wins.to_le_bytes());
        body.extend_from_slice(&entry.win_rate.to_le_bytes());
        body.push(entry.delta as u8);
        body.extend_from_slice(&entry.rank_delta.to_le_bytes());
    }
    body
}

/// Cursor over a 2028 body; every read bounds-checks. Returns the
/// entry plus bytes consumed (`rank == 0` rows carry no further
/// fields, matching the exe's zeroed-row arm).
fn decode_rank_entry(body: &[u8]) -> Result<(RankEntry, usize), Error> {
    if body.len() < 4 {
        return Err(Error::TruncatedMessage {
            need: 4,
            have: body.len(),
        });
    }
    let rank = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
    if rank == 0 {
        return Ok((RankEntry::default(), 4));
    }
    let (nickname, name_used) = decode_null_terminated(&body[4..], MAX_STRING_LEN)?;
    let tail = &body[4 + name_used..];
    if tail.len() < 13 {
        return Err(Error::TruncatedMessage {
            need: 4 + name_used + 13,
            have: body.len(),
        });
    }
    let delta = RankDelta::from_code(tail[12]).ok_or(Error::InvalidField {
        field: "rank delta type",
        reason: "not 0 (Up) or 1 (Down)",
    })?;
    Ok((
        RankEntry {
            rank,
            nickname,
            battles: i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]),
            wins: i32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]),
            win_rate: i32::from_le_bytes([tail[8], tail[9], tail[10], tail[11]]),
            delta,
            rank_delta: i32::from_le_bytes([tail[13], tail[14], tail[15], tail[16]]),
        },
        4 + name_used + 17,
    ))
}

/// `GetMusicPlayRanking` 2027 request: fetch the lanking board.
/// Empty body: the exe never sends this opcode (no `0x7eb` push or
/// dword anywhere in `OTwo.exe`, `O2Net32.dll`, `Player.dll`, or
/// `SendWebData.dll`), and Memoryer's `RankingController` handler
/// reads no request fields — the client fetch carries nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetMusicPlayRanking;

impl WireMessage for GetMusicPlayRanking {
    const COMMAND: u16 = 2027;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.is_empty() {
            Ok(Self)
        } else {
            Err(Error::TrailingBytes(body.len()))
        }
    }
}

/// `MusicPlayRankingAnswer` 2028 response: the lanking board.
///
/// Exe-verified handler VA `0x55b230` (file `0x15b230`, REA
/// `ev_7f09b7fa…`): one `i32` rank word, then — only when the word is
/// nonzero — a nickname string through the string-reader helper
/// (`0x555ea0`) plus three `i32` reads, one `u8` read, and one `i32`
/// read through the packet-reader helper (`0x555d20`; pushes at files
/// `0x15b302`/`0x15b30e`/`0x15b31a`/`0x15b326`/`0x15b335`), each row
/// staged into the rank-row vector at `0x77968c`; then an `i32` count
/// word, and — only when positive — that many rows in the same shape
/// (zero/negative counts paint no rows; a zero count paints ten blank
/// rows, a display fill the decoder reports as an empty list).
/// Dispatcher: base `0x7d3` index `0x19` holds entry `0x558545`
/// (address table file `0x158940`), whose stub relative-calls the
/// handler. Memoryer (`Messages/Responses/Ranking/
/// `MusicPlayRankingResponse.cs`): rank `int`, nickname string,
/// battles/wins/win-rate `int`s, `RankDeltaType` byte, rank-delta
/// `int`, with a leading self entry plus an `i32`-counted list — the
/// field order corroborates the exe read sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicPlayRankingAnswer {
    /// The viewer's own line (default when its rank word is zero).
    pub self_entry: RankEntry,
    /// Board lines in wire order.
    pub entries: Vec<RankEntry>,
}

impl WireMessage for MusicPlayRankingAnswer {
    const COMMAND: u16 = 2028;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = encode_rank_entry(&self.self_entry);
        let count = i32::try_from(self.entries.len()).unwrap_or(i32::MAX);
        body.extend_from_slice(&count.to_le_bytes());
        for entry in &self.entries {
            body.extend_from_slice(&encode_rank_entry(entry));
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (self_entry, mut at) = decode_rank_entry(body)?;
        let rest = body.get(at..).ok_or(Error::TruncatedMessage {
            need: at.saturating_add(4),
            have: body.len(),
        })?;
        if rest.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: at + 4,
                have: body.len(),
            });
        }
        let count = i32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
        at += 4;
        let mut entries = Vec::new();
        if count > 0 {
            let count = usize::try_from(count).unwrap_or(usize::MAX);
            entries.reserve(count.min(1024));
            for _ in 0..count {
                let (entry, used) = decode_rank_entry(&body[at..])?;
                entries.push(entry);
                at += used;
            }
        }
        Ok(Self {
            self_entry,
            entries,
        })
    }
}

/// `GetMusicScoreList` 2030 request: fetch per-song best ranks.
/// Empty body: both exe send sites (`push 0; push 0x7ee`, files
/// `0xac427` and `0xb2644`, indirect `+0x5c` send calls) stage no
/// fields — the first gates on the menu-state word at `0x77ee86`
/// being `2` (load + compare files `0xac402`/`0xac417`), the second on
/// the seat byte at `0x77cd30` being nonzero (load file `0xb2622`),
/// both additionally gated on dword `0x779964` being zero (compares
/// files `0xac41c`/`0xb2639`). Memoryer's `RankingController`
/// handler reads no request fields either. The gate globals stay
/// OPEN (no labels pinned).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetMusicScoreList;

impl WireMessage for GetMusicScoreList {
    const COMMAND: u16 = 2030;

    fn encode_body(&self) -> Vec<u8> {
        Vec::new()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.is_empty() {
            Ok(Self)
        } else {
            Err(Error::TrailingBytes(body.len()))
        }
    }
}

/// One song's best ranks in the `MusicScoreListAnswer` 2031 answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicScoreRow {
    /// Song id (`u16`, as selected per room).
    pub music_id: u16,
    /// Best rank marks per difficulty in `Difficulty` order
    /// (EX/NX/HX/RX; `Mozart.Metadata.Rank`: 0 none, 1 F … 7 S,
    /// 8 Perfect).
    pub difficulty_ranks: [u8; 4],
}

/// Wire size of one [`MusicScoreRow`]: `u16` id + four rank bytes.
const MUSIC_SCORE_ROW_LEN: usize = 6;

/// `MusicScoreListAnswer` 2031 response: per-song best ranks.
///
/// Exe-verified handler VA `0x55b590` (file `0x15b590`, REA
/// `ev_299779a2…`): one `i32` result word — negative results skip
/// the rows and still forward — else a `u16` row count, then that many
/// 6-byte rows (`push 6` at file `0x15b5fa`, call `0x15b603`) pushed
/// into the per-song/difficulty best table at `0x779814` through
/// `FUN_004c5700` (which divides the vector span by 6), then forward
/// (`0x12`, 2031, 0, 0). So count and row width are exe-pinned; the
/// row split (`u16` music id + four rank bytes) is Memoryer shape
/// (`Messages/Responses/Ranking/MusicScoreEntry`: `ushort MusicId`
/// plus a `Difficulty → Rank` map, both byte-backed, with an
/// `Empty`-prefix collection = no length word, exactly four inline
/// values in `Enum.GetValues<Difficulty>()` order) against an
/// exe-verified width. The paint-path read that would pin the
/// per-byte difficulty order stays OPEN (next step: the result-screen
/// lookup over `0x779814`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicScoreListAnswer {
    /// Server result word (negative = no rows, still refresh).
    pub result: i32,
    /// Best-rank rows in wire order.
    pub rows: Vec<MusicScoreRow>,
}

impl WireMessage for MusicScoreListAnswer {
    const COMMAND: u16 = 2031;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + 2 + self.rows.len() * MUSIC_SCORE_ROW_LEN);
        body.extend_from_slice(&self.result.to_le_bytes());
        let count = u16::try_from(self.rows.len()).unwrap_or(u16::MAX);
        body.extend_from_slice(&count.to_le_bytes());
        for row in &self.rows {
            body.extend_from_slice(&row.music_id.to_le_bytes());
            body.extend_from_slice(&row.difficulty_ranks);
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
        let result = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        if result < 0 {
            return Ok(Self {
                result,
                rows: Vec::new(),
            });
        }
        if body.len() < 6 {
            return Err(Error::TruncatedMessage {
                need: 6,
                have: body.len(),
            });
        }
        let count = usize::from(u16::from_le_bytes([body[4], body[5]]));
        let need = 6 + count * MUSIC_SCORE_ROW_LEN;
        if body.len() < need {
            return Err(Error::TruncatedMessage {
                need,
                have: body.len(),
            });
        }
        let mut rows = Vec::with_capacity(count);
        for index in 0..count {
            let at = 6 + index * MUSIC_SCORE_ROW_LEN;
            rows.push(MusicScoreRow {
                music_id: u16::from_le_bytes([body[at], body[at + 1]]),
                difficulty_ranks: [body[at + 2], body[at + 3], body[at + 4], body[at + 5]],
            });
        }
        Ok(Self { result, rows })
    }
}

/// `GetMusicMaxScore` 2032 request: fetch one song's best score.
///
/// The exe send (file `0x4151c`, in `FUN_0043e9b0`, REA
/// `ev_2e7c7bc9…`) pushes only the opcode (`push 0; push 0x7f0`,
/// indirect `+0x64` slot call) once the entry passes the music-list
/// membership predicate — no body bytes are staged at the site. The
/// two parameter words sit in adjacent globals by then: the selected
/// music id at `0x7797c0` (store file `0x41472`) and its note-count
/// word at `0x7797c2` (store file `0x414f2`). Memoryer
/// (`Messages/Requests/Ranking/MusicMaxScoreRequest.cs`) decodes
/// `ushort MusicId` + `ushort NoteCount`, which matches those two
/// staged words — so the request encodes them, but the wire transfer
/// of the globals through the `+0x64` send slot is unobserved and
/// stays OPEN (next step: trace the `+0x64` slot helper's body
/// mechanism, e.g. at the Authorize site file `0x74529`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetMusicMaxScore {
    /// Song id to look up.
    pub music_id: u16,
    /// Note count selecting the difficulty (EX/NX/HX match).
    pub note_count: u16,
}

/// Body length of [`GetMusicMaxScore`]: `u16` + `u16`.
const GET_MUSIC_MAX_SCORE_BODY_LEN: usize = 4;

impl WireMessage for GetMusicMaxScore {
    const COMMAND: u16 = 2032;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(GET_MUSIC_MAX_SCORE_BODY_LEN);
        body.extend_from_slice(&self.music_id.to_le_bytes());
        body.extend_from_slice(&self.note_count.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < GET_MUSIC_MAX_SCORE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: GET_MUSIC_MAX_SCORE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > GET_MUSIC_MAX_SCORE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - GET_MUSIC_MAX_SCORE_BODY_LEN,
            ));
        }
        Ok(Self {
            music_id: u16::from_le_bytes([body[0], body[1]]),
            note_count: u16::from_le_bytes([body[2], body[3]]),
        })
    }
}

/// `MusicMaxScoreAnswer` 2033 response: one song's best score.
///
/// Exe-verified handler VA `0x55b640` (file `0x15b640`, REA
/// `ev_45b5f508…`): one `i32` invalid word — nonzero skips the
/// packet — else one `u32` max-score read straight into `0x7797c8`
/// (push file `0x15b668`, call `0x15b672`), then the dialog-7 gate:
/// forward (`0x11`, 2033, 0, 0) only while the ControlList parent
/// (`0x762b44`) shows dialog kind 7. Dispatcher index `0x1e` holds
/// entry `0x558578`, whose stub relative-calls the handler. Memoryer
/// (`Messages/Responses/Ranking/MusicMaxScoreResponse.cs`): bool
/// `Invalid` as `i32` + `u32 MaxScore` — corroborates the two-word
/// shape against the exe-verified read sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicMaxScoreAnswer {
    /// True when the score carries no value (client drops it).
    pub invalid: bool,
    /// Best score on the song.
    pub max_score: u32,
}

/// Body length of [`MusicMaxScoreAnswer`]: `i32` + `u32`.
const MUSIC_MAX_SCORE_ANSWER_BODY_LEN: usize = 8;

impl WireMessage for MusicMaxScoreAnswer {
    const COMMAND: u16 = 2033;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(MUSIC_MAX_SCORE_ANSWER_BODY_LEN);
        body.extend_from_slice(&i32::from(self.invalid).to_le_bytes());
        body.extend_from_slice(&self.max_score.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < MUSIC_MAX_SCORE_ANSWER_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: MUSIC_MAX_SCORE_ANSWER_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > MUSIC_MAX_SCORE_ANSWER_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - MUSIC_MAX_SCORE_ANSWER_BODY_LEN,
            ));
        }
        Ok(Self {
            invalid: i32::from_le_bytes([body[0], body[1], body[2], body[3]]) != 0,
            max_score: u32::from_le_bytes([body[4], body[5], body[6], body[7]]),
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
