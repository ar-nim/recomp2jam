//! [`ScreenGame`]: the window's game loop, owning the [`ScreenFlow`]
//! and walking screens on input.

use std::collections::HashMap;

use super::super::avatar::{AvatarFigure, ScenePlacement};
use super::super::playing::{HighwayGeometry, LivePlay, PlaySkin};
use super::super::screen::{Screen, ScreenFlow};
use super::super::GameLoop;
use super::paint::{
    battle_verdict, paint_avi_scene, paint_channel_list, paint_channel_scene, paint_loading_scene,
    paint_lobby_backdrop, paint_playing_overlay, paint_result, paint_result_scene, paint_room_list,
    paint_room_scene, paint_screen, paint_waiting_members, paint_waiting_scene, scene_index,
    BattleVerdict, SceneArt,
};
use super::side::{
    can_return_side, can_visit_side, paint_side_scene, waiting_song_detail_line,
    waiting_song_title_line, SideScreen,
};
use crate::MenuKey;
use o2jam_audio::{AudioBackend, PcmSamples};
use o2jam_file::OjnListEntry;
use o2jam_game::RoomTeams;
use o2jam_net::play::SetRoomMusic;
use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry};
use o2jam_render::{draw_text, fill_rect, pack_xrgb, text_width, Frame, Rect};

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
    selected_music: Option<SetRoomMusic>,
    song_titles: HashMap<u16, String>,
    select_music_art: Option<SceneArt>,
    battle_teams: RoomTeams,
    battle_self_team: Option<u8>,
    side: Option<SideScreen>,
    side_origin: Option<Screen>,
    side_placements: Vec<ScenePlacement>,
    side_figures: HashMap<String, AvatarFigure>,
    scene_art: [Option<SceneArt>; 7],
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
            selected_music: None,
            song_titles: HashMap::new(),
            select_music_art: None,
            battle_teams: RoomTeams::new(),
            battle_self_team: None,
            side: None,
            side_origin: None,
            side_placements: Vec::new(),
            side_figures: HashMap::new(),
            scene_art: std::array::from_fn(|_| None),
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

    /// Stage real note/key art into the highway paint (procedural
    /// fallback stays on unstaged lanes).
    pub fn set_play_skin(&mut self, skin: PlaySkin) {
        self.playing.set_skin(skin);
    }

    /// Caption the avatar scene with the local player (`Lv.<n> <name>`;
    /// display only, scoring untouched).
    pub fn set_player(&mut self, level: u32, name: &str) {
        self.playing.scene_mut().set_player(level, name);
    }

    /// Caption the avatar scene song line (`X3.5 <title>`); the waiting
    /// choice re-syncs each Playing paint while one is selected.
    pub fn set_song_caption(&mut self, title: &str, speed_code: u8) {
        self.playing.scene_mut().set_song(title, speed_code);
    }

    /// Caption the VOL readout (display only).
    pub fn set_volume(&mut self, volume: u8) {
        self.playing.scene_mut().set_volume(volume);
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

    /// Remember the host song choice (`SetRoomMusic` 4000 body);
    /// Waiting paints its title and difficulty panel.
    pub fn set_selected_music(&mut self, selection: SetRoomMusic) -> &mut Self {
        self.selected_music = Some(selection);
        self
    }

    /// Stage song catalog titles (`OJNList.dat` order, read-only copy).
    /// Missing catalogs paint dialog art without rows.
    pub fn set_song_catalog(&mut self, entries: &[OjnListEntry<'_>]) -> &mut Self {
        self.song_titles.clear();
        for entry in entries.iter().filter(|entry| entry.is_song()) {
            let Ok(song_id) = u16::try_from(entry.song_id) else {
                continue;
            };
            self.song_titles
                .entry(song_id)
                .or_insert_with(|| entry.title.clone());
        }
        self
    }

    /// Stage real song-select dialog art (`DAILOG_SELECTMUSIC` placements
    /// plus figures loaded through `load_scene_figures`). Waiting layers
    /// it over the seats; empty art paints nothing extra.
    pub fn load_select_music_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.select_music_art = Some(SceneArt {
            placements,
            figures,
        });
        self
    }

    /// Catalog title for the chosen song, if staged.
    fn selected_song_title(&self) -> Option<&str> {
        let selection = self.selected_music.as_ref()?;
        self.song_titles
            .get(&selection.music_id)
            .map(String::as_str)
    }

    /// Waiting song panel rows: title row plus difficulty row.
    /// `None` while the choice or its title is missing; dialog art
    /// still paints then.
    fn selected_song_lines(&self) -> Option<[String; 2]> {
        let selection = self.selected_music.as_ref()?;
        let title = self.selected_song_title()?;
        Some([
            waiting_song_title_line(selection.music_id, title),
            waiting_song_detail_line(selection),
        ])
    }

    /// Stage battle totals (`RoomTeams` mirrors the server-side 4014/4016
    /// accumulation); the result verdict reads rank 1 from these totals.
    pub fn set_battle_teams(&mut self, teams: RoomTeams) -> &mut Self {
        self.battle_teams = teams;
        self
    }

    /// Claim the local battle team (winner art compares this seat team
    /// against rank 1; `None` clears the claim and hides the verdict).
    pub fn set_battle_self_team(&mut self, team: Option<u8>) -> &mut Self {
        self.battle_self_team = team;
        self
    }

    /// Rank-1 battle verdict for the claimed local team, if known.
    #[must_use]
    pub fn battle_verdict(&self) -> Option<BattleVerdict> {
        battle_verdict(&self.battle_teams, self.battle_self_team)
    }

    /// Stage real main-flow art for one screen: `.bnd` placements
    /// (via `bind_scene_rects`, or full-canvas for unbound blocks)
    /// plus figures loaded through `load_scene_figures`. Paint
    /// layers the live UI over it; empty art keeps the flat fallback.
    pub fn load_scene(
        &mut self,
        screen: Screen,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.scene_art[scene_index(screen)] = Some(SceneArt {
            placements,
            figures,
        });
        self
    }

    /// Staged art for one screen, if any.
    fn scene_art(&self, screen: Screen) -> Option<&SceneArt> {
        self.scene_art[scene_index(screen)].as_ref()
    }

    /// Open side screen, if any.
    #[must_use]
    pub fn side(&self) -> Option<SideScreen> {
        self.side
    }

    /// Stage real side art: `.bnd` placements plus figures loaded
    /// through [`load_scene_figures`](super::super::avatar::load_scene_figures).
    pub fn load_side_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.side_placements = placements;
        self.side_figures = figures;
        self
    }

    /// Open a side screen from a lobby; rejects other origins and
    /// remembers the lobby to return to.
    pub fn visit_side(&mut self, side: SideScreen) -> bool {
        if self.side.is_some() || !can_visit_side(self.flow.current(), side) {
            return false;
        }
        self.side_origin = Some(self.flow.current());
        self.side = Some(side);
        true
    }

    /// Return to the lobby that opened the side screen.
    pub fn leave_side(&mut self) -> bool {
        let (Some(side), Some(origin)) = (self.side, self.side_origin) else {
            return false;
        };
        if !can_return_side(side, origin) {
            return false;
        }
        self.side = None;
        self.side_origin = None;
        true
    }

    /// Paint with an explicit clock (tests use this; live paint reads
    /// the wall clock).
    pub fn paint_at(&mut self, frame: &mut Frame<'_>, now_ms: u64) {
        if self.side.is_some() {
            paint_side_scene(frame, &self.side_placements, &self.side_figures);
            return;
        }
        if self.flow.current() == Screen::Playing && self.playing.is_finished(now_ms) {
            let _ = self.flow.go(Screen::Result);
        }
        match self.flow.current() {
            Screen::Playing => self.paint_playing(frame, now_ms),
            Screen::Avi => self.paint_avi(frame),
            Screen::Channel => self.paint_channel(frame),
            Screen::Room => self.paint_room(frame),
            Screen::Waiting => self.paint_waiting(frame),
            Screen::Loading => self.paint_loading(frame),
            Screen::Result => self.paint_result(frame),
        }
    }

    /// Splash: staged logo art when present, else the flat card;
    /// the separately loaded splash figure paints on top either way.
    fn paint_avi(&self, frame: &mut Frame<'_>) {
        match self.scene_art(Screen::Avi) {
            Some(art) => paint_avi_scene(frame, art),
            None => paint_screen(frame, Screen::Avi),
        }
        if let Some(logo) = &self.splash {
            logo.paint(frame, 0, 0);
        }
    }

    /// Channel: staged art behind the live list, flat card fallback.
    fn paint_channel(&self, frame: &mut Frame<'_>) {
        match (self.scene_art(Screen::Channel), self.channels.is_empty()) {
            (Some(art), false) => {
                paint_channel_scene(frame, &self.channels, self.highlight, art);
            }
            (Some(art), true) => paint_lobby_backdrop(frame, Screen::Channel, art),
            (None, false) => paint_channel_list(frame, &self.channels, self.highlight),
            (None, true) => paint_screen(frame, Screen::Channel),
        }
    }

    /// Room lobby: staged art behind the live list, flat card fallback.
    fn paint_room(&self, frame: &mut Frame<'_>) {
        match (self.scene_art(Screen::Room), self.rooms.is_empty()) {
            (Some(art), false) => {
                paint_room_scene(frame, &self.rooms, self.room_highlight, art);
            }
            (Some(art), true) => paint_lobby_backdrop(frame, Screen::Room, art),
            (None, false) => paint_room_list(frame, &self.rooms, self.room_highlight),
            (None, true) => paint_screen(frame, Screen::Room),
        }
    }

    /// Waiting: staged art behind the live seats, flat card fallback,
    /// then the staged song-select dialog with the chosen-song panel.
    fn paint_waiting(&self, frame: &mut Frame<'_>) {
        match (self.scene_art(Screen::Waiting), &self.waiting) {
            (Some(art), Some(room)) => paint_waiting_scene(frame, room, art),
            (Some(art), None) => paint_lobby_backdrop(frame, Screen::Waiting, art),
            (None, Some(room)) => paint_waiting_members(frame, room),
            (None, None) => paint_screen(frame, Screen::Waiting),
        }
        self.paint_select_music_art(frame);
        self.paint_selected_song_panel(frame);
    }

    /// Layer staged song-select dialog figures over the waiting seats.
    /// Placements without a loaded figure paint nothing.
    fn paint_select_music_art(&self, frame: &mut Frame<'_>) {
        let Some(art) = self.select_music_art.as_ref() else {
            return;
        };
        art.paint_overlay(frame);
    }

    /// Paint the chosen-song panel rows (title, then difficulty).
    /// Missing choices or titles paint nothing: art without rows.
    fn paint_selected_song_panel(&self, frame: &mut Frame<'_>) {
        let Some([title_line, detail_line]) = self.selected_song_lines() else {
            return;
        };
        Self::paint_song_panel_line(frame, 0, &title_line);
        Self::paint_song_panel_line(frame, 1, &detail_line);
    }

    /// Panel rows stacked below the seat grid: title row 0, detail row 1.
    const SONG_PANEL_ROWS: usize = 2;
    /// Row step matching the lobby list height.
    const SONG_PANEL_ROW_H: i32 = 8;

    /// Top-left of song panel row `slot` (bottom-left, below the seats).
    fn song_panel_origin(frame: &Frame<'_>, slot: usize) -> (i32, i32) {
        let width = i32::try_from(frame.w).unwrap_or(i32::MAX);
        let height = i32::try_from(frame.h).unwrap_or(i32::MAX);
        let step = i32::try_from(slot).unwrap_or(i32::MAX);
        (width / 8, height * 3 / 4 + step * Self::SONG_PANEL_ROW_H)
    }

    /// One song panel row: dark scrim plus white ink.
    fn paint_song_panel_line(frame: &mut Frame<'_>, slot: usize, text: &str) {
        if slot >= Self::SONG_PANEL_ROWS || text.is_empty() {
            return;
        }
        let (x, y) = Self::song_panel_origin(frame, slot);
        let text_w = u32::try_from(text_width(text)).unwrap_or(u32::MAX);
        fill_rect(
            frame,
            &Rect {
                x: x - 4,
                y: y - 2,
                w: text_w.saturating_add(8),
                h: 9,
            },
            pack_xrgb(0x08, 0x08, 0x10),
        );
        draw_text(frame, x, y, text, pack_xrgb(0xFF, 0xFF, 0xFF));
    }

    /// Loading gate: staged art behind the progress bar.
    fn paint_loading(&self, frame: &mut Frame<'_>) {
        match self.scene_art(Screen::Loading) {
            Some(art) => paint_loading_scene(frame, art),
            None => paint_screen(frame, Screen::Loading),
        }
    }

    /// Gameplay: highway first, then staged gauges/panels ring the
    /// frame edges while notes keep falling underneath, then the
    /// right-side avatar scene (life bar, figure, song, VOL) with the
    /// bottom tally strip over both. Missing figures paint art
    /// without avatar.
    fn paint_playing(&mut self, frame: &mut Frame<'_>, now_ms: u64) {
        let geom = HighwayGeometry::centered(frame.w, frame.h);
        self.playing.paint(frame, &geom, now_ms);
        if let Some(art) = self.scene_art(Screen::Playing) {
            paint_playing_overlay(frame, art);
        }
        self.sync_song_caption();
        self.playing
            .paint_avatar_scene(frame, self.avatar.as_ref(), now_ms);
    }

    /// Re-caption the song line from the waiting choice while one is
    /// selected (manual captions stand while nothing is selected).
    fn sync_song_caption(&mut self) {
        let Some(selection) = self.selected_music.as_ref() else {
            return;
        };
        let speed_code = selection.speed;
        let title = self.selected_song_title().unwrap_or("").to_owned();
        self.playing.scene_mut().set_song(&title, speed_code);
    }

    /// Results: staged art (`StateResult_Top/Bottom.ojs` plus
    /// `Result_WinLose.ojs` via `FUN_004c2390`, file `0xc2390`) behind
    /// the avatar figure, digits, and the `Winner_image.ojs` /
    /// `Result_WinLose.ojs` rank-1 verdict.
    fn paint_result(&self, frame: &mut Frame<'_>) {
        let verdict = self.battle_verdict();
        match self.scene_art(Screen::Result) {
            Some(art) => paint_result_scene(
                frame,
                self.avatar.as_ref(),
                self.playing.score(),
                art,
                verdict,
            ),
            None => paint_result(frame, self.avatar.as_ref(), self.playing.score(), verdict),
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
        if self.side.is_some() {
            if key == MenuKey::Back {
                self.leave_side();
            }
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
        let note_y = 48 - (500.0 * 48.0 / 2000.0) as i32;
        // Lane 2 spans x 4..6 at 70 wide (native x55 w28); the note
        // insets one pixel each side.
        assert_eq!(buf[note_y as usize * 70 + 4], pack_xrgb(0x40, 0xC0, 0xFF));
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
    fn room_highlight_selects_details_panel() {
        // 200x150 fits all nine fallback detail lines; the open room
        // paints OPEN dim while the locked room paints LOCK red.
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(super::MenuKey::Confirm, true);
        let mut buf = vec![0u32; 200 * 150];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            0,
        );
        assert!(buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0x50, 0x50)));
        game.menu_key(super::MenuKey::Down, true);
        let mut buf = vec![0u32; 200 * 150];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            0,
        );
        assert!(buf.contains(&pack_xrgb(0xFF, 0x50, 0x50)));
        // List plus cursor keep working under the panel.
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
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

    fn waiting_song_catalog_blob() -> Vec<u8> {
        let mut rec = vec![0u8; 300];
        rec[8..11].copy_from_slice(b"ojn");
        rec[4..8].copy_from_slice(&757u32.to_le_bytes());
        rec[20..24].copy_from_slice(&124.0f32.to_le_bytes());
        rec[112..117].copy_from_slice(b"First");
        rec[176..180].copy_from_slice(b"Band");
        rec[240..251].copy_from_slice(b"o2ma757.ojm");
        rec
    }

    fn waiting_song_selection() -> o2jam_net::play::SetRoomMusic {
        o2jam_net::play::SetRoomMusic {
            music_id: 757,
            difficulty: 2,
            speed: 3,
        }
    }

    fn stage_waiting_song(game: &mut ScreenGame) {
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_selected_music(waiting_song_selection());
    }

    fn paint_waiting_frame(game: &mut ScreenGame, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            0,
        );
        buf
    }

    #[test]
    fn waiting_selected_song_paints_title_and_difficulty_rows() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        stage_waiting_song(&mut game);
        let buf = paint_waiting_frame(&mut game, 80, 64);
        // Panel sits below the seats: title `M...` at (10, 48),
        // detail `H...` at (10, 56); both glyphs light their origin.
        assert_eq!(buf[48 * 80 + 10], pack_xrgb(0xFF, 0xFF, 0xFF));
        assert_eq!(buf[56 * 80 + 10], pack_xrgb(0xFF, 0xFF, 0xFF));
        // Seats keep painting under the panel.
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    #[test]
    fn waiting_without_catalog_paints_art_without_rows() {
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        game.set_selected_music(waiting_song_selection());
        let buf = paint_waiting_frame(&mut game, 80, 64);
        // Dialog art lands at the origin; the unresolved panel leaves
        // the bottom strip on the flat background.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[48 * 80 + 10], background_of(Screen::Waiting));
        assert_eq!(buf[56 * 80 + 10], background_of(Screen::Waiting));
    }

    #[test]
    fn select_music_art_layers_over_waiting_without_hiding_seats() {
        let (placements, figures) = corner_scene_art("AlbumList.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        stage_waiting_song(&mut game);
        let buf = paint_waiting_frame(&mut game, 80, 64);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[48 * 80 + 10], pack_xrgb(0xFF, 0xFF, 0xFF));
        let (x, y) = seat_origin(1, 80, 64);
        assert_eq!(
            buf[(y + 1) as usize * 80 + (x - 6 + 1) as usize],
            pack_xrgb(0x30, 0xE0, 0x30)
        );
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

    fn walk_to_lobby(game: &mut ScreenGame, lobby: Screen) {
        game.lane_key(3, true);
        if lobby == Screen::Channel {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if lobby == Screen::Room {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn side_screens_open_from_every_lobby() {
        use super::super::side::ALL_SIDE_SCREENS;
        for lobby in [Screen::Channel, Screen::Room, Screen::Waiting] {
            for side in ALL_SIDE_SCREENS {
                let mut game = ScreenGame::new();
                walk_to_lobby(&mut game, lobby);
                assert!(game.visit_side(side), "{lobby:?} -> {side:?}");
                assert_eq!(game.side(), Some(side));
                assert_eq!(game.screen(), lobby);
            }
        }
    }

    #[test]
    fn side_screens_reject_main_flow_origins() {
        use super::super::side::ALL_SIDE_SCREENS;
        for side in ALL_SIDE_SCREENS {
            let mut splash = ScreenGame::new();
            assert!(!splash.visit_side(side));
            assert_eq!(splash.side(), None);
        }
        for side in ALL_SIDE_SCREENS {
            let mut game = ScreenGame::new();
            walk_to_playing(&mut game);
            assert!(!game.visit_side(side));
            assert_eq!(game.side(), None);
        }
    }

    #[test]
    fn side_visit_rejects_nested_sides() {
        use super::super::side::SideScreen;
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Room);
        assert!(game.visit_side(SideScreen::Shop));
        assert!(!game.visit_side(SideScreen::Planet));
        assert_eq!(game.side(), Some(SideScreen::Shop));
    }

    #[test]
    fn side_back_key_returns_to_lobby() {
        use super::super::side::SideScreen;
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Room);
        assert!(game.visit_side(SideScreen::MyRoom));
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.side(), None);
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn side_leaves_without_entry() {
        let mut game = ScreenGame::new();
        assert!(!game.leave_side());
    }

    #[test]
    fn side_paints_loaded_art() {
        use super::super::side::SideScreen;
        use crate::avatar::{AvatarFigure, FigureLayer, ScenePlacement};
        use o2jam_render::{Rect, Rgb555Image};
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Channel);
        game.load_side_scene(
            vec![ScenePlacement {
                id: "IDC_SHOP".to_owned(),
                code: 0,
                sprite: Some("Shop_Item.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            }],
            HashMap::from([("Shop_Item.ojs".to_owned(), figure)]),
        );
        assert!(game.visit_side(SideScreen::Shop));
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
        assert_eq!(game.screen(), Screen::Channel);
    }

    /// One red pixel figure for staged scene art.
    fn red_scene_figure() -> AvatarFigure {
        use crate::avatar::FigureLayer;
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// Full-canvas placement pinning one sprite at the frame origin.
    fn corner_scene_art(name: &str) -> (Vec<ScenePlacement>, HashMap<String, AvatarFigure>) {
        use crate::screens::paint::{unbound_scene_placements, SceneArt};
        let _ = SceneArt::default();
        let names = vec![name.to_owned()];
        let figures = HashMap::from([(name.to_owned(), red_scene_figure())]);
        (unbound_scene_placements(&names), figures)
    }

    #[test]
    fn scene_art_backs_channel_list() {
        let (placements, figures) = corner_scene_art("Dialog_ChannelSelectBar.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Channel, placements, figures);
        game.set_channels(two_channels());
        game.lane_key(3, true);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn scene_art_backs_waiting_seats() {
        let (placements, figures) = corner_scene_art("Wait_TeamA.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Waiting, placements, figures);
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
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    #[test]
    fn scene_art_overlays_playing_without_hiding_notes() {
        let (placements, figures) = corner_scene_art("Playing_Panel.ojs");
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 1500, 7)]);
        game.load_scene(Screen::Playing, placements, figures);
        walk_to_playing(&mut game);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            1000,
        );
        assert_eq!(game.screen(), Screen::Playing);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x40, 0xC0, 0xFF)));
    }

    #[test]
    fn scene_art_backs_result_digits() {
        let (placements, figures) = corner_scene_art("Result_WinLose.ojs");
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        game.load_scene(Screen::Result, placements, figures);
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
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn scene_art_backs_loading_bar() {
        use MenuKey;
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
    }

    fn battle_teams_self_wins() -> o2jam_game::RoomTeams {
        let mut teams = o2jam_game::RoomTeams::new();
        teams.set_team(0, 0);
        teams.set_team(1, 1);
        teams.submit(0, 100_000);
        teams.submit(1, 150_000);
        teams
    }

    fn finish_song(game: &mut ScreenGame) {
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(game);
        game.strike_at(2, 5000);
    }

    fn paint_result_frame(game: &mut ScreenGame) -> Vec<u32> {
        let mut buf = vec![0u32; 70 * 70];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 70,
            },
            7000,
        );
        buf
    }

    #[test]
    fn result_without_battle_claim_paints_no_verdict() {
        let mut game = ScreenGame::new();
        finish_song(&mut game);
        assert_eq!(game.battle_verdict(), None);
        let buf = paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        assert!(!buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn result_win_paints_gold_verdict_for_rank_one() {
        let mut game = ScreenGame::new();
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(1));
        assert_eq!(game.battle_verdict(), Some(BattleVerdict::Win));
        finish_song(&mut game);
        let buf = paint_result_frame(&mut game);
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn result_lose_paints_gray_verdict_off_rank_one() {
        let mut game = ScreenGame::new();
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(0));
        assert_eq!(game.battle_verdict(), Some(BattleVerdict::Lose));
        finish_song(&mut game);
        let buf = paint_result_frame(&mut game);
        assert!(buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn result_verdict_layers_over_winner_and_winlose_art() {
        use crate::screens::paint::unbound_scene_placements;
        let names = vec![
            "Winner_image.ojs".to_owned(),
            "Result_WinLose.ojs".to_owned(),
        ];
        let figures = HashMap::from([
            (names[0].clone(), red_scene_figure()),
            (names[1].clone(), red_scene_figure()),
        ]);
        let placements = unbound_scene_placements(&names);
        let mut game = ScreenGame::new();
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(1));
        game.load_scene(Screen::Result, placements, figures);
        finish_song(&mut game);
        let buf = paint_result_frame(&mut game);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    fn paint_playing_frame(game: &mut ScreenGame, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            1000,
        );
        buf
    }

    /// True while white text ink lands near a caption origin (glyph
    /// top-left cells are often blank, so scan the scrim box).
    fn white_near(buf: &[u32], w: usize, h: usize, origin: (i32, i32)) -> bool {
        let (ox, oy) = origin;
        for row in oy..oy + 9 {
            for col in ox..ox + 64 {
                let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
                    continue;
                };
                if r < h && c < w && buf[r * w + c] == pack_xrgb(0xFF, 0xFF, 0xFF) {
                    return true;
                }
            }
        }
        false
    }

    fn playing_avatar_game() -> ScreenGame {
        use o2jam_file::{parse_sprite55, SPRITE55_MAGIC};
        // 2x1 sprite-55: transparent black, then red (result-test shape).
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
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.set_player(43, "HOST");
        game.set_song_caption("First", 6);
        game.set_volume(5);
        game.strike_at(2, 5000);
        game
    }

    #[test]
    fn playing_paints_avatar_scene_with_captions_and_tally() {
        use crate::playing::AvatarLayout;
        let mut game = playing_avatar_game();
        let buf = paint_playing_frame(&mut game, 800, 600);
        assert_eq!(game.screen(), Screen::Playing);
        let layout = AvatarLayout::for_frame(800, 600);
        // Worn figure lands at the scene origin (red pixel after the
        // transparent one).
        let (fx, fy) = layout.figure_origin;
        assert_eq!(
            buf[fy as usize * 800 + fx as usize + 1],
            pack_xrgb(255, 0, 0)
        );
        // Life bar shows full; level, song, VOL, counts, EXIT ink white.
        assert_eq!(buf[56 * 800 + 340], pack_xrgb(0x20, 0xE0, 0x20));
        for origin in [
            layout.level_origin,
            layout.song_origin,
            layout.volume_origin,
            layout.counts_origin,
            layout.strip_title_origin,
            layout.exit_origin,
        ] {
            assert!(white_near(&buf, 800, 600, origin));
        }
        // The struck note's COOL feeds the tally through the score.
        assert_eq!(game.playing.score().counts[0], 1);
    }

    #[test]
    fn playing_without_figure_paints_scene_without_avatar() {
        use crate::playing::AvatarLayout;
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.set_player(43, "HOST");
        let buf = paint_playing_frame(&mut game, 800, 600);
        let layout = AvatarLayout::for_frame(800, 600);
        // No figure: the scene origin keeps the highway backdrop while
        // the captions still paint.
        let (fx, fy) = layout.figure_origin;
        assert_eq!(
            buf[fy as usize * 800 + fx as usize],
            pack_xrgb(0x0A, 0x0A, 0x0A)
        );
        let (lx, ly) = layout.level_origin;
        assert!(white_near(&buf, 800, 600, (lx, ly)));
    }

    #[test]
    fn playing_syncs_song_line_from_waiting_choice() {
        use crate::playing::AvatarLayout;
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_selected_music(waiting_song_selection());
        walk_to_playing(&mut game);
        let buf = paint_playing_frame(&mut game, 800, 600);
        // `M757` catalog title with speed X2 (code 3): the song line inks.
        let layout = AvatarLayout::for_frame(800, 600);
        assert!(white_near(&buf, 800, 600, layout.song_origin));
        assert!(white_near(&buf, 800, 600, layout.strip_title_origin));
    }
}
