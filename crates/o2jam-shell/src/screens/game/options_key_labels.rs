//! Key-tab caption painting: the bound DIK selects one frame of the
//! staged `OptionKey_Keytext.ojs` atlas (#715).
//!
//! Exe pin (7.55 `OTwo.exe`, SHA-256
//! `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`):
//! a 37-byte DIK-order array at dialog `+0x198` is filled by
//! `0x438080`; `0x439230` finds a bound DIK's index; `0x439000` /
//! `0x439780` repaint with frame `index + 1` and three origin classes.
//! `Interface1.opi` sha256
//! `d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584`
//! carries the 37 glyph records; capture `play-04-key1-bound` matches
//! rec12 ("A") after a rebind from rec34 ("L Shift").
//!
//! The armed face is OPEN: capture `play-03-key1-armed` differs from
//! rest only by the cursor blob (the 3 px sheet-origin split applies
//! to art at `(217,42)` and `.bnd` clicks at `(220,43)`). The outline
//! below is a labeled stand-in, not an exe face. Next evidence step:
//! re-run the arm capture with the cursor parked off the button.

use super::super::paint::{dialog_origin, paint_planet_options, SceneArt};
use super::super::side::{
    PlanetOptionTab, OPTION_KEY_3KEY_FIRST, OPTION_KEY_3KEY_MODE, OPTION_KEY_7KEY_MODE,
    OPTION_KEY_LANE_FIRST,
};
use crate::avatar::ScenePlacement;
use o2jam_render::{blit_rgb555, fill_rect, pack_xrgb, Frame, Rect};

const KEY_CAPTION_SPRITE: &str = "OptionKey_Keytext.ojs";

/// The `+0x198` DIK-order array (VAs `0x438080` fill, `0x439230` search):
/// letters `Q..P`, `A..'`, `Z..RShift`, then LShift, Space and Enter.
const DIK_ORDER: [u8; 37] = [
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1E, 0x1F, 0x20, 0x21,
    0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34,
    0x35, 0x36, 0x2A, 0x39, 0x1C,
];

/// OPEN stand-in ink for an armed lane. No exe face was captured.
const ARMED_OUTLINE_INK: u32 = pack_xrgb(0x00, 0xFF, 0xFF);
const ARMED_OUTLINE_THICKNESS: i32 = 2;

/// Bound lane DIKs and armed lane for one Key-tab repaint.
#[derive(Debug, Clone, Copy)]
pub(super) struct KeyCaptions {
    /// Raw `KEY_n` DIK codes (the exe's `+0x345..+0x34b` equivalents).
    /// Both key-count families read these shared lanes per spec #314
    /// story 6; the exe's separate 3-key default set (LShift/RShift/
    /// Enter in capture `play-02`) stays OPEN.
    pub(super) lane_dik: [u8; crate::options::OPTION_LANES],
    /// Which key-count family staged art shows.
    pub(super) three_key: bool,
    /// Lane armed for a rebind, if any.
    pub(super) armed_lane: Option<usize>,
}

/// Paint the Key tab with one bound glyph per visible lane button.
pub(super) fn paint_option_key_tab(
    frame: &mut Frame<'_>,
    art: Option<&SceneArt>,
    captions: KeyCaptions,
    settings: &crate::options::OptionsSettings,
) {
    let Some(art) = art.filter(|art| art.has_figures()) else {
        paint_planet_options(frame, art, PlanetOptionTab::Key, settings);
        return;
    };
    paint_dialog_without_captions(frame, art, settings);
    paint_bound_captions(frame, art, captions);
    if let Some(lane) = captions.armed_lane {
        paint_armed_stand_in(frame, art, captions.three_key, lane);
    }
}

/// Paint the staged sheet with the 37-frame atlas held back: the
/// generic dialog painter would stack all frames at one rect, while
/// the exe image control paints exactly the selected frame (VAs
/// `0x439000` / `0x439780`).
fn paint_dialog_without_captions(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    settings: &crate::options::OptionsSettings,
) {
    let without_captions = SceneArt {
        placements: art
            .placements
            .iter()
            .filter(|placed| placed.sprite.as_deref() != Some(KEY_CAPTION_SPRITE))
            .cloned()
            .collect(),
        figures: art.figures.clone(),
    };
    paint_planet_options(
        frame,
        Some(&without_captions),
        PlanetOptionTab::Key,
        settings,
    );
}

