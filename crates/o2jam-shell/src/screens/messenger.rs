//! Messenger sheet identity and the unpinned PopUpMenu action labels.
//!
//! The six `DIALOG_MESSENGER_*` blocks and their five PopUpMenu button
//! codes come from the shipped `ControlList_Interface.txt` (spec #315
//! evidence map). Their `.bnd` rects stage, but the qualified B8/B6
//! packs only pin the server-unavailable notice: no mounted sheet, no
//! action frame, and no send opcode. This module therefore names the
//! five buttons without inventing a semantic action or a wire shape.

use super::side::{myroom_p0_verdict, MYROOM_ALARM_CODE, MYROOM_ALARM_POPUP_CODE};

/// One lobby hosting the Messenger control family. The five codes
/// come from the spec #315 evidence map; only MyRoom has a qualified
/// exe press pack, so the other four name a capture step instead of
/// a response (tickets #629/#646/#648/#650/#652).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessengerHost {
    /// Room board (`IDC_BUTTON_SROOM_MESSENGER`).
    Room,
    /// Item Shop board.
    Shop,
    /// Avatar MyRoom side screen (the B8 press pack).
    MyRoom,
    /// Waiting room board.
    Waiting,
    /// Music Shop board.
    MusicShop,
}

impl MessengerHost {
    /// `ControlList` control code pinned by the spec evidence map.
    #[must_use]
    pub const fn control_code(self) -> u32 {
        match self {
            Self::Room => 0x1320_0048,
            Self::Shop => 0x1420_00B8,
            Self::MyRoom => super::side::MYROOM_MESSENGER_CODE,
            Self::Waiting => 0x1620_006A,
            Self::MusicShop => 0x1A20_0088,
        }
    }

    /// The one exe-observed response to this host's press, if any.
    /// `None` is an evidence gap, never a dead verdict.
    #[must_use]
    pub const fn exe_response_pin(self) -> Option<&'static str> {
        match self {
            Self::MyRoom => Some("B8 one-OK liveness sheet"),
            Self::Room | Self::Shop | Self::Waiting | Self::MusicShop => None,
        }
    }

    /// Next evidence step for an unpinned host arm. The pinned arm
    /// keeps the pack name instead, so a guard can tell them apart.
    #[must_use]
    pub const fn next_evidence(self) -> &'static str {
        match self {
            Self::Room | Self::Shop | Self::Waiting | Self::MusicShop => {
                "N-HOST: qualified exe press frame and input log on this host board"
            }
            Self::MyRoom => "B8 pack f9d4a695...f8b pins the liveness sheet",
        }
    }
}

/// All five host controls in spec order. Four arms stay OPEN (`N-HOST`),
/// so this table names codes and evidence, not live dispatch.
pub const MESSENGER_HOSTS: [MessengerHost; 5] = [
    MessengerHost::Room,
    MessengerHost::Shop,
    MessengerHost::MyRoom,
    MessengerHost::Waiting,
    MessengerHost::MusicShop,
];

/// `DIALOG_MESSENGER_MAIN` block: 77 parsed rects.
pub const MESSENGER_MAIN_BLOCK: &str = "DIALOG_MESSENGER_MAIN";
/// `DIALOG_MESSENGER_FRIENDADD` block: 6 parsed rects.
pub const MESSENGER_FRIENDADD_BLOCK: &str = "DIALOG_MESSENGER_FRIENDADD";
/// `DIALOG_MESSENGER_QUESTION` block: 7 parsed rects.
pub const MESSENGER_QUESTION_BLOCK: &str = "DIALOG_MESSENGER_QUESTION";
/// `DIALOG_MESSENGER_NOTE` block: 13 parsed rects.
pub const MESSENGER_NOTE_BLOCK: &str = "DIALOG_MESSENGER_NOTE";
/// `DIALOG_MESSENGER_POPUPMENU` block: 6 parsed rects.
pub const MESSENGER_POPUPMENU_BLOCK: &str = "DIALOG_MESSENGER_POPUPMENU";
/// `DIALOG_MESSENGER_INFORMATION` block: 5 parsed rects.
pub const MESSENGER_INFORMATION_BLOCK: &str = "DIALOG_MESSENGER_INFORMATION";

