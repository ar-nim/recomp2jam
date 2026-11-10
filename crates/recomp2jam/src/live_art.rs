//! Live session scene art: genuine 7.55 archive backdrops under the
//! windowed online walk (`run_play --window --online`).
//!
//! Every live screen stages through the State-keyed staging declaration
//! (`stage_state`/`stage_dialog`) — the same seam the shots walk, the
//! soak, and the session walk stage through — driven by live room
//! state instead of shot fixtures: the room arena stage resolves the venue,
//! and the room key mode plus team flag select the Playing family members.
//! Archives stay read-only; missing or unreadable archives keep the current
//! fallback paint, never failing.
//!
//! Differences from the shots staging, by intent: the Playing mode follows
//! the joined room (the shots pin the solo reference composition), the AVI
//! logo never loads here (an explicit `--interface` splash wins — the shell
//! exposes no splash query, so loading would clobber it), and no avatar,
//! catalog, or chat fixtures stage (the live session owns those).

use std::collections::HashMap;
use std::path::PathBuf;

use o2jam_shell::avatar::load_archive_sprite;
use o2jam_shell::avatar::PlayingArtMode;
use o2jam_shell::screens::paint::DIALOG_BAR_SPRITE;
use o2jam_shell::staging::{
    read_install_archives, stage_dialog, stage_state, Dialog, StageContext, State,
};
use o2jam_shell::{PlaySkin, Screen, ScreenGame};

/// Repository-local `install-root` (git-ignored game data).
fn install_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

/// Live Playing mode: the joined room's key mode and team flag (the
/// shots pin solo here; the live walk stages what the room names).
fn live_playing_mode(game: &ScreenGame) -> PlayingArtMode {
    PlayingArtMode {
        key_mode: game.room_key_mode(),
        team_play: game.room_team_play(),
    }
}

/// Art-less `STATE_CHANNEL`: the empty scene takes the staged path
/// (`paint_channel_scene` paints no fill under the live list) instead
/// of the flat `paint_channel_list` fallback — the shots walk does the
/// same. Archive-independent, so it always stages.
fn stage_empty_channel_scene(game: &mut ScreenGame) {
    game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
}

/// Stage Interface-archive scenes through the declaration; each scene
/// stages independently so one bad block never blanks the rest. Myroom
/// stages bound through its row (ruling 1): the archive holds
/// `State_Myroom.bnd`, so the room paints at `.bnd` rects. The Result
/// banner merges through its row; the song-select dialog art stages
/// over the Waiting seats.
fn stage_interface_scenes(game: &mut ScreenGame, archive: &[u8]) {
    let ctx = StageContext::default();
    for (screen, state) in [
        (Screen::Avi, State::Avi),
        (Screen::Room, State::Room),
        (Screen::Waiting, State::Waiting),
        (Screen::Planet, State::Planet),
        (Screen::Lanking, State::Lanking),
        (Screen::Shop, State::Shop),
        (Screen::MusicShop, State::MusicShop),
        (Screen::Myroom, State::MyRoom),
        (Screen::Result, State::Result),
    ] {
        if let Some(art) = stage_state(archive, state, ctx).art {
            game.load_scene(screen, art.placements, art.figures);
        }
    }
    stage_select_dialog_art(game, archive);
    stage_channel_dialog(game, archive);
    stage_planet_dialogs(game, archive);
}

/// Stage the planet-board modal sheets: the first-open options tab
/// (`Dialog_Option_*`; tab choice OPEN) and the quit-confirm
/// (`Dialog_Question1`). Both stay modal art (the game paints them
/// dialog-relative only while open), never scene slots. Each block
/// stages independently; a miss keeps the fallback popup/title.
/// Shared with the shots walk (one implementation for both).
pub(crate) fn stage_planet_dialogs(game: &mut ScreenGame, archive: &[u8]) {
    let ctx = StageContext::default();
    if let Some(art) = stage_dialog(archive, Dialog::PlanetOption, ctx).art {
        game.load_planet_option_scene(art.placements, art.figures);
    }
    if let Some(art) = stage_dialog(archive, Dialog::PlanetQuit, ctx).art {
        game.load_planet_quit_scene(art.placements, art.figures);
    }
}

