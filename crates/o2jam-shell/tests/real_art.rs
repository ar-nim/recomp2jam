//! Soak main scenes with genuine 7.55 archive art (local-only).
//!
//! Reads `install-root` archives read-only, stages each scene through
//! the State-keyed staging declaration (`stage_state`/`stage_dialog`)
//! plus `paint_scene_figures`, drives `ScreenGame` to that scene, and asserts
//! staged ink survives under the live UI ink. Skips (passes) when the
//! archives are absent. Never embeds asset bytes.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
use o2jam_render::{pack_xrgb, Frame};
use o2jam_shell::avatar::{paint_scene_figures, AvatarFigure, PlacementRole, ScenePlacement};
use o2jam_shell::screens::paint::RESULT_WINLOSE_SPRITE;
use o2jam_shell::screens::side::{
    ROOM_BACK_BUTTON_CODE, ROOM_BACK_BUTTON_ID, ROOM_OPTION_BUTTON_CODE, ROOM_OPTION_BUTTON_ID,
};
use o2jam_shell::staging::{
    read_install_archives, stage_dialog, stage_state, Dialog, InstallArchives, StageContext, State,
};
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

/// Probe frame dimensions (matches the `shots.rs` aspect).
const PROBE_W: usize = 400;
/// Probe frame height in pixels.
const PROBE_H: usize = 300;

/// Interface archive under `install-root` (Avi/Channel/Room/Waiting/Result).
const INTERFACE_ARCHIVE: &str = "O2Jam/Image/Interface1.opi";
/// Playing archive under `install-root` (Playing/Loading).
const PLAYING_ARCHIVE: &str = "O2Jam/Image/Playing1.opi";

/// `STATE_CHANNEL` names zero sprites (verified), so the soak stages
/// the code-mounted channel dialog art instead: the dialog body at
/// its record anchor plus the bnd-part figures — enter-button faces,
/// row bars, digit/count plates, scroll bodies — which the painter
/// mounts at their bnd-derived origins (see `docs/channel-fill.md`
/// "METHOD CORRECTION 2026-11-10"). The channel dialog opens OVER the
/// planet board (verified first screen, `docs/planet.md`), so the
/// soak stages the bound `State_Planet` block beneath the dialog art
/// too.
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

/// Archives read once per test (`None` per missing file means skip
/// the test).
fn install_archives() -> InstallArchives {
    read_install_archives(&install_root())
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
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Loading);
}

/// Walk the menu journey from splash to gameplay. The Loading
/// Confirm runs the gate, so the walk stages the session shape
/// first: a validated pick, the null audio rig, and the walk at
/// the armed phase (decrypt and Vorbis init already staged).
fn walk_to_playing(game: &mut ScreenGame) {
    walk_to_loading(game);
    game.set_selected_music(o2jam_net::play::SetRoomMusic {
        music_id: 757,
        difficulty: 1,
        speed: 0,
    });
    game.with_audio(Box::new(o2jam_audio::NullBackend::new()), HashMap::new());
    game.set_loading_status(o2jam_shell::screens::paint::LoadingStatus {
        progress: 1.0,
        title: "First".to_owned(),
        artist: "Band".to_owned(),
    });
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Playing);
}

#[test]
fn avi_paints_real_logo_through_the_bound_record() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some(archive) = install_archives().interface else {
        println!("skip avi: missing {}", path.display());
        return;
    };
    let Some(art) = stage_state(&archive, State::Avi, StageContext::default()).art else {
        panic!("avi: STATE_AVI staged no figures");
    };
    let (placements, figures) = (art.placements, art.figures);
    assert!(
        placements.iter().any(|placed| placed.id == "IDC_BACK_LOGO"),
        "avi: the bound IDC_BACK_LOGO record stages the logo"
    );
    let staged = staged_palette(&placements, &figures);
    let logo_ink: HashSet<u32> = {
        let Some(logo) = figures.get("O2mediaLogo.ojs") else {
            panic!("avi: O2mediaLogo.ojs staged no figure");
        };
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
    let frame = painted(&mut game, 0);
    assert_composed(&frame, &staged, &[], "avi");
    assert!(
        frame.iter().any(|pixel| logo_ink.contains(pixel)),
        "avi: staged logo ink missing"
    );
}

