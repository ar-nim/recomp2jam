//! Dialog control frame states: rest, hover, and active.
//!
//! `docs/hover.md` pins the exe's button language on room and planet
//! chrome. The qualified dialog packs carry rest frames only (spec
//! #316 story 17), so applying that language to dialog buttons is a
//! labeled stand-in. `docs/dialog-frame-states.md` names every sheet
//! whose states stay OPEN and the capture needed to pin them.
//!
//! Keeping the state here, apart from `ScreenGame`, lets the frame
//! choice be tested without a display or game data. The painter gets
//! the chosen record; geometry still comes from staged `.bnd` rects.

/// Which open dialog owns a pressed control. Pairing the press with
/// its host stops a stale press from lighting the next sheet that
/// happens to reuse the same control code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DialogHost {
    /// Planet quit-confirm (`Dialog_Question1`).
    QuitConfirm,
    /// Planet options tabs (`Dialog_Option_*`).
    Options,
    /// Relay-unavailable one-OK sheet (`Dialog_Information`).
    RelayDown,
    /// Refused channel-enter one-OK sheet.
    ChannelRefused,
    /// Room create sheet.
    RoomCreate,
    /// Room password sheet.
    RoomPassword,
    /// Room 07DA poll-starve one-OK sheet.
    RoomFuse,
    /// MyRoom Messenger liveness one-OK sheet.
    MessengerLiveness,
}

/// One control's frame state. Rest leaves the staged stack alone,
/// hover repaints record 0, and active (press held) repaints record 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum DialogControlFrame {
    /// No pointer or press on the control.
    #[default]
    Rest,
    /// Pointer on the control: full-colour body frame 0.
    Hover,
    /// Press held on the control: body frame 1.
    Active,
}

impl DialogControlFrame {
    /// Staged record the state repaints (`None` keeps the rest stack).
    #[must_use]
    pub(super) const fn record(self) -> Option<usize> {
        match self {
            Self::Rest => None,
            Self::Hover => Some(0),
            Self::Active => Some(1),
        }
    }
}

/// Pointer/press arm for the open dialog's controls.
#[derive(Debug, Default)]
pub(super) struct DialogFrameState {
    pressed: Option<(DialogHost, u32)>,
}

impl DialogFrameState {
    /// Arm one control on pointer-down.
    pub(super) fn arm_press(&mut self, host: DialogHost, code: u32) {
        self.pressed = Some((host, code));
    }

    /// Drop the held press on release or dismissal.
    pub(super) fn release(&mut self) {
        self.pressed = None;
    }

    /// Resolve one control's state. A held press wins over hover;
    /// an active control does not revert on hover, matching the
    /// active-tab rule in `docs/hover.md`.
    #[must_use]
    pub(super) fn frame_for(
        &self,
        host: DialogHost,
        hovered: Option<u32>,
        code: u32,
    ) -> DialogControlFrame {
        if self.pressed == Some((host, code)) {
            DialogControlFrame::Active
        } else if hovered == Some(code) {
            DialogControlFrame::Hover
        } else {
            DialogControlFrame::Rest
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_control_repaints_no_record() {
        let state = DialogFrameState::default();
        assert_eq!(
            state.frame_for(DialogHost::QuitConfirm, None, 0x1234),
            DialogControlFrame::Rest
        );
        assert_eq!(DialogControlFrame::Rest.record(), None);
    }

    #[test]
    fn hovered_control_repaints_the_first_record() {
        let state = DialogFrameState::default();
        assert_eq!(
            state.frame_for(DialogHost::QuitConfirm, Some(0x1234), 0x1234),
            DialogControlFrame::Hover
        );
        assert_eq!(DialogControlFrame::Hover.record(), Some(0));
    }

    #[test]
    fn held_press_arms_the_active_record_over_hover() {
        let mut state = DialogFrameState::default();
        state.arm_press(DialogHost::QuitConfirm, 0x1234);
        assert_eq!(
            state.frame_for(DialogHost::QuitConfirm, Some(0x1234), 0x1234),
            DialogControlFrame::Active
        );
        assert_eq!(DialogControlFrame::Active.record(), Some(1));
    }

    #[test]
    fn release_returns_the_control_to_hover() {
        let mut state = DialogFrameState::default();
        state.arm_press(DialogHost::QuitConfirm, 0x1234);
        state.release();
        assert_eq!(
            state.frame_for(DialogHost::QuitConfirm, Some(0x1234), 0x1234),
            DialogControlFrame::Hover
        );
    }

    #[test]
    fn press_on_another_host_does_not_light_the_open_sheet() {
        let mut state = DialogFrameState::default();
        state.arm_press(DialogHost::Options, 0x1234);
        assert_eq!(
            state.frame_for(DialogHost::QuitConfirm, None, 0x1234),
            DialogControlFrame::Rest,
            "a stale press must not repaint a different sheet"
        );
    }
}
