//! Screen scenes: real `.ojs`/`.bnd` art behind each [`Screen`],
//! with the working UI ink on top.
//!
//! Each scene paints its verified archive art first (lifecycle §5–11:
//! `O2mediaLogo.ojs`, channel/room/waiting/playing/result blocks,
//! `MusicLoading.ojs`) through the scene kit in [`crate::avatar`]
//! (`load_scene_figures`, `bind_scene_rects`, `paint_scene_figures`).
//! Rects stay verbatim on the native 800x600 canvas (rect `i` lays
//! out entry `i + 1`; Playing lays out entries 213–229); scaling to
//! the frame happens only at present, and live UI (channel list,
//! room list, waiting seats, loading bar, highway, result digits)
//! paints after the art. Bare [`paint_screen`] keeps the flat-color
//! fallback while no art is staged; staged scenes never paint flat cards.

use std::collections::HashMap;

use super::super::avatar::{
    paint_scene_figures, scene_block, scene_sprite_names, AvatarFigure, ScenePlacement, AVI_BLOCK,
    AVI_LOGO_SPRITE, CHANNEL_BLOCK, LOADING_SPRITE, RESULT_BLOCK, ROOM_BLOCK, WAITING_BLOCK,
};
use super::super::playing::{HighwayGeometry, PlayingView};
use super::super::screen::Screen;
use o2jam_file::ControlList;
use o2jam_game::RoomTeams;
use o2jam_net::{
    ChannelState, Difficulty, GameMode, GameSpeed, JoinedRoom, KeyMode, RoomListEntry, RoomSlot,
    RoomSlotState,
};
use o2jam_render::{
    draw_number, draw_text, fill_rect, number_width, pack_xrgb, scale_rect_to_frame, text_width,
    Frame, Rect,
};

/// Gameplay block: real UI lives in `Playing1.opi` (235 entries,
/// lifecycle §10).
pub const PLAYING_BLOCK: &str = "STATE_PLAYING";
/// Gameplay layout bind file: 17 rects over entries 213–229
/// (`EXIT`..`VOLUME`), one-to-one in file order.
pub const PLAYING_BIND_FILE: &str = "State_Playing.bnd";
/// Pre-game room layout bind file.
pub const WAITING_BIND_FILE: &str = "State_Waiting.bnd";
/// Results layout bind file (7 rects).
pub const RESULT_BIND_FILE: &str = "State_Result.bnd";

/// Staged art for one scene: `.bnd` placements plus figures loaded
/// through `load_scene_figures`. Empty while nothing is staged; every
/// scene painter falls back to flat colors then.
#[derive(Debug, Clone, Default)]
pub struct SceneArt {
    /// `.bnd` layout placements in file order.
    pub placements: Vec<ScenePlacement>,
    /// Decoded sprite bodies by archive entry name.
    pub figures: HashMap<String, AvatarFigure>,
}

impl SceneArt {
    /// Whether any figure decoded (placements alone paint nothing).
    #[must_use]
    pub fn has_figures(&self) -> bool {
        !self.figures.is_empty()
    }

    /// Paint staged figures at their verbatim native origins with no
    /// flat fill, so the OPI art keeps its `.bnd` layout. Empty art falls
    /// back to the flat background; placements without a loaded
    /// figure are skipped.
    pub fn paint_backdrop(&self, frame: &mut Frame<'_>, screen: Screen) {
        if !self.has_figures() {
            fill_background(frame, screen);
            return;
        }
        self.paint_overlay(frame);
    }

    /// Paint placed figures over the current frame content (gauges
    /// and panels ring the highway; see [`paint_playing_overlay`]).
    pub fn paint_overlay(&self, frame: &mut Frame<'_>) {
        paint_scene_figures(frame, &self.placements, &self.figures);
    }
}

/// Slot of one screen in the per-scene art table.
#[must_use]
pub const fn scene_index(screen: Screen) -> usize {
    match screen {
        Screen::Avi => 0,
        Screen::Channel => 1,
        Screen::Room => 2,
        Screen::Waiting => 3,
        Screen::Loading => 4,
        Screen::Playing => 5,
        Screen::Result => 6,
    }
}

/// Sprite bodies one `ControlList` block names, in file order (empty
/// for unknown blocks).
fn block_sprites(list: &ControlList, block: &str) -> Vec<String> {
    scene_block(list, block).map_or_else(Vec::new, scene_sprite_names)
}

/// Sprite bodies one block names, or the single-sprite fallback when
/// the block is missing or names nothing (splash/loading are
/// one-entry states).
fn block_or_single(list: &ControlList, block: &str, single: &str) -> Vec<String> {
    let names = block_sprites(list, block);
    if names.is_empty() {
        vec![single.to_owned()]
    } else {
        names
    }
}

/// Sprite bodies one main scene needs, in block order (lifecycle
/// §5–11). Splash and loading resolve to their single art sprites;
/// channel names its dialog bar and buttons (no `.bnd`: the only
/// full state without one); room, waiting, playing, and result name
/// their block entries for `bind_scene_rects`.
#[must_use]
pub fn main_scene_sprites(list: &ControlList, screen: Screen) -> Vec<String> {
    match screen {
        Screen::Avi => block_or_single(list, AVI_BLOCK, AVI_LOGO_SPRITE),
        Screen::Channel => block_sprites(list, CHANNEL_BLOCK),
        Screen::Room => block_sprites(list, ROOM_BLOCK),
        Screen::Waiting => block_sprites(list, WAITING_BLOCK),
        Screen::Loading => block_or_single(list, "STATE_LOADING", LOADING_SPRITE),
        Screen::Playing => block_sprites(list, PLAYING_BLOCK),
        Screen::Result => block_sprites(list, RESULT_BLOCK),
    }
}

/// Placements for blocks without a `.bnd` (splash, channel, loading):
/// no bind file means no layout to claim, so each sprite paints 1:1
/// at the native origin with an empty rect — never full-bleed.
/// `paint_scene_figures` draws each body at that origin; staged
/// bodies keep their native size while live UI ink paints on top.
#[must_use]
pub fn unbound_scene_placements(names: &[String]) -> Vec<ScenePlacement> {
    names
        .iter()
        .map(|name| ScenePlacement {
            id: String::new(),
            code: 0,
            sprite: Some(name.clone()),
            rect: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
        })
        .collect()
}

/// Title ink on lobby cards (dark on the light card).
fn title_color() -> u32 {
    pack_xrgb(0x20, 0x20, 0x20)
}

/// Scrim ink behind live UI over staged art: near-black, distinct
/// from every UI ink so text stays readable on bright OPI panels.
fn scrim_color() -> u32 {
    pack_xrgb(0x08, 0x08, 0x10)
}

/// Title ink over staged art (white on the scrim).
fn art_title_ink() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// Top-left of a centered title (shared by the card and art scenes).
fn title_origin(frame: &Frame<'_>, title: &str) -> (i32, i32) {
    let text_w = text_width(title) as i32;
    (
        frame.w as i32 / 2 - text_w / 2,
        frame.h as i32 / 2 - frame.h as i32 / 8 + 2,
    )
}