#[test]
fn channel_layers_list_over_real_dialog_art() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some(archive) = install_archives().interface else {
        println!("skip channel: missing {}", path.display());
        return;
    };
    // The channel dialog stages through the declaration (bound
    // `DIALOG_CHANNEL_INFO` bodies). The hover bar stays out: the exe
    // mounts it at the Dialog object, not from the table (OPEN), and
    // only the live window hand-stages it — without it the amber
    // cursor stand-in paints.
    let figures = stage_dialog(&archive, Dialog::ChannelInfo, StageContext::default())
        .art
        .map(|art| art.figures)
        .unwrap_or_default();
    assert!(
        !figures.is_empty(),
        "channel: dialog body staged no figures"
    );
    // Verified first screen (docs/planet.md, 2026-11-09): the channel
    // dialog opens OVER the planet board, so the soak stages the bound
    // State_Planet block (backdrop + emblems + promo + MENU bar)
    // beneath the dialog art, exactly like the shots walk.
    let Some(board) = stage_state(&archive, State::Planet, StageContext::default()).art else {
        panic!("channel: STATE_PLANET staged no board figures");
    };
    let (planet_placements, planet_figures) = (board.placements, board.figures);
    // Art-less baseline: same channels, both scene slots empty.
    let mut base_game = ScreenGame::new();
    base_game.set_channels(two_channels());
    base_game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
    base_game.load_scene(Screen::Planet, Vec::new(), HashMap::new());
    base_game.lane_key(3, true);
    base_game.click_planet(136, 88);
    let mut base = vec![0u32; CHANNEL_PROBE_W * CHANNEL_PROBE_H];
    base_game.paint_at(
        &mut Frame {
            buf: &mut base,
            w: CHANNEL_PROBE_W,
            h: CHANNEL_PROBE_H,
        },
        0,
    );
    // Dialog-only arm (no board staged): the strip below the 466-tall
    // body stays byte-identical to the art-less frame (pinned no-fill
    // corner, nothing beneath).
    let mut dialog_game = ScreenGame::new();
    dialog_game.set_channels(two_channels());
    dialog_game.load_scene(Screen::Channel, Vec::new(), figures.clone());
    dialog_game.load_scene(Screen::Planet, Vec::new(), HashMap::new());
    dialog_game.lane_key(3, true);
    dialog_game.click_planet(136, 88);
    assert_eq!(dialog_game.screen(), Screen::Channel);
    let mut dialog_frame = vec![0u32; CHANNEL_PROBE_W * CHANNEL_PROBE_H];
    dialog_game.paint_at(
        &mut Frame {
            buf: &mut dialog_frame,
            w: CHANNEL_PROBE_W,
            h: CHANNEL_PROBE_H,
        },
        0,
    );
    // Board-only arm: the staged board alone explains the MENU bar band.
    let mut board_game = ScreenGame::new();
    board_game.set_channels(two_channels());
    board_game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
    board_game.load_scene(Screen::Planet, planet_placements.clone(), planet_figures);
    board_game.lane_key(3, true);
    board_game.click_planet(136, 88);
    let mut board_frame = vec![0u32; CHANNEL_PROBE_W * CHANNEL_PROBE_H];
    board_game.paint_at(
        &mut Frame {
            buf: &mut board_frame,
            w: CHANNEL_PROBE_W,
            h: CHANNEL_PROBE_H,
        },
        0,
    );
    // Full composition: board beneath the opaque dialog sheet.
    let mut game = ScreenGame::new();
    game.set_channels(two_channels());
    game.load_scene(Screen::Channel, Vec::new(), figures);
    game.load_scene(
        Screen::Planet,
        planet_placements,
        planet_figures_full(&archive),
    );
    game.lane_key(3, true);
    game.click_planet(136, 88);
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
    // Reference-measured mount proof: every pixel the dialog-only
    // stage changes against the art-less baseline sits inside the
    // mounted 800x466 dialog body (record anchor (0,0)); nothing
    // paints at or below native y466 without board art, and some new
    // ink lands inside the measured panel core (x185-615, y150-464).
    let mut inside_core = false;
    for (index, (&now, &before)) in dialog_frame.iter().zip(base.iter()).enumerate() {
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
    // No-fill corner rule without board art: the strip below the body
    // stays byte-identical to the art-less frame.
    for index in 466 * CHANNEL_PROBE_W..dialog_frame.len() {
        assert_eq!(
            dialog_frame[index], base[index],
            "channel: fill below the body without board art"
        );
    }
    // Layered rule with board art: every strip pixel that differs from
    // the art-less frame must equal the board-only paint (the MENU bar
    // band is board ink, not invented fill), and the board inks the
    // strip at all — it stays visible beneath the dialog sheet.
    let mut strip_diff = false;
    for index in 466 * CHANNEL_PROBE_W..frame.len() {
        if frame[index] == base[index] {
            continue;
        }
        strip_diff = true;
        assert_eq!(
            frame[index], board_frame[index],
            "channel: strip ink unexplained by the board at pixel {index}"
        );
    }
    assert!(
        strip_diff,
        "channel: staged board inked nothing in the MENU bar strip"
    );
}

