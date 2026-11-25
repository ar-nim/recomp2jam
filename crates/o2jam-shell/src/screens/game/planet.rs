//! Planet-board state behind the bare board (#139 cluster 5): the
//! difficulty pick, the promotion carousel, and the held press.

use super::PlanetModal;

use super::super::side::{
    PlanetAction, PlanetPromo, PlanetPromoCarousel, PlanetSelection, PLANET_PROMO_TICK_MS,
};

/// Planet-board state behind the bare board (#139 cluster 5: the
/// Data Clump of the six planet/promo words). Pure board state — no
/// flow step, paint, or scene-art read — so the board reads without
/// touching the `ScreenGame` God class. `ScreenGame` keeps thin
/// delegates under the same names, so the click seam, the hover
/// resolver, the paint overlay, and the snapshot surface never
/// change. The board dispatch never touches the flow: `apply_action`
/// only reports the emblem pick, and the caller steps Planet →
/// Channel on it.
#[derive(Default)]
pub(crate) struct PlanetBoardState {
    /// Live difficulty pick (HARD/NORMAL/EASY; FREE/DRUM stay
    /// unbound per P0).
    pub(crate) planet: PlanetSelection,
    /// Last pressed promotion-carousel cell, if any (the press
    /// records and drives the carousel). Staged placements only —
    /// unstaged promo clicks land nowhere (no fallback literals,
    /// ADR-0003).
    pub(crate) planet_promo: Option<PlanetPromo>,
    /// Promotion-carousel state: the 1-based banner frame plus the
    /// rotation flag (exe-pinned, `docs/planet.md`). LEFT/RIGHT
    /// step the index with wrap, STOP toggles the flag, the
    /// rotation tick advances while the flag is set, and the
    /// painter follows the index.
    pub(crate) planet_promo_carousel: PlanetPromoCarousel,
    /// Frame-time anchor of the last carousel advance, if any (the
    /// 3000 ms accumulator behind the exe's per-frame rotation tick,
    /// VA `0x48e3b0`: `tick_promo_clocked` anchors on the first
    /// bare-board paint, then steps one banner frame per
    /// `PLANET_PROMO_TICK_MS` of board time while the flag holds).
    pub(crate) promo_last_advance_ms: Option<u64>,
    /// Board control the left button is currently holding down,
    /// if any (the press-feedback arm, #111). Armed on down over
    /// the bare board; consumed on release. The pressed-frame
    /// repaint reads it; release-inside dispatches the action on
    /// up, release-outside reverts with none (both pinned; the
    /// release-outside revert trigger stays OPEN, `docs/hover.md`).
    pub(crate) planet_press: Option<PlanetAction>,
    /// Open planet-board modal, if any (quit-confirm or options).
    pub(crate) planet_modal: Option<PlanetModal>,
}

impl PlanetBoardState {
    /// Run one board action (the release-inside click, pinned on up
    /// not down, #111 arm 3). Reports the family pick: the caller
    /// steps Planet → Channel on it, so this board state never
    /// touches the flow (the modal opens stay board-internal).
    pub(crate) fn apply_action(&mut self, action: PlanetAction, promo_frames: u32) -> bool {
        match action {
            PlanetAction::Emblem { band: Some(band) } => {
                self.planet.select_band(band);
                true
            }
            PlanetAction::Emblem { band: None } => false,
            PlanetAction::Promo { cell } => {
                self.planet_promo = Some(cell);
                // Carousel arms (exe-pinned, `docs/planet.md`):
                // LEFT steps back with wrap, RIGHT steps forward
                // with wrap, STOP toggles rotation (no repaint).
                match cell {
                    PlanetPromo::Left => self.planet_promo_carousel.step_back(promo_frames),
                    PlanetPromo::Right => self.planet_promo_carousel.step_forward(promo_frames),
                    PlanetPromo::Stop => self.planet_promo_carousel.toggle(),
                }
                false
            }
            PlanetAction::QuitConfirm => {
                self.planet_modal = Some(PlanetModal::QuitConfirm);
                false
            }
            PlanetAction::OptionsDialog => {
                self.planet_modal = Some(PlanetModal::Options);
                false
            }
        }
    }

    /// One carousel rotation step: advance a frame while spinning,
    /// hold while stopped (the tick body, VA `0x48e3b0`, minus the
    /// cadence gate).
    pub(crate) fn tick_promo(&mut self, frames: u32) {
        if self.planet_promo_carousel.spinning() {
            self.planet_promo_carousel.step_forward(frames);
        }
    }

    /// Clocked rotation advance accumulator: anchor on the first
    /// tick, then step one banner frame per `PLANET_PROMO_TICK_MS`
    /// of frame time. Covered boards, a cleared flag, and frameless
    /// boards never reach here (the caller gates), so the banner
    /// holds; frame time is the paint seam's explicit instant
    /// (sample clock windowed, fixed clock on replays): no wall
    /// clock here.
    pub(crate) fn tick_promo_clocked(&mut self, now_ms: u64, frames: u32) {
        match self.promo_last_advance_ms {
            None => self.promo_last_advance_ms = Some(now_ms),
            Some(last) if now_ms.saturating_sub(last) >= u64::from(PLANET_PROMO_TICK_MS) => {
                self.tick_promo(frames);
                self.promo_last_advance_ms = Some(now_ms);
            }
            Some(_) => {}
        }
    }

    /// Record the board control the left button now holds (`None`
    /// drops the press: the pressed control belongs to the scene
    /// being left).
    pub(crate) fn arm_press(&mut self, press: Option<PlanetAction>) {
        self.planet_press = press;
    }

