//! Chart-timed key-down plan for a replay (`E-864`, #653).
//!
//! The plan is derived from the parsed level's own hit onsets. It
//! never borrows an incidental capture's key timeline. Note payload
//! flags stay opaque ([`crate::chart`]): the 7.55 hold semantics are
//! unverified and the shell does not route key-up to a judgement, so
//! every playable onset is a tap. Adding hold tails needs that seam
//! first (OPEN, #625).

use crate::{chart::LevelChart, Lane};

/// One key-down in a replay, stamped with its intended song position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartPress {
    /// Zero-based order in the replay.
    pub sequence: usize,
    /// Lane 0..=6 (`kind - 2` in the source chart).
    pub lane: Lane,
    /// Intended song position in ms, from the chart tempo map.
    pub at_ms: u64,
    /// Key-sound sample id carried by the chart note.
    pub sample_id: u32,
}

/// A whole difficulty level as an ordered key-down plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartReplay {
    song_id: u16,
    level: u8,
    presses: Vec<ChartPress>,
}

impl ChartReplay {
    /// Build a replay from one parsed difficulty chart.
    #[must_use]
    pub fn new(song_id: u16, level: u8, chart: &LevelChart) -> Self {
        let mut presses: Vec<ChartPress> = chart
            .hit_onsets_ms()
            .into_iter()
            .map(|(lane, at_ms, sample_id)| ChartPress {
                sequence: 0,
                lane,
                at_ms,
                sample_id,
            })
            .collect();
        presses.sort_by_key(|press| (press.at_ms, press.lane, press.sample_id));
        for (sequence, press) in presses.iter_mut().enumerate() {
            press.sequence = sequence;
        }
        Self {
            song_id,
            level,
            presses,
        }
    }

    /// Song whose chart produced the plan.
    #[must_use]
    pub fn song_id(&self) -> u16 {
        self.song_id
    }

    /// Difficulty slice index (0 Easy, 1 Normal, 2 Hard).
    #[must_use]
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Planned key-downs, ordered by intended song position.
    #[must_use]
    pub fn presses(&self) -> &[ChartPress] {
        &self.presses
    }

    /// Number of playable notes in the source chart.
    #[must_use]
    pub fn note_count(&self) -> usize {
        self.presses.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{BackingNote, TempoMap, TimedHit};

    fn synthetic_chart() -> LevelChart {
        LevelChart {
            hits: vec![
                TimedHit {
                    lane: 4,
                    tick: 192,
                    sample_id: 7,
                    flags: 0,
                },
                TimedHit {
                    lane: 1,
                    tick: 0,
                    sample_id: 3,
                    flags: 0,
                },
            ],
            backing: vec![BackingNote {
                tick: 0,
                sample_id: 9,
                flags: 0,
            }],
            tempo: TempoMap {
                base_bpm: 120.0,
                changes: Vec::new(),
                meters: Vec::new(),
            },
        }
    }

    #[test]
    fn replay_uses_chart_onsets_and_ignores_backing_notes() {
        let replay = ChartReplay::new(864, 1, &synthetic_chart());
        assert_eq!(replay.note_count(), 2);
        assert_eq!(
            replay.presses(),
            &[
                ChartPress {
                    sequence: 0,
                    lane: 1,
                    at_ms: 0,
                    sample_id: 3,
                },
                ChartPress {
                    sequence: 1,
                    lane: 4,
                    at_ms: 2000,
                    sample_id: 7,
                },
            ]
        );
    }

    #[test]
    fn replay_keeps_the_chart_identity_for_evidence() {
        let replay = ChartReplay::new(864, 1, &synthetic_chart());
        assert_eq!(replay.song_id(), 864);
        assert_eq!(replay.level(), 1);
    }
}
