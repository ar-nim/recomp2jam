//! Screen scenes: real `.ojs`/`.bnd` art behind each [`Screen`],
//! with the working UI on top.
//!
//! Each scene paints its verified archive art first (lifecycle §5–11:
//! `O2mediaLogo.ojs`, channel/room/waiting/playing/result blocks,
//! `MusicLoading.ojs`) through the scene kit in [`crate::avatar`]
//! (`load_scene_figures`, `bind_scene_rects`, `paint_scene_figures`,
//! `NATIVE_WIDTH`/`HEIGHT`), then layers the live UI over it: channel
//! list, room list, waiting seats, loading bar, highway, result
//! digits. Layouts stay relative to the frame so headless tests can
//! paint tiny buffers. Bare [`paint_screen`] keeps the flat-color
//! fallback while no art is staged.

use std::collections::HashMap;

use super::super::avatar::{
    paint_scene_figures, scene_block, scene_sprite_names, AvatarFigure, ScenePlacement, AVI_BLOCK,
    AVI_LOGO_SPRITE, CHANNEL_BLOCK, LOADING_SPRITE, RESULT_BLOCK, ROOM_BLOCK, WAITING_BLOCK,
};
use super::super::playing::{HighwayGeometry, PlayingView};
use super::super::screen::Screen;
use o2jam_file::ControlList;
use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomSlot, RoomSlotState};
use o2jam_render::{
    draw_number, draw_text, fill_rect, number_width, pack_xrgb, text_width, Frame, Rect,
    NATIVE_HEIGHT, NATIVE_WIDTH,
};

/// Gameplay block: real UI lives in `Playing1.opi` (235 entries,
/// lifecycle §10).
pub const PLAYING_BLOCK: &str = "STATE_PLAYING";
/// Gameplay layout bind file (17 rects over the entry subset).
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

    /// Paint background plus every placed figure at its scaled
    /// layout origin; placements without a loaded figure are skipped.
    pub fn paint_backdrop(&self, frame: &mut Frame<'_>, screen: Screen) {
        fill_background(frame, screen);
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
/// every sprite covers the native canvas, so `paint_scene_figures`
/// draws each body full-bleed from the frame origin.
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
                w: NATIVE_WIDTH,
                h: NATIVE_HEIGHT,
            },
        })
        .collect()
}

/// Title ink on lobby cards (dark on the light card).
fn title_color() -> u32 {
    pack_xrgb(0x20, 0x20, 0x20)
}

/// Row height of one channel entry.
const CHANNEL_ROW_H: i32 = 8;

