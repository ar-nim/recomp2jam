//! Game-tab selection painting guards (#716).
//!
//! Drives `ScreenGame::paint_at` with genuine `Dialog_Option_Game`
//! staging and flips one value at a time. The exe paints one shared
//! 16x16 check body only for the selected arm; a flip must repaint
//! exactly those arm cells. Skips cleanly without `install-root`.

use std::collections::HashMap;
use std::path::Path;

use o2jam_render::Frame;
use o2jam_shell::avatar::{AvatarFigure, ScenePlacement};
use o2jam_shell::options::OptionsSettings;
use o2jam_shell::screens::game::PlanetModal;
use o2jam_shell::screens::paint::dialog_origin;
use o2jam_shell::screens::side::{
    PlanetOptionTab, OPTION_GAME_BLEND_NO, OPTION_GAME_BLEND_YES, OPTION_GAME_EFFECT_HIGH,
    OPTION_GAME_EFFECT_LOW, OPTION_GAME_EFFECT_NORMAL, OPTION_GAME_GUIDE_LONG,
    OPTION_GAME_GUIDE_MIDDLE, OPTION_GAME_GUIDE_NOUSE, OPTION_GAME_GUIDE_SHORT,
    OPTION_GAME_LONGNOTE_NONE, OPTION_GAME_LONGNOTE_USED, OPTION_GAME_MODE_FULL,
    OPTION_GAME_MODE_WIN, OPTION_GAME_NOTETYPE1, OPTION_GAME_NOTETYPE2,
};
use o2jam_shell::staging::{stage_dialog, Dialog, StageContext};
use o2jam_shell::{GameLoop, InputSnapshot, MenuKey, ScreenGame};

const WIDTH: usize = 800;
const HEIGHT: usize = 600;

/// Interface archive path, anchored so a hidden install-root skips.
fn interface_archive() -> Option<Vec<u8>> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install-root/O2Jam/Image/Interface1.opi"),
    )
    .ok()
}

/// Open the Game sheet with its genuine staged art and no session.
fn game_tab(placements: Vec<ScenePlacement>, figures: HashMap<String, AvatarFigure>) -> ScreenGame {
    let mut game = ScreenGame::new();
    game.menu_key(MenuKey::Back, true);
    game.click_planet(729, 580);
    assert_eq!(game.planet_modal(), Some(PlanetModal::Options));
    game.switch_option_tab(PlanetOptionTab::Game);
    game.load_planet_option_tab(PlanetOptionTab::Game, placements, figures);
    game
}

/// Native pixels for one static paint of the open Game sheet.
fn painted(game: &mut ScreenGame) -> Vec<u32> {
    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    game.paint_at(
        &mut Frame {
            buf: &mut buffer,
            w: WIDTH,
            h: HEIGHT,
        },
        0,
    );
    buffer
}

/// Bounding box of differing pixels, in native coordinates.
fn changed_bbox(before: &[u32], after: &[u32]) -> Option<(i32, i32, i32, i32)> {
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    for (index, (old, new)) in before.iter().zip(after).enumerate() {
        if old == new {
            continue;
        }
        let (x, y) = ((index % WIDTH) as i32, (index / WIDTH) as i32);
        let entry = bounds.get_or_insert((x, y, x, y));
        entry.0 = entry.0.min(x);
        entry.1 = entry.1.min(y);
        entry.2 = entry.2.max(x);
        entry.3 = entry.3.max(y);
    }
    bounds
}

/// Screen rect of one staged checkbox: dialog origin plus its parsed
/// `.bnd` rect. The check body is `(0,0)`-anchored, so this is also
/// the exe's paint point (3 px left of the click rect).
fn check_rect(placements: &[ScenePlacement], code: u32) -> (i32, i32, i32, i32) {
    let (origin_x, origin_y) = dialog_origin(placements);
    let placement = placements
        .iter()
        .find(|placed| placed.code == code)
        .unwrap_or_else(|| panic!("control {code:#x} stages"));
    (
        origin_x + placement.rect.x,
        origin_y + placement.rect.y,
        placement.rect.w as i32,
        placement.rect.h as i32,
    )
}

fn inside(x: i32, y: i32, rect: (i32, i32, i32, i32)) -> bool {
    (rect.0..rect.0 + rect.2).contains(&x) && (rect.1..rect.1 + rect.3).contains(&y)
}