/// Paint the active family's captions at their exe offset class.
fn paint_bound_captions(frame: &mut Frame<'_>, art: &SceneArt, captions: KeyCaptions) {
    let Some(figure) = art.figures.get(KEY_CAPTION_SPRITE) else {
        return;
    };
    let origin = dialog_origin(&art.placements);
    for placed in &art.placements {
        let Some(lane) = visible_lane(placed, captions.three_key) else {
            continue;
        };
        let Some(dik) = captions.lane_dik.get(lane) else {
            continue;
        };
        let Some(glyph) = glyph_index(*dik) else {
            continue;
        };
        let Some(layer) = figure.layer(glyph) else {
            continue;
        };
        let (x, y) = glyph_origin(&placed.rect, glyph, captions.three_key);
        blit_rgb555(
            frame,
            &layer.image.view(),
            origin.0 + x,
            origin.1 + y,
            Some(layer.colorkey),
        );
    }
}

/// Lane shown by one caption button in the active key-count family.
fn visible_lane(placed: &ScenePlacement, three_key: bool) -> Option<usize> {
    if three_key {
        visible_lane_in(placed.code, OPTION_KEY_3KEY_FIRST, 4)
    } else {
        visible_lane_in(placed.code, OPTION_KEY_LANE_FIRST, 7)
    }
}

fn visible_lane_in(code: u32, first: u32, count: u32) -> Option<usize> {
    let offset = code.checked_sub(first)?;
    (offset < count)
        .then(|| usize::try_from(offset).ok())
        .flatten()
}

/// Array index for a bound DIK (`0x439230`; `None` is the exe's
/// not-found sentinel, which never repaints a caption).
fn glyph_index(dik: u8) -> Option<usize> {
    DIK_ORDER.iter().position(|entry| *entry == dik)
}

/// Exe caption origin classes (`0x439000` / `0x439780`): the three
/// wide keys use the control origin, Space shifts down 6 px, and
/// Enter uses the control origin only in 3-key mode.
fn glyph_origin(rect: &Rect, glyph: usize, three_key: bool) -> (i32, i32) {
    match glyph {
        0x21 | 0x22 => (rect.x, rect.y),
        0x23 => (rect.x, rect.y + 6),
        0x24 if three_key => (rect.x, rect.y),
        0x24 => (rect.x, rect.y + 6),
        _ => (rect.x + 4, rect.y + 4),
    }
}

