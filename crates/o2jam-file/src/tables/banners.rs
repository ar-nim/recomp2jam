//! STEP1/2/3 banner-row lookups into the key-name table.

/// First key-name entry of the STEP1 banner row (`전체`, entry 19).
///
/// Sub-range base `0x645524` is entry 19 of the key-name table, so dialog
/// `+0x6bc` indexes this row directly (`mov 0x645524(,%edx,4)` at file
/// `0x46fec`, two-key site at file `0x476ca`, three-key site at file
/// `0x47e0f`). Rows are `전체/초급/중급/고급/최고급`.
pub const OJN_STEP1_NAME_BASE_ENTRY: usize = 19;

/// STEP1 banner rows (`전체` + four difficulty names).
pub const OJN_STEP1_NAME_COUNT: usize = 5;

/// STEP1 buttons (`STEP1_ALL/EASY/INTERMEDIATE/HIGH/HIGHEST`, codes
/// `0x2720101C..0x27201020` in `Interface1.opi`).
pub const OJN_STEP1_BUTTON_COUNT: u32 = 5;

/// STEP1 sentinel: dialog `+0x6bc` init value (no choice yet; files
/// `0x3d324` and `0x5156b`). The STEP1 filter gates on `> 4` (file
/// `0x46b67`, 5-arm table at file `0x47038`), so the sentinel never
/// filters; the highlight sync treats it as unselected (file `0x4fad2`).
pub const OJN_STEP1_SENTINEL: u32 = 5;

/// First key-name entry of the STEP2 banner row (`전체`, entry 50).
///
/// Sub-range base `0x6455a0` is entry 50 of the key-name table, so dialog
/// `+0x6c0` indexes this row directly (`mov 0x6455a0(,%edx,4)` at file
/// `0x476b6`, three-key site at file `0x47dfb`). Rows are
/// `전체/신곡/골드/블랙/블루/레드/실버`.
pub const OJN_STEP2_NAME_BASE_ENTRY: usize = 50;

/// STEP2 banner rows (`전체` + `신곡` + five label names).
pub const OJN_STEP2_NAME_COUNT: usize = 7;

/// STEP2 buttons (`STEP2_ALL/GOLD/BLACK/BLUE/RED/SILVER`, codes
/// `0x2720B021..0x2720B026` in `Interface1.opi`).
pub const OJN_STEP2_BUTTON_COUNT: u32 = 6;

/// STEP2 sentinel: dialog `+0x6c0` init value (no choice yet; files
/// `0x3d331` and `0x51578`). The STEP2 filter gates on `> 5` (file
/// `0x47274`, 6-arm table at file `0x47718`: arm 0 copies all, arms 1..=5
/// test `entry+0x4 == 1..=5`); the caller skips the filter entirely when
/// the sentinel holds (file `0x46d06`).
pub const OJN_STEP2_SENTINEL: u32 = 6;

/// First key-name entry of the STEP3 banner row (`전체`, entry 57).
///
/// Sub-range base `0x6455bc` is entry 57 of the key-name table, so dialog
/// `+0x6c4` indexes this row directly (`mov 0x6455bc(,%edx,4)` at file
/// `0x47de7`). Rows are
/// `전체/발라드/락/댄스/힙합/R&B/재즈/펑크/클래식/기타`.
pub const OJN_STEP3_NAME_BASE_ENTRY: usize = 57;

/// STEP3 banner rows (`전체` + seven genre names).
pub const OJN_STEP3_NAME_COUNT: usize = 8;

/// STEP3 buttons (`STEP3_ALL/BALLAD/ROCK/DANCE/HIPHOP_SOUL/JAZZ_FUNK/
/// CLASSIC/ETC`, codes `0x2720B027..0x2720B02E` in `Interface1.opi`).
pub const OJN_STEP3_BUTTON_COUNT: u32 = 8;

/// STEP3 sentinel: dialog `+0x6c4` init value (no choice yet; files
/// `0x3d33e` and `0x51585`). The STEP3 filter gates on `> 7` (file
/// `0x47958`, 8-arm table at file `0x47e50`); the caller skips the filter
/// entirely when the sentinel holds (file `0x473ca`).
pub const OJN_STEP3_SENTINEL: u32 = 8;

/// Resolve a STEP button value to its key-name banner entry.
///
/// Returns `base + step` for button values (`step < buttons`), else `None`
/// for the sentinel and anything beyond (ERRORS-06: no panic on
/// out-of-domain input; PERF-07: checked narrowing, no `as` casts).
fn step_banner_entry(step: u32, buttons: u32, base: usize) -> Option<usize> {
    if step >= buttons {
        return None;
    }
    let offset = usize::try_from(step).ok()?;
    base.checked_add(offset)
}

/// Resolve a STEP1 button value (`code - 0x2720101C`, written at file
/// `0x3f6c2`) to its key-name banner entry (base entry 19).
///
/// Values 0..=4 map onto `전체/초급/중급/고급/최고급`; the sentinel 5
/// (and anything beyond) selects no row.
#[must_use]
pub fn ojn_step1_banner_entry(step: u32) -> Option<usize> {
    step_banner_entry(step, OJN_STEP1_BUTTON_COUNT, OJN_STEP1_NAME_BASE_ENTRY)
}

