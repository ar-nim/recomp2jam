//! Screen painters: one flat-color scene per [`Screen`].
//!
//! Placeholder art until the OPI sprite pipeline lands: each screen
//! gets a distinct background plus a centered card, all via
//! [`fill_rect`] (clipped, so any buffer size is safe). Layouts are
//! relative to the frame so headless tests can paint tiny buffers.
//! [`ScreenGame`] owns a [`ScreenFlow`] and advances splash → channel
//! on any lane key; later screens wire their own inputs next.

use std::collections::HashMap;

use super::avatar::AvatarFigure;
use super::playing::{HighwayGeometry, LivePlay, PlayingView};
use super::screen::{Screen, ScreenFlow};
use super::GameLoop;
use o2jam_audio::{AudioBackend, PcmSamples};
use o2jam_net::{ChannelState, RoomListEntry};
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
fn paint_channel_list(frame: &mut Frame<'_>, channels: &[ChannelState], highlight: usize) {
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
fn paint_room_list(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
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
fn paint_result(frame: &mut Frame<'_>, avatar: Option<&AvatarFigure>, score: &o2jam_game::Score) {
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
fn background_of(screen: Screen) -> u32 {
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
            PlayingView::default().paint(frame, &HighwayGeometry::centered(frame.w, frame.h))
        }
        Screen::Result => paint_result(frame, None, &o2jam_game::Score::default()),
    }
}

/// Game loop that shows each screen's scene and walks the flow.
///
/// Input wiring so far: any lane key dismisses the splash into the
/// channel screen. Room/waiting/loading inputs land with their UIs.
pub struct ScreenGame {
    flow: ScreenFlow,
    playing: LivePlay,
    started_at: Option<std::time::Instant>,
    audio: Option<AudioRig>,
    avatar: Option<AvatarFigure>,
    splash: Option<AvatarFigure>,
    channels: Vec<ChannelState>,
    highlight: usize,
    rooms: Vec<RoomListEntry>,
    room_highlight: usize,
}

/// Key-sound bank plus backend for windowed play.
struct AudioRig {
    backend: Box<dyn AudioBackend>,
    bank: HashMap<u32, PcmSamples>,
}

impl ScreenGame {
    /// Start at the splash screen.
    #[must_use]
    pub fn new() -> Self {
        Self {
            flow: ScreenFlow::new(),
            playing: LivePlay::new(Vec::new()),
            started_at: None,
            audio: None,
            avatar: None,
            splash: None,
            channels: Vec::new(),
            highlight: 0,
            rooms: Vec::new(),
            room_highlight: 0,
        }
    }

    /// Load the result-screen avatar figure.
    pub fn load_avatar(&mut self, figure: AvatarFigure) -> &mut Self {
        self.avatar = Some(figure);
        self
    }

    /// Load real splash art (e.g. `O2mediaLogo.ojs`); the placeholder
    /// shows while none is loaded.
    pub fn load_splash(&mut self, figure: AvatarFigure) -> &mut Self {
        self.splash = Some(figure);
        self
    }

    /// Attach key sounds; struck notes play through the backend.
    /// Missing samples stay silent (no bank entry, no sound).
    pub fn with_audio(
        &mut self,
        backend: Box<dyn AudioBackend>,
        bank: HashMap<u32, PcmSamples>,
    ) -> &mut Self {
        self.audio = Some(AudioRig { backend, bank });
        self
    }

    /// Load a chart's onsets (with key-sound ids) into the highway.
    pub fn load_chart(&mut self, onsets: Vec<(u8, u64, u32)>) {
        self.playing = LivePlay::new(onsets);
    }

    /// Play every queued key sound through the attached backend.
    fn pump_sounds(&mut self) {
        let Some(rig) = self.audio.as_mut() else {
            self.playing.drain_sounds();
            return;
        };
        for sample in self.playing.drain_sounds() {
            if let Some(pcm) = rig.bank.get(&sample) {
                let _ = rig.backend.play_pcm(pcm);
            }
        }
    }

    /// Song position: wall clock since entering Playing.
    fn now_ms(&mut self) -> u64 {
        let start = *self.started_at.get_or_insert_with(std::time::Instant::now);
        start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }

    /// Step the flow, restarting the song clock on Playing entry.
    fn enter(&mut self, next: Screen) {
        if self.flow.go(next).is_ok() && next == Screen::Playing {
            self.started_at = Some(std::time::Instant::now());
        }
    }

    /// Strike a lane at an explicit clock, playing its key sound.
    /// Tests use this; live keys read the wall clock.
    pub fn strike_at(&mut self, lane: usize, now_ms: u64) {
        self.playing.strike_lane(lane as u8, now_ms);
        self.pump_sounds();
    }

    /// Screen currently shown.
    #[must_use]
    pub fn screen(&self) -> Screen {
        self.flow.current()
    }

    /// Channels from the last online list (empty offline).
    pub fn set_channels(&mut self, channels: Vec<ChannelState>) -> &mut Self {
        self.channels = channels;
        self.highlight = 0;
        self
    }

    /// Rooms from the last online list (empty offline). Empty slots
    /// (`Number` 0, which the server sends for every free slot) are
    /// not listed — display choice, the data stays unfiltered upstream.
    pub fn set_rooms(&mut self, rooms: Vec<RoomListEntry>) -> &mut Self {
        self.rooms = rooms.into_iter().filter(|room| room.number != 0).collect();
        self.room_highlight = 0;
        self
    }

    /// Paint with an explicit clock (tests use this; live paint reads
    /// the wall clock).
    pub fn paint_at(&mut self, frame: &mut Frame<'_>, now_ms: u64) {
        if self.flow.current() == Screen::Playing && self.playing.is_finished(now_ms) {
            let _ = self.flow.go(Screen::Result);
        }
        match self.flow.current() {
            Screen::Playing => {
                let geom = HighwayGeometry::centered(frame.w, frame.h);
                self.playing.paint(frame, &geom, now_ms);
            }
            Screen::Avi => {
                paint_screen(frame, Screen::Avi);
                if let Some(logo) = &self.splash {
                    logo.paint(frame, 0, 0);
                }
            }
            Screen::Channel if !self.channels.is_empty() => {
                paint_channel_list(frame, &self.channels, self.highlight);
            }
            Screen::Room if !self.rooms.is_empty() => {
                paint_room_list(frame, &self.rooms, self.room_highlight);
            }
            Screen::Result => paint_result(frame, self.avatar.as_ref(), self.playing.score()),
            screen => paint_screen(frame, screen),
        }
    }
}

impl Default for ScreenGame {
    fn default() -> Self {
        Self::new()
    }
}

impl GameLoop for ScreenGame {
    fn lane_key(&mut self, lane: usize, down: bool) {
        if !down {
            return;
        }
        match self.flow.current() {
            Screen::Avi => {
                let _ = self.flow.go(Screen::Channel);
            }
            Screen::Playing => {
                let now = self.now_ms();
                self.strike_at(lane, now);
            }
            _ => {}
        }
    }

    fn menu_key(&mut self, key: super::MenuKey, down: bool) {
        if !down {
            return;
        }
        match (self.flow.current(), key) {
            (Screen::Channel, super::MenuKey::Confirm) => self.enter(Screen::Room),
            (Screen::Channel, super::MenuKey::Up) => {
                self.highlight = self.highlight.saturating_sub(1);
            }
            (Screen::Channel, super::MenuKey::Down) => {
                let last = self.channels.len().saturating_sub(1);
                self.highlight = (self.highlight + 1).min(last);
            }
            (Screen::Room, super::MenuKey::Confirm) => self.enter(Screen::Waiting),
            (Screen::Room, super::MenuKey::Back) => self.enter(Screen::Channel),
            (Screen::Room, super::MenuKey::Up) => {
                self.room_highlight = self.room_highlight.saturating_sub(1);
            }
            (Screen::Room, super::MenuKey::Down) => {
                let last = self.rooms.len().saturating_sub(1);
                self.room_highlight = (self.room_highlight + 1).min(last);
            }
            (Screen::Waiting, super::MenuKey::Confirm) => self.enter(Screen::Loading),
            (Screen::Waiting, super::MenuKey::Back) => self.enter(Screen::Room),
            (Screen::Loading, super::MenuKey::Confirm) => self.enter(Screen::Playing),
            (Screen::Loading, super::MenuKey::Back) => self.enter(Screen::Waiting),
            (Screen::Result, super::MenuKey::Confirm) => self.enter(Screen::Waiting),
            (Screen::Result, super::MenuKey::Back) => self.enter(Screen::Room),
            _ => {}
        }
    }

    fn paint(&mut self, frame: &mut Frame<'_>) {
        let now = self.now_ms();
        self.paint_at(frame, now);
    }
}

#[cfg(test)]
mod screen_paint_tests {
    use super::*;

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
    fn splash_shows_loaded_art() {
        use super::super::{AvatarFigure, FigureLayer};
        let mut logo = AvatarFigure::default();
        logo.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        let mut game = ScreenGame::new();
        game.load_splash(logo);
        let mut buf = vec![0u32; 8 * 8];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 8,
                h: 8,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[63], pack_xrgb(0x08, 0x08, 0x10));
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

    #[test]
    fn loaded_chart_scales_notes_down_the_highway() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 1500, 7)]);
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
        ] {
            game.flow.go(next).unwrap();
        }
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            1000,
        );
        let note_y = 50 - (500.0 * 50.0 / 2000.0) as i32;
        // Lane 2 spans x 20..30; the note insets one pixel each side.
        assert_eq!(buf[note_y as usize * 70 + 22], pack_xrgb(0x40, 0xC0, 0xFF));
        assert_eq!(buf[0], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn strikes_play_banked_samples() {
        use o2jam_audio::{NullBackend, PcmSamples};
        let mut game = ScreenGame::new();
        let bank = HashMap::from([(
            7u32,
            PcmSamples {
                samples: vec![0i16; 8],
                sample_rate: 8000,
                channels: 1,
            },
        )]);
        game.with_audio(Box::new(NullBackend::new()), bank);
        game.load_chart(vec![(2, 5000, 7), (2, 6000, 999)]);
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
        ] {
            game.flow.go(next).unwrap();
        }
        game.strike_at(2, 5000);
        assert_eq!(game.playing.score().combo, 1);
        assert!(game.playing.drain_sounds().is_empty());
    }

    #[test]
    fn result_paints_loaded_avatar_over_card() {
        use o2jam_file::{parse_sprite55, SPRITE55_MAGIC};
        // 2x1 sprite-55: transparent black, then red.
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        for field in [0u16, 0u16, 2u16, 1u16] {
            blob.extend_from_slice(&field.to_le_bytes());
        }
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&4u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x7C]);
        let sprite = parse_sprite55(&blob).unwrap();
        let mut game = ScreenGame::new();
        game.load_avatar(AvatarFigure::from_sprite55(&sprite));
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            game.flow.go(next).unwrap();
        }
        // 256x256 result: card spans 96..160 both ways and the
        // figure paints at (96, 96); the red pixel lands at
        // (97, 96), the transparent one keeps card gray.
        let mut buf = vec![0u32; 256 * 256];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 256,
                h: 256,
            },
            0,
        );
        assert_eq!(buf[96 * 256 + 97], pack_xrgb(255, 0, 0));
        assert_eq!(buf[96 * 256 + 96], pack_xrgb(0xC0, 0xA0, 0x20));
    }

    fn walk_to_playing(game: &mut ScreenGame) {
        use super::super::MenuKey;
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
    }

    #[test]
    fn menu_walks_lobbies_to_playing() {
        use super::super::MenuKey;
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
        assert!(game.started_at.is_some());
    }

    #[test]
    fn menu_backs_out_of_lobbies() {
        use super::super::MenuKey;
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn chart_end_moves_to_result_with_score() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            7000,
        );
        assert_eq!(game.screen(), Screen::Result);
        // Score "200" top-left in white.
        assert_eq!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
    }

    #[test]
    fn unfinished_chart_stays_playing() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            4995,
        );
        assert_eq!(game.screen(), Screen::Playing);
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
    fn highlight_moves_with_bounds() {
        use super::super::MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Up, true);
        game.menu_key(MenuKey::Up, true);
        game.menu_key(MenuKey::Up, true);
        // Down,Down clamps at 1; three Ups clamp back at 0.
        let mut probe = ScreenGame::new();
        probe.set_channels(two_channels());
        probe.lane_key(0, true);
        probe.menu_key(MenuKey::Down, true);
        probe.menu_key(MenuKey::Down, true);
        assert_eq!(game.highlight, 0);
        assert_eq!(probe.highlight, 1);
        // Empty list never moves.
        let mut bare = ScreenGame::new();
        bare.lane_key(0, true);
        bare.menu_key(MenuKey::Down, true);
        assert_eq!(bare.highlight, 0);
    }

    #[test]
    fn channel_list_paints_text_and_cursor() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        // 64x48 list: rows from y 12, text x 16; '2' top bar first.
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[12 * 64 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
        assert_eq!(buf[12 * 64 + 10], pack_xrgb(0xFF, 0xD0, 0x20));
        assert_eq!(buf[0], background_of(Screen::Channel));
    }

    fn two_rooms() -> Vec<RoomListEntry> {
        vec![
            RoomListEntry {
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
            },
            RoomListEntry {
                number: 7,
                state_code: 1,
                title: "Locked".to_owned(),
                has_password: true,
                music_id: 0,
                difficulty: 0,
                key_mode: 2,
                game_mode: 0,
                speed: 1,
                capacity: 8,
                user_count: 1,
                min_level: 0,
                max_level: 0,
                skills: vec![],
                premium: false,
                room_type: 0,
            },
        ]
    }

    #[test]
    fn room_highlight_moves_with_bounds() {
        use super::super::MenuKey;
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Down, true);
        assert_eq!(game.room_highlight, 1);
        game.menu_key(MenuKey::Up, true);
        assert_eq!(game.room_highlight, 0);
    }

    #[test]
    fn room_list_paints_rows_and_cursor() {
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(super::super::MenuKey::Confirm, true);
        // 64x48 list: rows from y 12, text x 16; '3' top bar first.
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[12 * 64 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
        assert_eq!(buf[12 * 64 + 10], pack_xrgb(0xFF, 0xD0, 0x20));
        assert_eq!(buf[0], background_of(Screen::Room));
    }

    #[test]
    fn lane_key_dismisses_splash_into_channel() {
        let mut game = ScreenGame::new();
        assert_eq!(game.screen(), Screen::Avi);
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        let mut buf = vec![0u32; 16 * 12];
        game.paint(&mut Frame {
            buf: &mut buf,
            w: 16,
            h: 12,
        });
        assert_eq!(buf[0], background_of(Screen::Channel));
    }

    #[test]
    fn lane_release_keeps_splash() {
        let mut game = ScreenGame::new();
        game.lane_key(0, false);
        assert_eq!(game.screen(), Screen::Avi);
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
