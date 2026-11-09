//! Live session scene art: genuine 7.55 archive backdrops under the
//! windowed online walk (`run_play --window --online`).
//!
//! The shots walk (`shots.rs`) is the only path that stages scene art, so
//! every live screen paints the flat fallback. This module threads the same
//! staging pattern — bound art plus figures through
//! [`ScreenGame::load_scene`] — into the live flow, driven by live room
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

use o2jam_file::{Bnd, ControlList};
use o2jam_render::{Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::avatar::{
    bind_scene_rects, load_archive_sprite, load_scene_figures, load_scene_figures_for_room,
    load_scene_figures_for_stage, scene_block, scene_sprite_names, AvatarFigure, PlayingArtMode,
    ScenePlacement, AVI_BLOCK, LANKING_BLOCK, LOADING_SPRITE, MUSICSHOP_BLOCK, MYROOM_BLOCK,
    PLANET_BLOCK, RESULT_BLOCK, ROOM_BLOCK, SHOP_BLOCK, WAITING_BLOCK,
};
use o2jam_shell::screens::paint::{PLAYING_BLOCK, RESULT_WINLOSE_SPRITE};
use o2jam_shell::screens::side::SELECT_MUSIC_DIALOG_BLOCK;
use o2jam_shell::{PlaySkin, Screen, ScreenGame};

/// Interface archive under `install-root` (Avi/Room/Waiting/Result art).
const INTERFACE_ARCHIVE: &str = "O2Jam/Image/Interface1.opi";
/// Playing archive under `install-root` (Loading/Playing art).
const PLAYING_ARCHIVE: &str = "O2Jam/Image/Playing1.opi";
/// Interface control list entry.
const INTERFACE_LIST: &str = "ControlList_Interface.txt";
/// Playing control list entry.
const PLAYING_LIST: &str = "ControlList_Playing.txt";

/// Repository-local `install-root` (git-ignored game data).
fn install_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

/// Read one archive read-only, if present (`None` keeps the fallback).
fn read_archive(relative: &str) -> Option<Vec<u8>> {
    std::fs::read(install_root().join(relative)).ok()
}

/// Borrow one entry body by its stored size (never the reserved stride).
fn entry_body<'a>(archive: &'a [u8], entry_name: &str) -> Option<&'a [u8]> {
    let entries = o2jam_file::parse_opi(archive).ok()?;
    let entry = entries.iter().find(|entry| entry.name == entry_name)?;
    let size = usize::try_from(entry.size).ok()?;
    let end = entry.data_offset.checked_add(size)?;
    archive.get(entry.data_offset..end)
}

/// Parse a `ControlList_*.txt` entry from an archive.
fn control_list(archive: &[u8], entry_name: &str) -> Option<ControlList> {
    let body = entry_body(archive, entry_name)?;
    let text = o2jam_file::decode_control_text(body).ok()?;
    o2jam_file::parse_control_list(&text).ok()
}

/// Parse a `.bnd` layout entry from an archive.
fn bind_data(archive: &[u8], bound_file: &str) -> Option<Bnd> {
    let body = entry_body(archive, bound_file)?;
    o2jam_file::parse_bnd(body).ok()
}

