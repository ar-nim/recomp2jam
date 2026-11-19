//! Messenger sheet identity and the unpinned PopUpMenu action labels.
//!
//! The six `DIALOG_MESSENGER_*` blocks and their five PopUpMenu button
//! codes come from the shipped `ControlList_Interface.txt` (spec #315
//! evidence map). Their `.bnd` rects stage, but the qualified B8/B6
//! packs only pin the server-unavailable notice: no mounted sheet, no
//! action frame, and no send opcode. This module therefore names the
//! five buttons without inventing a semantic action or a wire shape.

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