/// Resolve a STEP2 button value (`code - 0x2720B021`, written at file
/// `0x3f7ee`) to its key-name banner entry (base entry 50).
///
/// Values 0..=5 map onto rows 0..=5 (`전체/신곡/골드/블랙/블루/레드`): this
/// is off by one against the STEP2 label filter (arm `n` tests
/// `entry+0x4 == n`, so GOLD shows row `신곡`, BLACK shows `골드`, and so
/// on; row 6 `실버` is unreachable through the six buttons). The mismatch
/// is observed behavior kept as-is; its intent stays OPEN. The sentinel 6
/// (and anything beyond) selects no row.
#[must_use]
pub fn ojn_step2_banner_entry(step: u32) -> Option<usize> {
    step_banner_entry(step, OJN_STEP2_BUTTON_COUNT, OJN_STEP2_NAME_BASE_ENTRY)
}

/// Resolve a STEP3 button value (`code - 0x2720B027`, written at file
/// `0x3f91b`) to its key-name banner entry (base entry 57).
///
/// Values 0..=7 map onto
/// `전체/발라드/락/댄스/힙합/R&B/재즈/펑크/클래식/기타`; the sentinel 8
/// (and anything beyond) selects no row.
#[must_use]
pub fn ojn_step3_banner_entry(step: u32) -> Option<usize> {
    step_banner_entry(step, OJN_STEP3_BUTTON_COUNT, OJN_STEP3_NAME_BASE_ENTRY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step1_banner_maps_each_button_to_its_name_row() {
        // STEP1 buttons (`code - 0x2720101C`, writer at file `0x3f6c2`)
        // index the entry-19 row (`mov 0x645524(,%edx,4)` at file
        // `0x46fec`): 0..=4 select `전체/초급/중급/고급/최고급`.
        for (button, want) in [0u32, 1, 2, 3, 4].iter().zip(19..24) {
            assert_eq!(ojn_step1_banner_entry(*button), Some(want));
        }
        assert_eq!(OJN_STEP1_NAME_COUNT, 5);
    }

    #[test]
    fn step1_banner_rejects_sentinel_and_beyond() {
        // Init value 5 (files `0x3d324`/`0x5156b`) means no STEP1 choice:
        // the filter gates on `> 4` (file `0x46b67`), so it selects no row.
        for step in [OJN_STEP1_SENTINEL, 6, 7, u32::MAX] {
            assert_eq!(ojn_step1_banner_entry(step), None);
        }
    }

    #[test]
    fn step2_banner_maps_buttons_onto_the_seven_name_row() {
        // STEP2 buttons (`code - 0x2720B021`, writer at file `0x3f7ee`)
        // index the entry-50 row (`mov 0x6455a0(,%edx,4)` at file
        // `0x476b6`): 0..=5 select rows 50..=55. Row 1 is `신곡`, so GOLD
        // banners `신곡`, BLACK banners `골드`, and so on through RED
        // banner `레드`; row 56 (`실버`) stays unreachable (OPEN mismatch).
        for (button, want) in [0u32, 1, 2, 3, 4, 5].iter().zip(50..56) {
            assert_eq!(ojn_step2_banner_entry(*button), Some(want));
        }
        assert_eq!(OJN_STEP2_NAME_COUNT, 7);
        assert_eq!(OJN_STEP2_BUTTON_COUNT, 6);
    }

    #[test]
    fn step2_banner_rejects_sentinel_and_beyond() {
        // Init value 6 (files `0x3d331`/`0x51578`) means no STEP2 choice:
        // the filter gates on `> 5` (file `0x47274`) and the caller skips
        // the filter on 6 (file `0x46d06`), so it selects no row.
        for step in [OJN_STEP2_SENTINEL, 7, 8, u32::MAX] {
            assert_eq!(ojn_step2_banner_entry(step), None);
        }
    }

    #[test]
    fn step3_banner_maps_each_button_to_its_genre_row() {
        // STEP3 buttons (`code - 0x2720B027`, writer at file `0x3f91b`)
        // index the entry-57 row (`mov 0x6455bc(,%edx,4)` at file
        // `0x47de7`): 0..=7 select
        // `전체/발라드/락/댄스/힙합/재즈/클래식/기타`.
        for (button, want) in [0u32, 1, 2, 3, 4, 5, 6, 7].iter().zip(57..65) {
            assert_eq!(ojn_step3_banner_entry(*button), Some(want));
        }
        assert_eq!(OJN_STEP3_NAME_COUNT, 8);
    }

    #[test]
    fn step3_banner_rejects_sentinel_and_beyond() {
        // Init value 8 (files `0x3d33e`/`0x51585`) means no STEP3 choice:
        // the filter gates on `> 7` (file `0x47958`) and the caller skips
        // the filter on 8 (file `0x473ca`), so it selects no row.
        for step in [OJN_STEP3_SENTINEL, 9, 10, u32::MAX] {
            assert_eq!(ojn_step3_banner_entry(step), None);
        }
    }
}
