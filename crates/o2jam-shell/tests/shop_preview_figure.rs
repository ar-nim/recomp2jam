//! Shop entry mascot staging (#877).
//!
//! TDD record: before the fix the staging guard failed with "shop entry
//! must stage the pinned shop-master body" — `stage_state(State::Shop)`
//! staged 64 figures and none was `Shop_Master_main.ojs`; the
//! `0x14600053` placement had no sprite. After staging the
//! code-mounted body by name, the guard passes against installed art.
//! The pinned exe frame shows the same mascot at its own anchor.

use std::collections::HashMap;

use o2jam_file::data_root::{install_root, pack_skip_report, root_has_game_data};
use o2jam_file::{AvatarItemRow, AvatarItemTable};
use o2jam_render::{Frame, Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
use o2jam_shell::screens::side::{paint_side_scene_selected, SideScreen};
use o2jam_shell::staging::{
    read_install_archives, stage_dialog, stage_state, Dialog, StageContext, StagedArt, State,
};
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

const SHOP_MASTER_CODE: u32 = 0x1460_0053;
const SHOP_MASTER_SPRITE: &str = "Shop_Master_main.ojs";
const SHOP_BUYITEM_CODE: u32 = 0x1420_0070;

#[test]
fn shop_entry_stages_the_exe_shop_master_mascot_in_the_bottom_right_pane() {
    // Live chain: main.rs::run_play -> live_art::stage_live_scenes
    // -> live_art::stage_interface_scenes -> stage_state(State::Shop)
    // -> stage_shop_master_mascot -> ScreenGame::load_scene(Screen::Shop, ...)
    // -> ScreenGame::visit_side -> ScreenGame::mount_staged_side_art.
    let Some(art) = stage_shop_art("shop_entry_stages_the_exe_shop_master_mascot") else {
        return;
    };

    assert!(
        art.figures.contains_key(SHOP_MASTER_SPRITE),
        "shop entry must stage the pinned shop-master body"
    );
    let placed = art
        .placements
        .iter()
        .find(|placed| placed.code == SHOP_MASTER_CODE)
        .expect("shop-master control stages from State_ItemShop.bnd");
    assert_eq!(placed.sprite.as_deref(), Some(SHOP_MASTER_SPRITE));
    assert_eq!(
        (placed.rect.x, placed.rect.y, placed.rect.w, placed.rect.h),
        (600, 445, 198, 121),
        "the mascot must land in the bottom-right pane rect"
    );
}

#[test]
fn shop_master_mascot_paints_at_the_exe_anchor_on_the_live_stage() {
    // Live chain: main.rs::run_play -> live_art::stage_live_scenes
    // -> live_art::stage_interface_scenes -> stage_state(State::Shop)
    // -> ScreenGame::load_scene(Screen::Shop, ...)
    // -> ScreenGame::visit_side -> ScreenGame::mount_staged_side_art
    // -> ScreenGame::paint_side -> paint_side_scene_selected.
    let Some(art) = stage_shop_art("shop_master_mascot_paints_at_the_exe_anchor") else {
        return;
    };

    let without = art_without_shop_master(&art);
    let painted = paint_stage(&art);
    let bare = paint_stage(&without);
    let width = usize::try_from(NATIVE_WIDTH).expect("native width");
    let changed = painted
        .iter()
        .zip(bare.iter())
        .enumerate()
        .filter(|(_, (ink, plain))| ink != plain)
        .map(|(index, _)| (index % width, index / width))
        .collect::<Vec<_>>();

    assert!(!changed.is_empty(), "staged shop mascot paints no pixels");
    assert!(
        changed
            .iter()
            .all(|(x, y)| (654..786).contains(x) && (446..561).contains(y)),
        "mascot ink must land at the pinned sprite anchor, not the pane origin"
    );
}

#[test]
fn shop_master_mascot_hides_while_the_shortfall_message_sheet_is_up() {
    // Live chain: main.rs::run_play -> live_art::stage_live_scenes
    // -> live_art::stage_interface_scenes -> stage_state(State::Shop)
    // -> ScreenGame::load_scene(Screen::Shop, ...)
    // -> ScreenGame::visit_side -> ScreenGame::click_shop(BUYITEM)
    // -> ScreenGame::paint_side -> paint_side_scene_selected keep gate.
    // Red proof before the gate: the mascot pixels stayed unchanged
    // across the BUYITEM shortfall sheet; the v8 exe cart frame hides
    // them while the MESSAGE sheet is up (#877).
    let Some(art) = stage_shop_art("shop_master_mascot_hides_while_message_sheet_is_up") else {
        return;
    };
    let mascot = art
        .figures
        .get(SHOP_MASTER_SPRITE)
        .cloned()
        .expect("staged mascot body");
    let placed = art
        .placements
        .iter()
        .find(|placed| placed.code == SHOP_MASTER_CODE)
        .cloned()
        .expect("staged mascot control");
    let buy_item = ScenePlacement {
        role: PlacementRole::Single,
        id: "IDC_BUTTON_SSHOP_BUYITEM".to_owned(),
        code: SHOP_BUYITEM_CODE,
        sprite: None,
        rect: Rect {
            x: 20,
            y: 560,
            w: 70,
            h: 30,
        },
    };
    let interface = read_install_archives(&install_root())
        .interface
        .expect("interface archive present for the staged MESSAGE sheet");
    let question1 = stage_dialog(&interface, Dialog::PlanetQuit, StageContext::default())
        .art
        .expect("Dialog_Question1 stages with installed data");
    let mut game = ScreenGame::new();
    game.load_side_scene(
        vec![placed, buy_item],
        HashMap::from([(SHOP_MASTER_SPRITE.to_owned(), mascot)]),
    );
    game.load_planet_quit_scene(question1.placements, question1.figures);
    game.menu_key(MenuKey::Back, true);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Room);
    assert!(game.visit_side(SideScreen::Shop));
    game.set_shop_catalog(&AvatarItemTable {
        rows: vec![AvatarItemRow {
            item_id: 1,
            item_type: 0,
            price: 4500,
            params: [0; 4],
            flags: (0, 0),
            name: "W8Good".to_owned(),
            description: String::new(),
            sprites: Vec::new(),
        }],
    });
    game.shop_cart_push(1);

    let before = painted_mascot_box(&mut game);
    game.click_shop(41, 574);
    assert!(
        game.shop_charge_confirm_open(),
        "BUYITEM must post the shortfall MESSAGE sheet"
    );
    let after = painted_mascot_box(&mut game);
    let changed = before
        .iter()
        .zip(after.iter())
        .filter(|(entry, sheet)| entry != sheet)
        .count();
    assert!(
        changed > 1000,
        "the exe hides the shop master while its MESSAGE sheet is up, changed={changed}"
    );
}

