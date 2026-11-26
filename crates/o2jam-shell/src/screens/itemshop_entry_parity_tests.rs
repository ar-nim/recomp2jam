//! Guards for the Item Shop entry frame in the paired-shot parity gate.

use std::sync::Arc;

use o2jam_net::{CharacterInfo, Gender};
use o2jam_render::{Frame, Rect, NATIVE_HEIGHT, NATIVE_WIDTH};

use crate::avatar::ScenePlacement;
use crate::avatar_source::{read_installed_avatar, AvatarSource};
use crate::screens::side::{SHOP_SET_CARD_CHILD_CODES, SHOP_SET_CARD_PARENT_IDS};
use crate::staging::{read_install_archives, stage_state, StageContext, State};
use crate::{GameLoop, MenuKey, Screen, ScreenGame};

/// The `SET_TITLE` child under each featured `SETITEM_EX` parent.
const CARD_TITLE_CODE: u32 = SHOP_SET_CARD_CHILD_CODES[1];
/// The experience-bonus `SET_TYPE3` child; its ink line sits at y=302.
const CARD_EXPERIENCE_CODE: u32 = SHOP_SET_CARD_CHILD_CODES[3];

/// Stage the archive-backed Shop face with one catalog strategy.
/// The returned rects come from the staged card children, not literals.
fn staged_shop_game(
    stage_catalog: impl FnOnce(&mut ScreenGame, &Arc<AvatarSource>),
) -> Option<(ScreenGame, Vec<Rect>)> {
    staged_shop_game_collecting(stage_catalog, CARD_TITLE_CODE)
}

/// Stage the same face while collecting one staged child code.
fn staged_shop_game_collecting(
    stage_catalog: impl FnOnce(&mut ScreenGame, &Arc<AvatarSource>),
    rect_code: u32,
) -> Option<(ScreenGame, Vec<Rect>)> {
    let root = o2jam_file::data_root::install_root();
    let archive = read_install_archives(&root).interface?;
    let source = read_installed_avatar(&root)?;
    let art = stage_state(&archive, State::Shop, StageContext::default()).art?;
    let rects = placed_rects(&art.placements, rect_code);
    let mut game = ScreenGame::new();
    stage_tester_identity(&mut game);
    stage_catalog(&mut game, &source);
    game.load_scene(Screen::Shop, art.placements, art.figures);
    walk_to_room(&mut game);
    assert!(
        game.visit_side(crate::screens::side::SideScreen::Shop),
        "Shop opens from the Room lobby"
    );
    Some((game, rects))
}

/// Stage the tester whose wallet and caption the entry frame shows.
fn stage_tester_identity(game: &mut ScreenGame) {
    game.set_myroom_character(CharacterInfo {
        nickname: "tester".to_owned(),
        gender: Gender::Male,
        gem: 15_533,
        level: 7,
        ..CharacterInfo::default()
    });
}

/// Walk the offline flow to the Room lobby that hosts side screens.
fn walk_to_room(game: &mut ScreenGame) {
    game.menu_key(MenuKey::Back, true);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(
        game.screen(),
        Screen::Room,
        "shop visit needs the Room lobby"
    );
}

/// Staged child rects under the four featured card parents.
fn placed_rects(placements: &[ScenePlacement], code: u32) -> Vec<Rect> {
    SHOP_SET_CARD_PARENT_IDS
        .iter()
        .filter_map(|parent| {
            placements
                .iter()
                .find(|placed| placed.code == code && placed.id == *parent)
                .map(|placed| placed.rect)
        })
        .collect()
}

/// Native-canvas pixels of one explicit-clock paint.
fn painted_pixels(game: &mut ScreenGame) -> Vec<u32> {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width");
    let height = usize::try_from(NATIVE_HEIGHT).expect("native height");
    let mut buffer = vec![0u32; width * height];
    game.paint_at(
        &mut Frame {
            buf: &mut buffer,
            w: width,
            h: height,
        },
        0,
    );
    buffer
}

/// Count pixels a staged catalog changed inside one native rect.
fn changed_pixels(before: &[u32], after: &[u32], rect: Rect) -> usize {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width");
    let height = usize::try_from(NATIVE_HEIGHT).expect("native height");
    let (left, top, right, bottom) = clipped_bounds(rect, width, height);
    (top..bottom)
        .flat_map(|row| (left..right).map(move |column| row * width + column))
        .filter(|index| before[*index] != after[*index])
        .count()
}

/// Exclusive frame bounds of one native rect.
fn clipped_bounds(rect: Rect, width: usize, height: usize) -> (usize, usize, usize, usize) {
    let left = rect.x.max(0) as usize;
    let top = rect.y.max(0) as usize;
    let right = (i64::from(rect.x) + i64::from(rect.w))
        .min(width as i64)
        .max(0) as usize;
    let bottom = (i64::from(rect.y) + i64::from(rect.h))
        .min(height as i64)
        .max(0) as usize;
    (left, top, right, bottom)
}