/// `IDC_BUTTON_MESSENGER_NOTE_SEND` in `DIALOG_MESSENGER_NOTE`
/// (`0x89200007` in the shipped ControlList). The Note sheet stages
/// its 13 rects, but no qualified pack mounts it or shows a SEND
/// reply body, so the control stays a labeled no-op.
pub const NOTE_SEND_CODE: u32 = 0x8920_0007;

/// Why SEND sends nothing: the next step is one tapped exe SEND press
/// naming its opcode and body (`N-WIRE`), then the P0 replacement
/// (`N-P0`). Until then no send shape is invented.
const NOTE_SEND_NOOP_LABEL: &str = "IDC_BUTTON_MESSENGER_NOTE_SEND: no exe send pin; no wire";

/// Resolve the NOTE SEND control to its labeled no-op. `None` for
/// every other Note code (e.g. CANCEL), which has no no-op verdict.
#[must_use]
pub fn note_noop_label(code: u32) -> Option<&'static str> {
    (code == NOTE_SEND_CODE).then_some(NOTE_SEND_NOOP_LABEL)
}

/// The five `IDC_BUTTON_MESSENGER_POPUPMENU0n` codes in ControlList
/// order. The shipped names number the buttons but say nothing about
/// what each one does, so dispatch stays a labeled no-op.
pub const POPUP_MENU_CODES: [u32; 5] = [
    0x8A20_0002,
    0x8A20_0003,
    0x8A20_0004,
    0x8A20_0005,
    0x8A20_0006,
];

/// No-op label for one PopUpMenu code: the control name plus the
/// reason it sends nothing. `None` off the five pinned buttons.
const POPUP_MENU_NOOP_LABELS: [&str; POPUP_MENU_CODES.len()] = [
    "IDC_BUTTON_MESSENGER_POPUPMENU01: action unpinned; no wire",
    "IDC_BUTTON_MESSENGER_POPUPMENU02: action unpinned; no wire",
    "IDC_BUTTON_MESSENGER_POPUPMENU03: action unpinned; no wire",
    "IDC_BUTTON_MESSENGER_POPUPMENU04: action unpinned; no wire",
    "IDC_BUTTON_MESSENGER_POPUPMENU05: action unpinned; no wire",
];

/// Resolve one pinned PopUpMenu button to its labeled no-op. A press
/// never queues a send: the exe action frames stay OPEN (`N-BACKEND`).
#[must_use]
pub fn popup_menu_noop_label(code: u32) -> Option<&'static str> {
    POPUP_MENU_CODES
        .iter()
        .position(|pinned| *pinned == code)
        .map(|index| POPUP_MENU_NOOP_LABELS[index])
}

/// Every messenger-owned P0 control that still answers nothing. The set
/// comes from spec #315 M14/M16/M19 and the no-op labels landed in
/// `02ecfd2` and `e123bfc`. The MyRoom host arm is absent: its B8 pack
/// replaced that P0 verdict with the liveness sheet.
pub const MESSENGER_P0_NOOP_CODES: [u32; 8] = [
    NOTE_SEND_CODE,
    POPUP_MENU_CODES[0],
    POPUP_MENU_CODES[1],
    POPUP_MENU_CODES[2],
    POPUP_MENU_CODES[3],
    POPUP_MENU_CODES[4],
    MYROOM_ALARM_CODE,
    MYROOM_ALARM_POPUP_CODE,
];

/// Label that keeps one P0 control inert. `None` off the P0 set.
#[must_use]
pub fn messenger_p0_noop_label(code: u32) -> Option<&'static str> {
    note_noop_label(code)
        .or_else(|| popup_menu_noop_label(code))
        .or_else(|| match code {
            MYROOM_ALARM_CODE | MYROOM_ALARM_POPUP_CODE => myroom_p0_verdict(code),
            _ => None,
        })
}

