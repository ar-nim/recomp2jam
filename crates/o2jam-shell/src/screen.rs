//! Screen state machine: the 7.55 client flow from launch to results.
//!
//! Transition map from `docs/lifecycle.md`: splash → planet (the
//! bare board) → channel (the dialog over the board) → room →
//! waiting → loading → playing → result, with retry/leave-back
//! edges. Only transitions the original performs exist here; the UI
//! of each screen comes later. Headless by design (no window needed).
//!
//! Boot correction (S0.3, owner): the splash dismiss lands on the
//! bare planet board, and a lit-emblem click opens the channel
//! dialog over it (the dialog X closes back). The earlier
//! dialog-at-boot observation came from launcher-preselected
//! sessions (`My_channel`/`My_planet`, `docs/planet.md`); that
//! preselection handling stays OPEN and is not modeled — `Channel`
//! and `Planet` are the dialog-up and bare-board phases of one
//! state (`parent+0xF64` in the exe), not two boot orders.

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
    /// Avatar room (`CStateMyroom`, `0x6A`; bound `State_Myroom.bnd`,
    /// 111 rects — the bind exists on the shipped archive; the old
    /// "absent" note is corrected 2026-11-10).
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
        (Avi, Planet)
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
    /// Scene-BGM cue this screen enters with: the `(id, repeat)` its
    /// entry passes the exe's scene-music selector (file `0x151820`
    /// = VA `0x551820`, 35 B, forwards `(id, loop)` to the bank
    /// behind `soundmgr[0x76832C] + 0x340C0`). The `id` is a
    /// one-based record over `Music/BGM.ojm` (blank entries
    /// included); [`o2jam_audio::BGM_SILENCE_ID`] (`-1`) stops the
    /// channel; `repeat` is the exe's loop flag. Every pair was
    /// re-read on our side (2026-11-11) at the caller's push site
    /// (file offset = VA − `0x400000`):
    ///
    /// | Screen | id | loop | Call site (file offset) |
    /// |---|---|---|---|
    /// | Avi | 8 | once | `0xF57D4` |
    /// | Channel / Planet | 9 | loop | `0x8DE28` |
    /// | Room | 3 | loop | `0x8EB87` |
    /// | Shop | 4 | loop | `0x987BE` |
    /// | Myroom | 5 | loop | `0xA3911` |
    /// | Waiting | 6 | loop | `0xB62C3` |
    /// | MusicShop | 10 (decimal) | loop | `0xC6E51` |
    /// | Result | `0x13` | once | `0xC1CAF` |
    ///
    /// On contradictions the exe arbitrates (`docs/coverage-audit.md`
    /// disagrees twice): the result entry pushes loop `0` (once, not
    /// loop) and the music-shop entry pushes decimal `10` (record 10
    /// names the shop track, which is also the records' semantic
    /// alignment). Result's id `0x13` names no `BGM.ojm` record (the
    /// bank holds 10 records), so it stays silent here — OPEN:
    /// identify which bank the worker holds when `0x4C1B90` cues it
    /// (`BGMwinter.ojm` is the candidate). Loading, playing, and
    /// lanking hold no pinned cue (no entry among the selector's
    /// immediate callers names them), so they cue silence; the
    /// option-preview cue `(0, 1)` exists in-exe but its PCM is
    /// likewise unresolved. The channel dialog lives inside the
    /// planet state (the `CStatePlanet` cue covers it).
    ///
    /// The windowed client drives every scene entry through this
    /// table (the `ScreenGame` entry hook); the launch client
    /// (`recomp2jam/src/main.rs`) delegates its table here so the
    /// pairs live in one place.
    #[must_use]
    pub const fn bgm_cue(self) -> (i32, bool) {
        use o2jam_audio::BGM_SILENCE_ID;
        match self {
            Screen::Avi => (8, false),
            Screen::Channel | Screen::Planet => (9, true),
            Screen::Room => (3, true),
            Screen::Shop => (4, true),
            Screen::Myroom => (5, true),
            Screen::Waiting => (6, true),
            Screen::MusicShop => (10, true),
            Screen::Result => (0x13, false),
            Screen::Loading | Screen::Playing | Screen::Lanking => (BGM_SILENCE_ID, false),
        }
    }

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
            Screen::Planet,
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
            Screen::Planet,
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
        flow.go(Screen::Planet).unwrap();
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
        flow.go(Screen::Planet).unwrap();
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
            Screen::Planet,
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

    #[test]
    fn splash_boots_into_the_bare_planet_board() {
        // S0.3 owner correction: boot shows the bare board (Planet),
        // never the dialog-up phase directly — the dialog opens only
        // through the lit-emblem edge (Planet→Channel). Launcher
        // preselection handling stays OPEN (see the module docs).
        let mut flow = ScreenFlow::new();
        assert!(flow.go(Screen::Channel).is_err());
        flow.go(Screen::Planet).unwrap();
        flow.go(Screen::Channel).unwrap();
        assert_eq!(flow.current(), Screen::Channel);
    }

    #[test]
    fn scene_bgm_cues_follow_the_pinned_table() {
        assert_eq!(Screen::Avi.bgm_cue(), (8, false));
        assert_eq!(Screen::Planet.bgm_cue(), (9, true));
        assert_eq!(Screen::Channel.bgm_cue(), (9, true));
        assert_eq!(Screen::Loading.bgm_cue(), (-1, false));
        assert_eq!(Screen::Playing.bgm_cue(), (-1, false));
    }
}