/// Title over staged art: dark scrim plus white ink (the card title
/// stays dark-on-light for the flat fallback).
fn paint_title_over_art(frame: &mut Frame<'_>, screen: Screen) {
    let title = title_of(screen);
    if title.is_empty() {
        return;
    }
    let (x, y) = title_origin(frame, title);
    let text_w = text_width(title) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 4,
            y: y - 2,
            w: (text_w + 8).max(0) as u32,
            h: 9,
        },
        scrim_color(),
    );
    draw_text(frame, x, y, title, art_title_ink());
}

/// Dark strip behind `row_count` channel/room rows so list ink
/// stays readable over staged panels (skips the frame origin where
/// staged art anchors at the native origin).
fn paint_list_scrim(frame: &mut Frame<'_>, row_count: usize) {
    if row_count == 0 || frame.w == 0 || frame.h == 0 {
        return;
    }
    let w = frame.w as i32;
    let h = frame.h as i32;
    fill_rect(
        frame,
        &Rect {
            x: w / 4 - 8,
            y: h / 4 - 2,
            w: (w / 2 + 16).max(0) as u32,
            h: (row_count as i32 * CHANNEL_ROW_H + 4).max(0) as u32,
        },
        scrim_color(),
    );
}

/// Row height of one channel entry.
const CHANNEL_ROW_H: i32 = 8;

/// Channel rows plus highlight cursor (no background fill: shared by
/// the flat fallback and the art scene).
fn paint_channel_rows(frame: &mut Frame<'_>, channels: &[ChannelState], highlight: usize) {
    paint_list_scrim(frame, channels.len());
    let w = frame.w as i32;
    let h = frame.h as i32;
    let text_x = w / 4;
    for (index, channel) in channels.iter().enumerate() {
        let row = index_as_row(index, h);
        let entry = format!(
            "{} {}/{}",
            channel.channel_id, channel.population, channel.capacity
        );
        let ink = if channel.active {
            pack_xrgb(0xFF, 0xFF, 0xFF)
        } else {
            pack_xrgb(0x70, 0x70, 0x70)
        };
        draw_text(frame, text_x, row, &entry, ink);
        if index == highlight {
            paint_cursor(frame, text_x, row);
        }
    }
}

/// Row of list entry `index` (channel and room lists share the shape).
fn index_as_row(index: usize, h: i32) -> i32 {
    h / 4 + i32::try_from(index).unwrap_or(i32::MAX) * CHANNEL_ROW_H
}

/// Amber highlight cursor left of a list row.
fn paint_cursor(frame: &mut Frame<'_>, text_x: i32, y: i32) {
    fill_rect(
        frame,
        &Rect {
            x: text_x - 6,
            y,
            w: 3,
            h: 5,
        },
        pack_xrgb(0xFF, 0xD0, 0x20),
    );
}

/// Paint the online channel list with a highlight cursor.
pub(crate) fn paint_channel_list(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    highlight: usize,
) {
    fill_background(frame, Screen::Channel);
    paint_title(frame, "CHANNEL");
    paint_channel_rows(frame, channels, highlight);
}

/// Paint staged channel art with the live list and cursor on top.
pub fn paint_channel_scene(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    highlight: usize,
    art: &SceneArt,
) {
    art.paint_backdrop(frame, Screen::Channel);
    paint_title_over_art(frame, Screen::Channel);
    paint_channel_rows(frame, channels, highlight);
}

/// One room row: `3 TITLE 5/8`, locked rooms suffixed `!`.
fn room_row_text(room: &RoomListEntry) -> String {
    let mut text = format!(
        "{} {} {}/{}",
        room.number, room.title, room.user_count, room.capacity
    );
    if room.has_password {
        text.push('!');
    }
    text
}

/// Room rows plus highlight cursor (no background fill: shared by
/// the flat fallback and the art scene).
fn paint_room_rows(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
    paint_list_scrim(frame, rooms.len());
    let w = frame.w as i32;
    let h = frame.h as i32;
    let text_x = w / 4;
    for (index, room) in rooms.iter().enumerate() {
        let row = index_as_row(index, h);
        draw_text(
            frame,
            text_x,
            row,
            &room_row_text(room),
            pack_xrgb(0xFF, 0xFF, 0xFF),
        );
        if index == highlight {
            paint_cursor(frame, text_x, row);
        }
    }
}

/// Title anchor of the highlighted-room panel (`SET 0x11` row
/// template in `STATE_ROOM`).
const ROOM_DETAIL_TITLE_ID: &str = "IDC_TEXT_SROOM_ROOMNAME";
/// Seat-count anchor (`ROOM_USER`).
const ROOM_DETAIL_USERS_ID: &str = "IDC_TEXT_SROOM_ROOM_USER";
/// Selected-song anchor.
const ROOM_DETAIL_MUSIC_ID: &str = "IDC_TEXT_SROOM_MUSICTITLE";
/// Difficulty anchor (second music-title slot).
const ROOM_DETAIL_DIFFICULTY_ID: &str = "IDC_TEXT_SROOM_MUSICTITLE2";
/// Scroll-speed icon anchor; text stands in for the icon body.
const ROOM_DETAIL_SPEED_ID: &str = "IDC_IMAGE_SROOM_SPEED";
/// Game-mode icon anchor (`Room_Mode.ojs`); text stands in.
const ROOM_DETAIL_MODE_ID: &str = "IDC_IMAGE_SROOM_MODE";
/// Key-mode anchor (`Room_Om.ojs` option-mode icon).
const ROOM_DETAIL_KEY_ID: &str = "IDC_IMAGE_SROOM_OM";
/// Level-gate text anchor beside `Room_LevelLimit.ojs`.
const ROOM_DETAIL_LEVEL_ID: &str = "IDC_TEXT_SROOM_LEVEL_RANGE";
/// Lock-icon anchor (`Room_Password.ojs`); text stands in.
const ROOM_DETAIL_LOCK_ID: &str = "IDC_IMAGE_SROOM_PASSWORD";

/// Detail lines per panel: title plus the eight entry fields.
const ROOM_DETAIL_LINES: usize = 9;

/// Game-mode label (`NORMAL`/`LIVE`; raw code while unknown).
#[must_use]
fn room_mode_label(game_mode: u8) -> String {
    match GameMode::from_u8(game_mode) {
        Ok(GameMode::Normal) => "NORMAL".to_owned(),
        Ok(GameMode::Live) => "LIVE".to_owned(),
        Err(_) => format!("MODE {game_mode}"),
    }
}

/// Selected-song label (the song table lives server-side, so the id).
#[must_use]
fn room_music_label(music_id: u16) -> String {
    format!("M{music_id}")
}

/// Difficulty label (`EX`/`NX`/`HX`/`RX`; raw code while unknown).
#[must_use]
fn room_difficulty_label(difficulty: u8) -> String {
    match Difficulty::from_code(difficulty) {
        Some(Difficulty::Ex) => "EX".to_owned(),
        Some(Difficulty::Nx) => "NX".to_owned(),
        Some(Difficulty::Hx) => "HX".to_owned(),
        Some(Difficulty::Rx) => "RX".to_owned(),
        None => format!("DIF {difficulty}"),
    }
}

/// Key-mode label (`3KEY`/`5KEY`/`7KEY`; raw code while unknown).
#[must_use]
fn room_key_label(key_mode: u8) -> String {
    match KeyMode::from_u8(key_mode) {
        Ok(KeyMode::ThreeKeys) => "3KEY".to_owned(),
        Ok(KeyMode::FiveKeys) => "5KEY".to_owned(),
        Ok(KeyMode::SevenKeys) => "7KEY".to_owned(),
        Err(_) => format!("KEY {key_mode}"),
    }
}

