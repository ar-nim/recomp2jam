//! Exe-pinned guards for the fixed single-line TEXT cell.
//!
//! The 7.55 cell is a fixed 6x12 GDI cell (`docs/gdi-text.md`). A
//! single-line rect places and clips the cell; it never shrinks the
//! glyphs to fit. Every exe number below is a background-difference
//! scan of `~/scratch/paired-shots-v3/states/` at threshold 40, not a
//! colour match and not a value read back from our painter.

use o2jam_file::data_root::install_root;
use o2jam_net::{CharacterInfo, Gender};
use o2jam_render::{Frame, Rect, NATIVE_HEIGHT, NATIVE_WIDTH};

use super::room::room_text_scale;
use super::text_ink::{ink_block, InkBlock};
use super::{
    cp949_cell_scale, paint_music_shop_search, text_frame_scale, CP949_TEXT_CELL_HEIGHT_PX,
};
use crate::avatar::{PlacementRole, ScenePlacement};
use crate::screens::game::ScreenGame;
use crate::screens::side::MSHOP_SEARCH_EDIT_CODE;
use crate::staging::{
    read_install_archives, stage_dialog, stage_state, Dialog, StageContext, State,
};
use crate::{GameLoop, MenuKey, Screen};

/// Room window from the brief. Exe corner is (33,73,132); ink rows
/// 288..293.
const ROOM_VALUE_WINDOW: [i32; 4] = [684, 285, 702, 298];
/// Third notice row, narrowed to its own black background so nearby
/// chrome stays out. Exe ink bbox (297,297)-(370,305).
const NOTICE_ROW_THREE_WINDOW: [i32; 4] = [296, 297, 371, 306];
/// Synthetic search-edit cell; the exe hint is measured in absolute
/// canvas coordinates, so this seam pins height only.
const SEARCH_HINT_WINDOW: [i32; 4] = [500, 20, 580, 32];

/// The size-bearing part of one ink block: height, width, first row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InkSignature {
    h: i32,
    w: i32,
    first_row: i32,
}

impl From<InkBlock> for InkSignature {
    fn from(block: InkBlock) -> Self {
        Self {
            h: block.h,
            w: block.w,
            first_row: block.first_row(),
        }
    }
}

/// Exe room value: `room/exe.png`, window above.
const EXE_ROOM_VALUE: InkSignature = InkSignature {
    h: 6,
    w: 4,
    first_row: 288,
};
/// Our room value at the fixed cell with no top bearing. This is the
/// measured side of the OPEN position and width gap, not an exe claim.
const OURS_ROOM_VALUE: InkSignature = InkSignature {
    h: 6,
    w: 5,
    first_row: 285,
};
/// Exe notice row 3: `channel/exe.png`, window above.
const EXE_NOTICE_ROW_THREE: InkSignature = InkSignature {
    h: 9,
    w: 74,
    first_row: 297,
};
/// Our notice row 3: multiline width-fit stays in force (#882).
const OURS_NOTICE_ROW_THREE: InkSignature = InkSignature {
    h: 9,
    w: 75,
    first_row: 297,
};
/// Exe search hint: `musicshop/exe.png`, window
/// (44,540)-(63,550), corner (156,186,222), bbox (44,541)-(62,548).
const EXE_SEARCH_HINT: InkSignature = InkSignature {
    h: 8,
    w: 19,
    first_row: 541,
};
/// Our hint in the synthetic rect at y20: the glyph core inks at the
/// rect top. Measured, not an exe match.
const OURS_SEARCH_HINT: InkSignature = InkSignature {
    h: 7,
    w: 39,
    first_row: 20,
};

fn native_frame(game: &mut ScreenGame) -> Vec<u32> {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
    let mut pixels = vec![0_u32; width * height];
    game.paint_at(
        &mut Frame {
            buf: &mut pixels,
            w: width,
            h: height,
        },
        0,
    );
    pixels
}

