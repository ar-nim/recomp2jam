//! Screen state machine: the 7.55 client flow from launch to results.
//!
//! Transition map from `docs/lifecycle.md`: splash → channel → room
//! → waiting → loading → playing → result, with retry/leave-back
//! edges. Only transitions the original performs exist here; the UI
//! of each screen comes later. Headless by design (no window needed).

/// Visible client screens on the main flow (state IDs from
/// `ControlList_Interface.txt`; see `docs/lifecycle.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Logo splash, first visible screen (`CStateAvi`).
    Avi,
    /// Channel select (`DIALOG_CHANNEL_INFO` popups, no state class).
    Channel,
    /// Multiplayer lobby (`CStateRoom`).
    Room,
    /// Pre-game room (`CStateWaiting` / `CLiveWaiting`).
    Waiting,
    /// Song load gate between waiting and playing.
    Loading,
    /// Gameplay (`CStateNormalPlaying` / `CStateLivePlaying`).
    Playing,
    /// Results (`CStateResult`).
    Result,
}

/// Rejected screen change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadTransition {
    /// Screen we tried to leave.
    pub from: Screen,
    /// Screen we tried to enter.
    pub to: Screen,
}

/// Whether the client may move from one screen to another.
#[must_use]
pub const fn can_enter(from: Screen, to: Screen) -> bool {
    use Screen::*;
    matches!(
        (from, to),
        (Avi, Channel)
            | (Channel, Room)
            | (Room, Waiting)
            | (Room, Channel)
            | (Waiting, Loading)
            | (Waiting, Room)
            | (Loading, Playing)
            | (Loading, Waiting)
            | (Playing, Result)
            | (Result, Waiting)
            | (Result, Room)
    )
}

/// Tracks the current screen; every change goes through [`can_enter`].
#[derive(Debug)]
pub struct ScreenFlow {
    current: Screen,
}

impl ScreenFlow {
    /// Boot into the splash screen.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: Screen::Avi,
        }
    }

    /// Current screen.
    #[must_use]
    pub fn current(&self) -> Screen {
        self.current
    }

    /// Move to the next screen, rejecting jumps the client never makes.
    pub fn go(&mut self, next: Screen) -> Result<(), BadTransition> {
        if !can_enter(self.current, next) {
            return Err(BadTransition {
                from: self.current,
                to: next,
            });
        }
        self.current = next;
        Ok(())
    }
}

impl Default for ScreenFlow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod screen_flow_tests {
    use super::*;

    #[test]
    fn boots_into_splash() {
        assert_eq!(ScreenFlow::new().current(), Screen::Avi);
    }

    #[test]
    fn walks_full_session_to_results() {
        let mut flow = ScreenFlow::new();
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            flow.go(next).unwrap();
        }
        assert_eq!(flow.current(), Screen::Result);
    }

    #[test]
    fn retries_from_result_back_to_waiting() {
        let mut flow = ScreenFlow::new();
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
            Screen::Waiting,
        ] {
            flow.go(next).unwrap();
        }
        assert_eq!(flow.current(), Screen::Waiting);
    }

    #[test]
    fn leaves_result_for_room_list() {
        let mut flow = ScreenFlow::default();
        flow.go(Screen::Channel).unwrap();
        flow.go(Screen::Room).unwrap();
        flow.go(Screen::Waiting).unwrap();
        flow.go(Screen::Loading).unwrap();
        flow.go(Screen::Playing).unwrap();
        flow.go(Screen::Result).unwrap();
        flow.go(Screen::Room).unwrap();
        assert_eq!(flow.current(), Screen::Room);
    }

    #[test]
    fn backs_out_of_waiting_to_room() {
        let mut flow = ScreenFlow::new();
        flow.go(Screen::Channel).unwrap();
        flow.go(Screen::Room).unwrap();
        flow.go(Screen::Waiting).unwrap();
        flow.go(Screen::Room).unwrap();
        assert_eq!(flow.current(), Screen::Room);
    }

    #[test]
    fn rejects_skipping_from_splash_to_play() {
        let mut flow = ScreenFlow::new();
        assert_eq!(
            flow.go(Screen::Playing),
            Err(BadTransition {
                from: Screen::Avi,
                to: Screen::Playing
            })
        );
        assert_eq!(flow.current(), Screen::Avi);
    }

    #[test]
    fn rejects_reentering_loading_from_results() {
        let mut flow = ScreenFlow::new();
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            flow.go(next).unwrap();
        }
        assert!(flow.go(Screen::Loading).is_err());
        assert!(flow.go(Screen::Playing).is_err());
    }
}