/// Stage the raw `Itemdata_Korea.dat` order (the pre-entry face).
fn stage_raw_catalog(game: &mut ScreenGame, source: &Arc<AvatarSource>) {
    game.set_shop_catalog(source.table());
}

/// Stage the pinned state-3 new-arrivals entry face.
fn stage_featured_catalog(game: &mut ScreenGame, source: &Arc<AvatarSource>) {
    game.stage_exe_item_shop_catalog(source.table());
}

#[test]
fn itemshop_entry_inks_featured_card_titles_at_the_staged_set_children() {
    // Live caller chain: main.rs::stage_online_identity ->
    // ScreenGame::stage_exe_item_shop_catalog -> ScreenGame::visit_side ->
    // ScreenGame::mount_staged_side_art -> ScreenGame::paint_at ->
    // ScreenGame::paint_shop -> paint_shop_board ->
    // paint_shop_featured_captions -> the staged SET_TITLE child rects.
    // Archives absent -> skip cleanly (CI has no game data).
    let Some((mut raw, title_rects)) = staged_shop_game(stage_raw_catalog) else {
        println!("SKIP itemshop_entry_inks_featured_card_titles_at_the_staged_set_children: install-root absent");
        return;
    };
    // Red proof: staging raw catalogs on both sides leaves the card
    // titles identical. The pinned new-arrivals face must change them.
    let Some((mut featured, _)) = staged_shop_game(stage_featured_catalog) else {
        println!("SKIP itemshop_entry_inks_featured_card_titles_at_the_staged_set_children: install-root absent");
        return;
    };
    assert_eq!(title_rects.len(), 4, "all four card titles stage");
    let raw_pixels = painted_pixels(&mut raw);
    let featured_pixels = painted_pixels(&mut featured);
    let ink: usize = title_rects
        .iter()
        .map(|rect| changed_pixels(&raw_pixels, &featured_pixels, *rect))
        .sum();
    assert!(
        ink > 100,
        "the featured entry must change its four staged card titles (changed {ink} pixels)"
    );
}

/// Ink bbox and median core colour of one text block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InkRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    colour: [i32; 3],
}

/// Exe values measured from the qualified entry crop (256-grey panel
/// background, core-ink threshold 48). The staged `SET_TYPE3` rect
/// pins x/y; the pitch comes from the four `.bnd` parent origins.
const EXE_PRICE_INK: InkRect = InkRect {
    x: 226,
    y: 302,
    w: 55,
    h: 8,
    colour: [132, 138, 132],
};
const CARD_INK_PITCH: i32 = 141;
/// Exe values for the blue `MY BAG` plate baked into `State_ItemShop.ojs`.
/// The plate has no `.bnd` control; its crop is pinned by the exe frame.
const EXE_MYBAG_PLATE_INK: InkRect = InkRect {
    x: 17,
    y: 416,
    w: 100,
    h: 59,
    colour: [33, 138, 239],
};

/// One XRGB8888 framebuffer word as independent RGB channels.
fn unpack_rgb(pixel: u32) -> [i32; 3] {
    [
        ((pixel >> 16) & 0xFF) as i32,
        ((pixel >> 8) & 0xFF) as i32,
        (pixel & 0xFF) as i32,
    ]
}

/// Median of one channel's sampled pixels.
fn median(values: &mut [i32]) -> i32 {
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2
    }
}

/// Median panel colour around a measurement window.
fn border_median(pixels: &[u32], rect: Rect) -> [i32; 3] {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width");
    let (left, top, right, bottom) = clipped_bounds(rect, width, pixels.len() / width);
    let mut channels = [Vec::new(), Vec::new(), Vec::new()];
    for column in left..right {
        for row in [top, bottom.saturating_sub(1)] {
            let rgb = unpack_rgb(pixels[row * width + column]);
            for (channel, value) in channels.iter_mut().zip(rgb) {
                channel.push(value);
            }
        }
    }
    for row in top..bottom {
        for column in [left, right - 1] {
            let rgb = unpack_rgb(pixels[row * width + column]);
            for (channel, value) in channels.iter_mut().zip(rgb) {
                channel.push(value);
            }
        }
    }
    [
        median(&mut channels[0]),
        median(&mut channels[1]),
        median(&mut channels[2]),
    ]
}

