//! `Vec`-backed deterministic snapshot source plus the shared
//! two-phase probe gesture both Linux arms drive.

use o2jam_shell::InputSnapshot;

use super::SnapshotSource;

/// Deterministic source: replays one frozen snapshot per call in order.
/// Exhausted reads return an empty snapshot, never a repeat, so a tap
/// cannot double-fire when the harness over-polls.
#[derive(Debug, Default)]
pub struct ScriptedSource {
    frames: Vec<InputSnapshot>,
    next: usize,
}

impl ScriptedSource {
    /// Replay these snapshots in order.
    #[must_use]
    pub fn new(frames: Vec<InputSnapshot>) -> Self {
        Self { frames, next: 0 }
    }

    /// How many snapshots are still queued.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.frames.len().saturating_sub(self.next)
    }
}

impl SnapshotSource for ScriptedSource {
    fn snapshot(&mut self) -> InputSnapshot {
        let frozen = self.frames.get(self.next).cloned().unwrap_or_default();
        self.next = self.next.saturating_add(1);
        frozen
    }
}

/// The shared probe case: silent control motion, center click (phase 1:
/// the click routes), gear hover, gear click (phase 2: the game acts).
/// Both Linux arms translate these snapshots into their backend's
/// injection; the verdict reads the game input log in both arms.
#[must_use]
pub fn probe_gesture(center: (i32, i32), gear: (i32, i32)) -> ScriptedSource {
    let control = (center.0 + 37, center.1 - 23);
    ScriptedSource::new(vec![
        InputSnapshot::motion_to(control.0, control.1),
        InputSnapshot::click_at(center.0, center.1),
        InputSnapshot::motion_to(gear.0, gear.1),
        InputSnapshot::click_at(gear.0, gear.1),
    ])
}

#[cfg(test)]
mod script_tests {
    use super::*;
    use o2jam_shell::InputEdge;

    #[test]
    fn script_replays_frames_in_order_then_empties() {
        let mut source = ScriptedSource::new(vec![
            InputSnapshot::motion_to(1, 2),
            InputSnapshot::click_at(3, 4),
        ]);
        assert_eq!(source.remaining(), 2);
        assert_eq!(source.snapshot().pointer(), Some((1, 2)));
        assert_eq!(source.snapshot().pointer(), Some((3, 4)));
        assert_eq!(source.remaining(), 0);
        let drained = source.snapshot();
        assert!(drained.edges().is_empty());
        assert_eq!(drained.pointer(), None);
    }

    #[test]
    fn probe_gesture_keeps_the_tap_inside_one_freeze() {
        let mut gesture = probe_gesture((400, 300), (729, 580));
        assert_eq!(gesture.remaining(), 4);
        gesture.snapshot();
        let tap = gesture.snapshot();
        assert_eq!(
            tap.edges(),
            &[
                InputEdge::PointerDown { x: 400, y: 300 },
                InputEdge::PointerUp { x: 400, y: 300 },
            ]
        );
        gesture.snapshot();
        let gear_tap = gesture.snapshot();
        assert_eq!(
            gear_tap.edges(),
            &[
                InputEdge::PointerDown { x: 729, y: 580 },
                InputEdge::PointerUp { x: 729, y: 580 },
            ]
        );
    }

    #[test]
    fn probe_control_motion_carries_no_edges() {
        let mut gesture = probe_gesture((400, 300), (729, 580));
        let control = gesture.snapshot();
        assert!(control.edges().is_empty());
        assert_eq!(control.pointer(), Some((437, 277)));
    }
}