/// Room board reached through the offline login walk, with the
/// tester whose value the exe crop shows.
fn staged_room_game() -> Option<ScreenGame> {
    let root = install_root();
    let installed = read_install_archives(&root);
    let archive = installed.interface.as_deref()?;
    let art = stage_state(archive, State::Room, StageContext::default()).art?;
    let mut game = ScreenGame::new();
    game.set_myroom_character(CharacterInfo {
        nickname: "tester".to_owned(),
        gender: Gender::Male,
        gem: 15_533,
        level: 7,
        ..CharacterInfo::default()
    });
    game.load_scene(Screen::Room, art.placements, art.figures);
    game.menu_key(MenuKey::Back, true);
    game.click_planet(386, 88);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Room, "confirm opens the Room board");
    Some(game)
}

/// Relay-down notice staged through the Channel board, with the
/// information sheet reached from the live emblem click.
fn staged_notice_game() -> Option<ScreenGame> {
    let root = install_root();
    let installed = read_install_archives(&root);
    let archive = installed.interface.as_deref()?;
    let context = StageContext::default();
    let board = stage_state(archive, State::Planet, context).art?;
    let dialog_figures = stage_dialog(archive, Dialog::ChannelInfo, context)
        .art?
        .figures;
    let notice = stage_dialog(archive, Dialog::Information, context).art?;
    let mut game = ScreenGame::new();
    game.load_scene(Screen::Planet, board.placements, board.figures);
    game.load_scene(Screen::Channel, Vec::new(), dialog_figures);
    game.menu_key(MenuKey::Back, true);
    game.click_planet(386, 88);
    assert_eq!(game.screen(), Screen::Channel, "emblem opens Channel");
    game.load_relay_down_scene(notice.placements, notice.figures);
    game.hold_channel_rows_for_relay_notice(Vec::new());
    game.open_relay_down_dialog();
    Some(game)
}

#[test]
fn one_single_line_cell_follows_the_frame_and_not_the_rect() {
    // Live caller chain: main.rs::run_play -> ScreenGame::paint_at ->
    // ScreenGame::paint_shop -> paint_shop_featured_captions ->
    // paint_text_left_in_cell -> cp949_cell_scale.
    // Seam: pure scale conversion; the pixel guards below observe the
    // same conversion through archive-backed live callers.
    assert_eq!(cp949_cell_scale(1.0), CP949_TEXT_CELL_HEIGHT_PX);
    assert_eq!(cp949_cell_scale(0.5), CP949_TEXT_CELL_HEIGHT_PX * 0.5);
    assert_eq!(room_text_scale(1.0), CP949_TEXT_CELL_HEIGHT_PX);
    assert_eq!(room_text_scale(0.5), CP949_TEXT_CELL_HEIGHT_PX * 0.5);
    assert_eq!(
        text_frame_scale(&Frame {
            buf: &mut [],
            w: 400,
            h: 300,
        }),
        0.5
    );
}

#[test]
fn room_value_height_matches_the_exe_six_pixel_cell_and_position_and_width_are_open() {
    // Live caller chain, read from the boot path:
    // main.rs::run_play -> Shell::run -> WindowHandler::paint ->
    // ScreenGame::paint -> ScreenGame::paint_at ->
    // ScreenGame::paint_room -> paint_room_user_panel ->
    // paint_room_panel_number -> paint_room_panel_value ->
    // paint_text_into_rect -> cp949_cell_scale.
    // Archives absent -> skip cleanly (CI has no game data).
    let Some(mut game) = staged_room_game() else {
        println!("SKIP room_value_height_matches_the_exe_six_pixel_cell_and_position_and_width_are_open: install-root absent");
        return;
    };
    let pixels = native_frame(&mut game);
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    let ours: InkSignature = ink_block(&pixels, width, ROOM_VALUE_WINDOW).into();

    assert_eq!(
        EXE_ROOM_VALUE,
        InkSignature {
            h: 6,
            w: 4,
            first_row: 288,
        },
        "exe pin from room/exe.png; re-measure before editing"
    );
    assert_eq!(
        ours, OURS_ROOM_VALUE,
        "OPEN: exe h6 w4 first row 288; ours h{} w{} first row {}",
        ours.h, ours.w, ours.first_row
    );
    assert_eq!(
        ours.h, EXE_ROOM_VALUE.h,
        "room value height must match the exe"
    );
    assert_ne!(
        ours.first_row, EXE_ROOM_VALUE.first_row,
        "OPEN: first ink row must stay open until the room-value bearing is measured"
    );
    assert_ne!(
        ours.w, EXE_ROOM_VALUE.w,
        "OPEN: ink width is size; the wider digit must stay open"
    );
}