/// Full-canvas placements for art without a `.bnd`.
fn full_canvas(names: &[String]) -> Vec<ScenePlacement> {
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

/// Stage a bound block: named figures plus `.bnd` placements. The live
/// arena stage rides along so `_` runtime refs resolve their
/// stage-numbered siblings; the live room mode selects the Playing
/// family members (unselected members leave empty slots that paint
/// nothing). `None` stages nothing — one bad block never blanks the rest.
fn stage_bound_block(
    archive: &[u8],
    list: &ControlList,
    block_name: &str,
    arena_stage: Option<u16>,
    mode: Option<&PlayingArtMode>,
) -> Option<(Vec<ScenePlacement>, HashMap<String, AvatarFigure>)> {
    let block = scene_block(list, block_name)?;
    let names = scene_sprite_names(block);
    if names.is_empty() {
        return None;
    }
    let figures = match mode {
        Some(mode) => load_scene_figures_for_room(archive, &names, arena_stage, mode),
        None => load_scene_figures_for_stage(archive, &names, arena_stage),
    };
    if figures.is_empty() {
        return None;
    }
    let bnd = bind_data(archive, block.bound_file.as_ref()?)?;
    let placements = bind_scene_rects(block, &bnd);
    if placements.is_empty() {
        return None;
    }
    Some((placements, figures))
}

/// Stage one sprite full-canvas (Loading gate).
fn stage_single_sprite(
    archive: &[u8],
    sprite_name: &str,
) -> Option<(Vec<ScenePlacement>, HashMap<String, AvatarFigure>)> {
    let names = vec![sprite_name.to_owned()];
    let figures = load_scene_figures(archive, &names);
    if figures.is_empty() {
        return None;
    }
    Some((full_canvas(&names), figures))
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

/// Stage Interface-archive scenes: Avi/Room/Waiting bound blocks, the
/// Result block plus its verdict banner body, and the song-select dialog
/// art over the Waiting seats. Each scene stages independently.
fn stage_interface_scenes(game: &mut ScreenGame, archive: &[u8]) {
    let Some(list) = control_list(archive, INTERFACE_LIST) else {
        return;
    };
    for (screen, block) in [
        (Screen::Avi, AVI_BLOCK),
        (Screen::Room, ROOM_BLOCK),
        (Screen::Waiting, WAITING_BLOCK),
        (Screen::Planet, PLANET_BLOCK),
        (Screen::Lanking, LANKING_BLOCK),
        (Screen::Shop, SHOP_BLOCK),
        (Screen::MusicShop, MUSICSHOP_BLOCK),
    ] {
        if let Some((placements, figures)) = stage_bound_block(archive, &list, block, None, None) {
            game.load_scene(screen, placements, figures);
        }
    }
    stage_result_scene(game, archive, &list);
    stage_select_dialog_art(game, archive, &list);
    stage_anchor_scene(game, archive, &list);
}

/// Stage an unbound block: sprite art plus anchor placements (no
/// `.bnd` bind file exists, so rects never enter the picture).
fn stage_anchor_scene(game: &mut ScreenGame, archive: &[u8], list: &ControlList) {
    use o2jam_shell::avatar::anchor_placements;
    let Some(block) = scene_block(list, MYROOM_BLOCK) else {
        return;
    };
    let names = scene_sprite_names(block);
    if names.is_empty() {
        return;
    }
    let figures = load_scene_figures(archive, &names);
    let placements = anchor_placements(block);
    game.load_scene(Screen::Myroom, placements, figures);
}

/// Stage the Result scene: the `STATE_RESULT` block art plus the
/// builder-mounted verdict banner body (the block carries only the
/// background — a missing body keeps the scene banner-less).
fn stage_result_scene(game: &mut ScreenGame, archive: &[u8], list: &ControlList) {
    let Some((placements, mut figures)) =
        stage_bound_block(archive, list, RESULT_BLOCK, None, None)
    else {
        return;
    };
    if let Some(banner) = load_archive_sprite(archive, RESULT_WINLOSE_SPRITE) {
        figures.insert(RESULT_WINLOSE_SPRITE.to_owned(), banner);
    }
    game.load_scene(Screen::Result, placements, figures);
}

/// Stage real song-select dialog art over the Waiting seats; empty art
/// paints nothing extra.
fn stage_select_dialog_art(game: &mut ScreenGame, archive: &[u8], list: &ControlList) {
    let Some(block) = scene_block(list, SELECT_MUSIC_DIALOG_BLOCK) else {
        return;
    };
    let names = scene_sprite_names(block);
    if names.is_empty() {
        return;
    }
    let figures = load_scene_figures(archive, &names);
    if figures.is_empty() {
        return;
    }
    let Some(bound_file) = block.bound_file.as_ref() else {
        return;
    };
    let Some(bnd) = bind_data(archive, bound_file) else {
        return;
    };
    let placements = bind_scene_rects(block, &bnd);
    if placements.is_empty() {
        return;
    }
    game.load_select_music_scene(placements, figures);
}

/// Stage Playing-archive scenes through the live room: the Loading gate
/// sprite plus the Playing gauges through the room arena stage and key
/// mode, and the stage-aware play skin (procedural lanes stay on a miss).
fn stage_playing_scenes(game: &mut ScreenGame, archive: &[u8]) {
    if let Some((placements, figures)) = stage_single_sprite(archive, LOADING_SPRITE) {
        game.load_scene(Screen::Loading, placements, figures);
    }
    let Some(list) = control_list(archive, PLAYING_LIST) else {
        return;
    };
    let mode = live_playing_mode(game);
    let Some((placements, figures)) = stage_bound_block(
        archive,
        &list,
        PLAYING_BLOCK,
        Some(game.arena_stage()),
        Some(&mode),
    ) else {
        return;
    };
    game.load_scene(Screen::Playing, placements, figures);
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
    if let Some(archive) = read_archive(INTERFACE_ARCHIVE) {
        stage_interface_scenes(game, &archive);
    }
    if let Some(archive) = read_archive(PLAYING_ARCHIVE) {
        stage_playing_scenes(game, &archive);
    }
}

/// Re-stage the Playing-archive scenes after the joined room changes
/// (join, host, arena echo): the venue and family members follow the
/// live arena stage and mode. Interface scenes never depend on the
/// room, so they keep their login-time staging.
pub fn restage_live_playing(game: &mut ScreenGame) {
    let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
        return;
    };
    stage_playing_scenes(game, &archive);
}

#[cfg(test)]
mod live_art_tests {
    use o2jam_net::{ChannelState, GameMode, JoinedRoom, RoomListEntry};
    use o2jam_render::{Frame, NATIVE_HEIGHT, NATIVE_WIDTH};
    use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

    use super::{
        control_list, load_scene_figures, read_archive, restage_live_playing, stage_bound_block,
        stage_live_scenes, INTERFACE_ARCHIVE, INTERFACE_LIST, PLAYING_ARCHIVE,
    };
    use o2jam_shell::avatar::{LANKING_BLOCK, MUSICSHOP_BLOCK, PLANET_BLOCK, SHOP_BLOCK};

    /// Archives behind the staging (`true` while `install-root` holds
    /// the 7.55 interface art; the suite stays green without it).
    fn interface_present() -> bool {
        read_archive(super::INTERFACE_ARCHIVE).is_some()
    }

    /// Archives behind the Playing staging.
    fn playing_present() -> bool {
        read_archive(PLAYING_ARCHIVE).is_some()
    }

    /// Skip helper: the staged-pixel comparisons need real archives.
    fn skip_without_archives() -> bool {
        if interface_present() && playing_present() {
            return false;
        }
        println!("skip: missing install-root archives");
        true
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

    /// Walk the offline flow to one screen (lane dismisses the splash,
    /// then one Confirm per edge).
    fn walk_to(game: &mut ScreenGame, target: Screen) {
        game.lane_key(3, true);
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
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let Some((placements, figures)) =
            stage_bound_block(&archive, &list, PLANET_BLOCK, None, None)
        else {
            panic!("planet: unstaged bound block");
        };
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
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let Some((placements, figures)) =
            stage_bound_block(&archive, &list, PLANET_BLOCK, None, None)
        else {
            panic!("planet: unstaged bound block");
        };
        let art = o2jam_shell::screens::paint::SceneArt {
            placements,
            figures,
        };
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
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let Some((placements, figures)) =
            stage_bound_block(&archive, &list, LANKING_BLOCK, None, None)
        else {
            panic!("lanking: unstaged bound block");
        };
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
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let Some((placements, figures)) =
            stage_bound_block(&archive, &list, LANKING_BLOCK, None, None)
        else {
            panic!("lanking: unstaged bound block");
        };
        let art = o2jam_shell::screens::paint::SceneArt {
            placements,
            figures,
        };
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

    #[test]
    fn myroom_stages_anchor_art_when_archives_present() {
        // No bind file exists, so placements carry sprite anchors
        // instead of rects; the full-bleed state art grows.
        if skip_without_archives() {
            return;
        }
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let block = o2jam_shell::avatar::scene_block(&list, o2jam_shell::avatar::MYROOM_BLOCK)
            .expect("myroom block");
        let names = o2jam_shell::avatar::scene_sprite_names(block);
        assert!(!names.is_empty());
        let figures = load_scene_figures(&archive, &names);
        assert!(figures.contains_key("State_Myroom.ojs"));
        let placements = o2jam_shell::avatar::anchor_placements(block);
        assert!(!placements.is_empty());
    }

    #[test]
    fn myroom_paints_real_art_when_archives_present() {
        // Unreachable screens still paint through their scene function
        // directly (see the planet twin).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let block = o2jam_shell::avatar::scene_block(&list, o2jam_shell::avatar::MYROOM_BLOCK)
            .expect("myroom block");
        let names = o2jam_shell::avatar::scene_sprite_names(block);
        let figures = load_scene_figures(&archive, &names);
        let placements = o2jam_shell::avatar::anchor_placements(block);
        let art = o2jam_shell::screens::paint::SceneArt {
            placements,
            figures,
        };
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
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let Some((placements, figures)) =
            stage_bound_block(&archive, &list, SHOP_BLOCK, None, None)
        else {
            panic!("shop: unstaged bound block");
        };
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
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let Some((placements, figures)) =
            stage_bound_block(&archive, &list, SHOP_BLOCK, None, None)
        else {
            panic!("shop: unstaged bound block");
        };
        let art = o2jam_shell::screens::paint::SceneArt {
            placements,
            figures,
        };
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
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let Some((placements, figures)) =
            stage_bound_block(&archive, &list, MUSICSHOP_BLOCK, None, None)
        else {
            panic!("music shop: unstaged bound block");
        };
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
        let archive = read_archive(INTERFACE_ARCHIVE).expect("guarded archive");
        let list = control_list(&archive, INTERFACE_LIST).expect("guarded list");
        let Some((placements, figures)) =
            stage_bound_block(&archive, &list, MUSICSHOP_BLOCK, None, None)
        else {
            panic!("music shop: unstaged bound block");
        };
        let art = o2jam_shell::screens::paint::SceneArt {
            placements,
            figures,
        };
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