/// Stage the full planet board figures for the layered channel soak:
/// the bound `State_Planet` block art (backdrop + emblems + promo +
/// MENU bar) the channel dialog opens over (`docs/planet.md`).
fn planet_figures_full(archive: &[u8]) -> HashMap<String, AvatarFigure> {
    stage_state(archive, State::Planet, StageContext::default())
        .art
        .map_or_else(HashMap::new, |art| art.figures)
}

#[test]
fn planet_layers_staged_art_under_the_live_cursor() {
    // The bound planet block stages behind the live chrome: a FREE
    // click stays unbound (P0), a HARD click records the pick AND
    // reopens the channel dialog (player-report corroborated,
    // per-emblem verification pending — docs/planet.md); back on the
    // board the amber cursor stand-in plus the staged button art all
    // ink the frame.
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some(archive) = install_archives().interface else {
        println!("skip planet: missing {}", path.display());
        return;
    };
    let Some(art) = stage_state(&archive, State::Planet, StageContext::default()).art else {
        panic!("planet: STATE_PLANET staged no figures");
    };
    let (placements, figures) = (art.placements, art.figures);
    let staged = staged_palette(&placements, &figures);
    let mut game = ScreenGame::new();
    game.load_scene(Screen::Planet, placements, figures);
    game.lane_key(3, true);
    assert_eq!(game.screen(), Screen::Planet, "boot → bare board");
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
    assert_eq!(
        game.screen(),
        Screen::Planet,
        "unbound clicks stay on the board"
    );
    game.click_planet(313 + 73, 32 + 56);
    assert_eq!(
        game.planet_selection(),
        Some(o2jam_shell::screens::side::PlanetBand::Hard)
    );
    assert_eq!(
        game.screen(),
        Screen::Channel,
        "planet: lit emblem reopens the channel dialog"
    );
    assert!(game.enter_screen(Screen::Planet), "dialog X → board");
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
    assert_eq!(game.screen(), Screen::Planet);
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
    let Some(archive) = install_archives().interface else {
        println!("skip room: missing {}", path.display());
        return;
    };
    let Some(art) = stage_state(&archive, State::Room, StageContext::default()).art else {
        panic!("room: STATE_ROOM staged no figures");
    };
    let (placements, figures) = (art.placements, art.figures);
    let staged = staged_palette(&placements, &figures);
    let mut game = ScreenGame::new();
    game.set_rooms(vec![one_room()]);
    game.load_scene(Screen::Room, placements, figures);
    game.lane_key(3, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Room);
    let frame = painted(&mut game, 0);
    assert_composed(&frame, &staged, &[WHITE_TEXT, AMBER_CURSOR], "room");
}

