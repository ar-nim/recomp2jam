//! Screen scenes: flat-color placeholder art per [`Screen`].
//!
//! Layouts stay relative to the frame so headless tests can paint
//! tiny buffers. Real sprite art overrides per screen as it lands.

use super::super::avatar::AvatarFigure;
use super::super::playing::{HighwayGeometry, PlayingView};
use super::super::screen::Screen;
use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomSlot, RoomSlotState};
use o2jam_render::{
    draw_number, draw_text, fill_rect, number_width, pack_xrgb, text_width, Frame, Rect,
};

/// Title ink on lobby cards (dark on the light card).
fn title_color() -> u32 {
    pack_xrgb(0x20, 0x20, 0x20)
}

/// Row height of one channel entry.
const CHANNEL_ROW_H: i32 = 8;

/// Paint the online channel list with a highlight cursor.
pub(crate) fn paint_channel_list(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    highlight: usize,
) {
    let (w, h) = (frame.w as i32, frame.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: w as u32,
            h: h as u32,
        },
        background_of(Screen::Channel),
    );
    paint_title(frame, "CHANNEL");
    let text_x = w / 4;
    for (i, channel) in channels.iter().enumerate() {
        let y = h / 4 + i as i32 * CHANNEL_ROW_H;
        let entry = format!(
            "{} {}/{}",
            channel.channel_id, channel.population, channel.capacity
        );
        let ink = if channel.active {
            pack_xrgb(0xFF, 0xFF, 0xFF)
        } else {
            pack_xrgb(0x70, 0x70, 0x70)
        };
        draw_text(frame, text_x, y, &entry, ink);
        if i == highlight {
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
    }
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

/// Paint the online room list with a highlight cursor.
pub(crate) fn paint_room_list(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
    let (w, h) = (frame.w as i32, frame.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: w as u32,
            h: h as u32,
        },
        background_of(Screen::Room),
    );
    paint_title(frame, "ROOM");
    let text_x = w / 4;
    for (i, room) in rooms.iter().enumerate() {
        let y = h / 4 + i as i32 * CHANNEL_ROW_H;
        draw_text(
            frame,
            text_x,
            y,
            &room_row_text(room),
            pack_xrgb(0xFF, 0xFF, 0xFF),
        );
        if i == highlight {
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
    }
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

/// Paint the eight waiting-room seats with names and ready flags.
pub(crate) fn paint_waiting_members(frame: &mut Frame<'_>, room: &JoinedRoom) {
    let (w, h) = (frame.w, frame.h);
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: w as u32,
            h: h as u32,
        },
        background_of(Screen::Waiting),
    );
    paint_title(frame, "WAITING");
    for (index, seat) in waiting_seats(room).iter().enumerate() {
        paint_waiting_seat(frame, index, seat);
    }
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

/// Paint a lobby scene (channel / room / waiting share the shape).
fn paint_lobby(frame: &mut Frame<'_>, background: u32) {
    paint_card(frame, background, pack_xrgb(0xD0, 0xD0, 0xD0));
}

/// Paint the loading gate scene.
fn paint_loading(frame: &mut Frame<'_>) {
    let (w, h) = (frame.w as i32, frame.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: w as u32,
            h: h as u32,
        },
        pack_xrgb(0x18, 0x18, 0x18),
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
    if let Some(figure) = avatar {
        figure.paint(frame, frame.w as i32 / 2 - 32, frame.h as i32 / 2 - 32);
    }
    let points = u32::try_from(score.points).unwrap_or(u32::MAX);
    draw_number(frame, 16, 16, points, pack_xrgb(0xFF, 0xFF, 0xFF));
    draw_number(frame, 16, 32, score.max_combo, pack_xrgb(0xFF, 0xFF, 0x40));
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
    use crate::screen::Screen;
    use o2jam_net::{JoinedRoom, RoomMember, RoomSlot};
    use o2jam_render::{pack_xrgb, Frame};

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
}
