//! Soak main scenes with genuine 7.55 archive art (local-only).
//!
//! Reads `install-root` archives read-only, stages each scene through
//! `load_scene_figures` plus `bind_scene_rects` (bound blocks) plus
//! `paint_scene_figures`, drives `ScreenGame` to that scene, and asserts
//! staged ink survives under the live UI ink. Skips (passes) when the
//! archives are absent. Never embeds asset bytes.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use o2jam_file::{Bnd, ControlList};
use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
use o2jam_render::{pack_xrgb, Frame, Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::avatar::{
    bind_scene_rects, load_avi_logo, load_scene_figures, paint_scene_figures, scene_block,
    scene_sprite_names, AvatarFigure, ScenePlacement, AVI_BLOCK, LOADING_SPRITE, PLANET_BLOCK,
    RESULT_BLOCK, ROOM_BLOCK, WAITING_BLOCK,
};
use o2jam_shell::screens::paint::{CHANNEL_DIALOG_SPRITE, CHANNEL_ENTER_SPRITE, PLAYING_BLOCK};
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

/// Probe frame dimensions (matches the `shots.rs` aspect).
const PROBE_W: usize = 400;
/// Probe frame height in pixels.
const PROBE_H: usize = 300;

/// Interface archive under `install-root` (Avi/Channel/Room/Waiting/Result).
const INTERFACE_ARCHIVE: &str = "O2Jam/Image/Interface1.opi";
/// Playing archive under `install-root` (Playing/Loading).
const PLAYING_ARCHIVE: &str = "O2Jam/Image/Playing1.opi";
/// Interface control list entry.
const INTERFACE_LIST: &str = "ControlList_Interface.txt";
/// Playing control list entry.
const PLAYING_LIST: &str = "ControlList_Playing.txt";

/// `STATE_CHANNEL` names zero sprites (verified), so the soak stages
/// the code-mounted channel dialog art instead: the dialog body at
/// its record anchor plus the enter-button frames at the
/// reference-measured faces (see `docs/channel-fill.md`).
const CHANNEL_PROBE_W: usize = 800;
/// Native probe height for the channel soak (record anchors exact).
const CHANNEL_PROBE_H: usize = 600;
/// Small corner life gauge: the Playing bound window stages no decodable
/// art (shadows only, TBD subset), and the block's full-bleed backdrops
/// would blanket the highway, so the soak pins this gauge full-canvas.
/// Its native anchor sits at the frame corner, clear of the note lane.
const PLAYING_GAUGE_SPRITE: &str = "Avatar_Info_LifeGauge.ojs";

/// Live list text ink.
const WHITE_TEXT: u32 = pack_xrgb(0xFF, 0xFF, 0xFF);
/// Highlight cursor ink.
const AMBER_CURSOR: u32 = pack_xrgb(0xFF, 0xD0, 0x20);
/// Waiting ready-dot ink.
const READY_GREEN: u32 = pack_xrgb(0x30, 0xE0, 0x30);
/// Loading progress-bar ink.
const LOADING_GREEN: u32 = pack_xrgb(0x30, 0xB0, 0x30);
/// Highway note ink.
const NOTE_BLUE: u32 = pack_xrgb(0x40, 0xC0, 0xFF);

/// Repository-local `install-root` (git-ignored game data).
fn install_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

/// Archive path under `install-root`.
fn archive_path(name: &str) -> PathBuf {
    install_root().join(name)
}

/// Read an archive, if present (`None` means skip the test).
fn read_archive(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

/// Slice one entry body by its stored size (never the reserved stride).
fn entry_body(archive: &[u8], entry_name: &str) -> Option<Vec<u8>> {
    let entries = o2jam_file::parse_opi(archive).ok()?;
    let entry = entries.iter().find(|entry| entry.name == entry_name)?;
    let size = usize::try_from(entry.size).ok()?;
    let end = entry.data_offset.checked_add(size)?;
    let body = archive.get(entry.data_offset..end)?;
    Some(body.to_vec())
}

/// Parse a `ControlList_*.txt` entry from an archive.
fn control_list(archive: &[u8], entry_name: &str) -> Option<ControlList> {
    let body = entry_body(archive, entry_name)?;
    let text = o2jam_file::decode_control_text(&body).ok()?;
    o2jam_file::parse_control_list(&text).ok()
}

/// Parse a `.bnd` layout entry from an archive.
fn bind_data(archive: &[u8], bound_file: &str) -> Option<Bnd> {
    let body = entry_body(archive, bound_file)?;
    o2jam_file::parse_bnd(&body).ok()
}

/// Full-canvas placements for blocks without a `.bnd`.
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

/// Stage a bound block: named figures plus `.bnd` placements.
fn stage_bound_block(
    archive: &[u8],
    list: &ControlList,
    block_name: &str,
) -> Option<(Vec<ScenePlacement>, HashMap<String, AvatarFigure>)> {
    let block = scene_block(list, block_name)?;
    let names = scene_sprite_names(block);
    if names.is_empty() {
        return None;
    }
    let figures = load_scene_figures(archive, &names);
    if figures.is_empty() {
        return None;
    }
    let bound_file = block.bound_file.clone()?;
    let bnd = bind_data(archive, &bound_file)?;
    let placements = bind_scene_rects(block, &bnd);
    if placements.is_empty() {
        return None;
    }
    Some((placements, figures))
}

/// Stage one sprite full-canvas (Channel fallback, Loading gate).
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

/// Distinct staged pixels painted alone on a blank frame.
fn staged_palette(
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
) -> HashSet<u32> {
    let mut buf = vec![0u32; PROBE_W * PROBE_H];
    paint_scene_figures(
        &mut Frame {
            buf: &mut buf,
            w: PROBE_W,
            h: PROBE_H,
        },
        placements,
        figures,
    );
    buf.into_iter().filter(|pixel| *pixel != 0).collect()
}

/// Staged ink survives the UI overlay, every UI ink paints, and the
/// frame is never one flat fill.
fn assert_composed(frame: &[u32], staged: &HashSet<u32>, ui_inks: &[u32], scene: &str) {
    assert!(!staged.is_empty(), "{scene}: no staged ink decoded");
    assert!(
        frame.iter().any(|pixel| staged.contains(pixel)),
        "{scene}: staged ink missing under UI"
    );
    for ink in ui_inks {
        assert!(frame.contains(ink), "{scene}: UI ink missing");
    }
    assert!(
        frame.iter().any(|pixel| *pixel != frame[0]),
        "{scene}: frame is one flat fill"
    );
}

/// Paint a driven game headlessly into a fresh buffer.
fn painted(game: &mut ScreenGame, now_ms: u64) -> Vec<u32> {
    let mut buf = vec![0u32; PROBE_W * PROBE_H];
    game.paint_at(
        &mut Frame {
            buf: &mut buf,
            w: PROBE_W,
            h: PROBE_H,
        },
        now_ms,
    );
    buf
}

/// Two listed channels: one joinable, one dark.
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

/// One room row.
fn one_room() -> RoomListEntry {
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
    }
}

/// Joined room with one ready host beside our own seat.
fn seated_room() -> JoinedRoom {
    let host = RoomMember {
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
            member: Some(host),
        }],
        skills: Vec::new(),
        premium: false,
        champion: None,
        champion_streak: None,
    }
}

