//! Shop entry preview-figure staging (#877).
//!
//! TDD record: before the fix this guard failed with "shop entry must
//! stage the pinned shop-master body" — `stage_state(State::Shop)`
//! staged 64 figures and none was `Shop_Master_main.ojs`; the
//! `0x14600053` placement had no sprite. After staging the
//! code-mounted body by name, the guard passes against installed art.
//! The pinned exe frame shows the same body at its own anchor.

use o2jam_file::data_root::{install_root, root_has_game_data};
use o2jam_render::{Frame, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::screens::side::paint_side_scene_selected;
use o2jam_shell::staging::{read_install_archives, stage_state, StageContext, StagedArt, State};

const SHOP_MASTER_CODE: u32 = 0x1460_0053;
const SHOP_MASTER_SPRITE: &str = "Shop_Master_main.ojs";

#[test]
fn shop_entry_stages_the_exe_shop_master_figure_in_the_bottom_right_pane() {
    // Live caller chain: main.rs::run_play -> live_art::stage_live_scenes
    // -> live_art::stage_interface_scenes -> stage_state(State::Shop)
    // -> stage_shop_master_preview -> paint_side_scene_selected.
    // Data-free arm: skip cleanly (CI has no install-root).
    let root = install_root();
    if !root_has_game_data(&root) {
        return;
    }
    let Some(interface) = read_install_archives(&root).interface else {
        return;
    };
    let art = stage_state(&interface, State::Shop, StageContext::default())
        .art
        .expect("shop art stages from installed data");

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
        "the figure must land in the bottom-right pane rect"
    );
}

#[test]
fn shop_master_figure_paints_at_the_exe_anchor_on_the_live_stage() {
    // Live caller chain: main.rs::run_play -> live_art::stage_live_scenes
    // -> live_art::stage_interface_scenes -> stage_state(State::Shop)
    // -> ScreenGame::load_side_scene -> ScreenGame::paint_side
    // -> paint_side_scene_selected. Data-free arm: skip cleanly.
    let root = install_root();
    if !root_has_game_data(&root) {
        return;
    }
    let Some(interface) = read_install_archives(&root).interface else {
        return;
    };
    let art = stage_state(&interface, State::Shop, StageContext::default())
        .art
        .expect("shop art stages from installed data");

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

    assert!(!changed.is_empty(), "staged shop master paints no pixels");
    assert!(
        changed
            .iter()
            .all(|(x, y)| (654..786).contains(x) && (446..561).contains(y)),
        "mascot ink must land at the pinned sprite anchor, not the pane origin"
    );
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