/// Scroll-speed label (`X0.5`..`X6`, `XR`; raw code while unknown).
#[must_use]
fn room_speed_label(speed: u8) -> String {
    match GameSpeed::from_code(speed) {
        Some(GameSpeed::X05) => "X0.5".to_owned(),
        Some(GameSpeed::X10) => "X1".to_owned(),
        Some(GameSpeed::X15) => "X1.5".to_owned(),
        Some(GameSpeed::X20) => "X2".to_owned(),
        Some(GameSpeed::X25) => "X2.5".to_owned(),
        Some(GameSpeed::X30) => "X3".to_owned(),
        Some(GameSpeed::X35) => "X3.5".to_owned(),
        Some(GameSpeed::X40) => "X4".to_owned(),
        Some(GameSpeed::X45) => "X4.5".to_owned(),
        Some(GameSpeed::X50) => "X5".to_owned(),
        Some(GameSpeed::X60) => "X6".to_owned(),
        Some(GameSpeed::Xr) => "XR".to_owned(),
        None => format!("SPD {speed}"),
    }
}

/// Seat label (`seated/capacity`).
#[must_use]
fn room_capacity_label(room: &RoomListEntry) -> String {
    format!("{}/{}", room.user_count, room.capacity)
}

/// Level-gate label (`LV min-max`).
#[must_use]
fn room_level_label(room: &RoomListEntry) -> String {
    format!("LV {}-{}", room.min_level, room.max_level)
}

/// Lock label (`LOCK` with a password, `OPEN` without).
#[must_use]
fn room_lock_label(has_password: bool) -> &'static str {
    if has_password {
        "LOCK"
    } else {
        "OPEN"
    }
}

/// Lock-line ink: red while a password is set, dim otherwise.
fn room_lock_ink(locked: bool) -> u32 {
    if locked {
        pack_xrgb(0xFF, 0x50, 0x50)
    } else {
        pack_xrgb(0x90, 0x90, 0x90)
    }
}

/// Ordered detail entries for one room: (`State_Room.bnd`
/// control id, text).
#[must_use]
fn room_detail_entries(room: &RoomListEntry) -> [(&'static str, String); ROOM_DETAIL_LINES] {
    [
        (ROOM_DETAIL_TITLE_ID, room.title.clone()),
        (ROOM_DETAIL_USERS_ID, room_capacity_label(room)),
        (ROOM_DETAIL_MUSIC_ID, room_music_label(room.music_id)),
        (
            ROOM_DETAIL_DIFFICULTY_ID,
            room_difficulty_label(room.difficulty),
        ),
        (ROOM_DETAIL_KEY_ID, room_key_label(room.key_mode)),
        (ROOM_DETAIL_MODE_ID, room_mode_label(room.game_mode)),
        (ROOM_DETAIL_SPEED_ID, room_speed_label(room.speed)),
        (ROOM_DETAIL_LEVEL_ID, room_level_label(room)),
        (
            ROOM_DETAIL_LOCK_ID,
            room_lock_label(room.has_password).to_owned(),
        ),
    ]
}

/// Scaled frame origin of one `State_Room.bnd` control (`None`
/// while the staged art names no such control).
#[must_use]
fn room_detail_bnd_origin(
    placements: &[ScenePlacement],
    id: &str,
    frame: &Frame<'_>,
) -> Option<(i32, i32)> {
    let placement = placements.iter().find(|placed| placed.id == id)?;
    let scaled = scale_rect_to_frame(&placement.rect, frame.w, frame.h);
    Some((scaled.x, scaled.y))
}

/// Fallback origin of detail line `slot` without staged art: stacked
/// rows under the list so the list plus cursor keep their room.
fn room_detail_fallback_origin(slot: usize, row_count: usize, frame: &Frame<'_>) -> (i32, i32) {
    let rows = i32::try_from(row_count).unwrap_or(i32::MAX);
    let slot_offset = i32::try_from(slot).unwrap_or(i32::MAX);
    (
        frame.w as i32 / 4,
        frame.h as i32 / 4 + rows * CHANNEL_ROW_H + 4 + slot_offset * CHANNEL_ROW_H,
    )
}

/// One detail line: dark scrim plus ink (shared by the flat fallback
/// and the art scene).
fn paint_room_detail_line(frame: &mut Frame<'_>, x: i32, y: i32, text: &str, ink: u32) {
    let text_w = text_width(text) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 4,
            y: y - 2,
            w: (text_w + 8).max(0) as u32,
            h: 9,
        },
        scrim_color(),
    );
    draw_text(frame, x, y, text, ink);
}

/// Details of the highlighted room: title, song, difficulty, key
/// mode, game mode, speed, seats, level gate, lock. Lines anchor to
/// their `State_Room.bnd` rects while staged art names them and
/// stack under the list otherwise; nothing paints without a room.
fn paint_room_details(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    placements: Option<&[ScenePlacement]>,
) {
    let Some(room) = rooms.get(highlight) else {
        return;
    };
    for (slot, (id, text)) in room_detail_entries(room).iter().enumerate() {
        let origin = placements
            .and_then(|placed| room_detail_bnd_origin(placed, id, frame))
            .unwrap_or_else(|| room_detail_fallback_origin(slot, rooms.len(), frame));
        let ink = if *id == ROOM_DETAIL_LOCK_ID {
            room_lock_ink(room.has_password)
        } else {
            art_title_ink()
        };
        paint_room_detail_line(frame, origin.0, origin.1, text, ink);
    }
}

/// Paint the online room list with a highlight cursor.
pub(crate) fn paint_room_list(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
    fill_background(frame, Screen::Room);
    paint_title(frame, "ROOM");
    paint_room_rows(frame, rooms, highlight);
    paint_room_details(frame, rooms, highlight, None);
}

/// Paint staged lobby art with the live room list and cursor on top.
pub fn paint_room_scene(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: &SceneArt,
) {
    art.paint_backdrop(frame, Screen::Room);
    paint_title_over_art(frame, Screen::Room);
    paint_room_rows(frame, rooms, highlight);
    paint_room_details(frame, rooms, highlight, Some(&art.placements));
}

/// Seats shown on the waiting screen: self plus the seven wire slots.
pub(crate) const WAITING_SEATS: usize = 8;
/// Peer slots carried beside self in [`JoinedRoom`].
const WAITING_PEERS: usize = 7;
/// Waiting rows share the lobby row height.
const WAITING_ROW_H: i32 = 8;

/// One waiting-room seat view: display name plus team/ready state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WaitingSeat {
    team: u8,
    name: String,
    kind: SeatKind,
}

/// Seat occupancy: only occupied seats carry a name and ready flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeatKind {
    Occupied { ready: bool },
    Empty,
    Locked,
}

impl WaitingSeat {
    /// Display name (`None` for vacant/locked seats).
    #[must_use]
    pub(crate) fn display_name(&self) -> Option<&str> {
        match self.kind {
            SeatKind::Occupied { .. } => Some(&self.name),
            SeatKind::Empty | SeatKind::Locked => None,
        }
    }

    /// Ready flag (vacant/locked seats are never ready).
    #[must_use]
    pub(crate) fn is_ready(&self) -> bool {
        matches!(self.kind, SeatKind::Occupied { ready: true })
    }
}