/// Channel rows plus highlight cursor (no background fill: shared by
/// the flat fallback and the art scene).
fn paint_channel_rows(frame: &mut Frame<'_>, channels: &[ChannelState], highlight: usize) {
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
    paint_title(frame, "CHANNEL");
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

/// Paint the online room list with a highlight cursor.
pub(crate) fn paint_room_list(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
    fill_background(frame, Screen::Room);
    paint_title(frame, "ROOM");
    paint_room_rows(frame, rooms, highlight);
}

/// Paint staged lobby art with the live room list and cursor on top.
pub fn paint_room_scene(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: &SceneArt,
) {
    art.paint_backdrop(frame, Screen::Room);
    paint_title(frame, "ROOM");
    paint_room_rows(frame, rooms, highlight);
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
    paint_title(frame, "WAITING");
    paint_waiting_rows(frame, room);
}

/// Paint one seat row plus its ready dot (vacant seats have no dot).
fn paint_waiting_seat(frame: &mut Frame<'_>, index: usize, seat: &WaitingSeat) {
    let occupied = seat.display_name().is_some();
    let (x, y) = seat_origin(index, frame.w, frame.h);
    let ink = if occupied {
        pack_xrgb(0xFF, 0xFF, 0xFF)
    } else {
        pack_xrgb(0x70, 0x70, 0x70)
    };
    draw_text(frame, x, y, &waiting_row_text(index, seat), ink);
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
    let text_w = text_width(title) as i32;
    draw_text(
        frame,
        frame.w as i32 / 2 - text_w / 2,
        frame.h as i32 / 2 - frame.h as i32 / 8 + 2,
        title,
        title_color(),
    );
}

/// Staged lobby art behind an empty list: backdrop plus title.
pub fn paint_lobby_backdrop(frame: &mut Frame<'_>, screen: Screen, art: &SceneArt) {
    art.paint_backdrop(frame, screen);
    paint_title(frame, title_of(screen));
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

/// Loading progress bar (no background fill: shared by the flat
/// fallback and the art scene).
fn paint_loading_bar(frame: &mut Frame<'_>) {
    let (w, h) = (frame.w as i32, frame.h as i32);
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

/// Score, best combo, and avatar figure (no background fill: shared
/// by the flat fallback and the art scene). Grade letters stay out
/// (mapping OPEN).
fn paint_result_content(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
) {
    if let Some(figure) = avatar {
        figure.paint(frame, frame.w as i32 / 2 - 32, frame.h as i32 / 2 - 32);
    }
    let points = u32::try_from(score.points).unwrap_or(u32::MAX);
    draw_number(frame, 16, 16, points, pack_xrgb(0xFF, 0xFF, 0xFF));
    draw_number(frame, 16, 32, score.max_combo, pack_xrgb(0xFF, 0xFF, 0x40));
}

/// Paint the results scene: score, best combo, and a loaded avatar
/// figure over the card. Grade letters stay out (mapping OPEN).
pub(crate) fn paint_result(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
) {
    paint_card(
        frame,
        pack_xrgb(0x10, 0x10, 0x08),
        pack_xrgb(0xC0, 0xA0, 0x20),
    );
    paint_result_content(frame, avatar, score);
}

/// Paint staged result art (`Result_WinLose.ojs`, number fonts) with
/// the avatar figure and live digits on top.
pub fn paint_result_scene(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
    art: &SceneArt,
) {
    art.paint_backdrop(frame, Screen::Result);
    paint_result_content(frame, avatar, score);
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
        Screen::Result => paint_result(frame, None, &o2jam_game::Score::default()),
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
                paint_result_scene(&mut frame, None, &o2jam_game::Score::default(), art)
            }
        }
        buf
    }

    #[test]
    fn splash_covers_background_with_card() {
        let buf = painted(Screen::Avi, 32, 24);
        assert_eq!(buf[0], pack_xrgb(0x08, 0x08, 0x10));
        // Card top-left corner, above the version stamp.
        assert_eq!(buf[9 * 32 + 12], pack_xrgb(0x20, 0x60, 0xC0));
    }

    #[test]
    fn splash_stamps_version_in_card() {
        // 32x24 splash: "755" starts at x = 16 - 11/2 = 11, y = 10;
        // the '7' top bar lights the first three pixels.
        let buf = painted(Screen::Avi, 32, 24);
        let white = pack_xrgb(0xFF, 0xFF, 0xFF);
        assert_eq!(buf[10 * 32 + 11], white);
        assert_eq!(buf[10 * 32 + 12], white);
        assert_eq!(buf[10 * 32 + 13], white);
        assert_eq!(buf[10 * 32 + 14], pack_xrgb(0x20, 0x60, 0xC0));
    }

    #[test]
    fn every_screen_paints_its_background() {
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
    fn unbound_placements_span_the_native_canvas() {
        let placed = unbound_scene_placements(&["A.ojs".to_owned(), "B.ojs".to_owned()]);
        assert_eq!(placed.len(), 2);
        for placement in &placed {
            assert_eq!(
                placement.rect,
                Rect {
                    x: 0,
                    y: 0,
                    w: NATIVE_WIDTH,
                    h: NATIVE_HEIGHT,
                }
            );
        }
        assert_eq!(placed[0].sprite.as_deref(), Some("A.ojs"));
    }

    #[test]
    fn lobby_cards_carry_titles() {
        // 128x96 channel card spans x 48..80, y 36..60; the title
        // starts at x = 64 - 27/2 = 51, y = 38, so the 'C' top
        // bar (011) lights x+1..x+2 on card gray.
        let buf = painted(Screen::Channel, 128, 96);
        let ink = pack_xrgb(0x20, 0x20, 0x20);
        let card = pack_xrgb(0xD0, 0xD0, 0xD0);
        assert_eq!(buf[38 * 128 + 51], card);
        assert_eq!(buf[38 * 128 + 52], ink);
        assert_eq!(buf[38 * 128 + 53], ink);
        let room = painted(Screen::Room, 128, 96);
        assert!(room.contains(&ink));
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
        // Seat 2 (empty) has no dot: background shows through.
        let (empty_x, empty_y) = seat_origin(2, 80, 64);
        assert_eq!(
            buf[(empty_y + 1) as usize * 80 + (empty_x - 6 + 1) as usize],
            waiting_bg
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
        // flat splash whites out keeps the logo background instead.
        let buf = paint_art(Screen::Avi, &corner_art(), 32, 24);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[10 * 32 + 11], background_of(Screen::Avi));
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
    fn empty_backdrops_match_plain_backgrounds() {
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
}