/// Exe send pin that would replace a P0 no-op. No pin exists today, so
/// a dispatch cannot be added without first naming its evidence here;
/// the P0 guard goes red at that point (`N-P0`).
#[must_use]
pub const fn messenger_p0_send_pin(_code: u32) -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_send_stays_a_labeled_noop_without_an_exe_send_pin() {
        // seam: the whole claim is the no-op contract — the 13-rect
        // Note sheet stages, but the qualified packs never mount it
        // or show a SEND action frame (`N-WIRE`, then `N-P0`), so no
        // send opcode or body exists to reach from boot.
        let label = note_noop_label(NOTE_SEND_CODE).expect("SEND has a no-op label");
        assert!(label.contains("IDC_BUTTON_MESSENGER_NOTE_SEND"), "{label}");
        assert!(label.contains("no exe send pin"), "{label}");
        assert!(label.contains("no wire"), "{label}");
        assert_eq!(
            note_noop_label(0x8920_0008),
            None,
            "CANCEL is not the SEND control"
        );
    }

    #[test]
    fn popup_menu_buttons_stay_labeled_noops_off_the_pinned_sheet() {
        // seam: this is the whole action contract — the qualified
        // packs pin five button codes but no action frame, so no
        // dispatch or wire shape exists to reach from boot.
        let labels: Vec<&str> = POPUP_MENU_CODES
            .iter()
            .map(|code| {
                popup_menu_noop_label(*code)
                    .unwrap_or_else(|| panic!("button {code:#x} has no label"))
            })
            .collect();
        assert_eq!(labels.len(), 5, "five ControlList buttons");
        for label in &labels {
            assert!(label.contains("action unpinned"), "{label}");
            assert!(label.contains("no wire"), "{label}");
        }
        assert_eq!(
            popup_menu_noop_label(0x8A20_0007),
            None,
            "backdrop rect has no action label"
        );
    }

    #[test]
    fn messenger_p0_noops_stay_unpinned_and_answer_nothing() {
        // seam: this is the whole P0 contract. The six Messenger sheets park
        // with no exe mount (`N-MOUNT`) and no send pack (`N-WIRE`), so no
        // boot chain can reach a dispatch; the labels and the pin marker are
        // the complete action surface. Reviewers flip this guard when a pin
        // lands, before any dispatch is written.
        use super::super::side::{
            myroom_p0_verdict, MYROOM_ALARM_CODE, MYROOM_ALARM_POPUP_CODE, MYROOM_MESSENGER_CODE,
            MYROOM_WEBAVATAR_CODE,
        };
        assert_eq!(
            MESSENGER_P0_NOOP_CODES,
            [
                NOTE_SEND_CODE,
                0x8A20_0002,
                0x8A20_0003,
                0x8A20_0004,
                0x8A20_0005,
                0x8A20_0006,
                MYROOM_ALARM_CODE,
                MYROOM_ALARM_POPUP_CODE,
            ],
            "P0 set named by spec #315 M14/M16/M19 and e123bfc"
        );
        for code in MESSENGER_P0_NOOP_CODES {
            let label = messenger_p0_noop_label(code)
                .unwrap_or_else(|| panic!("P0 control {code:#X} lost its label"));
            assert!(
                label.contains("no wire") || label.contains("paints nothing"),
                "P0 control {code:#X} still answers nothing: {label}"
            );
            assert_eq!(
                messenger_p0_send_pin(code),
                None,
                "P0 control {code:#X} has no exe send pin"
            );
        }
        assert_eq!(
            messenger_p0_noop_label(MYROOM_WEBAVATAR_CODE),
            None,
            "a MyRoom P0 outside the Messenger family has no Messenger label"
        );
        assert_eq!(
            myroom_p0_verdict(MYROOM_MESSENGER_CODE),
            None,
            "the B8 pack replaced the host P0 arm; the sheet family P0 stays"
        );
    }
}
