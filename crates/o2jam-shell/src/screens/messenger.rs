//! Messenger sheet identity and the unpinned PopUpMenu action labels.
//!
//! The six `DIALOG_MESSENGER_*` blocks and their five PopUpMenu button
//! codes come from the shipped `ControlList_Interface.txt` (spec #315
//! evidence map). Their `.bnd` rects stage, but the qualified B8/B6
//! packs only pin the server-unavailable notice: no mounted sheet, no
//! action frame, and no send opcode. This module therefore names the
//! five buttons without inventing a semantic action or a wire shape.

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