/// Team button letter (`0` -> `A` .. `7` -> `H`); `?` when out of range.
fn team_letter(team: u8) -> char {
    if team < 8 {
        (b'A' + team) as char
    } else {
        '?'
    }
}

/// Our own seat: name is local-only (the wire sends no self nickname).
fn self_seat(room: &JoinedRoom) -> WaitingSeat {
    WaitingSeat {
        team: room.team,
        name: "YOU".to_owned(),
        kind: SeatKind::Occupied { ready: false },
    }
}

/// Vacant seat placeholder.
fn empty_seat() -> WaitingSeat {
    WaitingSeat {
        team: 0,
        name: String::new(),
        kind: SeatKind::Empty,
    }
}

/// View for one wire slot: named when occupied, locked/empty otherwise.
fn peer_seat(slot: &RoomSlot) -> WaitingSeat {
    match &slot.member {
        Some(member) => WaitingSeat {
            team: member.team,
            name: member.nickname.clone(),
            kind: SeatKind::Occupied {
                ready: member.ready,
            },
        },
        None if slot.state_code == RoomSlotState::Locked as i32 => WaitingSeat {
            team: 0,
            name: String::new(),
            kind: SeatKind::Locked,
        },
        None => empty_seat(),
    }
}

/// Eight seat views: our seat first, then the wire slots, padded empty.
///
/// The wire carries seven peer slots beside self (`JoinedRoom.slots`);
/// together they are the eight waiting-room seats.
#[must_use]
pub(crate) fn waiting_seats(room: &JoinedRoom) -> Vec<WaitingSeat> {
    let mut seats = Vec::with_capacity(WAITING_SEATS);
    seats.push(self_seat(room));
    for slot in room.slots.iter().take(WAITING_PEERS) {
        seats.push(peer_seat(slot));
    }
    while seats.len() < WAITING_SEATS {
        seats.push(empty_seat());
    }
    seats
}

/// One seat row: seat, team, name, flag (`R` ready, `-` waiting).
/// Vacant seats keep their index with a `---`/`LOCK` placeholder.
fn waiting_row_text(index: usize, seat: &WaitingSeat) -> String {
    match seat.kind {
        SeatKind::Occupied { ready } => {
            let flag = if ready { 'R' } else { '-' };
            format!(
                "{} {} {} {}",
                index,
                team_letter(seat.team),
                seat.name,
                flag
            )
        }
        SeatKind::Empty => format!("{index} - ---"),
        SeatKind::Locked => format!("{index} - LOCK"),
    }
}

/// Ready-dot ink: green when ready, dim otherwise.
fn ready_dot_color(ready: bool) -> u32 {
    if ready {
        pack_xrgb(0x30, 0xE0, 0x30)
    } else {
        pack_xrgb(0x50, 0x50, 0x50)
    }
}

/// Top-left of seat `index`: two columns by four rows, relative to frame.
#[must_use]
pub(crate) fn seat_origin(index: usize, w: usize, h: usize) -> (i32, i32) {
    let col = (index % 2) as i32;
    let row = (index / 2) as i32;
    (
        w as i32 / 8 + col * (w as i32 / 2),
        h as i32 / 4 + row * WAITING_ROW_H,
    )
}

/// Seat rows plus ready dots (no background fill: shared by the flat
/// fallback and the art scene).
fn paint_waiting_rows(frame: &mut Frame<'_>, room: &JoinedRoom) {
    for (index, seat) in waiting_seats(room).iter().enumerate() {
        paint_waiting_seat(frame, index, seat);
    }
}

/// Paint the eight waiting-room seats with names and ready flags.
pub(crate) fn paint_waiting_members(frame: &mut Frame<'_>, room: &JoinedRoom) {
    fill_background(frame, Screen::Waiting);
    paint_title(frame, "WAITING");
    paint_waiting_rows(frame, room);
}

/// Paint staged waiting art (team buttons, avatar covers) with the
/// live seats and ready dots on top.
pub fn paint_waiting_scene(frame: &mut Frame<'_>, room: &JoinedRoom, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Waiting);
    paint_title_over_art(frame, Screen::Waiting);
    paint_waiting_rows(frame, room);
}

/// Dark strip behind one seat row so seat ink stays readable over
/// staged art (vacant rows get one too: the placeholder must read).
fn paint_seat_scrim(frame: &mut Frame<'_>, x: i32, y: i32, text: &str) {
    let text_w = text_width(text) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 8,
            y: y - 1,
            w: (text_w + 12).max(0) as u32,
            h: 7,
        },
        scrim_color(),
    );
}

/// Paint one seat row plus its ready dot (vacant seats have no dot).
fn paint_waiting_seat(frame: &mut Frame<'_>, index: usize, seat: &WaitingSeat) {
    let occupied = seat.display_name().is_some();
    let (x, y) = seat_origin(index, frame.w, frame.h);
    let row = waiting_row_text(index, seat);
    paint_seat_scrim(frame, x, y, &row);
    let ink = if occupied {
        pack_xrgb(0xFF, 0xFF, 0xFF)
    } else {
        pack_xrgb(0x70, 0x70, 0x70)
    };
    draw_text(frame, x, y, &row, ink);
    if occupied {
        fill_rect(
            frame,
            &Rect {
                x: x - 6,
                y,
                w: 3,
                h: 5,
            },
            ready_dot_color(seat.is_ready()),
        );
    }
}

/// Title tucked just inside the card's top edge.
fn paint_title(frame: &mut Frame<'_>, title: &str) {
    if title.is_empty() {
        return;
    }
    let (x, y) = title_origin(frame, title);
    draw_text(frame, x, y, title, title_color());
}

/// Staged lobby art behind an empty list: verbatim backdrop plus
/// a readable title (no flat card).
pub fn paint_lobby_backdrop(frame: &mut Frame<'_>, screen: Screen, art: &SceneArt) {
    art.paint_backdrop(frame, screen);
    paint_title_over_art(frame, screen);
}

/// Client version stamped on the splash (matches the 7.55 target).
const SPLASH_VERSION: u32 = 755;
/// Splash version color (white on the logo card).
fn version_color() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// Paint the splash (logo) scene.
fn paint_avi(frame: &mut Frame<'_>) {
    paint_card(
        frame,
        pack_xrgb(0x08, 0x08, 0x10),
        pack_xrgb(0x20, 0x60, 0xC0),
    );
    let text_w = number_width(SPLASH_VERSION) as i32;
    draw_number(
        frame,
        frame.w as i32 / 2 - text_w / 2,
        frame.h as i32 / 2 - 2,
        SPLASH_VERSION,
        version_color(),
    );
}

/// Paint staged splash art (`O2mediaLogo.ojs`): the real logo
/// replaces the card plus version stamp.
pub fn paint_avi_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Avi);
}

/// Paint a lobby scene (channel / room / waiting share the shape).
fn paint_lobby(frame: &mut Frame<'_>, background: u32) {
    paint_card(frame, background, pack_xrgb(0xD0, 0xD0, 0xD0));
}

