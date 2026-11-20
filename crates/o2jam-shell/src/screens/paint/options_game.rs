//! Game-tab selection paint (#716).
//!
//! The exe's Game sheet authors one 16x16 check body per arm and shows
//! only the arm matching the live value. The check is `(0,0)`-anchored,
//! so its paint point is the staged `.bnd` rect origin plus the dialog
//! origin — `(268,325)`/`(392,325)` for NoteType1/2 in the 7.55
//! capture, three pixels left of the click rect. Unchecked arms show
//! the sheet background (the effect row alone authors its own empty-box
//! faces under the checks).
//!
//! This is the Game arm only: the Key and Sound arms keep their own
//! painters, and no geometry is hard-coded here.

use crate::avatar::ScenePlacement;
use crate::options::{note_tail_of_guide, NoteGuide, OptionsSettings};
use crate::screens::side::{
    OPTION_GAME_BLEND_NO, OPTION_GAME_BLEND_YES, OPTION_GAME_EFFECT_HIGH, OPTION_GAME_EFFECT_LOW,
    OPTION_GAME_EFFECT_NORMAL, OPTION_GAME_GUIDE_LONG, OPTION_GAME_GUIDE_MIDDLE,
    OPTION_GAME_GUIDE_NOUSE, OPTION_GAME_GUIDE_SHORT, OPTION_GAME_LONGNOTE_NONE,
    OPTION_GAME_LONGNOTE_USED, OPTION_GAME_MODE_FULL, OPTION_GAME_MODE_WIN, OPTION_GAME_NOTETYPE1,
    OPTION_GAME_NOTETYPE2,
};

use super::{dialog_origin, paint_dialog_figure, Frame, SceneArt};

/// Effect-row unchecked faces (`Option_Game_EffectUnCheck1.ojs` and
/// `Option_Game_EffectUnCheck2.ojs`, `0x2f500012`/`0x2f500013`).
/// They author the empty boxes and captions under the three check
/// bodies, so the selected check must paint after them.
const EFFECT_UNCHECK_CODES: [u32; 2] = [0x2F50_0012, 0x2F50_0013];

/// Paint the Game sheet in one pass, skipping the check bodies whose
/// arm is not selected. All other staged bodies (backdrop, captions,
/// unchecked boxes) paint exactly as in the shared dialog painter.
pub(super) fn paint_game_tab(frame: &mut Frame<'_>, art: &SceneArt, settings: &OptionsSettings) {
    if !art.has_figures() {
        return;
    }
    let origin = dialog_origin(&art.placements);
    for placement in &art.placements {
        if EFFECT_UNCHECK_CODES.contains(&placement.code) {
            paint_placement(frame, art, placement, origin);
        }
    }
    for placement in &art.placements {
        if EFFECT_UNCHECK_CODES.contains(&placement.code)
            || is_unselected_check(placement, settings)
        {
            continue;
        }
        paint_placement(frame, art, placement, origin);
    }
}

/// Paint one staged body at its dialog-relative rect or record anchor.
fn paint_placement(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    placement: &ScenePlacement,
    origin: (i32, i32),
) {
    let Some(sprite) = placement.sprite.as_ref() else {
        return;
    };
    let Some(figure) = art.figures.get(sprite) else {
        return;
    };
    if !figure.is_empty() {
        paint_dialog_figure(frame, placement, figure, origin);
    }
}

/// Whether a staged control is a Game check body for an unselected
/// value. The control id is the exe's repaint source (file `0x3a7c0`);
/// each branch names the value the exe compares against.
fn is_unselected_check(placement: &ScenePlacement, settings: &OptionsSettings) -> bool {
    match placement.code {
        OPTION_GAME_NOTETYPE1 => settings.note_type() != 0,
        OPTION_GAME_NOTETYPE2 => settings.note_type() != 1,
        OPTION_GAME_GUIDE_LONG => settings.note_tail() != note_tail_of_guide(NoteGuide::Long),
        OPTION_GAME_GUIDE_MIDDLE => settings.note_tail() != note_tail_of_guide(NoteGuide::Middle),
        OPTION_GAME_GUIDE_SHORT => settings.note_tail() != note_tail_of_guide(NoteGuide::Short),
        OPTION_GAME_GUIDE_NOUSE => settings.note_tail() != note_tail_of_guide(NoteGuide::NoUse),
        _ => is_unselected_non_guide_check(placement.code, settings),
    }
}

/// The remaining check families: effect, EQ, window mode, longnote.
fn is_unselected_non_guide_check(code: u32, settings: &OptionsSettings) -> bool {
    match code {
        OPTION_GAME_EFFECT_HIGH => settings.effect_level() != 0,
        OPTION_GAME_EFFECT_NORMAL => settings.effect_level() != 1,
        OPTION_GAME_EFFECT_LOW => settings.effect_level() != 2,
        OPTION_GAME_BLEND_YES => settings.blend_half() != 1,
        OPTION_GAME_BLEND_NO => settings.blend_half() != 0,
        OPTION_GAME_MODE_FULL => settings.mode() != 2,
        OPTION_GAME_MODE_WIN => settings.mode() != 1,
        OPTION_GAME_LONGNOTE_USED => !settings.longnote_used(),
        OPTION_GAME_LONGNOTE_NONE => settings.longnote_used(),
        _ => false,
    }
}