/// Flip one value and assert the repaint stays inside its arm cells.
fn assert_flip_repaints_arm_cells(
    placements: &[ScenePlacement],
    figures: HashMap<String, AvatarFigure>,
    first: impl FnOnce(&mut OptionsSettings),
    second: impl FnOnce(&mut OptionsSettings),
    codes: &[u32],
) {
    let mut game = game_tab(placements.to_vec(), figures);
    first(game.options_mut());
    let before = painted(&mut game);
    second(game.options_mut());
    let after = painted(&mut game);
    let Some((x1, y1, x2, y2)) = changed_bbox(&before, &after) else {
        panic!("flipping a game value painted no pixels");
    };
    for y in [y1, y2] {
        for x in [x1, x2] {
            assert!(
                codes
                    .iter()
                    .any(|code| inside(x, y, check_rect(placements, *code))),
                "changed pixel ({x},{y}) outside the flipped arm cells"
            );
        }
    }
    let touched = codes
        .iter()
        .filter(|code| changed_inside(&before, &after, check_rect(placements, **code)) > 0)
        .count();
    assert!(touched >= 2, "a flip must clear one arm and light another");
}

/// Number of pixels that differ inside one native rect.
fn changed_inside(before: &[u32], after: &[u32], rect: (i32, i32, i32, i32)) -> usize {
    (rect.1..rect.1 + rect.3)
        .flat_map(|y| (rect.0..rect.0 + rect.2).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            before[y as usize * WIDTH + x as usize] != after[y as usize * WIDTH + x as usize]
        })
        .count()
}

#[test]
fn flipping_note_type_repaints_its_two_check_cells() {
    let Some(archive) = interface_archive() else {
        println!("SKIP flipping_note_type_repaints_its_two_check_cells: install-root absent");
        return;
    };
    let art = stage_dialog(&archive, Dialog::PlanetOptionGame, StageContext::default())
        .art
        .expect("game sheet stages with real bytes");
    assert_eq!(
        check_rect(&art.placements, OPTION_GAME_NOTETYPE1),
        (268, 325, 16, 16),
        "the check paints at the art origin, 3 px left of its click rect"
    );
    assert_eq!(
        check_rect(&art.placements, OPTION_GAME_NOTETYPE2),
        (392, 325, 16, 16)
    );
    assert_flip_repaints_arm_cells(
        &art.placements,
        art.figures,
        |settings| settings.set_note_type(0),
        |settings| settings.set_note_type(1),
        &[OPTION_GAME_NOTETYPE1, OPTION_GAME_NOTETYPE2],
    );
}

#[test]
fn flipping_guide_level_repaints_only_the_guide_cells() {
    let Some(archive) = interface_archive() else {
        println!("SKIP flipping_guide_level_repaints_only_the_guide_cells: install-root absent");
        return;
    };
    let art = stage_dialog(&archive, Dialog::PlanetOptionGame, StageContext::default())
        .art
        .expect("game sheet stages with real bytes");
    assert_flip_repaints_arm_cells(
        &art.placements,
        art.figures,
        |settings| settings.set_note_tail(3),
        |settings| settings.set_note_tail(0),
        &[
            OPTION_GAME_GUIDE_LONG,
            OPTION_GAME_GUIDE_MIDDLE,
            OPTION_GAME_GUIDE_SHORT,
            OPTION_GAME_GUIDE_NOUSE,
        ],
    );
}

#[test]
fn flipping_blend_half_repaints_only_the_eq_cells() {
    let Some(archive) = interface_archive() else {
        println!("SKIP flipping_blend_half_repaints_only_the_eq_cells: install-root absent");
        return;
    };
    let art = stage_dialog(&archive, Dialog::PlanetOptionGame, StageContext::default())
        .art
        .expect("game sheet stages with real bytes");
    assert_flip_repaints_arm_cells(
        &art.placements,
        art.figures,
        |settings| settings.set_blend_half(1),
        |settings| settings.set_blend_half(0),
        &[OPTION_GAME_BLEND_YES, OPTION_GAME_BLEND_NO],
    );
}