/// Loading progress bar over a dark track (no background fill:
/// shared by the flat fallback and the art scene).
fn paint_loading_bar(frame: &mut Frame<'_>) {
    let (w, h) = (frame.w as i32, frame.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: w / 4 - 2,
            y: h / 2 - 10,
            w: (w / 2 + 4).max(0) as u32,
            h: 20,
        },
        scrim_color(),
    );
    fill_rect(
        frame,
        &Rect {
            x: w / 4,
            y: h / 2 - 8,
            w: (w / 2) as u32,
            h: 16,
        },
        pack_xrgb(0x30, 0xB0, 0x30),
    );
}

/// Paint the loading gate scene.
fn paint_loading(frame: &mut Frame<'_>) {
    fill_background(frame, Screen::Loading);
    paint_loading_bar(frame);
}

/// Paint staged loading art (`MusicLoading.ojs`) with the live
/// progress bar on top.
pub fn paint_loading_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Loading);
    paint_loading_bar(frame);
}

/// Battle verdict art mounted by the result builder `FUN_004c2390`
/// (file `0xc2390`): `StateResult_Top.ojs` plus
/// `StateResult_Bottom.ojs` plus `Result_WinLose.ojs` (file `0x24a35c`).
/// The play-scene winner banner `Winner_image.ojs` (file `0x24b014`)
/// names the same verdict; the result scene surfaces both verdict arts.
/// Winner is rank 1 by team score only (`RoomTeams::winner`); team
/// totals stay server-side (per-member 4014/4016). Grade letters stay
/// out (mapping OPEN).
pub const RESULT_TOP_SPRITE: &str = "StateResult_Top.ojs";
/// Bottom half mounted by `FUN_004c2390` (file `0xc2390`).
pub const RESULT_BOTTOM_SPRITE: &str = "StateResult_Bottom.ojs";
/// Win/lose verdict mounted by `FUN_004c2390` (file `0xc2390`).
pub const RESULT_WINLOSE_SPRITE: &str = "Result_WinLose.ojs";
/// Winner banner from `STATE_PLAYING` (file `0x24b014`), surfaced on
/// the result scene as the same rank-1 verdict.
pub const WINNER_IMAGE_SPRITE: &str = "Winner_image.ojs";

/// Sprites the result builder `FUN_004c2390` (file `0xc2390`) mounts,
/// in load order: top, bottom, then the win/lose verdict.
#[must_use]
pub fn result_loader_sprites() -> [&'static str; 3] {
    [
        RESULT_TOP_SPRITE,
        RESULT_BOTTOM_SPRITE,
        RESULT_WINLOSE_SPRITE,
    ]
}

/// Battle verdict: rank 1 by team score wins (`RoomTeams::winner`).
/// Solo or team, grades play no part (mapping OPEN).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleVerdict {
    /// Local team holds rank 1.
    Win,
    /// Another team holds rank 1.
    Lose,
}

/// Rank-1 verdict for the local team: `Win` when it holds the top
/// total, `Lose` otherwise. `None` while the winner or the local
/// team is unknown (empty room, solo with no seat claimed).
#[must_use]
pub fn battle_verdict(teams: &RoomTeams, self_team: Option<u8>) -> Option<BattleVerdict> {
    let winner = teams.winner()?;
    let local = self_team?;
    if local == winner {
        Some(BattleVerdict::Win)
    } else {
        Some(BattleVerdict::Lose)
    }
}

/// Verdict caption standing in for the staged `Winner_image.ojs` /
/// `Result_WinLose.ojs` bodies until those figures bind.
fn verdict_text(verdict: BattleVerdict) -> &'static str {
    match verdict {
        BattleVerdict::Win => "WIN",
        BattleVerdict::Lose => "LOSE",
    }
}

/// Verdict ink: gold for a win, dim gray for a loss.
fn verdict_ink(verdict: BattleVerdict) -> u32 {
    match verdict {
        BattleVerdict::Win => pack_xrgb(0xFF, 0xD0, 0x20),
        BattleVerdict::Lose => pack_xrgb(0x90, 0x90, 0x90),
    }
}

/// Paint the rank-1 verdict line under the score digits; nothing
/// while the verdict is unknown.
fn paint_battle_verdict(frame: &mut Frame<'_>, verdict: Option<BattleVerdict>) {
    let Some(known) = verdict else {
        return;
    };
    draw_text(frame, 16, 48, verdict_text(known), verdict_ink(known));
}

/// Score, best combo, verdict, and avatar figure (no background
/// fill: shared by the flat fallback and the art scene). A dark
/// scrim sits behind the digits so they read over staged art.
/// Grade letters stay out (mapping OPEN).
fn paint_result_content(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
    verdict: Option<BattleVerdict>,
) {
    paint_digits_scrim(frame);
    if let Some(figure) = avatar {
        figure.paint(frame, frame.w as i32 / 2 - 32, frame.h as i32 / 2 - 32);
    }
    let points = u32::try_from(score.points).unwrap_or(u32::MAX);
    draw_number(frame, 16, 16, points, pack_xrgb(0xFF, 0xFF, 0xFF));
    draw_number(frame, 16, 32, score.max_combo, pack_xrgb(0xFF, 0xFF, 0x40));
    paint_battle_verdict(frame, verdict);
}

/// Dark sheet behind the result digits and verdict line so the
/// score ink stays readable over staged result panels.
fn paint_digits_scrim(frame: &mut Frame<'_>) {
    if frame.w == 0 || frame.h == 0 {
        return;
    }
    fill_rect(
        frame,
        &Rect {
            x: 8,
            y: 8,
            w: 64,
            h: 52,
        },
        scrim_color(),
    );
}

/// Paint the results scene: score, best combo, rank-1 verdict, and a
/// loaded avatar figure over the card. Grade letters stay out
/// (mapping OPEN).
pub(crate) fn paint_result(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
    verdict: Option<BattleVerdict>,
) {
    paint_card(
        frame,
        pack_xrgb(0x10, 0x10, 0x08),
        pack_xrgb(0xC0, 0xA0, 0x20),
    );
    paint_result_content(frame, avatar, score, verdict);
}

/// Paint staged result art (`StateResult_Top/Bottom.ojs` plus
/// `Result_WinLose.ojs` via `FUN_004c2390`, file `0xc2390`) with the
/// avatar figure, live digits, and the `Winner_image.ojs` /
/// `Result_WinLose.ojs` verdict on top.
pub fn paint_result_scene(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
    art: &SceneArt,
    verdict: Option<BattleVerdict>,
) {
    art.paint_backdrop(frame, Screen::Result);
    paint_result_content(frame, avatar, score, verdict);
}

/// Paint staged gameplay art over the highway: life/jam gauges and
/// panels ring the frame edges while notes keep falling underneath.
pub fn paint_playing_overlay(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_overlay(frame);
}

/// Fill the whole frame with one screen's background.
fn fill_background(frame: &mut Frame<'_>, screen: Screen) {
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: u32::try_from(frame.w).unwrap_or(u32::MAX),
            h: u32::try_from(frame.h).unwrap_or(u32::MAX),
        },
        background_of(screen),
    );
}

/// Background plus a centered card.
fn paint_card(frame: &mut Frame<'_>, background: u32, card: u32) {
    let (w, h) = (frame.w as i32, frame.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: w as u32,
            h: h as u32,
        },
        background,
    );
    fill_rect(
        frame,
        &Rect {
            x: w / 2 - w / 8,
            y: h / 2 - h / 8,
            w: (w / 4) as u32,
            h: (h / 4) as u32,
        },
        card,
    );
}

