//! STATUS caption sources for MyRoom (#799): the exe's format words
//! and the 99-entry next-up threshold table. Numeric facts come from
//! the 7.55 oracle (OTwo.exe SHA-256
//! `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`);
//! no decompiled source is kept here.

/// 99 little-endian `u32` thresholds at exe file `0x24776c`
/// (VA `0x64776c`), read with REA/Ghidra 12.2, evidence
/// `ev_b062b9faf5ba4bc445bcda0356993bae5fb098c8959e8d645ccf84f6367a8f7b`.
/// The STATUS writer indexes this table by the level word when that
/// word is below 99 (EV `ev_21ce4a71...`, file `0xa7a80`).
pub const NEXT_UP_THRESHOLDS: [u32; 99] = [
    884, 1819, 2839, 3978, 5270, 6749, 8449, 10404, 12648, 15215, 18139, 21454, 25194, 29393,
    34085, 39304, 45084, 51459, 58463, 66130, 74494, 83589, 93449, 104108, 115600, 127959, 141219,
    155414, 170578, 186745, 203949, 222224, 241604, 262123, 283815, 306714, 330854, 356269, 382993,
    411060, 440504, 471359, 503659, 537438, 572730, 609569, 647989, 688024, 729708, 773075, 818159,
    864994, 913614, 964053, 1016345, 1070524, 1126624, 1184679, 1244723, 1306790, 1370914, 1437129,
    1505469, 1575968, 1648660, 1723579, 1800759, 1880234, 1962038, 2046205, 2132769, 2221764,
    2313224, 2407183, 2503675, 2602734, 2704394, 2808689, 2915653, 3025320, 3137724, 3252899,
    3370879, 3491698, 3615390, 3741989, 3871529, 4004044, 4139568, 4278135, 4419779, 4564534,
    4712434, 4863513, 5017805, 5175344, 5336164, 5500299, 5667783,
];

/// Threshold printed by the next-up cell for one level. Levels outside
/// the table print nothing in the exe: no guess is substituted.
#[must_use]
pub const fn next_up_threshold(level: i32) -> Option<u32> {
    if level < 0 || level >= NEXT_UP_THRESHOLDS.len() as i32 {
        return None;
    }
    // Index is in range by the guard above; the cast cannot wrap.
    Some(NEXT_UP_THRESHOLDS[level as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_table_starts_with_the_pinned_first_three_values() {
        assert_eq!(NEXT_UP_THRESHOLDS[..3], [884, 1819, 2839]);
    }

    #[test]
    fn threshold_is_present_for_level_zero_but_absent_at_the_table_end() {
        assert_eq!(next_up_threshold(0), Some(884));
        assert_eq!(next_up_threshold(98), Some(5_667_783));
        assert_eq!(next_up_threshold(99), None);
        assert_eq!(next_up_threshold(-1), None);
    }
}