/// Walk the menu journey from splash to the loading gate.
fn walk_to_loading(game: &mut ScreenGame) {
    game.lane_key(3, true);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Loading);
}

/// Walk the menu journey from splash to gameplay.
fn walk_to_playing(game: &mut ScreenGame) {
    walk_to_loading(game);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Playing);
}

/// Read the Interface archive and its control list, if present.
fn interface_and_list() -> Option<(Vec<u8>, ControlList)> {
    let path = archive_path(INTERFACE_ARCHIVE);
    let archive = read_archive(&path)?;
    let list = control_list(&archive, INTERFACE_LIST)?;
    Some((archive, list))
}

/// Read the Playing archive and its control list, if present.
fn playing_and_list() -> Option<(Vec<u8>, ControlList)> {
    let path = archive_path(PLAYING_ARCHIVE);
    let archive = read_archive(&path)?;
    let list = control_list(&archive, PLAYING_LIST)?;
    Some((archive, list))
}

#[test]
fn avi_layers_real_logo_over_bound_backdrop() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some((archive, list)) = interface_and_list() else {
        println!("skip avi: missing {}", path.display());
        return;
    };
    let Some((placements, figures)) = stage_bound_block(&archive, &list, AVI_BLOCK) else {
        panic!("avi: STATE_AVI staged no figures");
    };
    let Some(logo) = load_avi_logo(&archive) else {
        panic!("avi: O2mediaLogo.ojs decoded no figure");
    };
    let staged = staged_palette(&placements, &figures);
    let splash_ink: HashSet<u32> = {
        let mut buf = vec![0u32; PROBE_W * PROBE_H];
        logo.paint(
            &mut Frame {
                buf: &mut buf,
                w: PROBE_W,
                h: PROBE_H,
            },
            0,
            0,
        );
        buf.into_iter().filter(|pixel| *pixel != 0).collect()
    };
    let mut game = ScreenGame::new();
    game.load_scene(Screen::Avi, placements, figures);
    game.load_splash(logo);
    let frame = painted(&mut game, 0);
    assert_composed(&frame, &staged, &[], "avi");
    assert!(
        frame.iter().any(|pixel| splash_ink.contains(pixel)),
        "avi: splash logo ink missing"
    );
}

