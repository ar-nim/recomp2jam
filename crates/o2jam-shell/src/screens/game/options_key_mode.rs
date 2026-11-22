//! One mode filter for the Option Key sheet: the staged controls a
//! player can see and click, and the lane a staged caption names.
//!
//! Exe pin (7.55, SHA-256 `83a04541…72c823b`): toggle handler
//! `0x4383d0` switches both control groups and the selector states;
//! repaint readers `0x439000` / `0x439780` use separate key arrays,
//! with the latter skipping 3K family index 1. Both the painter and
//! `OptionsDialog::hit` consume these rules, so a hidden family is
//! never painted or clickable.

use super::super::side::{OPTION_KEY_3KEY_FIRST, OPTION_KEY_LANE_FIRST};

/// Staged 3K mode selector (`OptionKey_Select_3Key.ojs`).
const THREE_KEY_SELECTOR: u32 = 0x2550_0006;
/// Staged 7K mode selector (`OptionKey_Select.ojs`).
const SEVEN_KEY_SELECTOR: u32 = 0x2550_0007;
/// Staged 3K board panel (`OptionKey_3Keydown.ojs`).
const THREE_KEY_PANEL: u32 = 0x2550_0013;
/// Staged 7K board panel (`OptionKey_Keydown.ojs`).
const SEVEN_KEY_PANEL: u32 = 0x2550_0014;

/// Captions staged for the 3K family (`0x25209008..0B`).
const THREE_KEY_CAPTIONS: std::ops::RangeInclusive<u32> =
    OPTION_KEY_3KEY_FIRST..=OPTION_KEY_3KEY_FIRST + 3;
/// Captions staged for the 7K family (`0x2520900C..12`).
const SEVEN_KEY_CAPTIONS: std::ops::RangeInclusive<u32> =
    OPTION_KEY_LANE_FIRST..=OPTION_KEY_LANE_FIRST + 6;

/// Is one staged control visible and clickable in this mode?
/// Controls outside both families (tabs, SAVE, mode buttons) stay.
#[must_use]
pub(super) fn is_visible(code: u32, three_key: bool) -> bool {
    !belongs_to_family(code, !three_key)
}

/// Lane named by a staged caption code in the active family. The
/// skipped 3K index has no caption, so it never double-inks its
/// shared rect.
#[must_use]
pub(super) fn caption_lane(code: u32, three_key: bool) -> Option<usize> {
    let offset = if three_key {
        THREE_KEY_CAPTIONS
            .contains(&code)
            .then(|| code - OPTION_KEY_3KEY_FIRST)?
    } else {
        SEVEN_KEY_CAPTIONS
            .contains(&code)
            .then(|| code - OPTION_KEY_LANE_FIRST)?
    };
    let lane = usize::try_from(offset).ok()?;
    is_armable_lane(lane, three_key).then_some(lane)
}

/// Lanes that carry a caption, an arm, and a validated bind in
/// each mode: 3K skips index 1 (its duplicate staged rect).
#[must_use]
pub(super) fn active_lane_indexes(three_key: bool) -> &'static [usize] {
    const THREE_KEY: [usize; 3] = [0, 2, 3];
    const SEVEN_KEY: [usize; 7] = [0, 1, 2, 3, 4, 5, 6];
    if three_key {
        &THREE_KEY
    } else {
        &SEVEN_KEY
    }
}

/// Can a lane be armed and written in this mode?
#[must_use]
pub(super) fn is_armable_lane(lane: usize, three_key: bool) -> bool {
    active_lane_indexes(three_key).contains(&lane)
}

/// Next armed lane after a successful write (`0x4395e0` / `0x438e80`).
#[must_use]
pub(super) fn next_armed_lane(lane: usize, three_key: bool) -> usize {
    let active = active_lane_indexes(three_key);
    let position = active.iter().position(|&active_lane| active_lane == lane);
    active[(position.unwrap_or(0) + 1) % active.len()]
}

fn belongs_to_family(code: u32, three_key: bool) -> bool {
    let (selector, panel, captions) = if three_key {
        (THREE_KEY_SELECTOR, THREE_KEY_PANEL, &THREE_KEY_CAPTIONS)
    } else {
        (SEVEN_KEY_SELECTOR, SEVEN_KEY_PANEL, &SEVEN_KEY_CAPTIONS)
    };
    code == selector || code == panel || captions.contains(&code)
}