/// Background color per screen (lobbies share the card shape).
pub(crate) fn background_of(screen: Screen) -> u32 {
    match screen {
        Screen::Avi => pack_xrgb(0x08, 0x08, 0x10),
        Screen::Channel => pack_xrgb(0x10, 0x20, 0x50),
        Screen::Room => pack_xrgb(0x10, 0x50, 0x20),
        Screen::Waiting => pack_xrgb(0x50, 0x40, 0x10),
        Screen::Loading => pack_xrgb(0x18, 0x18, 0x18),
        Screen::Playing => pack_xrgb(0x0A, 0x0A, 0x0A),
        Screen::Result => pack_xrgb(0x10, 0x10, 0x08),
    }
}

/// Title drawn on each lobby card.
fn title_of(screen: Screen) -> &'static str {
    match screen {
        Screen::Channel => "CHANNEL",
        Screen::Room => "ROOM",
        Screen::Waiting => "WAITING",
        _ => "",
    }
}

/// Paint one screen's scene.
pub fn paint_screen(frame: &mut Frame<'_>, screen: Screen) {
    match screen {
        Screen::Avi => paint_avi(frame),
        Screen::Channel | Screen::Room | Screen::Waiting => {
            paint_lobby(frame, background_of(screen));
            paint_title(frame, title_of(screen));
        }
        Screen::Loading => paint_loading(frame),
        Screen::Playing => {
            PlayingView::default().paint(frame, &HighwayGeometry::centered(frame.w, frame.h));
        }
        Screen::Result => paint_result(frame, None, &o2jam_game::Score::default(), None),
    }
}

#[cfg(test)]
mod paint_tests {
    use super::*;
    use crate::avatar::FigureLayer;
    use crate::screen::Screen;
    use o2jam_net::{JoinedRoom, RoomMember, RoomSlot};
    use o2jam_render::{pack_xrgb, Frame, Rgb555Image};

    fn painted(screen: Screen, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        paint_screen(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            screen,
        );
        buf
    }

    /// One red pixel figure (colorkey white, so the pixel paints).
    fn red_figure() -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// Art staging one red pixel at the frame origin.
    fn corner_art() -> SceneArt {
        SceneArt {
            placements: unbound_scene_placements(&["Logo.ojs".to_owned()]),
            figures: HashMap::from([("Logo.ojs".to_owned(), red_figure())]),
        }
    }

    fn paint_art(screen: Screen, art: &SceneArt, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        let mut frame = Frame {
            buf: &mut buf,
            w,
            h,
        };
        match screen {
            Screen::Avi => paint_avi_scene(&mut frame, art),
            Screen::Channel => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Room => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Waiting => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Loading => paint_loading_scene(&mut frame, art),
            Screen::Playing => {
                paint_screen(&mut frame, screen);
                paint_playing_overlay(&mut frame, art);
            }
            Screen::Result => {
                paint_result_scene(&mut frame, None, &o2jam_game::Score::default(), art, None)
            }
        }
        buf
    }