#[test]
fn channel_layers_list_over_real_dialog_art() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some(archive) = read_archive(&path) else {
        println!("skip channel: missing {}", path.display());
        return;
    };
    let names = vec![
        CHANNEL_DIALOG_SPRITE.to_owned(),
        CHANNEL_ENTER_SPRITE.to_owned(),
    ];
    let figures = load_scene_figures(&archive, &names);
    assert!(
        !figures.is_empty(),
        "channel: dialog body staged no figures"
    );
    // Art-less baseline: same channels, empty scene art.
    let mut base_game = ScreenGame::new();
    base_game.set_channels(two_channels());
    base_game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
    base_game.lane_key(3, true);
    let mut base = vec![0u32; CHANNEL_PROBE_W * CHANNEL_PROBE_H];
    base_game.paint_at(
        &mut Frame {
            buf: &mut base,
            w: CHANNEL_PROBE_W,
            h: CHANNEL_PROBE_H,
        },
        0,
    );
    let mut game = ScreenGame::new();
    game.set_channels(two_channels());
    game.load_scene(Screen::Channel, Vec::new(), figures);
    game.lane_key(3, true);
    assert_eq!(game.screen(), Screen::Channel);
    let mut frame = vec![0u32; CHANNEL_PROBE_W * CHANNEL_PROBE_H];
    game.paint_at(
        &mut Frame {
            buf: &mut frame,
            w: CHANNEL_PROBE_W,
            h: CHANNEL_PROBE_H,
        },
        0,
    );
    // Staged dialog ink survives under the live list ink; the frame
    // is never one flat fill.
    assert!(
        frame.iter().any(|&px| px != frame[0]),
        "channel: frame is one flat fill"
    );
    for ink in [WHITE_TEXT, AMBER_CURSOR] {
        assert!(frame.contains(&ink), "channel: UI ink {ink:#x} missing");
    }
    // Reference-measured mount proof: every pixel the staged art
    // changes against the art-less baseline sits inside the mounted
    // 800x466 dialog body (record anchor (0,0)); nothing paints at
    // or below native y466 (no-fill corner), and some new ink lands
    // inside the measured panel core (x185-615, y150-464).
    let mut inside_core = false;
    for (index, (&now, &before)) in frame.iter().zip(base.iter()).enumerate() {
        if now == before {
            continue;
        }
        let x = index % CHANNEL_PROBE_W;
        let y = index / CHANNEL_PROBE_W;
        assert!(
            y < 466,
            "channel: staged ink below the dialog body at ({x},{y})"
        );
        if (185..=615).contains(&x) && (150..=464).contains(&y) {
            inside_core = true;
        }
    }
    assert!(
        inside_core,
        "channel: no staged ink inside the measured panel core"
    );
    // Bottom strip untouched (pinned no-fill beyond the art).
    for index in 466 * CHANNEL_PROBE_W..frame.len() {
        assert_eq!(frame[index], base[index], "channel: fill below the body");
    }
}

#[test]
fn planet_layers_staged_art_under_the_live_cursor() {
    // The bound planet block stages behind the live chrome: a FREE
    // click stays unbound (P0), a HARD click records the pick, and
    // the amber cursor stand-in plus the staged button art all ink
    // the frame.
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some((archive, list)) = interface_and_list() else {
        println!("skip planet: missing {}", path.display());
        return;
    };
    let Some((placements, figures)) = stage_bound_block(&archive, &list, PLANET_BLOCK) else {
        panic!("planet: STATE_PLANET staged no figures");
    };
    let staged = staged_palette(&placements, &figures);
    let mut game = ScreenGame::new();
    game.load_scene(Screen::Planet, placements, figures);
    game.lane_key(3, true);
    assert!(game.enter_screen(Screen::Planet), "Channel→Planet edge");
    game.click_planet(614 + 49, 32 + 56);
    assert_eq!(
        game.planet_selection(),
        None,
        "planet: FREE click must stay unbound (P0)"
    );
    game.click_planet(510 + 49, 32 + 56);
    assert_eq!(
        game.planet_selection(),
        None,
        "planet: DRUM click must stay unbound (P0)"
    );
    game.click_planet(313 + 73, 32 + 56);
    assert_eq!(
        game.planet_selection(),
        Some(o2jam_shell::screens::side::PlanetBand::Hard)
    );
    let frame = painted(&mut game, 0);
    assert_composed(&frame, &staged, &[AMBER_CURSOR], "planet");
}