/// Labeled OPEN stand-in for the armed rebind lane. The playthrough
/// found no exe arm face, so this outline is explicitly not a 7.55
/// behavior claim; it only makes the pending rebind visible.
fn paint_armed_stand_in(frame: &mut Frame<'_>, art: &SceneArt, three_key: bool, lane: usize) {
    let origin = dialog_origin(&art.placements);
    for placed in &art.placements {
        if visible_lane(placed, three_key) != Some(lane) {
            continue;
        }
        let thickness = ARMED_OUTLINE_THICKNESS;
        let Rect { x, y, w, h } = placed.rect;
        let width = i32::try_from(w).unwrap_or(i32::MAX);
        let height = i32::try_from(h).unwrap_or(i32::MAX);
        for rect in [
            Rect {
                x: origin.0 + x,
                y: origin.1 + y,
                w,
                h: thickness as u32,
            },
            Rect {
                x: origin.0 + x,
                y: origin.1 + y + height - thickness,
                w,
                h: thickness as u32,
            },
            Rect {
                x: origin.0 + x,
                y: origin.1 + y,
                w: thickness as u32,
                h,
            },
            Rect {
                x: origin.0 + x + width - thickness,
                y: origin.1 + y,
                w: thickness as u32,
                h,
            },
        ] {
            fill_rect(frame, &rect, ARMED_OUTLINE_INK);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{PlanetModal, ScreenGame};
    use super::*;
    use crate::avatar::FigureLayer;
    use crate::staging::{stage_dialog, Dialog, StageContext};
    use crate::{GameLoop, MenuKey};

    const ART_ORIGIN: (i32, i32) = (217, 42);

    fn interface_archive() -> Option<Vec<u8>> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../install-root/O2Jam/Image/Interface1.opi"),
        )
        .ok()
    }

    fn key_art() -> Option<SceneArt> {
        let archive = interface_archive()?;
        let staged = stage_dialog(&archive, Dialog::PlanetOption, StageContext::default())
            .art
            .expect("KEY sheet stages with real bytes");
        Some(SceneArt {
            placements: staged.placements,
            figures: staged.figures,
        })
    }

    fn bound_lanes(first: u8) -> [u8; crate::options::OPTION_LANES] {
        let mut lanes = [0u8; crate::options::OPTION_LANES];
        lanes[0] = first;
        lanes
    }

    fn glyph_mismatches(buf: &[u32], layer: &FigureLayer, at: (i32, i32)) -> usize {
        let mut expected = vec![0u32; 800 * 600];
        blit_rgb555(
            &mut Frame {
                buf: &mut expected,
                w: 800,
                h: 600,
            },
            &layer.image.view(),
            at.0,
            at.1,
            Some(layer.colorkey),
        );
        expected
            .iter()
            .zip(buf)
            .filter(|(want, painted)| **want != 0 && **want != **painted)
            .count()
    }

    fn paint_key_tab(art: &SceneArt, captions: KeyCaptions) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        paint_option_key_tab(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            Some(art),
            captions,
            &crate::options::OptionsSettings::default(),
        );
        buf
    }

    #[test]
    fn three_key_button_paints_the_bound_shift_glyph_at_the_captured_origin() {
        let Some(art) = key_art() else {
            return;
        };
        // Capture play-02: 3KEY_1 = L Shift (DIK 0x2A), rec34 at
        // rect-local (101,294) + the 3 px art-origin split.
        let buf = paint_key_tab(
            &art,
            KeyCaptions {
                lane_dik: bound_lanes(0x2A),
                three_key: true,
                armed_lane: None,
            },
        );
        let figure = art
            .figures
            .get(KEY_CAPTION_SPRITE)
            .expect("key glyph atlas stages");
        let layer = figure.layer(34).expect("L Shift glyph decodes");
        assert_eq!(
            glyph_mismatches(&buf, layer, (318, 336)),
            0,
            "3KEY_1 must paint rec34 at the capture's art origin"
        );
    }

    #[test]
    fn seven_key_button_repaints_the_new_glyph_after_a_rebind() {
        let Some(art) = key_art() else {
            return;
        };
        // Capture play-04: a rebind to A (DIK 0x1E) puts rec12 at the
        // key button rect (89,304) + (4,4), the default offset class.
        let before = paint_key_tab(
            &art,
            KeyCaptions {
                lane_dik: bound_lanes(0x1F),
                three_key: false,
                armed_lane: None,
            },
        );
        let after = paint_key_tab(
            &art,
            KeyCaptions {
                lane_dik: bound_lanes(0x1E),
                three_key: false,
                armed_lane: None,
            },
        );
        let figure = art
            .figures
            .get(KEY_CAPTION_SPRITE)
            .expect("key glyph atlas stages");
        let layer = figure.layer(12).expect("A glyph decodes");
        assert_eq!(
            glyph_mismatches(&after, layer, (310, 350)),
            0,
            "KEY1 must repaint rec12 at rect (89,304) + (4,4)"
        );
        let old_glyph = figure.layer(13).expect("S glyph decodes");
        assert_eq!(
            glyph_mismatches(&before, old_glyph, (310, 350)),
            0,
            "KEY1 shows the old S glyph before the rebind"
        );
        assert!(
            glyph_mismatches(&after, old_glyph, (310, 350)) > 0,
            "the old glyph must disappear after the rebind"
        );
    }

    #[test]
    fn armed_lane_has_a_distinct_open_stand_in_face() {
        let Some(art) = key_art() else {
            return;
        };
        let rest = paint_key_tab(
            &art,
            KeyCaptions {
                lane_dik: bound_lanes(0x1F),
                three_key: false,
                armed_lane: None,
            },
        );
        let armed = paint_key_tab(
            &art,
            KeyCaptions {
                lane_dik: bound_lanes(0x1F),
                three_key: false,
                armed_lane: Some(0),
            },
        );
        let outline_pixel = (ART_ORIGIN.1 + 304) as usize * 800 + (ART_ORIGIN.0 + 89) as usize;
        assert_ne!(
            rest[outline_pixel], ARMED_OUTLINE_INK,
            "the unarmed button must not carry stand-in ink there"
        );
        assert_eq!(
            armed[outline_pixel], ARMED_OUTLINE_INK,
            "the OPEN stand-in must make an armed lane visible"
        );
    }

    #[test]
    fn gear_opened_key_tab_paints_the_bound_caption_through_the_live_modal() {
        // Live caller chain: gear click -> PlanetModal::Options ->
        // ScreenGame::paint_planet_modal -> this painter. The seam
        // guards above pin the frame math; this guard pins the wiring.
        let Some(art) = key_art() else {
            return;
        };
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(729, 580);
        assert_eq!(game.planet_modal(), Some(PlanetModal::Options));
        game.load_planet_option_tab(
            PlanetOptionTab::Key,
            art.placements.clone(),
            art.figures.clone(),
        );
        game.switch_option_tab(PlanetOptionTab::Key);
        game.options_mut()
            .rebind_lane(0, winit::keyboard::KeyCode::KeyA);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_planet_modal(&mut Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        });
        let figure = art
            .figures
            .get(KEY_CAPTION_SPRITE)
            .expect("key glyph atlas stages");
        assert_eq!(
            glyph_mismatches(&buf, figure.layer(12).expect("A glyph decodes"), (310, 350)),
            0,
            "the live modal must paint the rebound A glyph"
        );
    }

    fn open_key_sheet(art: &SceneArt) -> ScreenGame {
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(729, 580);
        assert_eq!(game.planet_modal(), Some(PlanetModal::Options));
        game.load_planet_option_tab(
            PlanetOptionTab::Key,
            art.placements.clone(),
            art.figures.clone(),
        );
        game.switch_option_tab(PlanetOptionTab::Key);
        game
    }

    /// Screen point on one staged control. `top_edge` picks a point
    /// that no overlapping lane rect also contains (3K rects sit
    /// partly on top of the 7K rects in `Interface1.opi`).
    fn staged_point(art: &SceneArt, code: u32, top_edge: bool) -> (i32, i32) {
        let placed = art
            .placements
            .iter()
            .find(|placed| placed.code == code)
            .expect("control is staged");
        let origin = dialog_origin(&art.placements);
        (
            origin.0 + placed.rect.x + half_width(&placed.rect),
            origin.1
                + placed.rect.y
                + if top_edge {
                    0
                } else {
                    half_height(&placed.rect)
                },
        )
    }

    fn half_width(rect: &Rect) -> i32 {
        i32::try_from(rect.w / 2).expect("staged rect fits a screen")
    }

    fn half_height(rect: &Rect) -> i32 {
        i32::try_from(rect.h / 2).expect("staged rect fits a screen")
    }

    fn click_staged(game: &mut ScreenGame, at: (i32, i32)) {
        game.ingest_snapshot(&crate::InputSnapshot::click_at(at.0, at.1));
    }

    fn paint_live_key_tab(game: &ScreenGame) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        game.paint_planet_modal(&mut Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        });
        buf
    }

    fn changed_outside_captions(before: &[u32], after: &[u32]) -> usize {
        (0..600)
            .flat_map(|y| (0..800).map(move |x| (x, y)))
            .filter(|(x, y)| !(310..490).contains(x) || !(340..373).contains(y))
            .filter(|(x, y)| {
                before[*y as usize * 800 + *x as usize]
                    != after[*y as usize * 800 + *x as usize]
            })
            .count()
    }

    fn changed_in_rect(before: &[u32], after: &[u32], rect: &Rect) -> usize {
        let width = i32::try_from(rect.w).expect("staged rect fits a screen");
        let height = i32::try_from(rect.h).expect("staged rect fits a screen");
        (rect.y..rect.y + height)
            .flat_map(move |y| (rect.x..rect.x + width).map(move |x| (x, y)))
            .filter(|(x, y)| {
                before[*y as usize * 800 + *x as usize]
                    != after[*y as usize * 800 + *x as usize]
            })
            .count()
    }

    #[test]
    fn gear_opened_key_tab_seeds_the_exe_three_key_caption() {
        // Live caller chain: gear click -> PlanetModal::Options ->
        // ScreenGame::paint_planet_modal -> options_key_labels::paint_option_key_tab.
        // Exe constructor 0x438080 selects 3K; capture play-02 pins
        // 3KEY_1 = L Shift (rec34). Archives absent -> skip.
        let Some(art) = key_art() else {
            return;
        };
        let game = open_key_sheet(&art);
        assert!(
            game.options().three_key(),
            "the exe opens the Key sheet in 3K mode"
        );
        let painted = paint_live_key_tab(&game);
        let figure = art
            .figures
            .get(KEY_CAPTION_SPRITE)
            .expect("key glyph atlas stages");
        let l_shift = figure.layer(34).expect("L Shift glyph decodes");
        assert_eq!(
            glyph_mismatches(&painted, l_shift, (318, 336)),
            0,
            "3KEY_1 must show the pinned L Shift glyph on first open"
        );
    }

    #[test]
    fn key_mode_toggle_switches_the_painted_family_and_ignores_inactive_lanes() {
        // Live caller chain: pointer -> ScreenGame::ingest_snapshot ->
        // OptionsDialog::click -> click_key_tab -> set_three_key ->
        // paint_planet_modal -> paint_option_key_tab -> paint_dialog_scene.
        // Exe toggle handler 0x4383d0 swaps control groups. Archives absent -> skip.
        let Some(art) = key_art() else {
            return;
        };
        let mut game = open_key_sheet(&art);
        let three_key_buffer = paint_live_key_tab(&game);

        click_staged(&mut game, staged_point(&art, OPTION_KEY_7KEY_MODE, false));
        assert!(!game.options().three_key(), "7KEY switches the mode");
        let seven_key_buffer = paint_live_key_tab(&game);
        assert!(
            changed_outside_captions(&three_key_buffer, &seven_key_buffer) > 0,
            "the toggle must repaint panels/selectors, not only captions"
        );

        click_staged(&mut game, staged_point(&art, OPTION_KEY_3KEY_FIRST, true));
        assert_eq!(
            game.option_rebind_armed(),
            None,
            "a 3K lane click is ignored in 7K mode"
        );
        click_staged(&mut game, staged_point(&art, OPTION_KEY_LANE_FIRST + 6, false));
        assert_eq!(
            game.option_rebind_armed(),
            Some(6),
            "a 7K lane click still arms its lane"
        );

        game.capture_raw_key(winit::keyboard::KeyCode::KeyQ, true);
        game.switch_option_tab(PlanetOptionTab::Key);
        click_staged(&mut game, staged_point(&art, OPTION_KEY_3KEY_MODE, false));
        assert!(game.options().three_key(), "3KEY switches back");
        click_staged(&mut game, staged_point(&art, OPTION_KEY_LANE_FIRST + 6, false));
        assert_eq!(
            game.option_rebind_armed(),
            None,
            "a 7K lane click is ignored in 3K mode"
        );
    }

    #[test]
    fn three_key_duplicate_rect_paints_only_the_kept_family_index() {
        // Exe repaint 0x439780 skips 3K family index 1; 0x25209009
        // and 0x2520900A share one staged rect. Archives absent -> skip.
        let Some(art) = key_art() else {
            return;
        };
        let origin = dialog_origin(&art.placements);
        let rect = art
            .placements
            .iter()
            .find(|placed| placed.code == OPTION_KEY_3KEY_FIRST + 2)
            .expect("the kept 3K caption is staged")
            .rect;
        let mut kept_dik = [0u8; crate::options::OPTION_LANES];
        kept_dik[2] = 0x1F;
        let mut both_dik = kept_dik;
        both_dik[1] = 0x1E;
        let kept = paint_key_tab(
            &art,
            KeyCaptions {
                lane_dik: kept_dik,
                three_key: true,
                armed_lane: None,
            },
        );
        let both = paint_key_tab(
            &art,
            KeyCaptions {
                lane_dik: both_dik,
                three_key: true,
                armed_lane: None,
            },
        );
        let at = Rect {
            x: origin.0 + rect.x,
            y: origin.1 + rect.y,
            w: rect.w,
            h: rect.h,
        };
        assert_eq!(
            changed_in_rect(&kept, &both, &at),
            0,
            "the skipped 3K index must not ink the shared rect twice"
        );
    }

    #[test]
    fn rebind_advances_the_arm_and_keeps_seven_key_binds_separate() {
        // Live caller chain: pointer -> OptionsDialog::click ->
        // click_key_tab -> window::route_key -> GameLoop::capture_raw_key
        // -> OptionsDialog::rebind_armed_lane -> paint_planet_modal.
        // Exe 0x4395e0/0x438e80 write +0x18a + armed, advance the arm,
        // and read mode-specific key arrays. Archives absent -> skip.
        let Some(art) = key_art() else {
            return;
        };
        let mut game = open_key_sheet(&art);
        click_staged(&mut game, staged_point(&art, OPTION_KEY_3KEY_FIRST, true));
        assert_eq!(game.option_rebind_armed(), Some(0));
        assert!(game.capture_raw_key(winit::keyboard::KeyCode::KeyA, true));
        assert_eq!(
            game.option_rebind_armed(),
            Some(2),
            "3K arm advances past skipped index 1"
        );

        let figure = art
            .figures
            .get(KEY_CAPTION_SPRITE)
            .expect("key glyph atlas stages");
        let a_glyph = figure.layer(12).expect("A glyph decodes");
        let three_key_buffer = paint_live_key_tab(&game);
        assert_eq!(
            glyph_mismatches(&three_key_buffer, a_glyph, (318, 336)),
            0,
            "the 3K bind table must hold the new A glyph"
        );

        game.switch_option_tab(PlanetOptionTab::Key);
        click_staged(&mut game, staged_point(&art, OPTION_KEY_7KEY_MODE, false));
        let seven_key_buffer = paint_live_key_tab(&game);
        let default_s = figure.layer(13).expect("S glyph decodes");
        assert_eq!(
            glyph_mismatches(&seven_key_buffer, default_s, (306, 346)),
            0,
            "a 3K rebind must not overwrite the 7K bind table"
        );

        click_staged(&mut game, staged_point(&art, OPTION_KEY_LANE_FIRST, false));
        assert_eq!(game.option_rebind_armed(), Some(0));
        assert!(game.capture_raw_key(winit::keyboard::KeyCode::KeyQ, true));
        assert_eq!(
            game.option_rebind_armed(),
            Some(1),
            "7K arm advances to the next lane"
        );
        let q_glyph = figure.layer(0).expect("Q glyph decodes");
        let seven_key_rebound = paint_live_key_tab(&game);
        assert_eq!(
            glyph_mismatches(&seven_key_rebound, q_glyph, (306, 346)),
            0,
            "the 7K bind table must hold the new Q glyph"
        );

        game.switch_option_tab(PlanetOptionTab::Key);
        click_staged(&mut game, staged_point(&art, OPTION_KEY_3KEY_MODE, false));
        let three_key_restored = paint_live_key_tab(&game);
        assert_eq!(
            glyph_mismatches(&three_key_restored, a_glyph, (318, 336)),
            0,
            "the 3K table keeps its A glyph after a 7K rebind"
        );
    }

    #[test]
    fn key_tab_paints_only_the_bound_frame_not_the_whole_atlas() {
        let Some(art) = key_art() else {
            return;
        };
        let buf = paint_key_tab(
            &art,
            KeyCaptions {
                lane_dik: bound_lanes(0x2A),
                three_key: true,
                armed_lane: None,
            },
        );
        let figure = art
            .figures
            .get(KEY_CAPTION_SPRITE)
            .expect("key glyph atlas stages");
        assert_eq!(figure.len(), 37, "the shipped atlas has 37 glyphs");
        let unbound = figure.layer(0).expect("Q glyph decodes");
        assert!(
            glyph_mismatches(&buf, unbound, (318, 336)) > 0,
            "the loader-default Q frame must not stack on the bound glyph"
        );
    }
}