/// Stage the channel dialog art through the declaration: the bound
/// `DIALOG_CHANNEL_INFO` bodies, plus the hand-staged hover bar. The
/// painter mounts each figure at its bnd-derived origin (bar at row
/// origin +1, bar-local children, scroll bodies on the track);
/// absent figures fall back to the labeled stand-ins. The hover
/// highlight bar (`Dialog_ChannelSelectBar.ojs`, 348x44) stays
/// code-mounted — live-confirmed sprite record at dialog+0x8C
/// (2026-11-09) — and paints at the hovered row's rect.
pub(crate) fn stage_channel_dialog(game: &mut ScreenGame, archive: &[u8]) {
    let staged = stage_dialog(archive, Dialog::ChannelInfo, StageContext::default());
    let mut figures = staged.art.map(|art| art.figures).unwrap_or_default();
    // The hover bar stays hand-staged: the exe mounts it at the Dialog
    // object (`+0x8C`), not from the table (OPEN).
    if let Some(bar) = load_archive_sprite(archive, DIALOG_BAR_SPRITE) {
        figures.insert(DIALOG_BAR_SPRITE.to_owned(), bar);
    }
    if figures.is_empty() {
        return;
    }
    // Placements stay empty: the Channel painter mounts dialog art at
    // its anchors (exe-verified), never at `.bnd` rects.
    game.load_scene(Screen::Channel, Vec::new(), figures);
}

/// Stage real song-select dialog art over the Waiting seats; empty art
/// paints nothing extra.
fn stage_select_dialog_art(game: &mut ScreenGame, archive: &[u8]) {
    let Some(art) = stage_dialog(archive, Dialog::SelectMusic, StageContext::default()).art else {
        return;
    };
    if art.figures.is_empty() {
        return;
    }
    game.load_select_music_scene(art.placements, art.figures);
}

/// Stage Playing-archive scenes through the live room: the Loading gate
/// sprite plus the Playing gauges through the room arena stage and key
/// mode, and the stage-aware play skin (procedural lanes stay on a miss).
fn stage_playing_scenes(game: &mut ScreenGame, archive: &[u8]) {
    if let Some(art) = stage_state(archive, State::Loading, StageContext::default()).art {
        game.load_scene(Screen::Loading, art.placements, art.figures);
    }
    let mode = live_playing_mode(game);
    let ctx = StageContext {
        arena_stage: Some(game.arena_stage()),
        key_mode: mode.key_mode,
        team_play: mode.team_play,
    };
    let Some(art) = stage_state(archive, State::Playing, ctx).art else {
        return;
    };
    game.load_scene(Screen::Playing, art.placements, art.figures);
    if let Some(skin) = PlaySkin::from_archive_for_stage(archive, Some(game.arena_stage())) {
        game.set_play_skin(skin);
    }
}

/// Stage genuine 7.55 scene art under every live screen when
/// `install-root` holds the archives; missing or unreadable archives
/// keep the fallback paint. Read-only and infallible: staging never
/// moves the flow, so call it right after the login lists land.
pub fn stage_live_scenes(game: &mut ScreenGame) {
    stage_empty_channel_scene(game);
    let installed = read_install_archives(&install_root());
    if let Some(archive) = &installed.interface {
        stage_interface_scenes(game, archive);
    }
    if let Some(archive) = &installed.playing {
        stage_playing_scenes(game, archive);
    }
}

