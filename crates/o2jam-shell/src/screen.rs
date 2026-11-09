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
    /// Solo missions (`CStatePlanet`, `0x11`).
    Planet,
    /// Rankings display (`STATE_LANKING`, `0x37`, no state class).
    /// Spelling follows the shipped block name.
    Lanking,
    /// Avatar room (`CStateMyroom`, `0x6A`; bind file absent — paints
    /// at sprite anchors, never rects).
    Myroom,
    /// Item shop (`CStateShop`, `0xA4`; shared Payment backends).
    Shop,
    /// Song shop (`CStateMShop`, `0x6B`; `jsBuyMusic` bridge).
    MusicShop,
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
// Grouped by source screen to read as the client journey (see
// docs/lifecycle.md), not by destination as the lint prefers.
#[allow(
    clippy::unnested_or_patterns,
    reason = "source-grouped transition table"
)]
pub const fn can_enter(from: Screen, to: Screen) -> bool {
    use Screen::{Avi, Channel, Loading, Planet, Playing, Result, Room, Waiting};
    matches!(
        (from, to),
        (Avi, Channel)
            | (Channel, Room)
            | (Channel, Planet)
            | (Room, Waiting)
            | (Room, Channel)
            | (Planet, Channel)
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

impl Screen {
    /// Lobbies hosting the §12 side screens (channel/room/waiting).
    ///
    /// Side views (shop, planet, …) open from these screens and
    /// return to them; see `screens::side`.
    #[must_use]
    pub const fn is_side_lobby(self) -> bool {
        matches!(self, Screen::Channel | Screen::Room | Screen::Waiting)
    }
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
    ///
    /// # Errors
    ///
    /// `BadTransition` when the edge is not in the transition map.
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
    fn lobbies_host_side_screens() {
        for screen in [Screen::Channel, Screen::Room, Screen::Waiting] {
            assert!(screen.is_side_lobby(), "{screen:?}");
        }
    }

    #[test]
    fn main_flow_screens_host_no_side_screens() {
        for screen in [
            Screen::Avi,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            assert!(!screen.is_side_lobby(), "{screen:?}");
        }
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
