//! FREEINFO captions driven by the exe's pass word (#800).
//!
//! `FUN_004a8010` (VA `0x4a8010`, file `0xa8010`, EV
//! `ev_bf4287e718ef7c085b5513838dc663397805504008bfd6e740c4e1eb4031d3d2`)
//! picks two FREEINFO lines and up to four skill rows from the word at
//! `0x768b34`. The wire writer of that word is not traced, so our
//! staged [`MyRoomPassInfo`] supplies it explicitly; the source stays
//! OPEN. No browser or purchase claim is made here.

use super::{MYROOM_FREEINFO_COUNT, MYROOM_FREEINFO_FIRST_CODE};

/// Staged pass state for MyRoom's status face. Every field starts
/// empty, so the exe's blank case is the default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MyRoomPassInfo {
    /// Value of the exe's pass/membership word `0x768b34`.
    pub word: Option<u32>,
    /// Date string the exe formats into its `%s` slots.
    pub until: String,
    /// Names from the four 28-byte skill records, in record order.
    pub skills: Vec<String>,
}

/// First caption code of the four per-row skill labels. The exe's
/// loop starts at `0x15400043`, 28 bytes per record
/// (file `0xa8010`). The trailing two codes are not named FREEINFO in
/// the block dump; no row paints until a skill record is staged.
const FIRST_SKILL_ROW_CODE: u32 = MYROOM_FREEINFO_FIRST_CODE + 2;
/// Number of skill rows the exe loop can reach.
const SKILL_ROW_COUNT: usize = MYROOM_FREEINFO_COUNT;

/// The two fixed lines for one pass word, plus the extra date line the
/// 0x100 branch writes. `None` leaves every cell blank, like an
/// unstaged word.
fn pass_copy(word: u32) -> Option<(&'static str, &'static str, Option<&'static str>)> {
    match word {
        0 => Some((
            "'무한반지 이용중'",
            "모든 스킬반지를 사용 할 수 있습니다.",
            None,
        )),
        1 => Some((
            "오투잼 PC방 이용 중",
            "전곡을 자유롭게 연주 할 수 있습니다.",
            None,
        )),
        0x20 | 0x800 => Some((
            "'전곡 무료 이용중'",
            "모든 곡을 자유롭게 연주 할 수 있습니다.",
            None,
        )),
        0x100 => Some((
            "자유이용제",
            "모든 곡을 자유롭게 연주 할 수 있습니다.",
            Some("until"),
        )),
        0x2 | 0x4 | 0x8 | 0x10 | 0x40 | 0x80 | 0x200 | 0x400 | 0x420 => Some((
            "자유이용제",
            "모든 곡을 자유롭게 연주 할 수 있습니다.",
            None,
        )),
        _ => None,
    }
}

/// FREEINFO lines and skill rows for the staged pass state. Codes are
/// painted in order; a later skill row replaces an earlier line at the
/// same code, matching the exe's two write passes (file `0xa8010`).
#[must_use]
pub fn myroom_freeinfo_captions(pass: &MyRoomPassInfo) -> Vec<(u32, String)> {
    let Some(word) = pass.word else {
        return Vec::new();
    };
    let Some((headline, detail, extra)) = pass_copy(word) else {
        return Vec::new();
    };
    let mut captions = vec![
        (
            MYROOM_FREEINFO_FIRST_CODE,
            format!("{headline} ({})", pass.until),
        ),
        (MYROOM_FREEINFO_FIRST_CODE + 1, detail.to_owned()),
    ];
    if extra == Some("until") {
        captions.push((
            MYROOM_FREEINFO_FIRST_CODE + 2,
            format!("{} 까지", pass.until),
        ));
    }
    for (index, name) in pass.skills.iter().take(SKILL_ROW_COUNT).enumerate() {
        if name.is_empty() {
            continue;
        }
        let code = FIRST_SKILL_ROW_CODE + u32::try_from(index).unwrap_or(u32::MAX);
        captions.push((code, name.clone()));
    }
    let mut deduped: Vec<(u32, String)> = Vec::with_capacity(captions.len());
    for (code, text) in captions {
        if let Some(existing) = deduped.iter_mut().find(|(seen, _)| *seen == code) {
            existing.1 = text;
        } else {
            deduped.push((code, text));
        }
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picroom_pass_inks_its_pinned_two_lines() {
        let pass = MyRoomPassInfo {
            word: Some(1),
            until: "2026-12-31".to_owned(),
            skills: Vec::new(),
        };
        let captions = myroom_freeinfo_captions(&pass);
        assert_eq!(captions[0].1, "오투잼 PC방 이용 중 (2026-12-31)");
        assert_eq!(captions[1].1, "전곡을 자유롭게 연주 할 수 있습니다.");
    }

    #[test]
    fn long_pass_adds_the_date_line_and_four_skill_rows() {
        let pass = MyRoomPassInfo {
            word: Some(0x100),
            until: "2027-01-01".to_owned(),
            skills: (1..=4).map(|index| format!("Skill{index}")).collect(),
        };
        let captions = myroom_freeinfo_captions(&pass);
        assert_eq!(captions.len(), 6);
        assert_eq!(captions[2].0, FIRST_SKILL_ROW_CODE);
        assert_eq!(
            captions[2].1, "Skill1",
            "the first row overwrites the date cell"
        );
        assert_eq!(captions[5].1, "Skill4");
    }

    #[test]
    fn long_pass_without_skill_rows_keeps_its_extra_date_line() {
        let pass = MyRoomPassInfo {
            word: Some(0x100),
            until: "2027-01-01".to_owned(),
            skills: Vec::new(),
        };
        let captions = myroom_freeinfo_captions(&pass);
        assert_eq!(captions[2].1, "2027-01-01 까지");
    }

    #[test]
    fn unstaged_and_unknown_pass_words_leave_the_cells_blank() {
        assert!(myroom_freeinfo_captions(&MyRoomPassInfo::default()).is_empty());
        let unknown = MyRoomPassInfo {
            word: Some(0xFFFF_FFFF),
            ..MyRoomPassInfo::default()
        };
        assert!(myroom_freeinfo_captions(&unknown).is_empty());
    }

    #[test]
    fn skill_rows_never_exceed_the_exe_four_record_loop() {
        let pass = MyRoomPassInfo {
            word: Some(4),
            skills: (1..=8).map(|index| format!("Skill{index}")).collect(),
            ..MyRoomPassInfo::default()
        };
        assert_eq!(myroom_freeinfo_captions(&pass).len(), 6);
    }
}