/// Re-stage the Playing-archive scenes after the joined room changes
/// (join, host, arena echo): the venue and family members follow the
/// live arena stage and mode. Interface scenes never depend on the
/// room, so they keep their login-time staging.
pub fn restage_live_playing(game: &mut ScreenGame) {
    let Some(archive) = read_install_archives(&install_root()).playing else {
        return;
    };
    stage_playing_scenes(game, &archive);
}

#[cfg(test)]
mod live_art_tests {
    use o2jam_net::{ChannelState, GameMode, JoinedRoom, RoomListEntry};
    use o2jam_render::{Frame, NATIVE_HEIGHT, NATIVE_WIDTH};
    use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

    use super::{install_root, restage_live_playing, stage_live_scenes};
    use o2jam_shell::staging::{
        read_install_archives, stage_state, StageContext, StagedArt, State,
    };

    /// Archives behind the staging (`true` while `install-root` holds
    /// the 7.55 interface art; the suite stays green without it).
    fn interface_present() -> bool {
        read_install_archives(&install_root()).interface.is_some()
    }

    /// Archives behind the Playing staging.
    fn playing_present() -> bool {
        read_install_archives(&install_root()).playing.is_some()
    }

    /// Skip helper: the staged-pixel comparisons need real archives.
    fn skip_without_archives() -> bool {
        if interface_present() && playing_present() {
            return false;
        }
        println!("skip: missing install-root archives");
        true
    }

    /// One Interface State staged through the declaration, if the
    /// archives hold it.
    fn declared_interface(state: State) -> Option<StagedArt> {
        let archive = read_install_archives(&install_root()).interface?;
        stage_state(&archive, state, StageContext::default()).art
    }

    /// Declared staging as the scene painters' art value.
    fn scene_art_of(staged: StagedArt) -> o2jam_shell::screens::paint::SceneArt {
        o2jam_shell::screens::paint::SceneArt {
            placements: staged.placements,
            figures: staged.figures,
        }
    }