    #[test]
    fn splash_paints_logo_at_native_origin_without_stamp() {
        // Unbound logo art paints 1:1 at the native origin: logo
        // pixel at the origin, no flat card, no version-stamp white.
        let buf = paint_art(Screen::Avi, &corner_art(), 32, 24);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(!buf.contains(&pack_xrgb(0x20, 0x60, 0xC0)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn flat_fallback_covers_bare_frames() {
        // While no art is staged the flat colors remain (the flat
        // list/member painters depend on them); staged scenes never
        // use them.
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            let buf = painted(screen, 16, 12);
            assert_eq!(buf[0], background_of(screen), "{screen:?}");
        }
    }

    #[test]
    fn every_art_scene_hides_flat_fill() {
        // Staged art-first composition: the logo pixel survives at
        // the origin and no flat background shows through.
        let art = corner_art();
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            let buf = paint_art(screen, &art, 16, 12);
            assert_eq!(buf[0], pack_xrgb(255, 0, 0), "{screen:?}");
            assert!(
                !buf.iter().all(|pixel| *pixel == background_of(screen)),
                "{screen:?} paints a flat fill"
            );
        }
    }

    #[test]
    fn scene_indexes_cover_every_screen_once() {
        let indexes = [
            scene_index(Screen::Avi),
            scene_index(Screen::Channel),
            scene_index(Screen::Room),
            scene_index(Screen::Waiting),
            scene_index(Screen::Loading),
            scene_index(Screen::Playing),
            scene_index(Screen::Result),
        ];
        let mut sorted = indexes;
        sorted.sort_unstable();
        assert_eq!(sorted, [0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn unbound_placements_pin_origin_without_size() {
        // No `.bnd` means no layout: unbound sprites paint 1:1 at
        // the native origin with an empty rect, never full-bleed.
        let placed = unbound_scene_placements(&["A.ojs".to_owned(), "B.ojs".to_owned()]);
        assert_eq!(placed.len(), 2);
        for placement in &placed {
            assert_eq!(
                placement.rect,
                Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                }
            );
        }
        assert_eq!(placed[0].sprite.as_deref(), Some("A.ojs"));
    }

    #[test]
    fn lobby_titles_stay_readable_over_art() {
        // 128x96 channel backdrop: white title ink over the dark
        // scrim, never the flat card gray.
        let buf = paint_art(Screen::Channel, &corner_art(), 128, 96);
        let scrim = pack_xrgb(0x08, 0x08, 0x10);
        let white = pack_xrgb(0xFF, 0xFF, 0xFF);
        let card = pack_xrgb(0xD0, 0xD0, 0xD0);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&white));
        assert!(buf.contains(&scrim));
        assert!(!buf.contains(&card));
    }

    fn waiting_room() -> JoinedRoom {
        let member = RoomMember {
            nickname: "host".to_owned(),
            level: 9,
            gender: 0,
            gem: 0,
            master: true,
            team: 0,
            ready: true,
            music_state: 0,
            equipment: [0; 17],
            music_ids: Vec::new(),
            cash_point: 0,
            free_pass: 0,
            playing_state: 0,
            administrator: false,
            super_manager: false,
        };
        JoinedRoom {
            member_id: 0,
            team: 1,
            title: "Jam".to_owned(),
            music_id: 0,
            arena: 0,
            key_mode: 2,
            game_mode: 0,
            difficulty: 0,
            speed: 1,
            has_super_manager: false,
            user_count: 2,
            slots: vec![
                RoomSlot {
                    index: 0,
                    state_code: 1,
                    member: Some(member),
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
            ],
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    #[test]
    fn waiting_seats_pad_self_plus_slots_to_eight() {
        let seats = waiting_seats(&waiting_room());
        assert_eq!(seats.len(), WAITING_SEATS);
        assert_eq!(seats[0].display_name(), Some("YOU"));
        assert_eq!(seats[1].display_name(), Some("host"));
        assert!(seats[1].is_ready());
        assert!(!seats[0].is_ready());
        assert_eq!(seats[2].display_name(), None);
        assert_eq!(seats[3].kind, SeatKind::Locked);
        assert!(seats[4..].iter().all(|seat| seat.kind == SeatKind::Empty));
    }

    #[test]
    fn waiting_row_text_shows_name_team_and_ready() {
        let seats = waiting_seats(&waiting_room());
        assert_eq!(waiting_row_text(1, &seats[1]), "1 A host R");
        assert_eq!(waiting_row_text(0, &seats[0]), "0 B YOU -");
        assert_eq!(waiting_row_text(2, &seats[2]), "2 - ---");
        assert_eq!(waiting_row_text(3, &seats[3]), "3 - LOCK");
    }

    #[test]
    fn waiting_paints_ready_dots_for_seats() {
        let room = waiting_room();
        let mut buf = vec![0u32; 80 * 64];
        paint_waiting_members(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            &room,
        );
        let waiting_bg = background_of(Screen::Waiting);
        let ready_green = pack_xrgb(0x30, 0xE0, 0x30);
        let dim = pack_xrgb(0x50, 0x50, 0x50);
        assert_eq!(buf[0], waiting_bg);
        // Seat 1 (ready host) sits in column 1, row 0: dot at (44, 16).
        let (host_x, host_y) = seat_origin(1, 80, 64);
        assert_eq!(
            buf[(host_y + 1) as usize * 80 + (host_x - 6 + 1) as usize],
            ready_green
        );
        // Seat 0 (self, not ready) keeps a dim dot.
        let (self_x, self_y) = seat_origin(0, 80, 64);
        assert_eq!(
            buf[(self_y + 1) as usize * 80 + (self_x - 6 + 1) as usize],
            dim
        );
        // Seat 2 (empty) has no dot: the seat scrim shows through.
        let (empty_x, empty_y) = seat_origin(2, 80, 64);
        assert_eq!(
            buf[(empty_y + 1) as usize * 80 + (empty_x - 6 + 1) as usize],
            scrim_color()
        );
    }

    #[test]
    fn paint_is_safe_on_empty_frame() {
        let mut buf = vec![];
        paint_screen(
            &mut Frame {
                buf: &mut buf,
                w: 0,
                h: 0,
            },
            Screen::Playing,
        );
    }

    fn scene_control_text() -> String {
        [
            "NUMBER_OF_STATE 0x03",
            "STATE_CHANNEL 0x02",
            "{",
            "BOUND",
            "IDC_BAR 0X00100000",
            "IDC_SELECT 0X10500001 \"Dialog_ChannelSelectBar.ojs\"",
            "}",
            "STATE_ROOM 0x02",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X00100000",
            "IDC_LIST 0X10500001 \"Room_List.ojs\"",
            "}",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn main_sprites_name_channel_dialog_art() {
        let list = o2jam_file::parse_control_list(&scene_control_text()).unwrap();
        assert_eq!(
            main_scene_sprites(&list, Screen::Channel),
            vec!["Dialog_ChannelSelectBar.ojs".to_owned()]
        );
        assert_eq!(
            main_scene_sprites(&list, Screen::Room),
            vec!["Room_List.ojs".to_owned()]
        );
    }

    #[test]
    fn main_sprites_fall_back_to_single_art() {
        let list = o2jam_file::parse_control_list(&scene_control_text()).unwrap();
        assert_eq!(
            main_scene_sprites(&list, Screen::Avi),
            vec![AVI_LOGO_SPRITE.to_owned()]
        );
        assert_eq!(
            main_scene_sprites(&list, Screen::Loading),
            vec![LOADING_SPRITE.to_owned()]
        );
        assert!(main_scene_sprites(&list, Screen::Playing).is_empty());
        assert!(main_scene_sprites(&list, Screen::Result).is_empty());
    }

    #[test]
    fn avi_scene_shows_logo_without_stamp() {
        // Corner art paints the logo pixel; the stamp row that the
        // flat splash whites out stays unpainted (blank canvas)
        // under staged art.
        let buf = paint_art(Screen::Avi, &corner_art(), 32, 24);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[10 * 32 + 11], 0);
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    fn two_channels() -> Vec<ChannelState> {
        vec![
            ChannelState {
                gateway_id: 0,
                channel_id: 2,
                capacity: 100,
                population: 5,
                active: true,
            },
            ChannelState {
                gateway_id: 0,
                channel_id: 9,
                capacity: 0,
                population: 0,
                active: false,
            },
        ]
    }

    #[test]
    fn channel_scene_layers_list_over_art() {
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_channels(),
            0,
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    fn two_rooms() -> Vec<RoomListEntry> {
        vec![RoomListEntry {
            number: 3,
            state_code: 1,
            title: "Jam".to_owned(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 2,
            game_mode: 0,
            speed: 1,
            capacity: 8,
            user_count: 5,
            min_level: 0,
            max_level: 0,
            skills: vec![],
            premium: false,
            room_type: 0,
        }]
    }

    #[test]
    fn room_scene_layers_rows_over_art() {
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_room_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_rooms(),
            0,
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    /// Room fixture with every detail field set to a distinct value.
    fn detailed_room() -> RoomListEntry {
        RoomListEntry {
            number: 3,
            state_code: 1,
            title: "Jam".to_owned(),
            has_password: true,
            music_id: 757,
            difficulty: 2,
            key_mode: 1,
            game_mode: 1,
            speed: 3,
            capacity: 8,
            user_count: 5,
            min_level: 10,
            max_level: 99,
            skills: vec![],
            premium: false,
            room_type: 0,
        }
    }

    #[test]
    fn room_mode_names_each_game_mode() {
        assert_eq!(room_mode_label(0), "NORMAL");
        assert_eq!(room_mode_label(1), "LIVE");
        assert_eq!(room_mode_label(9), "MODE 9");
    }

    #[test]
    fn room_music_shows_song_id() {
        assert_eq!(room_music_label(757), "M757");
        assert_eq!(room_music_label(0), "M0");
    }

    #[test]
    fn room_difficulty_names_each_band() {
        assert_eq!(room_difficulty_label(0), "EX");
        assert_eq!(room_difficulty_label(1), "NX");
        assert_eq!(room_difficulty_label(2), "HX");
        assert_eq!(room_difficulty_label(3), "RX");
        assert_eq!(room_difficulty_label(9), "DIF 9");
    }

    #[test]
    fn room_key_names_each_key_mode() {
        assert_eq!(room_key_label(0), "3KEY");
        assert_eq!(room_key_label(1), "5KEY");
        assert_eq!(room_key_label(2), "7KEY");
        assert_eq!(room_key_label(7), "KEY 7");
    }

    #[test]
    fn room_speed_names_each_step() {
        let expected = [
            "X0.5", "X1", "X1.5", "X2", "X2.5", "X3", "X3.5", "X4", "X4.5", "X5", "X6", "XR",
        ];
        for (code, label) in expected.iter().enumerate() {
            assert_eq!(&room_speed_label(code as u8), label);
        }
        assert_eq!(room_speed_label(12), "SPD 12");
    }

    #[test]
    fn room_capacity_shows_seats_over_capacity() {
        assert_eq!(room_capacity_label(&detailed_room()), "5/8");
    }

    #[test]
    fn room_level_shows_min_to_max_gate() {
        assert_eq!(room_level_label(&detailed_room()), "LV 10-99");
    }

    #[test]
    fn room_lock_names_password_state() {
        assert_eq!(room_lock_label(true), "LOCK");
        assert_eq!(room_lock_label(false), "OPEN");
        assert_eq!(
            room_lock_ink(true),
            pack_xrgb(0xFF, 0x50, 0x50),
            "locked line reads red"
        );
    }

    #[test]
    fn room_detail_entries_cover_every_panel_line() {
        let entries = room_detail_entries(&detailed_room());
        assert_eq!(entries.len(), ROOM_DETAIL_LINES);
        let ids: Vec<&str> = entries.iter().map(|(id, _)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "each line anchors to its own rect");
        let by_id = |want: &str| {
            entries
                .iter()
                .find(|(id, _)| *id == want)
                .map(|(_, text)| text.clone())
                .unwrap()
        };
        assert_eq!(by_id(ROOM_DETAIL_TITLE_ID), "Jam");
        assert_eq!(by_id(ROOM_DETAIL_USERS_ID), "5/8");
        assert_eq!(by_id(ROOM_DETAIL_MUSIC_ID), "M757");
        assert_eq!(by_id(ROOM_DETAIL_DIFFICULTY_ID), "HX");
        assert_eq!(by_id(ROOM_DETAIL_KEY_ID), "5KEY");
        assert_eq!(by_id(ROOM_DETAIL_MODE_ID), "LIVE");
        assert_eq!(by_id(ROOM_DETAIL_SPEED_ID), "X2");
        assert_eq!(by_id(ROOM_DETAIL_LEVEL_ID), "LV 10-99");
        assert_eq!(by_id(ROOM_DETAIL_LOCK_ID), "LOCK");
    }

    /// Placements pinning the title rect apart from the rest: the
    /// title lands at native (80, 60), every other line at native
    /// (720, 540).
    fn detail_placements() -> Vec<ScenePlacement> {
        let title = ScenePlacement {
            id: ROOM_DETAIL_TITLE_ID.to_owned(),
            code: 0,
            sprite: None,
            rect: Rect {
                x: 80,
                y: 60,
                w: 80,
                h: 60,
            },
        };
        let rest = room_detail_entries(&detailed_room())
            .iter()
            .filter(|(id, _)| *id != ROOM_DETAIL_TITLE_ID)
            .map(|(id, _)| ScenePlacement {
                id: (*id).to_owned(),
                code: 0,
                sprite: None,
                rect: Rect {
                    x: 720,
                    y: 540,
                    w: 80,
                    h: 60,
                },
            })
            .collect::<Vec<_>>();
        std::iter::once(title).chain(rest).collect()
    }

    #[test]
    fn room_details_anchor_to_bnd_rects() {
        // 200x150 frame: native (80, 60) scales to (20, 15), so the
        // title scrim opens at (16, 13) — clear of the list scrim
        // (x 42.., y 35..47) and the centered title.
        let rooms = vec![detailed_room()];
        let mut buf = vec![0u32; 200 * 150];
        paint_room_details(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            &rooms,
            0,
            Some(&detail_placements()),
        );
        assert_eq!(buf[13 * 200 + 16], scrim_color());
        assert!(buf.contains(&art_title_ink()));
    }

    #[test]
    fn room_details_fall_back_under_list_without_staged_ids() {
        // Flat fallback stacks the first detail line under the one
        // list row: x 50, y 49, so its scrim opens at (46, 47) —
        // below the row ink (y 37..42) and the list scrim (y 35..47).
        let rooms = vec![detailed_room()];
        let mut buf = vec![0u32; 200 * 150];
        paint_room_list(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            &rooms,
            0,
        );
        assert_eq!(buf[47 * 200 + 46], scrim_color());
        // List plus cursor keep working under the panel.
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        // Locked room paints the red lock line.
        assert!(buf.contains(&pack_xrgb(0xFF, 0x50, 0x50)));
    }

    #[test]
    fn room_details_stay_empty_without_a_room() {
        let mut buf = vec![0u32; 64 * 48];
        paint_room_list(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &[],
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Room));
        assert!(!buf.contains(&scrim_color()));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0x50, 0x50)));
        assert!(!buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn waiting_scene_keeps_ready_dots_over_art() {
        let art = corner_art();
        let room = waiting_room();
        let mut buf = vec![0u32; 80 * 64];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            &room,
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    #[test]
    fn loading_scene_keeps_bar_over_art() {
        let buf = paint_art(Screen::Loading, &corner_art(), 64, 48);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
    }

    #[test]
    fn result_scene_layers_digits_and_avatar_over_art() {
        let art = corner_art();
        let score = o2jam_game::Score {
            points: 200,
            max_combo: 7,
            ..o2jam_game::Score::default()
        };
        let mut buf = vec![0u32; 64 * 48];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            Some(&red_figure()),
            &score,
            &art,
            None,
        );
        // Logo pixel at the origin, avatar red inside the frame, and
        // the white score digits top-left.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(255, 0, 0)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn playing_overlay_keeps_highway_marks() {
        let buf = paint_art(Screen::Playing, &corner_art(), 70, 60);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0xE0, 0xE0, 0xE0)));
    }

    #[test]
    fn empty_art_falls_back_to_flat() {
        // No figures staged: the backdrop keeps the flat fallback
        // so a bare frame never shows garbage. Staged scenes (see
        // `every_art_scene_hides_flat_fill`) never take this path.
        let art = SceneArt::default();
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            assert!(!art.has_figures());
            let buf = paint_art(screen, &art, 16, 12);
            assert_eq!(buf[0], background_of(screen), "{screen:?}");
        }
    }

    fn battle_room() -> RoomTeams {
        let mut teams = RoomTeams::new();
        teams.set_team(0, 0);
        teams.set_team(1, 1);
        teams.submit(0, 100_000);
        teams.submit(1, 150_000);
        teams
    }

    #[test]
    fn result_loader_names_top_bottom_and_winlose() {
        assert_eq!(
            result_loader_sprites(),
            [
                "StateResult_Top.ojs",
                "StateResult_Bottom.ojs",
                "Result_WinLose.ojs"
            ]
        );
        assert_eq!(RESULT_WINLOSE_SPRITE, "Result_WinLose.ojs");
        assert_eq!(WINNER_IMAGE_SPRITE, "Winner_image.ojs");
    }

    #[test]
    fn battle_verdict_goes_to_rank_one_by_score() {
        let teams = battle_room();
        assert_eq!(teams.winner(), Some(1));
        assert_eq!(battle_verdict(&teams, Some(1)), Some(BattleVerdict::Win));
        assert_eq!(battle_verdict(&teams, Some(0)), Some(BattleVerdict::Lose));
    }

    #[test]
    fn battle_verdict_stays_empty_without_winner_or_self() {
        let empty = RoomTeams::new();
        assert_eq!(battle_verdict(&empty, Some(0)), None);
        let teams = battle_room();
        assert_eq!(battle_verdict(&teams, None), None);
    }

    #[test]
    fn result_win_paints_gold_caption() {
        let mut buf = vec![0u32; 64 * 64];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 64,
            },
            None,
            &o2jam_game::Score::default(),
            Some(BattleVerdict::Win),
        );
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn result_lose_paints_gray_caption() {
        let mut buf = vec![0u32; 64 * 64];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 64,
            },
            None,
            &o2jam_game::Score::default(),
            Some(BattleVerdict::Lose),
        );
        assert!(buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn result_without_verdict_paints_no_caption() {
        let mut buf = vec![0u32; 64 * 64];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 64,
            },
            None,
            &o2jam_game::Score::default(),
            None,
        );
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        assert!(!buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn result_scene_layers_verdict_over_staged_art() {
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 64];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 64,
            },
            None,
            &o2jam_game::Score::default(),
            &art,
            Some(BattleVerdict::Win),
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }
}
