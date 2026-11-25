//! Cart-residual chrome (#886): the 7.55 shop backdrop bakes its
//! bottom buttons and messenger lamp into `State_ItemShop.ojs`. The
//! matching `ControlList` bodies are hit targets, not a second paint.
//!
//! Oracle: v3 quiesced cart frame
//! `~/scratch/paired-shots-v3/states/itemshop-cart/exe.png` (7.55
//! 83a04541…); at (720,575) it has the gray baked BACK plate
//! `(197,194,197)`, and at (630,575) the purple baked lamp
//! `(197,154,255)`. The pre-fix stage paints `(0,132,231)` and
//! `(255,173,82)` there from stacked button sprites.

use o2jam_file::data_root::{install_root, pack_skip_report, root_has_game_data};
use o2jam_file::{AvatarItemRow, AvatarItemTable};
use o2jam_render::{Frame, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::screens::side::{ShopPanel, SideScreen};
use o2jam_shell::staging::{read_install_archives, stage_state, StageContext, StagedArt, State};
use o2jam_shell::{GameLoop, InputSnapshot, MenuKey, Screen, ScreenGame};

/// Live chain: main.rs::run_play -> live_art::stage_live_scenes
/// -> live_art::stage_interface_scenes -> stage_state(State::Shop)
/// -> ScreenGame::load_side_scene(Screen::Shop)
/// -> ScreenGame::visit_side -> ScreenGame::paint_side
/// -> paint_side_scene_selected. A data-free run skips at the seam.
#[test]
fn shop_back_and_messenger_lamp_stay_baked_into_the_backdrop() {
    let Some(art) = stage_shop_art("shop_back_and_messenger_lamp_stay_baked") else {
        return;
    };
    let mut game = room_game();
    game.load_side_scene(art.placements, art.figures);
    assert!(game.visit_side(SideScreen::Shop));
    let frame = paint_native(&mut game);

    assert_color_near(pixel(&frame, 720, 575), [197, 194, 197], "BACK plate");
    assert_color_near(pixel(&frame, 630, 575), [197, 154, 255], "messenger lamp");
    assert_color_near(pixel(&frame, 680, 575), [123, 178, 255], "? plate");
    assert_eq!(game.screen(), Screen::Room);
}

/// The chrome controls still stage their `.bnd` hit rects: our
/// click path must reach BACK at the baked button geometry.
#[test]
fn baked_back_button_keeps_its_staged_hit_rect() {
    let Some(art) = stage_shop_art("baked_back_button_keeps_its_staged_hit_rect") else {
        return;
    };
    let mut game = room_game();
    game.load_side_scene(art.placements, art.figures);
    assert!(game.visit_side(SideScreen::Shop));
    assert_eq!(
        game.staged_side_control_rect(0x1420_0002)
            .map(|rect| (rect.x, rect.y, rect.w, rect.h)),
        Some((711, 569, 83, 24))
    );
}

/// The cart-state gate must hide BAGSORT even when its body is
/// present. Restoring the body makes this guard fail if the gate
/// regresses; the staged rect stays for hit testing either way.
#[test]
fn bagsort_plate_hides_when_the_cart_has_a_good() {
    let Some(art) = stage_shop_art("bagsort_plate_hides_when_the_cart_has_a_good") else {
        return;
    };
    let mut game = room_game();
    let mut art = art;
    for placed in &mut art.placements {
        if placed.code == SHOP_BAGSORT_CODE {
            placed.sprite = Some(SHOP_BAGSORT_SPRITE.to_owned());
        }
    }
    assert!(
        art.figures.contains_key(SHOP_BAGSORT_SPRITE),
        "the installed archive must stage the sort body for this guard"
    );
    game.load_side_scene(art.placements, art.figures);
    assert!(game.visit_side(SideScreen::Shop));
    game.set_shop_catalog(&one_good_catalog());

    let entry = paint_native(&mut game);
    assert_eq!(game.shop_panel(), ShopPanel::MyBag);
    assert_ne!(
        pixel(&entry, 40, 558),
        [148, 198, 222],
        "the entry face shows a sort plate, not the cart face"
    );

    game.ingest_snapshot(&InputSnapshot::click_at(238, 263));
    assert_eq!(game.shop_panel(), ShopPanel::ShoppingBag);
    assert_eq!(game.shop_cart().len(), 1);
    let cart = paint_native(&mut game);
    assert_color_near(
        pixel(&cart, 65, 558),
        [148, 198, 222],
        "cart face covers sort plate",
    );

    game.ingest_snapshot(&InputSnapshot::click_at(41, 574));
    assert!(game.shop_charge_confirm_open());
    let sheet = paint_native(&mut game);
    assert_ne!(
        pixel(&sheet, 40, 558),
        [247, 239, 239],
        "the MESSAGE-sheet cart frame must not restore the sort plate"
    );
    assert_ne!(
        pixel(&sheet, 630, 575),
        [255, 173, 82],
        "the MESSAGE-sheet cart frame must not restore the orange lamp"
    );
}

const SHOP_BAGSORT_CODE: u32 = 0x1420_00B7;
const SHOP_BAGSORT_SPRITE: &str = "Btn_BagSort.ojs";

fn one_good_catalog() -> AvatarItemTable {
    AvatarItemTable {
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
    }
}

fn room_game() -> ScreenGame {
    let mut game = ScreenGame::new();
    game.menu_key(MenuKey::Back, true);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Room);
    game
}

fn stage_shop_art(test_name: &str) -> Option<StagedArt> {
    let root = install_root();
    if !root_has_game_data(&root) {
        println!("{}", pack_skip_report(&root, test_name));
        return None;
    }
    let interface = read_install_archives(&root).interface?;
    Some(
        stage_state(&interface, State::Shop, StageContext::default())
            .art
            .expect("shop art stages from installed data"),
    )
}

fn paint_native(game: &mut ScreenGame) -> Vec<u32> {
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
    buf
}

/// 5-bit RGB expansion differs by at most one step per channel.
fn assert_color_near(actual: [u8; 3], oracle: [u8; 3], label: &str) {
    for (channel, (got, want)) in actual.into_iter().zip(oracle).enumerate() {
        let delta = i16::from(got) - i16::from(want);
        assert!(
            delta.abs() <= 4,
            "{label} channel {channel}: got {got}, oracle {want}"
        );
    }
}

fn pixel(frame: &[u32], x: usize, y: usize) -> [u8; 3] {
    let word = frame[y * usize::try_from(NATIVE_WIDTH).expect("native width") + x];
    [
        u8::try_from((word >> 16) & 0xFF).expect("byte"),
        u8::try_from((word >> 8) & 0xFF).expect("byte"),
        u8::try_from(word & 0xFF).expect("byte"),
    ]
}