#[test]
fn room_stages_pinned_back_button() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some(archive) = install_archives().interface else {
        println!("skip room back: missing {}", path.display());
        return;
    };
    let Some(art) = stage_state(&archive, State::Room, StageContext::default()).art else {
        panic!("room back: STATE_ROOM staged no art");
    };
    let Some(placed) = art
        .placements
        .iter()
        .find(|placed| placed.code == ROOM_BACK_BUTTON_CODE)
    else {
        panic!("room back: IDC_BUTTON_SROOM_BACK missing from staged STATE_ROOM");
    };
    assert_eq!(placed.id, ROOM_BACK_BUTTON_ID);
    assert_eq!((placed.rect.x, placed.rect.y), (711, 569));
    assert_eq!((placed.rect.w, placed.rect.h), (83, 24));
}

#[test]
fn room_stages_pinned_option_button() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some(archive) = install_archives().interface else {
        println!("skip room option: missing {}", path.display());
        return;
    };
    let Some(art) = stage_state(&archive, State::Room, StageContext::default()).art else {
        panic!("room option: STATE_ROOM staged no art");
    };
    let Some(placed) = art
        .placements
        .iter()
        .find(|placed| placed.code == ROOM_OPTION_BUTTON_CODE)
    else {
        panic!("room option: IDC_BUTTON_SROOM_OPTION missing from staged STATE_ROOM");
    };
    assert_eq!(placed.id, ROOM_OPTION_BUTTON_ID);
    assert_eq!((placed.rect.x, placed.rect.y), (727, 1));
    assert_eq!((placed.rect.w, placed.rect.h), (60, 32));
}

#[test]
fn waiting_layers_seats_over_bound_waiting_art() {
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some(archive) = install_archives().interface else {
        println!("skip waiting: missing {}", path.display());
        return;
    };
    let Some(art) = stage_state(&archive, State::Waiting, StageContext::default()).art else {
        panic!("waiting: STATE_WAITING staged no figures");
    };
    let (placements, figures) = (art.placements, art.figures);
    let staged = staged_palette(&placements, &figures);
    let mut game = ScreenGame::new();
    game.set_joined_room(seated_room());
    game.load_scene(Screen::Waiting, placements, figures);
    game.lane_key(3, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Waiting);
    let frame = painted(&mut game, 0);
    assert_composed(&frame, &staged, &[WHITE_TEXT, READY_GREEN], "waiting");
}

/// Bottom edge (native px) of the Waiting scene's top button band:
/// every chrome button fully inside this band belongs to the top row
/// (the seat parents at y 55 with h 150 reach far below it).
const WAITING_TOP_BAND_BOTTOM: i32 = 120;

