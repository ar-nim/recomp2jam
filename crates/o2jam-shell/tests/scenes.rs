//! Scene-by-scene parity harness: every screen on the main flow,
//! checked for composition (what it paints) and for the transitions,
//! input, and timing edges the 7.55 client performs.
//!
//! Composition pins art-backed layering per scene: a staged figure
//! behind the live UI (figure ink present, UI ink on top, never one
//! flat fill), plus the flat fallback that shows with no art staged
//! (see `docs/parity.md`: art cells stay FAIL until real `.ojs`/`.bnd`
//! bytes load, which cannot ship in-repo). Transitions pin the
//! documented journey (`docs/lifecycle.md`): splash → channel → room
//! → waiting → loading → playing → result, with retry/leave-back
//! edges. Only the public API is used, the way the window uses it.

use std::collections::HashMap;

use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
use o2jam_render::{
    has_ink_near, pack_xrgb, Frame, Rect, Rgb555Image, NATIVE_HEIGHT, NATIVE_WIDTH,
};
use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
use o2jam_shell::{
    can_enter, paint_screen, AvatarFigure, FigureLayer, GameLoop, MenuKey, Screen, ScreenFlow,
    ScreenGame,
};

/// Corner ink of the splash placeholder (logo card on dark ground).
fn splash_background() -> u32 {
    pack_xrgb(0x08, 0x08, 0x10)
}

/// Corner ink of the channel list placeholder.
fn channel_background() -> u32 {
    pack_xrgb(0x10, 0x20, 0x50)
}

/// Corner ink of the room list placeholder.
fn room_background() -> u32 {
    pack_xrgb(0x10, 0x50, 0x20)
}

/// Corner ink of the waiting-room placeholder.
fn waiting_background() -> u32 {
    pack_xrgb(0x50, 0x40, 0x10)
}

/// Corner ink of the loading gate placeholder.
fn loading_background() -> u32 {
    pack_xrgb(0x18, 0x18, 0x18)
}

/// Corner ink of the playing highway placeholder.
fn playing_background() -> u32 {
    pack_xrgb(0x0A, 0x0A, 0x0A)
}

/// Corner ink of the results placeholder.
fn result_background() -> u32 {
    pack_xrgb(0x10, 0x10, 0x08)
}

