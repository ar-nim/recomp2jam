//! Avatar-room scene painters, moved out of the shared module (#139).
//!
//! Every painter reads staged `.bnd` rects and decoded figure bodies;
//! no fallback geometry or literal placement lives here.

use std::collections::HashMap;

use super::{paint_text_into_rect, SceneArt, CP949_INK};
use crate::avatar::{paint_placement_layer, AvatarFigure, ScenePlacement};
use crate::screen::Screen;
use crate::screens::side::{
    myroom_action_for_code, MyRoomAction, MyRoomLowerTab, MyRoomState, MYROOM_AVATAR_CODE,
    MYROOM_BAG_FIRST_CODE, MYROOM_GIFT_FIRST_CODE, MYROOM_GIFT_PAGE_DIGITS,
    MYROOM_GIFT_PAGE_FIRST_CODE, MYROOM_NOTICEBOARD_CODE, MYROOM_PCROOMMARK_CODE,
    MYROOM_SELECT_CODE, MYROOM_SLOT_IMAGE_FIRST_CODE,
};
use o2jam_render::{scale_rect_to_frame, Frame, Rect};

/// NOTICEBOARD frame selected by the exe's notice writer
/// (`FUN_004a9890`, file `0xa9890`, EV `ev_f3b4a8b0…`): frame 2 when
/// the staged body carries it, otherwise the normal all-layer stack.
/// No geometry lives here — the `.bnd` placement still owns the rect.
#[must_use]
pub fn myroom_notice_board_frame(
    placed: &ScenePlacement,
    figures: &HashMap<String, AvatarFigure>,
) -> Option<usize> {
    if placed.code != MYROOM_NOTICEBOARD_CODE {
        return None;
    }
    let sprite = placed.sprite.as_deref()?;
    figures.get(sprite)?.layer(2).map(|_| 2)
}

/// Lower-panel page-number frame staged by the exe's tab painter
/// (`FUN_004a8ba0`, file `0xa8ba0`, EV `ev_c1d8c405…`): frame 1 for
/// MY BAG and frame 2 for PRESENT BOX on the four `0x15500054..57`
/// digits. `None` for every other placement and when that body has no
/// such layer, so the normal staged stack still paints. The bag pager's
/// own digit run `0x1520002F..32` is untouched by that function and
/// keeps its normal art. OPEN: no live frame pair pins the digit frame
/// yet; only this static tab selection is claimed.
#[must_use]
pub fn myroom_page_number_frame(
    placed: &ScenePlacement,
    tab: MyRoomLowerTab,
    figures: &HashMap<String, AvatarFigure>,
) -> Option<usize> {
    let offset = placed.code.checked_sub(MYROOM_GIFT_PAGE_FIRST_CODE)?;
    if offset >= MYROOM_GIFT_PAGE_DIGITS {
        return None;
    }
    let index = match tab {
        MyRoomLowerTab::Bag => 1,
        MyRoomLowerTab::Gifts => 2,
    };
    let sprite = placed.sprite.as_deref()?;
    figures.get(sprite)?.layer(index).map(|_| index)
}

/// PC-room mark liveness for one staged placement. `FUN_004a3730`
/// (VA `0x4a3730`, file `0xa3730`, EV
/// `ev_1b8cb1354f11145172bc8bdb441a1e311a2176cd19b4f64a09b67ab1df39c30f`)
/// looks up `0x1550006a` and stages its frame 1 only when the pass word
/// `0x768b34` is exactly `1` or `0x800`; every other word has no else
/// arm and paints no mark. Our client reads the same word from the
/// staged [`MyRoomPassInfo`](crate::screens::side::MyRoomPassInfo);
/// its wire writer stays OPEN.
#[must_use]
pub fn myroom_pc_room_mark_kept(placed: &ScenePlacement, pass_word: Option<u32>) -> bool {
    let is_pc_room = pass_word.is_some_and(|word| word == 1 || word == 0x800);
    placed.code != MYROOM_PCROOMMARK_CODE || is_pc_room
}