#[test]
fn waiting_top_buttons_sit_on_their_own_block_rects() {
    // Install-guarded (ticket #57, follow-up to #40): every top-band
    // chrome button of the staged STATE_WAITING scene (block decl
    // `0x63`, `State_Waiting.bnd`) must pair with its `.bnd` rect by
    // the code-ordinal rule — re-derived here from an independent
    // ControlList + bind parse, not from the staging output — must
    // equal that rect exactly, and must overlap no other top button.
    // This is the per-button no-overlap lock of the #40
    // template-at-origin family: a clone inheriting a template origin
    // or a pairing slip lands here as a wrong rect or an overlap.
    // Skips when install-root is absent.
    let path = archive_path(INTERFACE_ARCHIVE);
    let Some(archive) = install_archives().interface else {
        println!("skip waiting top buttons: missing {}", path.display());
        return;
    };
    // Independent oracle: parse the ControlList and the bind section
    // straight from the archive, pair entries with rects through the
    // shared exe-verified rule, and index the expected rect by code.
    let entries = o2jam_file::parse_opi(&archive).expect("Interface1.opi parses");
    let list_entry = o2jam_file::find_opi_entry(&entries, "ControlList_Interface.txt")
        .expect("ControlList_Interface.txt entry exists");
    let list_body = o2jam_file::opi_entry_body(&archive, list_entry).expect("list body in bounds");
    let list_text = o2jam_file::decode_control_text(list_body).expect("list text decodes");
    let list = o2jam_file::parse_control_list(&list_text).expect("control list parses");
    let block = o2jam_shell::avatar::scene_block(&list, "STATE_WAITING").expect("block exists");
    let bind_name = block
        .bound_file
        .clone()
        .expect("STATE_WAITING carries its BOUND line");
    let bind_entry = o2jam_file::find_opi_entry(&entries, &bind_name).expect("bind entry exists");
    let bind_body = o2jam_file::opi_entry_body(&archive, bind_entry).expect("bind body in bounds");
    let bind = o2jam_file::parse_bnd(bind_body).expect("bind parses");
    let expected: HashMap<u32, o2jam_render::Rect> =
        o2jam_file::pair_entries_with_rects(block, &bind)
            .into_iter()
            .filter_map(|placed| {
                placed.rect.map(|corners| {
                    (
                        placed.entry.code,
                        o2jam_render::Rect {
                            x: corners.x1 as i32,
                            y: corners.y1 as i32,
                            w: corners.x2 - corners.x1,
                            h: corners.y2 - corners.y1,
                        },
                    )
                })
            })
            .collect();
    // The staging under test.
    let Some(art) = stage_state(&archive, State::Waiting, StageContext::default()).art else {
        panic!("waiting top buttons: STATE_WAITING staged no art");
    };
    let top_buttons: Vec<&ScenePlacement> = art
        .placements
        .iter()
        .filter(|placed| {
            placed.role == PlacementRole::Single
                && placed.id.starts_with("IDC_BUTTON_SWAIT")
                && placed.rect.y + placed.rect.h as i32 <= WAITING_TOP_BAND_BOTTOM
        })
        .collect();
    assert!(
        top_buttons.len() >= 8,
        "the top band stages the chrome button row (team buttons plus scroll/change-title)"
    );
    for placed in &top_buttons {
        let want = expected
            .get(&placed.code)
            .unwrap_or_else(|| panic!("{} pairs with a .bnd rect", placed.id));
        assert_eq!(
            &placed.rect, want,
            "{} sits on its own State_Waiting.bnd rect",
            placed.id
        );
    }
    // Per-button no-overlap: no two top-band buttons share a pixel.
    for (index, a) in top_buttons.iter().enumerate() {
        for b in &top_buttons[index + 1..] {
            let overlap_x =
                a.rect.x < b.rect.x + b.rect.w as i32 && b.rect.x < a.rect.x + a.rect.w as i32;
            let overlap_y =
                a.rect.y < b.rect.y + b.rect.h as i32 && b.rect.y < a.rect.y + a.rect.h as i32;
            assert!(
                !(overlap_x && overlap_y),
                "top buttons {} and {} overlap",
                a.id,
                b.id
            );
        }
    }
}

#[test]
fn loading_layers_bar_over_real_loading_gate() {
    let path = archive_path(PLAYING_ARCHIVE);
    let Some(archive) = install_archives().playing else {
        println!("skip loading: missing {}", path.display());
        return;
    };
    let Some(art) = stage_state(&archive, State::Loading, StageContext::default()).art else {
        panic!("loading: MusicLoading.ojs staged no figures");
    };
    let (placements, figures) = (art.placements, art.figures);
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
    let Some(archive) = install_archives().playing else {
        println!("skip playing: missing {}", path.display());
        return;
    };
    // The Playing subset stages through the room mode of a fresh
    // game (7-key solo): the venue, the selected note background,
    // and the gauge family, including the corner life gauge.
    let probe = ScreenGame::new();
    let ctx = StageContext {
        arena_stage: Some(probe.arena_stage()),
        key_mode: probe.room_key_mode(),
        team_play: probe.room_team_play(),
        chart_slot: 0,
    };
    let Some(art) = stage_state(&archive, State::Playing, ctx).art else {
        panic!("playing: STATE_PLAYING staged no figures");
    };
    let (placements, figures) = (art.placements, art.figures);
    assert!(
        figures.contains_key(PLAYING_GAUGE_SPRITE),
        "playing: gauge sprite decoded nothing"
    );
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
    let Some(archive) = install_archives().interface else {
        println!("skip result: missing {}", path.display());
        return;
    };
    let Some(art) = stage_state(&archive, State::Result, StageContext::default()).art else {
        panic!("result: STATE_RESULT staged no figures");
    };
    let (placements, figures) = (art.placements, art.figures);
    assert!(
        figures.contains_key(RESULT_WINLOSE_SPRITE),
        "result: verdict banner stages through the declaration"
    );
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