#[test]
fn flipping_mode_repaints_only_the_mode_cells() {
    let Some(archive) = interface_archive() else {
        println!("SKIP flipping_mode_repaints_only_the_mode_cells: install-root absent");
        return;
    };
    let art = stage_dialog(&archive, Dialog::PlanetOptionGame, StageContext::default())
        .art
        .expect("game sheet stages with real bytes");
    assert_flip_repaints_arm_cells(
        &art.placements,
        art.figures,
        |settings| settings.set_mode(2),
        |settings| settings.set_mode(1),
        &[OPTION_GAME_MODE_FULL, OPTION_GAME_MODE_WIN],
    );
}

#[test]
fn flipping_longnote_mode_repaints_only_the_longnote_cells() {
    let Some(archive) = interface_archive() else {
        println!(
            "SKIP flipping_longnote_mode_repaints_only_the_longnote_cells: install-root absent"
        );
        return;
    };
    let art = stage_dialog(&archive, Dialog::PlanetOptionGame, StageContext::default())
        .art
        .expect("game sheet stages with real bytes");
    assert_flip_repaints_arm_cells(
        &art.placements,
        art.figures,
        |settings| settings.set_longnote_mode(1),
        |settings| settings.set_longnote_mode(0),
        &[OPTION_GAME_LONGNOTE_USED, OPTION_GAME_LONGNOTE_NONE],
    );
}

#[test]
fn flipping_effect_level_repaints_only_the_effect_cells() {
    let Some(archive) = interface_archive() else {
        println!("SKIP flipping_effect_level_repaints_only_the_effect_cells: install-root absent");
        return;
    };
    let art = stage_dialog(&archive, Dialog::PlanetOptionGame, StageContext::default())
        .art
        .expect("game sheet stages with real bytes");
    assert_flip_repaints_arm_cells(
        &art.placements,
        art.figures,
        |settings| settings.set_effect_level(0),
        |settings| settings.set_effect_level(2),
        &[
            OPTION_GAME_EFFECT_HIGH,
            OPTION_GAME_EFFECT_NORMAL,
            OPTION_GAME_EFFECT_LOW,
        ],
    );
}

#[test]
fn game_tab_default_repaints_every_value_family_in_one_click() {
    let Some(archive) = interface_archive() else {
        println!(
            "SKIP game_tab_default_repaints_every_value_family_in_one_click: install-root absent"
        );
        return;
    };
    let art = stage_dialog(&archive, Dialog::PlanetOptionGame, StageContext::default())
        .art
        .expect("game sheet stages with real bytes");
    let mut game = game_tab(art.placements.clone(), art.figures);
    {
        let settings = game.options_mut();
        settings.set_note_type(1);
        settings.set_note_tail(3);
        settings.set_blend_half(0);
        settings.set_mode(1);
        settings.set_longnote_mode(0);
        settings.set_effect_level(2);
    }
    let before = painted(&mut game);

    let default_code = PlanetOptionTab::Game.default_code();
    let (x, y, w, h) = check_rect(&art.placements, default_code);
    game.ingest_snapshot(&InputSnapshot::click_at(x + w / 2, y + h / 2));
    let settings = game.options();
    assert_eq!(
        (
            settings.note_type(),
            settings.note_tail(),
            settings.blend_half(),
            settings.mode(),
            settings.longnote_mode(),
            settings.effect_level(),
        ),
        (0, 2, 1, 2, 1, 0),
        "DEFAULT restores every exe-pinned game value"
    );
    let after = painted(&mut game);

    for family in [
        &[OPTION_GAME_NOTETYPE1, OPTION_GAME_NOTETYPE2][..],
        &[
            OPTION_GAME_GUIDE_LONG,
            OPTION_GAME_GUIDE_MIDDLE,
            OPTION_GAME_GUIDE_SHORT,
            OPTION_GAME_GUIDE_NOUSE,
        ][..],
        &[OPTION_GAME_BLEND_YES, OPTION_GAME_BLEND_NO][..],
        &[OPTION_GAME_MODE_FULL, OPTION_GAME_MODE_WIN][..],
        &[OPTION_GAME_LONGNOTE_USED, OPTION_GAME_LONGNOTE_NONE][..],
        &[
            OPTION_GAME_EFFECT_HIGH,
            OPTION_GAME_EFFECT_NORMAL,
            OPTION_GAME_EFFECT_LOW,
        ][..],
    ] {
        let repainted: usize = family
            .iter()
            .map(|code| changed_inside(&before, &after, check_rect(&art.placements, *code)))
            .sum();
        assert!(repainted > 0, "DEFAULT left a value family unpainted");
    }
}
