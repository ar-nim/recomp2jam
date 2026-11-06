//! [`ScreenGame`]: the window's game loop, owning the [`ScreenFlow`]
//! and walking screens on input.

use std::collections::HashMap;

use super::super::avatar::AvatarFigure;
use super::super::playing::{HighwayGeometry, LivePlay};
use super::super::screen::{Screen, ScreenFlow};
use super::super::GameLoop;
use super::paint::{
    paint_channel_list, paint_result, paint_room_list, paint_screen, paint_waiting_members,
};
use crate::MenuKey;
use o2jam_audio::{AudioBackend, PcmSamples};
use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry};
use o2jam_render::Frame;

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
    waiting: Option<JoinedRoom>,
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
            playing: LivePlay::new(&[]),
            started_at: None,
            audio: None,
            avatar: None,
            splash: None,
            channels: Vec::new(),
            highlight: 0,
            rooms: Vec::new(),
            room_highlight: 0,
            waiting: None,
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
        self.playing = LivePlay::new(&onsets);
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

    /// Song position: wall clock since entering Playing (saturates).
    fn now_ms(&mut self) -> u64 {
        let start = *self.started_at.get_or_insert_with(std::time::Instant::now);
        u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
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

    /// Remember the joined room; Waiting paints its eight seats.
    pub fn set_joined_room(&mut self, room: JoinedRoom) -> &mut Self {
        self.waiting = Some(room);
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
            Screen::Waiting => {
                if let Some(room) = &self.waiting {
                    paint_waiting_members(frame, room);
                } else {
                    paint_screen(frame, Screen::Waiting);
                }
            }
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

    fn menu_key(&mut self, key: MenuKey, down: bool) {
        if !down {
            return;
        }
        match (self.flow.current(), key) {
            (Screen::Channel, MenuKey::Up) => {
                self.highlight = self.highlight.saturating_sub(1);
            }
            (Screen::Channel, MenuKey::Down) => {
                let last = self.channels.len().saturating_sub(1);
                self.highlight = (self.highlight + 1).min(last);
            }
            (Screen::Room, MenuKey::Up) => {
                self.room_highlight = self.room_highlight.saturating_sub(1);
            }
            (Screen::Room, MenuKey::Down) => {
                let last = self.rooms.len().saturating_sub(1);
                self.room_highlight = (self.room_highlight + 1).min(last);
            }
            (Screen::Loading, MenuKey::Confirm) => self.enter(Screen::Playing),
            (Screen::Room, MenuKey::Back) => self.enter(Screen::Channel),
            (Screen::Channel, MenuKey::Confirm)
            | (Screen::Waiting, MenuKey::Back)
            | (Screen::Result, MenuKey::Back) => self.enter(Screen::Room),
            (Screen::Room, MenuKey::Confirm)
            | (Screen::Loading, MenuKey::Back)
            | (Screen::Result, MenuKey::Confirm) => self.enter(Screen::Waiting),
            (Screen::Waiting, MenuKey::Confirm) => self.enter(Screen::Loading),
            _ => {}
        }
    }

    fn paint(&mut self, frame: &mut Frame<'_>) {
        let now = self.now_ms();
        self.paint_at(frame, now);
    }
}

#[cfg(test)]
mod game_tests {
    use super::super::paint::{background_of, seat_origin};
    use super::*;
    use crate::avatar::AvatarFigure;
    use crate::screen::Screen;
    use crate::{GameLoop, MenuKey};
    use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
    use o2jam_render::{pack_xrgb, Frame};

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
        use MenuKey;
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
    }

    #[test]
    fn menu_walks_lobbies_to_playing() {
        use MenuKey;
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
        use MenuKey;
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
        use MenuKey;
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
        use MenuKey;
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
        game.menu_key(super::MenuKey::Confirm, true);
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
            slots: vec![RoomSlot {
                index: 0,
                state_code: 1,
                member: Some(member),
            }],
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    fn walk_to_waiting(game: &mut ScreenGame) {
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn waiting_paints_joined_members_with_ready_flags() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        let mut buf = vec![0u32; 80 * 64];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Waiting));
        // Ready host sits at seat 1: green ready dot left of its row.
        let (x, y) = seat_origin(1, 80, 64);
        assert_eq!(
            buf[(y + 1) as usize * 80 + (x - 6 + 1) as usize],
            pack_xrgb(0x30, 0xE0, 0x30)
        );
    }

    #[test]
    fn waiting_without_room_paints_lobby_card() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        let mut buf = vec![0u32; 16 * 12];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 16,
                h: 12,
            },
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Waiting));
    }

    #[test]
    fn splash_shows_loaded_art() {
        use crate::avatar::{AvatarFigure, FigureLayer};
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
}