#[test]
fn channel_notice_row_three_height_and_row_match_the_exe_width_is_open() {
    // Live caller chain: main.rs::run_play -> ScreenGame::paint_at ->
    // ScreenGame::paint_channel -> paint_relay_down_dialog ->
    // paint_information_message -> paint_text_into_rect ->
    // cp949_multiline_scale (the #882 width-fit leg, unchanged here).
    // Archives absent -> skip cleanly (CI has no game data).
    let Some(mut game) = staged_notice_game() else {
        println!("SKIP channel_notice_row_three_height_and_row_match_the_exe_width_is_open: install-root absent");
        return;
    };
    let pixels = native_frame(&mut game);
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    let ours: InkSignature = ink_block(&pixels, width, NOTICE_ROW_THREE_WINDOW).into();

    assert_eq!(
        EXE_NOTICE_ROW_THREE,
        InkSignature {
            h: 9,
            w: 74,
            first_row: 297,
        },
        "exe pin from channel/exe.png; re-measure before editing"
    );
    assert_eq!(
        ours, OURS_NOTICE_ROW_THREE,
        "OPEN: exe h9 w74 first row 297; ours h{} w{} first row {}",
        ours.h, ours.w, ours.first_row
    );
    assert_eq!(
        ours.h, EXE_NOTICE_ROW_THREE.h,
        "notice height must match the exe"
    );
    assert_eq!(
        ours.first_row, EXE_NOTICE_ROW_THREE.first_row,
        "notice first ink row must match the exe"
    );
    assert_ne!(
        ours.w, EXE_NOTICE_ROW_THREE.w,
        "OPEN: ink width is size; the wider Hangul advance must stay open"
    );
}

#[test]
fn music_shop_hint_height_and_position_are_open_against_the_exe() {
    // Live caller chain: main.rs::run_play -> GameLoop::ingest_snapshot
    // -> ScreenGame::paint_side -> paint_music_shop_search ->
    // paint_text_into_rect -> cp949_cell_scale.
    // Seam: a synthetic 80x20 edit cell exercises that painter and its
    // fixed cell. It cannot compare position or width to the exe: the
    // exe edit rect is not staged here. Next step: stage the exe edit
    // rect, then pin first row and advance from an exe-pixel scan.
    let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    let edit = ScenePlacement {
        role: PlacementRole::Single,
        id: String::new(),
        code: MSHOP_SEARCH_EDIT_CODE,
        sprite: None,
        rect: Rect {
            x: 500,
            y: 20,
            w: 80,
            h: 20,
        },
    };
    let mut pixels = vec![0_u32; width * height];
    paint_music_shop_search(
        &mut Frame {
            buf: &mut pixels,
            w: width,
            h: height,
        },
        &[edit],
        "",
        &[],
        &[],
    );
    let ours: InkSignature = ink_block(&pixels, width, SEARCH_HINT_WINDOW).into();

    assert_eq!(
        EXE_SEARCH_HINT,
        InkSignature {
            h: 8,
            w: 19,
            first_row: 541,
        },
        "exe pin from musicshop/exe.png; re-measure before editing"
    );
    assert_eq!(
        ours, OURS_SEARCH_HINT,
        "OPEN: exe h8 w19 first row 541; ours h{} w{} first row {}",
        ours.h, ours.w, ours.first_row
    );
    assert_ne!(
        ours.h, EXE_SEARCH_HINT.h,
        "OPEN: exe h8, ours h{}; next step is pinning the hint's font class",
        ours.h
    );
    assert_ne!(
        ours.first_row, EXE_SEARCH_HINT.first_row,
        "OPEN: next step is staging the exe edit rect before comparing rows"
    );
    assert_ne!(
        ours.w, EXE_SEARCH_HINT.w,
        "OPEN: next step is pinning the hint's advance from the exe edit rect"
    );
}