/// Ink bbox and median core colour inside one measurement window.
fn ink_rect(pixels: &[u32], search: Rect, is_ink: impl Fn([i32; 3]) -> bool) -> InkRect {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width");
    let (left, top, right, bottom) = clipped_bounds(search, width, pixels.len() / width);
    let (mut min_x, mut min_y) = (i32::MAX, i32::MAX);
    let (mut max_x, mut max_y) = (i32::MIN, i32::MIN);
    let mut channels = [Vec::new(), Vec::new(), Vec::new()];
    for row in top..bottom {
        for column in left..right {
            let rgb = unpack_rgb(pixels[row * width + column]);
            if !is_ink(rgb) {
                continue;
            }
            min_x = min_x.min(column as i32);
            min_y = min_y.min(row as i32);
            max_x = max_x.max(column as i32);
            max_y = max_y.max(row as i32);
            for (channel, value) in channels.iter_mut().zip(rgb) {
                channel.push(value);
            }
        }
    }
    assert!(min_x <= max_x, "the measured block must carry ink");
    InkRect {
        x: min_x,
        y: min_y,
        w: max_x - min_x + 1,
        h: max_y - min_y + 1,
        colour: [
            median(&mut channels[0]),
            median(&mut channels[1]),
            median(&mut channels[2]),
        ],
    }
}

/// Core ink of an attribute line against its staged dark panel cell.
fn price_line_ink(pixels: &[u32], cell: Rect) -> InkRect {
    let search = Rect {
        x: cell.x,
        y: cell.y + 2,
        w: 70,
        h: 8,
    };
    let background = border_median(pixels, search);
    ink_rect(pixels, search, |rgb| {
        rgb.iter()
            .zip(background)
            .map(|(value, panel)| (value - panel).abs())
            .max()
            .expect("three channels")
            > 48
    })
}

/// Core ink of the blue `MY BAG` plate inside its exe-measured window.
fn mybag_plate_ink(pixels: &[u32]) -> InkRect {
    ink_rect(
        pixels,
        Rect {
            x: 10,
            y: 410,
            w: 115,
            h: 70,
        },
        |rgb| rgb[0] < 60 && rgb[2] > 170,
    )
}

/// Owner tolerances: position 6, height 3, width 12, colour 40/channel.
fn assert_ink_matches(label: &str, actual: InkRect, expected: InkRect) {
    let offset = (actual.x - expected.x).abs() + (actual.y - expected.y).abs();
    let height = (actual.h - expected.h).abs();
    let width = (actual.w - expected.w).abs();
    let colour = (0..3)
        .map(|channel| (actual.colour[channel] - expected.colour[channel]).abs())
        .max()
        .expect("three channels");
    assert!(
        offset <= 6 && height <= 3 && width <= 12 && colour <= 40,
        "{label}: got {actual:?}, want {expected:?}; off={offset} dh={height} dw={width} dc={colour}"
    );
}

#[test]
fn itemshop_entry_price_tags_match_the_exe_ink_blocks() {
    // Live caller chain: main.rs::stage_online_identity ->
    // ScreenGame::stage_exe_item_shop_catalog -> ScreenGame::visit_side ->
    // ScreenGame::paint_at -> ScreenGame::paint_shop -> paint_shop_board ->
    // paint_shop_featured_captions -> staged SET_TYPE2 child rects.
    // Archives absent -> skip cleanly (CI has no game data).
    let Some((mut game, cells)) =
        staged_shop_game_collecting(stage_featured_catalog, CARD_EXPERIENCE_CODE)
    else {
        println!("SKIP itemshop_entry_price_tags_match_the_exe_ink_blocks: install-root absent");
        return;
    };
    assert_eq!(cells.len(), 4, "all four price lines stage");
    let pixels = painted_pixels(&mut game);
    for (slot, cell) in cells.iter().enumerate() {
        let slot = i32::try_from(slot).expect("four card slots");
        let expected = InkRect {
            x: EXE_PRICE_INK.x + CARD_INK_PITCH * slot,
            ..EXE_PRICE_INK
        };
        assert_ink_matches(
            &format!("price tag {slot}"),
            price_line_ink(&pixels, *cell),
            expected,
        );
    }
}

#[test]
fn itemshop_entry_mybag_plate_matches_the_exe_ink_block() {
    // Live caller chain: main.rs::stage_online_identity ->
    // ScreenGame::stage_exe_item_shop_catalog -> ScreenGame::visit_side ->
    // ScreenGame::mount_staged_side_art -> ScreenGame::paint_at ->
    // ScreenGame::paint_side -> the baked MY BAG plate in State_ItemShop.ojs.
    // Archives absent -> skip cleanly (CI has no game data).
    let Some((mut game, _)) = staged_shop_game(stage_featured_catalog) else {
        println!("SKIP itemshop_entry_mybag_plate_matches_the_exe_ink_block: install-root absent");
        return;
    };
    let pixels = painted_pixels(&mut game);
    assert_ink_matches(
        "MY BAG plate",
        mybag_plate_ink(&pixels),
        EXE_MYBAG_PLATE_INK,
    );
}
