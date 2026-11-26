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

/// Stage the archive-backed Shop face with one catalog strategy.
/// The returned rects come from the staged card children, not literals.
fn staged_shop_game(
    stage_catalog: impl FnOnce(&mut ScreenGame, &Arc<AvatarSource>),
) -> Option<(ScreenGame, Vec<Rect>)> {
    let root = o2jam_file::data_root::install_root();
    let archive = read_install_archives(&root).interface?;
    let source = read_installed_avatar(&root)?;
    let art = stage_state(&archive, State::Shop, StageContext::default()).art?;
    let title_rects = card_title_rects(&art.placements);
    let mut game = ScreenGame::new();
    stage_tester_identity(&mut game);
    stage_catalog(&mut game, &source);
    game.load_scene(Screen::Shop, art.placements, art.figures);
    walk_to_room(&mut game);
    assert!(
        game.visit_side(crate::screens::side::SideScreen::Shop),
        "Shop opens from the Room lobby"
    );
    Some((game, title_rects))
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

/// Staged `SET_TITLE` rects under the four featured card parents.
fn card_title_rects(placements: &[ScenePlacement]) -> Vec<Rect> {
    SHOP_SET_CARD_PARENT_IDS
        .iter()
        .filter_map(|parent| {
            placements
                .iter()
                .find(|placed| placed.code == CARD_TITLE_CODE && placed.id == *parent)
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