    /// One lobby row for the staged paint path.
    fn lobby_row() -> RoomListEntry {
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
            user_count: 1,
            min_level: 0,
            max_level: 0,
            skills: Vec::new(),
            premium: false,
            room_type: 0,
        }
    }

    /// Joined room behind the live Playing mode: arena stage 2 with a
    /// 7-key solo table, so the venue and family members resolve.
    fn live_room() -> JoinedRoom {
        JoinedRoom {
            member_id: 0,
            team: 0,
            title: "Jam".to_owned(),
            music_id: 0,
            arena: 2,
            key_mode: 2,
            game_mode: GameMode::Normal as u8,
            difficulty: 0,
            speed: 1,
            has_super_manager: false,
            user_count: 1,
            slots: Vec::new(),
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    /// Game with live login data: channels, rooms, and the joined room.
    fn live_game() -> ScreenGame {
        let mut game = ScreenGame::new();
        game.set_channels(vec![ChannelState {
            gateway_id: 0,
            channel_id: 2,
            capacity: 100,
            population: 5,
            active: true,
        }]);
        game.set_rooms(vec![lobby_row()]);
        game.set_joined_room(live_room());
        game
    }

    /// Walk the offline flow to one screen (lane dismisses the splash
    /// onto the bare board, then one Confirm per edge: emblem →
    /// dialog → room → waiting → loading).
    fn walk_to(game: &mut ScreenGame, target: Screen) {
        game.lane_key(3, true);
        if target == Screen::Planet {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if target == Screen::Channel {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if target == Screen::Room {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if target == Screen::Waiting {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if target == Screen::Loading {
            return;
        }
        // The last Confirm is the Loading→Playing gate (S7): arm it.
        {
            use crate::loading::{loading_phase_status, LOADING_GATE_STEPS};
            use o2jam_audio::NullBackend;
            use std::collections::HashMap;
            if game.selected_music().is_none() {
                game.set_selected_music(o2jam_net::SetRoomMusic {
                    music_id: 757,
                    difficulty: 0,
                    speed: 0,
                });
            }
            if !game.has_audio() {
                game.with_audio(Box::new(NullBackend::new()), HashMap::new());
            }
            if let Some(status) = loading_phase_status(LOADING_GATE_STEPS, "Song", "Band") {
                game.set_loading_status(status);
            }
        }
        game.menu_key(MenuKey::Confirm, true);
    }

    /// Native-canvas pixels of one explicit-clock paint.
    fn painted_pixels(game: &mut ScreenGame, now_ms: u64) -> Vec<u32> {
        let width = usize::try_from(NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            now_ms,
        );
        buf
    }

    /// Bare versus staged paints of one screen walked the same way.
    fn paint_pair(target: Screen, now_ms: u64) -> (Vec<u32>, Vec<u32>) {
        let mut bare = live_game();
        walk_to(&mut bare, target);
        let bare_pixels = painted_pixels(&mut bare, now_ms);
        let mut staged = live_game();
        stage_live_scenes(&mut staged);
        walk_to(&mut staged, target);
        let staged_pixels = painted_pixels(&mut staged, now_ms);
        (bare_pixels, staged_pixels)
    }

    #[test]
    fn staging_never_moves_the_flow() {
        let mut game = live_game();
        walk_to(&mut game, Screen::Waiting);
        stage_live_scenes(&mut game);
        assert_eq!(game.screen(), Screen::Waiting);
        restage_live_playing(&mut game);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn staging_never_fails_on_an_empty_game() {
        let mut game = ScreenGame::new();
        stage_live_scenes(&mut game);
        restage_live_playing(&mut game);
        assert_eq!(game.screen(), Screen::Avi);
    }

    #[test]
    fn planet_stages_bound_art_when_archives_present() {
        // Seventeen bound entries pair seventeen `.bnd` rects by code
        // ordinal (self included, with the miss rect); the grown figures
        // carry the full-bleed state art. Pixel cover rides the shell
        // scene painter (covered there); the walk cannot reach Planet
        // without invented input, so no pixels assert here.
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::Planet).expect("planet: unstaged state");
        let (placements, figures) = (art.placements, art.figures);
        assert_eq!(placements.len(), 17);
        assert!(figures.contains_key("State_Planet.ojs"));
    }

    #[test]
    fn planet_paints_real_art_when_archives_present() {
        // The walk cannot reach Planet without invented input, so this
        // paints the staged scene directly: real art must ink the frame
        // beyond the flat card.
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art = scene_art_of(declared_interface(State::Planet).expect("planet: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_planet_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
            None,
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "planet staged art paints nothing"
        );
    }

    #[test]
    fn lanking_stages_bound_art_when_archives_present() {
        // Rank rows need the fetch opcode (OPEN), so this pins the
        // staging shape only: placements pair the 73 `.bnd` rects by
        // code ordinal, and the full-bleed state art grows. Pixel cover
        // rides the shell scene painter (covered there).
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::Lanking).expect("lanking: unstaged state");
        let (placements, figures) = (art.placements, art.figures);
        assert!(!placements.is_empty());
        assert!(figures.contains_key("State_Lanking.ojs"));
    }

    #[test]
    fn lanking_paints_real_art_when_archives_present() {
        // Unreachable screens still paint through their scene function
        // directly (see the planet twin).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art =
            scene_art_of(declared_interface(State::Lanking).expect("lanking: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_lanking_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "lanking staged art paints nothing"
        );
    }

    /// Game holding declaration art directly, mirroring the live
    /// coverage (empty channel, every interface State, the select and
    /// channel dialogs plus the hand-staged hover bar, the planet
    /// sheets, the Loading gate, the Playing subset through the live
    /// room, and the stage-aware skin). `None` without the archives.
    fn declaration_game() -> Option<ScreenGame> {
        use o2jam_shell::avatar::{load_archive_sprite, PlayingArtMode};
        use o2jam_shell::screens::paint::DIALOG_BAR_SPRITE;
        use o2jam_shell::staging::{stage_dialog, Dialog};
        use std::collections::HashMap;
        let installed = read_install_archives(&install_root());
        let interface = installed.interface?;
        let playing = installed.playing?;
        let mut expected = live_game();
        expected.load_scene(Screen::Channel, Vec::new(), HashMap::new());
        let ctx = StageContext::default();
        for (screen, state) in [
            (Screen::Avi, State::Avi),
            (Screen::Room, State::Room),
            (Screen::Waiting, State::Waiting),
            (Screen::Planet, State::Planet),
            (Screen::Lanking, State::Lanking),
            (Screen::Shop, State::Shop),
            (Screen::MusicShop, State::MusicShop),
            (Screen::Myroom, State::MyRoom),
            (Screen::Result, State::Result),
        ] {
            if let Some(art) = stage_state(&interface, state, ctx).art {
                expected.load_scene(screen, art.placements, art.figures);
            }
        }
        if let Some(art) =
            stage_dialog(&interface, Dialog::SelectMusic, StageContext::default()).art
        {
            expected.load_select_music_scene(art.placements, art.figures);
        }
        let mut channel_figures =
            stage_dialog(&interface, Dialog::ChannelInfo, StageContext::default())
                .art
                .map(|art| art.figures)
                .unwrap_or_default();
        if let Some(bar) = load_archive_sprite(&interface, DIALOG_BAR_SPRITE) {
            channel_figures.insert(DIALOG_BAR_SPRITE.to_owned(), bar);
        }
        expected.load_scene(Screen::Channel, Vec::new(), channel_figures);
        if let Some(art) =
            stage_dialog(&interface, Dialog::PlanetOption, StageContext::default()).art
        {
            expected.load_planet_option_scene(art.placements, art.figures);
        }
        if let Some(art) = stage_dialog(&interface, Dialog::PlanetQuit, StageContext::default()).art
        {
            expected.load_planet_quit_scene(art.placements, art.figures);
        }
        if let Some(art) = stage_state(&playing, State::Loading, StageContext::default()).art {
            expected.load_scene(Screen::Loading, art.placements, art.figures);
        }
        let mode = PlayingArtMode {
            key_mode: expected.room_key_mode(),
            team_play: expected.room_team_play(),
        };
        let playing_ctx = StageContext {
            arena_stage: Some(expected.arena_stage()),
            key_mode: mode.key_mode,
            team_play: mode.team_play,
        };
        if let Some(art) = stage_state(&playing, State::Playing, playing_ctx).art {
            expected.load_scene(Screen::Playing, art.placements, art.figures);
        }
        if let Some(skin) =
            o2jam_shell::PlaySkin::from_archive_for_stage(&playing, Some(expected.arena_stage()))
        {
            expected.set_play_skin(skin);
        }
        Some(expected)
    }

    #[test]
    fn live_matches_declaration_on_walked_screens_when_archives_present() {
        // No-delta oracle: a game staged by the live window must paint
        // every walked screen exactly like a game holding declaration
        // art directly. Myroom is the one intended delta and stays out
        // (the flow never enters it); its bound wiring rides the
        // `myroom_stages_bound_art` row plus the art-equivalence probe.
        if skip_without_archives() {
            return;
        }
        for target in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
        ] {
            let mut expected = declaration_game().expect("guarded game");
            let mut live = live_game();
            stage_live_scenes(&mut live);
            walk_to(&mut expected, target);
            walk_to(&mut live, target);
            assert_eq!(expected.screen(), live.screen());
            assert_eq!(
                painted_pixels(&mut expected, 0),
                painted_pixels(&mut live, 0),
                "live diverges from the declaration on {target:?}"
            );
        }
        let mut expected = declaration_game().expect("guarded game");
        let mut live = live_game();
        stage_live_scenes(&mut live);
        for game in [&mut expected, &mut live] {
            game.load_chart(vec![(2u8, 60_000u64, 7u32)]);
        }
        walk_to(&mut expected, Screen::Playing);
        walk_to(&mut live, Screen::Playing);
        assert_eq!(live.screen(), Screen::Playing);
        assert_eq!(
            painted_pixels(&mut expected, 0),
            painted_pixels(&mut live, 0),
            "live diverges from the declaration on Playing"
        );
    }

    #[test]
    fn myroom_stages_bound_art_when_archives_present() {
        // Bound `.bnd` pairing (ruling 1): placements pair the rects by
        // code ordinal and the full-bleed state art grows.
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::MyRoom).expect("myroom: unstaged state");
        assert!(!art.placements.is_empty());
        assert!(art.figures.contains_key("State_Myroom.ojs"));
    }

    #[test]
    fn myroom_paints_real_art_when_archives_present() {
        // Unreachable screens still paint through their scene function
        // directly (see the planet twin).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art = scene_art_of(declared_interface(State::MyRoom).expect("myroom: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_myroom_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "myroom staged art paints nothing"
        );
    }

    #[test]
    fn shop_stages_bound_art_when_archives_present() {
        // Catalogue rows need item data joins (OPEN); this pins the
        // staging shape only (gapless ordinal pairing per docs/shop.md).
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::Shop).expect("shop: unstaged state");
        let (placements, figures) = (art.placements, art.figures);
        assert!(!placements.is_empty());
        assert!(figures.contains_key("State_ItemShop.ojs"));
    }

    #[test]
    fn shop_paints_real_art_when_archives_present() {
        // Unreachable screens still paint through their scene function
        // directly (see the planet twin).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art = scene_art_of(declared_interface(State::Shop).expect("shop: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_shop_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "shop staged art paints nothing"
        );
    }

    #[test]
    fn music_shop_stages_bound_art_when_archives_present() {
        // Catalogue rows need the song index join (OPEN); this pins
        // the staging shape only.
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::MusicShop).expect("music shop: unstaged state");
        let (placements, figures) = (art.placements, art.figures);
        assert!(!placements.is_empty());
        assert!(figures.contains_key("State_MShop.ojs"));
    }

    #[test]
    fn music_shop_paints_real_art_when_archives_present() {
        // Unreachable screens still paint through their scene function
        // directly (see the planet twin).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art =
            scene_art_of(declared_interface(State::MusicShop).expect("music shop: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_music_shop_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "music shop staged art paints nothing"
        );
    }

    #[test]
    fn room_paint_uses_staged_bound_art_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let (bare, staged) = paint_pair(Screen::Room, 0);
        assert_ne!(bare, staged);
    }

    #[test]
    fn waiting_paint_uses_staged_bound_art_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let (bare, staged) = paint_pair(Screen::Waiting, 0);
        assert_ne!(bare, staged);
    }

    #[test]
    fn loading_paint_uses_staged_gate_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let (bare, staged) = paint_pair(Screen::Loading, 0);
        assert_ne!(bare, staged);
    }

    #[test]
    fn playing_paint_uses_staged_venue_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let mut bare = live_game();
        bare.load_chart(vec![(2u8, 60_000u64, 7u32)]);
        walk_to(&mut bare, Screen::Playing);
        let bare_pixels = painted_pixels(&mut bare, 0);
        let mut staged = live_game();
        staged.load_chart(vec![(2u8, 60_000u64, 7u32)]);
        stage_live_scenes(&mut staged);
        walk_to(&mut staged, Screen::Playing);
        assert_eq!(staged.screen(), Screen::Playing);
        let staged_pixels = painted_pixels(&mut staged, 0);
        assert_ne!(bare_pixels, staged_pixels);
    }

    #[test]
    fn result_paint_uses_staged_banner_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let (bare, staged) = paint_pair(Screen::Playing, 0);
        assert_ne!(bare, staged);
    }
}