/// Paint one screen headlessly into a fresh buffer.
fn painted_screen(screen: Screen, w: usize, h: usize) -> Vec<u32> {
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

/// Paint a driven game headlessly into a fresh buffer.
fn painted_game(game: &mut ScreenGame, w: usize, h: usize, now_ms: u64) -> Vec<u32> {
    let mut buf = vec![0u32; w * h];
    game.paint_at(
        &mut Frame {
            buf: &mut buf,
            w,
            h,
        },
        now_ms,
    );
    buf
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
    game.with_audio(
        Box::new(o2jam_audio::NullBackend::new()),
        std::collections::HashMap::new(),
    );
    game.set_loading_status(o2jam_shell::screens::paint::LoadingStatus {
        progress: 1.0,
        title: "First".to_owned(),
        artist: "Band".to_owned(),
    });
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Playing);
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

/// One room row; the lock flag switches the `!` marker.
fn room_row(locked: bool) -> RoomListEntry {
    RoomListEntry {
        number: 3,
        state_code: 1,
        title: "Jam".to_owned(),
        has_password: locked,
        music_id: 0,
        difficulty: 0,
        key_mode: 2,
        game_mode: 0,
        speed: 1,
        capacity: 8,
        user_count: 5,
        min_level: 0,
        max_level: 0,
        skills: Vec::new(),
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

#[test]
fn splash_paints_logo_card_over_background() {
    let buf = painted_screen(Screen::Avi, 32, 24);
    assert_eq!(buf[0], splash_background());
    let card = pack_xrgb(0x20, 0x60, 0xC0);
    assert!(buf.contains(&card));
    assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
}

#[test]
fn channel_lists_entries_with_highlight_cursor() {
    let mut game = ScreenGame::new();
    game.set_channels(two_channels());
    game.lane_key(3, true);
    game.click_planet(136, 88);
    assert_eq!(game.screen(), Screen::Channel);
    let buf = painted_game(&mut game, 64, 48, 0);
    assert_eq!(buf[0], channel_background());
    assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
    assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
}

#[test]
fn room_marks_locked_room_row() {
    // The row line is 96 px wide here: the proportional TTF run is
    // wider than the old bitmap run, so the `!` mark must stay on
    // frame for the open/locked rows to differ.
    let mut open_game = ScreenGame::new();
    open_game.set_rooms(vec![room_row(false)]);
    open_game.lane_key(3, true);
    open_game.click_planet(136, 88);
    open_game.menu_key(MenuKey::Confirm, true);
    let mut locked_game = ScreenGame::new();
    locked_game.set_rooms(vec![room_row(true)]);
    locked_game.lane_key(3, true);
    locked_game.click_planet(136, 88);
    locked_game.menu_key(MenuKey::Confirm, true);
    let open = painted_game(&mut open_game, 96, 48, 0);
    let locked = painted_game(&mut locked_game, 96, 48, 0);
    assert_eq!(open[0], room_background());
    assert!(open.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    // Locked rows suffix `!` on the room line; open rows do not —
    // the two frames differ in text ink.
    assert_ne!(open, locked);
}

#[test]
fn waiting_shows_ready_dot_for_occupied_seat() {
    let mut game = ScreenGame::new();
    game.set_joined_room(seated_room());
    game.lane_key(3, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Waiting);
    let buf = painted_game(&mut game, 80, 64, 0);
    assert_eq!(buf[0], waiting_background());
    assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
}

#[test]
fn loading_shows_progress_bar() {
    let mut game = ScreenGame::new();
    walk_to_loading(&mut game);
    let buf = painted_game(&mut game, 64, 48, 0);
    assert_eq!(buf[0], loading_background());
    assert!(buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
}

#[test]
fn playing_places_struck_note_on_highway() {
    let mut game = ScreenGame::new();
    game.load_chart(vec![(2, 1500, 7)]);
    walk_to_playing(&mut game);
    let buf = painted_game(&mut game, 70, 60, 1000);
    assert_eq!(buf[0], playing_background());
    assert!(buf.contains(&pack_xrgb(0x40, 0xC0, 0xFF)));
}

#[test]
fn finished_song_lands_on_result_without_invented_score_digits() {
    let mut game = ScreenGame::new();
    game.load_chart(vec![(2, 5000, 7)]);
    walk_to_playing(&mut game);
    game.strike_at(2, 5000);
    let buf = painted_game(&mut game, 70, 60, 7000);
    assert_eq!(game.screen(), Screen::Result);
    assert_eq!(buf[0], result_background());
    // The invented (16,16) score anchor stays UNPAINTED (anchors
    // pending REA, docs/cresult-anchors.md).
    assert_ne!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
    assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
}

#[test]
fn menu_walks_full_session_to_results() {
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
}

#[test]
fn result_retries_to_waiting_and_leaves_to_room() {
    let mut retry = ScreenGame::new();
    retry.load_chart(vec![(2, 5000, 7)]);
    walk_to_playing(&mut retry);
    retry.strike_at(2, 5000);
    let mut buf = vec![0u32; 70 * 60];
    retry.paint_at(
        &mut Frame {
            buf: &mut buf,
            w: 70,
            h: 60,
        },
        7000,
    );
    retry.menu_key(MenuKey::Confirm, true);
    assert_eq!(retry.screen(), Screen::Waiting);

    let mut leave = ScreenGame::new();
    leave.load_chart(vec![(2, 5000, 7)]);
    walk_to_playing(&mut leave);
    leave.strike_at(2, 5000);
    let mut buf = vec![0u32; 70 * 60];
    leave.paint_at(
        &mut Frame {
            buf: &mut buf,
            w: 70,
            h: 60,
        },
        7000,
    );
    leave.menu_key(MenuKey::Back, true);
    assert_eq!(leave.screen(), Screen::Room);
}

#[test]
fn waiting_backs_out_to_room_list() {
    let mut game = ScreenGame::new();
    game.lane_key(3, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Waiting);
    game.menu_key(MenuKey::Back, true);
    assert_eq!(game.screen(), Screen::Room);
}

#[test]
fn loading_back_returns_to_waiting() {
    let mut game = ScreenGame::new();
    walk_to_loading(&mut game);
    game.menu_key(MenuKey::Back, true);
    assert_eq!(game.screen(), Screen::Waiting);
}

#[test]
fn splash_ignores_key_release() {
    let mut game = ScreenGame::new();
    game.lane_key(0, false);
    assert_eq!(game.screen(), Screen::Avi);
}

#[test]
fn unfinished_chart_stays_in_playing() {
    let mut game = ScreenGame::new();
    game.load_chart(vec![(2, 5000, 7)]);
    walk_to_playing(&mut game);
    let buf = painted_game(&mut game, 70, 60, 4995);
    assert_eq!(game.screen(), Screen::Playing);
    assert_eq!(buf[0], playing_background());
}

#[test]
fn flow_rejects_jumps_the_client_never_makes() {
    let illegal = [
        (Screen::Avi, Screen::Playing),
        (Screen::Avi, Screen::Room),
        (Screen::Avi, Screen::Channel),
        (Screen::Avi, Screen::Lanking),
        (Screen::Channel, Screen::Playing),
        (Screen::Room, Screen::Loading),
        (Screen::Planet, Screen::Playing),
        (Screen::Planet, Screen::Waiting),
        (Screen::Lanking, Screen::Playing),
        (Screen::Lanking, Screen::Waiting),
        (Screen::Avi, Screen::Myroom),
        (Screen::Myroom, Screen::Playing),
        (Screen::Myroom, Screen::Waiting),
        (Screen::Avi, Screen::Shop),
        (Screen::Shop, Screen::Playing),
        (Screen::Shop, Screen::Waiting),
        (Screen::Avi, Screen::MusicShop),
        (Screen::MusicShop, Screen::Playing),
        (Screen::MusicShop, Screen::Waiting),
        (Screen::Loading, Screen::Result),
        (Screen::Playing, Screen::Waiting),
        (Screen::Result, Screen::Loading),
        (Screen::Result, Screen::Playing),
    ];
    for (from, to) in illegal {
        assert!(!can_enter(from, to), "{from:?} -> {to:?}");
    }
    let mut flow = ScreenFlow::new();
    assert!(flow.go(Screen::Playing).is_err());
    assert_eq!(flow.current(), Screen::Avi);
}

#[test]
fn flow_allows_every_documented_edge() {
    let journey = [
        (Screen::Avi, Screen::Planet),
        (Screen::Planet, Screen::Channel),
        (Screen::Channel, Screen::Room),
        (Screen::Channel, Screen::Planet),
        (Screen::Room, Screen::Waiting),
        (Screen::Room, Screen::Channel),
        (Screen::Waiting, Screen::Loading),
        (Screen::Waiting, Screen::Room),
        (Screen::Loading, Screen::Playing),
        (Screen::Loading, Screen::Waiting),
        (Screen::Playing, Screen::Result),
        (Screen::Result, Screen::Waiting),
        (Screen::Result, Screen::Room),
    ];
    assert_eq!(journey.len(), 13);
    // (Planet→Channel once: boot board → dialog; Avi→Channel never:
    // launcher preselection is not a boot order.)
    for (from, to) in journey {
        assert!(can_enter(from, to), "{from:?} -> {to:?}");
    }
}

/// Staged-figure ink: one red pixel proving archive art sits behind
/// the live UI (a synthetic stand-in; real `.ojs` bytes stay out of
/// the repo).
fn staged_ink() -> u32 {
    pack_xrgb(255, 0, 0)
}

/// One decoded red pixel; colorkey white keeps the pixel opaque.
fn staged_figure() -> AvatarFigure {
    let mut figure = AvatarFigure::default();
    figure.push_layer(FigureLayer {
        image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
        dx: 0,
        dy: 0,
        colorkey: 0xFFFF,
    });
    figure
}

/// Stage one full-canvas figure for a screen (mirrors the unbound
/// blocks: splash, channel, and loading cover the native canvas, so
/// the figure lands on the frame origin at any size).
fn stage_screen_art(game: &mut ScreenGame, screen: Screen, name: &str) {
    let placements = vec![ScenePlacement {
        role: PlacementRole::Single,
        id: "IDC_HARNESS".to_owned(),
        code: 0,
        sprite: Some(name.to_owned()),
        rect: Rect {
            x: 0,
            y: 0,
            w: NATIVE_WIDTH,
            h: NATIVE_HEIGHT,
        },
    }];
    game.load_scene(
        screen,
        placements,
        HashMap::from([(name.to_owned(), staged_figure())]),
    );
}

/// Art-backed composition: the staged figure shows through, every
/// live UI ink paints on top, and the frame is never one flat fill.
fn assert_composed(buf: &[u32], ui_inks: &[u32]) {
    assert!(buf.contains(&staged_ink()));
    for ink in ui_inks {
        // Text inks antialias through the TTF: probe the near-ink
        // band instead of exact pixels.
        assert!(has_ink_near(buf, *ink));
    }
    assert!(buf.iter().any(|pixel| *pixel != buf[0]));
}

#[test]
fn splash_paints_logo_through_scene_art_only() {
    // One staging path serves the splash: the bound `STATE_AVI`
    // record stages the logo through `load_scene`; no second splash
    // slot paints on top.
    let mut game = ScreenGame::new();
    stage_screen_art(&mut game, Screen::Avi, "O2mediaLogo.ojs");
    let buf = painted_game(&mut game, 32, 24, 0);
    assert_composed(&buf, &[]);
}

#[test]
fn channel_layers_list_over_staged_art() {
    let mut game = ScreenGame::new();
    game.set_channels(two_channels());
    stage_screen_art(&mut game, Screen::Channel, "Dialog_ChannelSelectBar.ojs");
    game.lane_key(3, true);
    game.click_planet(136, 88);
    assert_eq!(game.screen(), Screen::Channel);
    let buf = painted_game(&mut game, 64, 48, 0);
    // The staged select bar is the highlight: it paints at the
    // highlighted row's rect (scaled (216,row_top) ≈ (17,17)), the
    // live list inks on top, and the amber cursor stand-in is gone.
    assert_eq!(buf[17 * 64 + 17], staged_ink());
    assert_composed(&buf, &[pack_xrgb(0xFF, 0xFF, 0xFF)]);
    assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
}

#[test]
fn room_layers_rows_over_staged_art() {
    let mut game = ScreenGame::new();
    game.set_rooms(vec![room_row(false)]);
    stage_screen_art(&mut game, Screen::Room, "Room_List.ojs");
    game.lane_key(3, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Room);
    let buf = painted_game(&mut game, 64, 48, 0);
    assert_composed(
        &buf,
        &[pack_xrgb(0xFF, 0xFF, 0xFF), pack_xrgb(0xFF, 0xD0, 0x20)],
    );
}

#[test]
fn waiting_layers_seats_over_staged_art() {
    let mut game = ScreenGame::new();
    game.set_joined_room(seated_room());
    stage_screen_art(&mut game, Screen::Waiting, "Wait_TeamA.ojs");
    game.lane_key(3, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Waiting);
    let buf = painted_game(&mut game, 80, 64, 0);
    assert_composed(
        &buf,
        &[pack_xrgb(0xFF, 0xFF, 0xFF), pack_xrgb(0x30, 0xE0, 0x30)],
    );
}

#[test]
fn loading_layers_bar_over_staged_art() {
    let mut game = ScreenGame::new();
    stage_screen_art(&mut game, Screen::Loading, "MusicLoading.ojs");
    walk_to_loading(&mut game);
    let buf = painted_game(&mut game, 64, 48, 0);
    assert_composed(&buf, &[pack_xrgb(0x30, 0xB0, 0x30)]);
}

#[test]
fn playing_layers_notes_over_staged_gauges() {
    let mut game = ScreenGame::new();
    game.load_chart(vec![(2, 1500, 7)]);
    stage_screen_art(&mut game, Screen::Playing, "Playing_Panel.ojs");
    walk_to_playing(&mut game);
    let buf = painted_game(&mut game, 70, 60, 1000);
    assert_composed(&buf, &[pack_xrgb(0x40, 0xC0, 0xFF)]);
}

#[test]
fn result_layers_staged_art_without_invented_digits() {
    let mut game = ScreenGame::new();
    game.load_chart(vec![(2, 5000, 7)]);
    stage_screen_art(&mut game, Screen::Result, "Result_WinLose.ojs");
    walk_to_playing(&mut game);
    game.strike_at(2, 5000);
    let buf = painted_game(&mut game, 70, 60, 7000);
    assert_eq!(game.screen(), Screen::Result);
    assert_composed(&buf, &[]);
    // The invented (16,16) score anchor stays UNPAINTED over the
    // staged art (anchors pending REA, docs/cresult-anchors.md).
    assert_ne!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
    assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
}