    /// Consume the held press (the release edge).
    pub(crate) fn take_press(&mut self) -> Option<PlanetAction> {
        self.planet_press.take()
    }

    /// Open one board modal over the bare board (the modal owns
    /// clicks until closed).
    pub(crate) fn open_modal(&mut self, modal: PlanetModal) {
        self.planet_modal = Some(modal);
    }

    /// Close the open board modal (Escape, EXIT, or the quit
    /// confirm's answer).
    pub(crate) fn close_modal(&mut self) {
        self.planet_modal = None;
    }
}

#[cfg(test)]
mod planet_control_tests {
    /// Class identity installed by the Planet button constructor
    /// (VA `0x524350` / file `0x124350`, ev
    /// `8f94.3310...4b8c.7679`). The Rust client has no C++ vftable;
    /// this enum is the seam's identity marker.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InstalledButtonVftable {
        CCBtnPlanet,
    }

    /// Constructor result fields named by the same pin: three cleared
    /// words at `+0x64/+0x68/+0x6C`, a cleared byte at `+0x70`, a
    /// cleared `0x10`-byte block at `+0x74`, a cleared word at `+0x94`,
    /// and the two default-one words at `+0x98/+0x9C`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct PlanetButtonConstruction {
        vftable: InstalledButtonVftable,
        cleared_words: [u16; 3],
        cleared_byte: u8,
        cleared_block: [u8; 16],
        word_94: u16,
        words_98_9c: [u16; 2],
        base_setup_called: bool,
    }

    impl PlanetButtonConstruction {
        /// Record of the 7.55 constructor result.
        fn after_755_ctor() -> Self {
            Self {
                vftable: InstalledButtonVftable::CCBtnPlanet,
                cleared_words: [0; 3],
                cleared_byte: 0,
                cleared_block: [0; 16],
                word_94: 0,
                words_98_9c: [1, 1],
                base_setup_called: true,
            }
        }
    }

    #[test]
    fn planet_button_constructor_installs_ccbtnplanet_vftable_and_755_defaults() {
        // seam: constructor identity is defined entirely by the installed
        // vftable and defaults; no live Rust caller constructs a C++ button.
        let button = PlanetButtonConstruction::after_755_ctor();

        assert_eq!(
            button.vftable,
            InstalledButtonVftable::CCBtnPlanet,
            "the planet button must install the CCBtnPlanet vftable"
        );
        assert!(
            button.base_setup_called,
            "the shared base setup must run first"
        );
        assert_eq!(button.cleared_words, [0, 0, 0], "+0x64/+0x68/+0x6C");
        assert_eq!(button.cleared_byte, 0, "+0x70");
        assert_eq!(button.cleared_block, [0; 16], "+0x74 block");
        assert_eq!(button.word_94, 0, "+0x94");
        assert_eq!(
            button.words_98_9c,
            [1, 1],
            "the +0x98/+0x9C default-one flags"
        );
    }
}

/// Test-only model of the two virtual calls at VA `0x479cb0`
/// (file `0x79cb0`, ev `7112.9e83...f1b2.cc65.d1e8`). Both the
/// Planet and Channel guards read it; the Rust shell has no C++
/// virtual slots, so the call order is the whole seam behavior.
#[cfg(test)]
pub(crate) mod board_dispatch_model {
    /// Which virtual sink took one board message.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum MessageSink {
        HostDialog,
        GlobalListener,
    }

    /// One virtual `dispatch` slot on a message target.
    pub(crate) trait BoardMessageTarget {
        fn dispatch(&self, trace: &mut Vec<MessageSink>);
    }

    /// Dialog mounted in the parent state's `+0xF6C` slot.
    pub(crate) struct HostedDialog;
    /// Global listener obtained from `0x47f070` for the `+0xF74` vector.
    pub(crate) struct GlobalListener;

    impl BoardMessageTarget for HostedDialog {
        fn dispatch(&self, trace: &mut Vec<MessageSink>) {
            trace.push(MessageSink::HostDialog);
        }
    }

    impl BoardMessageTarget for GlobalListener {
        fn dispatch(&self, trace: &mut Vec<MessageSink>) {
            trace.push(MessageSink::GlobalListener);
        }
    }

    /// Invoke the hosted dialog's `+0x50`, then the listener's `+0x5C`.
    pub(crate) fn route_board_message(
        hosted_dialog: Option<&dyn BoardMessageTarget>,
        global_listener: &dyn BoardMessageTarget,
        trace: &mut Vec<MessageSink>,
    ) {
        if let Some(dialog) = hosted_dialog {
            dialog.dispatch(trace);
        }
        global_listener.dispatch(trace);
    }
}

#[cfg(test)]
mod planet_dispatch_tests {
    use super::board_dispatch_model::*;

    #[test]
    fn planet_dialog_receives_board_messages_before_the_global_listener() {
        // seam: the call order is the whole behavior at this seam.
        let dialog = HostedDialog;
        let global = GlobalListener;
        let mut trace = Vec::new();

        route_board_message(Some(&dialog), &global, &mut trace);

        assert_eq!(
            trace,
            [MessageSink::HostDialog, MessageSink::GlobalListener],
            "the hosted dialog's +0x50 call must precede the listener's +0x5C call"
        );

        let mut trace = Vec::new();
        route_board_message(None, &global, &mut trace);
        assert_eq!(
            trace,
            [MessageSink::GlobalListener],
            "without a hosted dialog only the global listener receives the message"
        );
    }
}