/// Frame the exe stages on the PC-room mark image control. `None`
/// leaves unrelated placements and unpainted bodies on their normal
/// staged stack; a mark without a second frame never claims pixels.
#[must_use]
pub fn myroom_pc_room_mark_frame(
    placed: &ScenePlacement,
    pass_word: Option<u32>,
    figures: &HashMap<String, AvatarFigure>,
) -> Option<usize> {
    const MARK_FRAME: usize = 1;
    if placed.code != MYROOM_PCROOMMARK_CODE || !myroom_pc_room_mark_kept(placed, pass_word) {
        return None;
    }
    let sprite = placed.sprite.as_deref()?;
    figures.get(sprite)?.layer(MARK_FRAME).map(|_| MARK_FRAME)
}

/// Avatar room: staged bound art, flat card fallback.
/// Slot images name no bodies and captions join nowhere, so figures
/// alone paint — no slot icons, captions, or selection state.
pub fn paint_myroom_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Myroom);
}

/// Equipment wells over the staged `0X15600013..22` rects: each
/// worn 2001 word carries its item record byte `+0x18`, and that byte
/// names the well (state entry `FUN_004a3730`, EV `ev_004adf63…`,
/// file `0xa3730`). The earlier ordinal join in `ee8aa26` only holds
/// when those bytes follow array order; the contradiction is pinned
/// in `docs/myroom.md`. `worn` is the resolved (well, item id) run.
/// The row's shop icon (sprite slot 0, never worn) inks the well.
/// No word, row, portrait, or staged well means no ink — no fallback
/// art or literal geometry (ADR-0003).
pub fn paint_myroom_slot_icons(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    worn: &[(u8, u32)],
    portraits: &HashMap<u32, AvatarFigure>,
) {
    for (well, item_id) in worn {
        let Some(portrait) = portraits.get(item_id) else {
            continue;
        };
        let code = MYROOM_SLOT_IMAGE_FIRST_CODE + u32::from(*well);
        let Some(placed) = placements.iter().find(|placed| placed.code == code) else {
            continue;
        };
        let rect = scale_rect_to_frame(&placed.rect, frame.w, frame.h);
        portrait.paint_visible_centered(frame, &rect, frame.w, frame.h, 0);
    }
}

/// Avatar-room rows and captions over staged placements: bag names
/// into the `MYBAGITEM` rects, gift names into the `GIFTBOX_EX`
/// rects, identity/gift strings into their TEXT rects, and the
/// archive's `Myroom_MybagSelect.oji` frame body centered on the
/// selected populated cell of the active lower tab (bag or gifts);
/// a blank cell paints no frame (exe2 empty-bag pack). Slot presses
/// only record the cursor:
/// the exe3 pack `25c5997e…` pins a hair-slot press after a bag pick
/// leaving the bag rows and cell frame unchanged (story #23).
/// The frame body and its `.bnd` placement are archive-pinned;
/// centering it on a picked cell is ours-by-design (`docs/myroom.md`
/// OPEN 8). Without staged art no stand-in paints.
/// Everything rides staged `.bnd` rects in file order; without
/// staged rows nothing paints (no fallback geometry is invented).
#[allow(clippy::too_many_arguments)]
pub fn paint_myroom_data(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
    state: MyRoomState,
    bag_names: &[String],
    gift_names: &[String],
    captions: &[(u32, String)],
) {
    for (code, text) in captions {
        if text.is_empty() {
            continue;
        }
        if let Some(placed) = placements.iter().find(|placed| placed.code == *code) {
            paint_text_into_rect(frame, placed.rect, text, CP949_INK);
        }
    }
    match state.lower_tab() {
        MyRoomLowerTab::Bag => {
            paint_myroom_cells(frame, placements, bag_names, MYROOM_BAG_FIRST_CODE);
            let picked = populated_cell_index(state.bag_cell(), bag_names)
                .and_then(|cell| myroom_cell_placement(placements, MYROOM_BAG_FIRST_CODE, cell))
                .map(|placed| placed.rect);
            if let Some(rect) = picked {
                paint_myroom_selection(frame, placements, figures, &rect);
            }
        }
        MyRoomLowerTab::Gifts => {
            paint_myroom_cells(frame, placements, gift_names, MYROOM_GIFT_FIRST_CODE);
            let picked = populated_cell_index(state.gift_cell(), gift_names)
                .and_then(|cell| myroom_cell_placement(placements, MYROOM_GIFT_FIRST_CODE, cell))
                .map(|placed| placed.rect);
            if let Some(rect) = picked {
                paint_myroom_selection(frame, placements, figures, &rect);
            }
        }
    }
}