#[test]
fn planet_without_art_keeps_the_flat_card() {
    // No staged art: the caller's flat-card fallback paints (the
    // scene painter is never invoked), and a click still records
    // the pick through the pinned rects.
    let mut game = ScreenGame::new();
    game.lane_key(3, true);
    assert!(game.enter_screen(Screen::Planet));
    game.click_planet(206 + 49, 32 + 56);
    assert_eq!(
        game.planet_selection(),
        Some(o2jam_shell::screens::side::PlanetBand::Normal)
    );
    let frame = painted(&mut game, 0);
    assert!(
        frame.iter().any(|&px| px != frame[0]),
        "planet: flat card painted nothing"
    );
}

#[test]
fn room_layers_rows_over_bound_room_art() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some((archive, list)) = interface_and_list() else {
        println!("skip room: missing {}", path.display());
        return;
    };
    let Some((placements, figures)) = stage_bound_block(&archive, &list, ROOM_BLOCK) else {
        panic!("room: STATE_ROOM staged no figures");
    };
    let staged = staged_palette(&placements, &figures);
    let mut game = ScreenGame::new();
    game.set_rooms(vec![one_room()]);
    game.load_scene(Screen::Room, placements, figures);
    game.lane_key(3, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Room);
    let frame = painted(&mut game, 0);
    assert_composed(&frame, &staged, &[WHITE_TEXT, AMBER_CURSOR], "room");
}

#[test]
fn waiting_layers_seats_over_bound_waiting_art() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some((archive, list)) = interface_and_list() else {
        println!("skip waiting: missing {}", path.display());
        return;
    };
    let Some((placements, figures)) = stage_bound_block(&archive, &list, WAITING_BLOCK) else {
        panic!("waiting: STATE_WAITING staged no figures");
    };
    let staged = staged_palette(&placements, &figures);
    let mut game = ScreenGame::new();
    game.set_joined_room(seated_room());
    game.load_scene(Screen::Waiting, placements, figures);
    game.lane_key(3, true);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Waiting);
    let frame = painted(&mut game, 0);
    assert_composed(&frame, &staged, &[WHITE_TEXT, READY_GREEN], "waiting");
}

#[test]
fn loading_layers_bar_over_real_loading_gate() {
    let path = archive_path(PLAYING_ARCHIVE);
    let Some(archive) = read_archive(&path) else {
        println!("skip loading: missing {}", path.display());
        return;
    };
    let Some((placements, figures)) = stage_single_sprite(&archive, LOADING_SPRITE) else {
        panic!("loading: MusicLoading.ojs staged no figures");
    };
    let staged = staged_palette(&placements, &figures);
    let mut game = ScreenGame::new();
    game.load_scene(Screen::Loading, placements, figures);
    walk_to_loading(&mut game);
    let frame = painted(&mut game, 0);
    assert_composed(&frame, &staged, &[LOADING_GREEN], "loading");
}

#[test]
fn playing_layers_notes_over_bound_gauges() {
    let path = archive_path(PLAYING_ARCHIVE);
    let Some((archive, list)) = playing_and_list() else {
        println!("skip playing: missing {}", path.display());
        return;
    };
    let Some((mut placements, figures)) = stage_bound_block(&archive, &list, PLAYING_BLOCK) else {
        panic!("playing: STATE_PLAYING staged no figures");
    };
    assert!(
        figures.contains_key(PLAYING_GAUGE_SPRITE),
        "playing: gauge sprite decoded nothing"
    );
    placements.extend(full_canvas(&[PLAYING_GAUGE_SPRITE.to_owned()]));
    let staged = staged_palette(&placements, &figures);
    let mut game = ScreenGame::new();
    game.load_chart(vec![(2, 1500, 7)]);
    game.load_scene(Screen::Playing, placements, figures);
    walk_to_playing(&mut game);
    let frame = painted(&mut game, 1000);
    assert_composed(&frame, &staged, &[NOTE_BLUE], "playing");
}

#[test]
fn result_layers_digits_over_bound_result_art() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some((archive, list)) = interface_and_list() else {
        println!("skip result: missing {}", path.display());
        return;
    };
    let Some((placements, figures)) = stage_bound_block(&archive, &list, RESULT_BLOCK) else {
        panic!("result: STATE_RESULT staged no figures");
    };
    let staged = staged_palette(&placements, &figures);
    let mut game = ScreenGame::new();
    game.load_chart(vec![(2, 5000, 7)]);
    game.load_scene(Screen::Result, placements, figures);
    walk_to_playing(&mut game);
    game.strike_at(2, 5000);
    let frame = painted(&mut game, 7000);
    assert_eq!(game.screen(), Screen::Result);
    assert_composed(&frame, &staged, &[WHITE_TEXT], "result");
}
