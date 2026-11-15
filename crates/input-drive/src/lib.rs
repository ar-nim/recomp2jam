//! `input-drive` — snapshot sources for input tests (ticket #127).
//!
//! The game pulls one [`InputSnapshot`] per frame and sweeps it once
//! through [`GameLoop::ingest_snapshot`] (ADR-0013). A driver below this
//! seam produces snapshots; it never calls into the game between
//! freezes, so the forwarding gap behind #118 stays structurally
//! impossible.
//!
//! Live caller chain (both Linux arms of `input_smoke`): the harness
//! freezes one [`SnapshotSource::snapshot`] per tick, injects the frozen
//! edges through the backend (Wayland fixture wire events or XTEST into
//! a self-spawned Xvfb), the game freezes them again in
//! `WindowHandler::deliver_snapshot`, sweeps them once via
//! `GameLoop::ingest_snapshot`, then paints.
//!
//! Headless caller chain (unit tests): [`drive_one_frame`] freezes the
//! source and calls `ingest_snapshot` directly — same freeze-then-sweep
//! order, no backend.

use o2jam_shell::GameLoop;

pub use o2jam_shell::{InputEdge, InputSnapshot};

pub mod backend;
pub mod log;
pub mod script;
#[cfg(target_os = "linux")]
pub mod x11;

pub use backend::{auto_backend, select_backend, InputBackend};
pub use script::{probe_gesture, ScriptedSource};

/// One frozen input frame per harness tick. Concrete sources translate
/// one backend event stream into snapshots; each source owns its pointer
/// cache, held state, and edge queue.
pub trait SnapshotSource {
    /// Freeze and return the next snapshot.
    fn snapshot(&mut self) -> InputSnapshot;
}

/// Freeze one source frame and sweep it through the game once: the
/// headless form of the live harness chain (freeze, ingest, paint).
/// Returns the frozen snapshot for assertions.
pub fn drive_one_frame(
    source: &mut impl SnapshotSource,
    game: &mut impl GameLoop,
) -> InputSnapshot {
    let snapshot = source.snapshot();
    game.ingest_snapshot(&snapshot);
    snapshot
}

#[cfg(test)]
mod drive_tests {
    use super::*;
    use o2jam_shell::InputEdge;

    struct EdgeRecorder {
        seen: Vec<InputEdge>,
    }

    impl GameLoop for EdgeRecorder {
        fn lane_key(&mut self, _lane: usize, _down: bool) {}
        fn ingest_snapshot(&mut self, snapshot: &InputSnapshot) {
            self.seen.extend(snapshot.edges().iter().copied());
        }
        fn paint(&mut self, _frame: &mut o2jam_render::Frame<'_>) {}
    }

    #[test]
    fn driven_tap_delivers_both_edges_in_one_sweep() {
        let mut source = ScriptedSource::new(vec![InputSnapshot::click_at(100, 200)]);
        let mut game = EdgeRecorder { seen: Vec::new() };
        let frozen = drive_one_frame(&mut source, &mut game);
        assert_eq!(
            frozen.edges(),
            &[
                InputEdge::PointerDown { x: 100, y: 200 },
                InputEdge::PointerUp { x: 100, y: 200 },
            ]
        );
        assert_eq!(game.seen, frozen.edges());
    }

    #[test]
    fn driven_frames_arrive_in_script_order() {
        let mut source = ScriptedSource::new(vec![
            InputSnapshot::motion_to(10, 20),
            InputSnapshot::click_at(30, 40),
        ]);
        let mut game = EdgeRecorder { seen: Vec::new() };
        let first = drive_one_frame(&mut source, &mut game);
        let second = drive_one_frame(&mut source, &mut game);
        assert_eq!(first.pointer(), Some((10, 20)));
        assert_eq!(second.pointer(), Some((30, 40)));
        assert_eq!(game.seen.len(), 2);
    }
}