/// Keep only a selection whose staged row actually carries a name.
/// The exe2 empty-cell pack paints no frame on a blank row.
fn populated_cell_index(selected: Option<usize>, names: &[String]) -> Option<usize> {
    let index = selected?;
    names
        .get(index)
        .is_some_and(|name| !name.is_empty())
        .then_some(index)
}

/// Paint the archive selection frame over one staged cell rect.
/// The frame's sprite and `.bnd` placement come from the
/// `MYBAGSELECT` control; its size comes from the decoded record.
/// The destination is centered on the staged cell — no literal
/// geometry and no amber fill.
fn paint_myroom_selection(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
    cell: &Rect,
) {
    let Some(selection) = placements
        .iter()
        .find(|placed| placed.code == MYROOM_SELECT_CODE)
    else {
        return;
    };
    let Some(sprite) = selection.sprite.as_deref() else {
        return;
    };
    let Some(layer) = figures.get(sprite).and_then(|figure| figure.layer(0)) else {
        return;
    };
    let (Ok(frame_w), Ok(frame_h)) = (u32::try_from(layer.image.w), u32::try_from(layer.image.h))
    else {
        return;
    };
    let view = ScenePlacement {
        rect: centered_frame_rect(cell, frame_w, frame_h),
        ..selection.clone()
    };
    paint_placement_layer(frame, &view, figures, 0, true, None);
}

/// Center the selection frame on a staged cell, using only that
/// cell's rect and the sprite's stored dimensions.
fn centered_frame_rect(cell: &Rect, frame_w: u32, frame_h: u32) -> Rect {
    let cell_half_w = i32::try_from(cell.w / 2).unwrap_or(0);
    let cell_half_h = i32::try_from(cell.h / 2).unwrap_or(0);
    let frame_half_w = i32::try_from(frame_w / 2).unwrap_or(0);
    let frame_half_h = i32::try_from(frame_h / 2).unwrap_or(0);
    Rect {
        x: cell.x + cell_half_w - frame_half_w,
        y: cell.y + cell_half_h - frame_half_h,
        w: frame_w,
        h: frame_h,
    }
}

/// One row cell by position within its staged family run (bag and
/// gift cells count in staged file order, the same order the click
/// dispatch counts — painter and clicker share the cells).
fn myroom_cell_placement(
    placements: &[ScenePlacement],
    first: u32,
    index: usize,
) -> Option<&ScenePlacement> {
    let mut seen = 0usize;
    for placed in placements {
        let is_cell = myroom_action_for_code(placed.code).is_some_and(|action| match action {
            MyRoomAction::PickBagCell(_) => first == MYROOM_BAG_FIRST_CODE,
            MyRoomAction::PickGiftCell(_) => first == MYROOM_GIFT_FIRST_CODE,
            _ => false,
        });
        if !is_cell {
            continue;
        }
        if seen == index {
            return Some(placed);
        }
        seen += 1;
    }
    None
}

/// Row strings into one staged cell family in file order: the
/// `n`th string paints the `n`th staged cell rect. Extra strings
/// paint nowhere; missing strings leave later cells blank.
fn paint_myroom_cells(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    names: &[String],
    first: u32,
) {
    let cells: Vec<&ScenePlacement> = {
        let mut cells = Vec::new();
        let mut index = 0usize;
        while let Some(placed) = myroom_cell_placement(placements, first, index) {
            cells.push(placed);
            index += 1;
        }
        cells
    };
    for (placed, name) in cells.iter().zip(names.iter()) {
        if !name.is_empty() {
            paint_text_into_rect(frame, placed.rect, name, CP949_INK);
        }
    }
}

/// Avatar-room portrait: the staged worn figure center-stage in
/// its staged AVATAR rect (`MYROOM_AVATAR_CODE`), breathing on the
/// paint clock through the #193 per-run pipeline
/// (`AvatarFigure::paint_visible_centered`). Without a staged figure
/// or rect the slot stays empty — art alone, never a fallback pose.
pub fn paint_myroom_figure(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figure: Option<&AvatarFigure>,
    now_ms: u64,
) {
    let (Some(figure), Some(slot)) = (
        figure,
        placements
            .iter()
            .find(|placed| placed.code == MYROOM_AVATAR_CODE),
    ) else {
        return;
    };
    let slot = scale_rect_to_frame(&slot.rect, frame.w, frame.h);
    figure.paint_visible_centered(frame, &slot, frame.w, frame.h, now_ms);
}