/// One paint of the pinned mascot box on the native canvas.
fn painted_mascot_box(game: &mut ScreenGame) -> Vec<u32> {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width");
    let height = usize::try_from(NATIVE_HEIGHT).expect("native height");
    let mut buf = vec![0u32; width * height];
    game.paint_at(
        &mut Frame {
            buf: &mut buf,
            w: width,
            h: height,
        },
        0,
    );
    let pixels = &buf;
    (446..561)
        .flat_map(move |y| (654..786).map(move |x| pixels[y * width + x]))
        .collect()
}

/// Stage the live shop board, printing the named SKIP line when the
/// data pack is absent (CI has no install-root).
fn stage_shop_art(test_name: &str) -> Option<StagedArt> {
    let root = install_root();
    if !root_has_game_data(&root) {
        println!("{}", pack_skip_report(&root, test_name));
        return None;
    }
    let Some(interface) = read_install_archives(&root).interface else {
        println!("{}", pack_skip_report(&root, test_name));
        return None;
    };
    Some(
        stage_state(&interface, State::Shop, StageContext::default())
            .art
            .expect("shop art stages from installed data"),
    )
}

/// Same staged board with the mascot body removed, for a paint diff.
fn art_without_shop_master(art: &StagedArt) -> StagedArt {
    let mut bare = art.clone();
    bare.figures.remove(SHOP_MASTER_SPRITE);
    for placed in &mut bare.placements {
        if placed.code == SHOP_MASTER_CODE {
            placed.sprite = None;
        }
    }
    bare
}

/// One paint of the bound stage on a native-size frame.
fn paint_stage(art: &StagedArt) -> Vec<u32> {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width");
    let height = usize::try_from(NATIVE_HEIGHT).expect("native height");
    let mut buf = vec![0u32; width * height];
    paint_side_scene_selected(
        &mut Frame {
            buf: &mut buf,
            w: width,
            h: height,
        },
        &art.placements,
        &art.figures,
        |_| true,
        |_| None,
    );
    buf
}
