//! Rest, hover and active frame states for the six parked Messenger sheets.
//!
//! The button language comes from the room/planet sweep in `docs/hover.md`;
//! no qualified Messenger hover or press pack pins a record pair, so this
//! is a labeled stand-in, not an exe pixel claim. `docs/messenger-frame-states.md`
//! names every sheet and the capture needed to replace it.
//!
//! The state stays separate from `ScreenGame` so it can be tested with no
//! display and no game data. A record is only selected when the staged
//! figure carries that layer; a missing record paints nothing (ADR-0003).

use crate::avatar::AvatarFigure;
use crate::staging::MessengerSheet;

/// One control's frame state. Rest leaves the staged stack alone,
/// hover repaints record 0, and active (press held) repaints record 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessengerFrame {
    /// No pointer or press on the control.
    #[default]
    Rest,
    /// Pointer on the control: full-colour body frame 0.
    Hover,
    /// Press held on the control: body frame 1.
    Active,
}

impl MessengerFrame {
    /// Staged record this state repaints. `None` when rest has no
    /// repaint or the staged figure lacks the record, so no frame is
    /// invented from a sheet that only carries one.
    #[must_use]
    pub fn record(self, figure: &AvatarFigure) -> Option<usize> {
        let index = match self {
            Self::Rest => return None,
            Self::Hover => 0,
            Self::Active => 1,
        };
        figure.layer(index).map(|_| index)
    }
}

/// Pointer/press arm for one parked Messenger sheet. The sheet is part
/// of the press key so a stale press cannot light another sheet that
/// reuses the same control code.
#[derive(Debug, Default)]
pub struct MessengerFrameState {
    pressed: Option<(MessengerSheet, u32)>,
}

impl MessengerFrameState {
    /// Arm one control on pointer-down.
    pub fn arm_press(&mut self, sheet: MessengerSheet, code: u32) {
        self.pressed = Some((sheet, code));
    }

    /// Drop the held press on release or dismissal.
    pub fn release(&mut self) {
        self.pressed = None;
    }

    /// Resolve one control's state. A held press wins over hover,
    /// matching the dialog family's active-tab rule.
    #[must_use]
    pub fn frame_for(
        &self,
        sheet: MessengerSheet,
        hovered: Option<u32>,
        code: u32,
    ) -> MessengerFrame {
        if self.pressed == Some((sheet, code)) {
            MessengerFrame::Active
        } else if hovered == Some(code) {
            MessengerFrame::Hover
        } else {
            MessengerFrame::Rest
        }
    }
}

/// Exe-verified frame pair for one sheet. Every row stays `None` until
/// a qualified mount capture names the rest/hover/active records; the
/// guard in `staging/tests.rs` flips when a pin is claimed without it.
#[must_use]
pub const fn messenger_frame_exe_pin(_sheet: MessengerSheet) -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_record_figure() -> AvatarFigure {
        use crate::avatar::FigureLayer;
        let mut figure = AvatarFigure::default();
        for _ in 0..2 {
            figure.push_layer(FigureLayer {
                image: o2jam_render::Rgb555Image::decode(&[0u8; 8], 2, 2)
                    .expect("two-record fixture"),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    #[test]
    fn messenger_rest_leaves_the_staged_stack_alone() {
        let state = MessengerFrameState::default();
        assert_eq!(
            state.frame_for(MessengerSheet::Main, None, 0x1234),
            MessengerFrame::Rest
        );
        assert_eq!(MessengerFrame::Rest.record(&two_record_figure()), None);
    }

    #[test]
    fn messenger_hover_selects_the_first_record_when_the_sheet_carries_it() {
        let state = MessengerFrameState::default();
        assert_eq!(
            state.frame_for(MessengerSheet::FriendAdd, Some(0x1234), 0x1234),
            MessengerFrame::Hover
        );
        assert_eq!(MessengerFrame::Hover.record(&two_record_figure()), Some(0));
    }

    #[test]
    fn messenger_held_press_selects_the_second_record_over_hover() {
        let mut state = MessengerFrameState::default();
        state.arm_press(MessengerSheet::Note, 0x1234);
        assert_eq!(
            state.frame_for(MessengerSheet::Note, Some(0x1234), 0x1234),
            MessengerFrame::Active
        );
        assert_eq!(MessengerFrame::Active.record(&two_record_figure()), Some(1));
    }

    #[test]
    fn messenger_release_returns_the_control_to_hover() {
        let mut state = MessengerFrameState::default();
        state.arm_press(MessengerSheet::Question, 0x1234);
        state.release();
        assert_eq!(
            state.frame_for(MessengerSheet::Question, Some(0x1234), 0x1234),
            MessengerFrame::Hover
        );
    }

    #[test]
    fn messenger_press_on_another_sheet_does_not_light_the_open_sheet() {
        let mut state = MessengerFrameState::default();
        state.arm_press(MessengerSheet::PopUpMenu, 0x1234);
        assert_eq!(
            state.frame_for(MessengerSheet::Information, None, 0x1234),
            MessengerFrame::Rest,
            "a stale press must not repaint a different sheet"
        );
    }

    #[test]
    fn messenger_single_record_control_has_no_active_frame() {
        let mut figure = AvatarFigure::default();
        figure.push_layer(crate::avatar::FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0u8; 8], 2, 2)
                .expect("single-record fixture"),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        assert_eq!(MessengerFrame::Hover.record(&figure), Some(0));
        assert_eq!(
            MessengerFrame::Active.record(&figure),
            None,
            "no invented frame 1 where the staged art has one record"
        );
    }
}
