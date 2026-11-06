//! Skill-ring lane effects: mirror, shuffle, per-measure reshuffle.
//!
//! How the 7.55 client applies them (behavior paraphrased from the
//! executable; never original code):
//! - The session modifier byte lives at VA `0x7789e4` (file `0x3789e4`).
//!   It mirrors room field `+0x20d`, which the room message `0x24200002`
//!   handler syncs over the network (`FUN_00436360`, file `0x36360`,
//!   dossier `ev_6838819a…`). Session defaults (byte = 2) are written by
//!   `FUN_00495110` (file `0x95110`, dossier `ev_5b520d70…`).
//! - Lane-map setup `FUN_00571c30` (file `0x171c30`, dossier
//!   `ev_41695a9b…`) seeds the CRT generator (`_srand` at VA `0x5f7229`,
//!   file `0x1f7229`) and builds the source→destination lane table:
//!   full reversal for mirror, [`shuffled_full`] for the N-lane ring,
//!   [`shuffled_narrow`] for the 3-lane ring, identity otherwise.
//!   The shuffle width (`+0x82c`) is initialized to 7 at file `0x1717e3`.
//! - The gameplay planner `FUN_00574330` (file `0x174330`, dossier
//!   `ev_d880e00e…`) rebuilds the table at every plain measure and once
//!   per hold-note span, then spawns notes through it.
//!
//! Which ring item selects which byte value is decided room-side and
//! stays OPEN; only the byte semantics above are verified in-exe. The
//! CRT generator itself is not reproduced here — builders take any
//! `next_random` word source, so tests pin the swap algorithm with a
//! stub instead of depending on CRT internals.

use super::Lane;
use crate::chart::LevelChart;
use o2jam_file::OJN_TICKS_PER_MEASURE;

/// Lane count the N-lane shuffle width starts at (7-key play).
pub const FULL_WIDTH_LANES: usize = 7;
/// Slot count of the narrow shuffle table (lanes 0..4, slot 1 unused).
pub const NARROW_SLOT_COUNT: usize = 4;
/// Narrow-table slot the builders never touch.
pub const NARROW_UNUSED_SLOT: usize = 1;
/// Modulus the narrow builder draws (`rand() % 3`).
pub const NARROW_DRAW_MODULUS: u32 = 3;

/// Source→destination lane table: `destination(source)` selects which
/// lane object plays a note read from `source`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneMap {
    destinations: Vec<Lane>,
}

impl LaneMap {
    /// Straight-through map: every source plays its own lane.
    #[must_use]
    pub fn identity(width: usize) -> Self {
        Self {
            destinations: (0..width as Lane).collect(),
        }
    }

    /// Mirror map: lane `i` plays lane `width - 1 - i` (left-right flip,
    /// no randomness — the in-exe mirror path).
    #[must_use]
    pub fn mirrored(width: usize) -> Self {
        let width = width as Lane;
        Self {
            destinations: (0..width).map(|source| width - 1 - source).collect(),
        }
    }

    /// N-lane shuffle: identity fill, then one `rand() % width` swap per
    /// lane (`FUN_00573940`, file `0x173940`, dossier `ev_e65f8622…`).
    /// An empty width yields an empty map (the original divides by the
    /// width, so callers must pass the real lane count).
    pub fn shuffled_full(width: usize, next_random: &mut impl FnMut() -> u32) -> Self {
        let mut map = Self::identity(width);
        for source in 0..width {
            let drawn = (next_random() % width as u32) as usize;
            map.destinations.swap(source, drawn);
        }
        map
    }

    /// 3-lane shuffle over [`NARROW_SLOT_COUNT`] slots, skipping
    /// [`NARROW_UNUSED_SLOT`] (`FUN_00573a10`, file `0x173a10`, dossier
    /// `ev_327cea5d…`): each touched slot draws `rand() % 3`; a drawn 1
    /// swaps slot 2 with slot `i % 3`, any other draw swaps slot `drawn`
    /// with slot `i % width`.
    pub fn shuffled_narrow(width: usize, next_random: &mut impl FnMut() -> u32) -> Self {
        let mut map = Self::identity(NARROW_SLOT_COUNT);
        for source in 0..NARROW_SLOT_COUNT {
            if source == NARROW_UNUSED_SLOT {
                continue;
            }
            let drawn = next_random() % NARROW_DRAW_MODULUS;
            if drawn == NARROW_UNUSED_SLOT as u32 {
                let other = source % NARROW_DRAW_MODULUS as usize;
                map.destinations.swap(2, other);
            } else {
                let other = source % width;
                map.destinations.swap(drawn as usize, other);
            }
        }
        map
    }

    /// Destination lane for a note read from `source`, or `None` when
    /// `source` is outside the table.
    #[must_use]
    pub fn destination(&self, source: Lane) -> Option<Lane> {
        self.destinations.get(usize::from(source)).copied()
    }

    /// Slot count of this table.
    #[must_use]
    pub fn width(&self) -> usize {
        self.destinations.len()
    }

    /// Spawn a chart through this map: every hit on a covered source
    /// lane moves to its destination lane (backing notes are untouched).
    pub fn apply_to_chart(&self, chart: &mut LevelChart) {
        for hit in &mut chart.hits {
            if let Some(destination) = self.destination(hit.lane) {
                hit.lane = destination;
            }
        }
        chart.hits.sort_by_key(|hit| hit.tick);
    }
}

/// Measure index of an absolute tick (192 ticks per measure).
#[must_use]
pub fn measure_index_of_tick(tick: u64) -> u64 {
    tick / u64::from(OJN_TICKS_PER_MEASURE)
}

/// Per-measure rebuild rule for the panic-style ring.
///
/// Mirrors the planner (`FUN_00574330`, file `0x174330`, dossier
/// `ev_d880e00e…`): the lane table is rebuilt at every measure without
/// hold notes and once at the start of each hold-note span, so notes
/// inside one hold span keep a single map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MeasureReshuffle {
    hold_span_open: bool,
}

impl MeasureReshuffle {
    /// Whether the next measure needs a freshly built table.
    pub fn should_rebuild_map(&mut self, measure_has_hold: bool) -> bool {
        let rebuild = !measure_has_hold || !self.hold_span_open;
        self.hold_span_open = measure_has_hold;
        rebuild
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{TempoMap, TimedHit};

    fn stub_random(words: Vec<u32>) -> impl FnMut() -> u32 {
        let mut words = words.into_iter();
        move || words.next().expect("stub randomness exhausted")
    }

    fn empty_tempo() -> TempoMap {
        TempoMap {
            base_bpm: 120.0,
            changes: Vec::new(),
            meters: Vec::new(),
        }
    }

    #[test]
    fn mirror_reverses_seven_lanes() {
        let map = LaneMap::mirrored(FULL_WIDTH_LANES);
        assert_eq!(
            (0..7).map(|lane| map.destination(lane)).collect::<Vec<_>>(),
            [
                Some(6),
                Some(5),
                Some(4),
                Some(3),
                Some(2),
                Some(1),
                Some(0)
            ]
        );
    }

    #[test]
    fn mirror_is_its_own_inverse() {
        let map = LaneMap::mirrored(FULL_WIDTH_LANES);
        for lane in 0..FULL_WIDTH_LANES as Lane {
            let there = map.destination(lane).expect("covered lane");
            assert_eq!(map.destination(there), Some(lane));
        }
    }

    #[test]
    fn full_shuffle_keeps_a_permutation() {
        let mut random = stub_random(vec![3, 1, 4, 1, 5, 9, 2]);
        let map = LaneMap::shuffled_full(FULL_WIDTH_LANES, &mut random);
        let mut destinations: Vec<Lane> = (0..7)
            .map(|lane| map.destination(lane).expect("covered lane"))
            .collect();
        destinations.sort_unstable();
        assert_eq!(destinations, [0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn full_shuffle_matches_fixed_draw_golden() {
        // Draws are `word % 7` per lane 0..6: 3,1,4,1,5,2,6.
        let mut random = stub_random(vec![3, 1, 4, 8, 5, 9, 6]);
        let map = LaneMap::shuffled_full(FULL_WIDTH_LANES, &mut random);
        let destinations: Vec<Lane> = (0..7)
            .map(|lane| map.destination(lane).expect("covered lane"))
            .collect();
        assert_eq!(destinations, vec![3, 0, 2, 1, 5, 4, 6]);
    }

    #[test]
    fn narrow_shuffle_leaves_slot_one_untouched() {
        let mut random = stub_random(vec![1, 2, 0, 1]);
        let map = LaneMap::shuffled_narrow(FULL_WIDTH_LANES, &mut random);
        assert_eq!(map.width(), NARROW_SLOT_COUNT);
        assert_eq!(map.destination(NARROW_UNUSED_SLOT as Lane), Some(1));
    }

    #[test]
    fn narrow_shuffle_stays_a_permutation_of_four_slots() {
        let mut random = stub_random(vec![2, 0, 1, 2, 0, 1]);
        let map = LaneMap::shuffled_narrow(FULL_WIDTH_LANES, &mut random);
        let mut destinations: Vec<Lane> = (0..4)
            .map(|lane| map.destination(lane).expect("covered slot"))
            .collect();
        destinations.sort_unstable();
        assert_eq!(destinations, [0, 1, 2, 3]);
    }

    #[test]
    fn destination_returns_none_outside_the_table() {
        let map = LaneMap::identity(4);
        assert_eq!(map.destination(4), None);
    }

    #[test]
    fn remapped_chart_moves_hits_through_the_map() {
        let mut chart = LevelChart {
            hits: vec![
                TimedHit {
                    lane: 0,
                    tick: 0,
                    sample_id: 1,
                    flags: 0,
                },
                TimedHit {
                    lane: 6,
                    tick: 192,
                    sample_id: 2,
                    flags: 0,
                },
            ],
            backing: Vec::new(),
            tempo: empty_tempo(),
        };
        LaneMap::mirrored(FULL_WIDTH_LANES).apply_to_chart(&mut chart);
        assert_eq!(chart.hits[0].lane, 6);
        assert_eq!(chart.hits[1].lane, 0);
    }

    #[test]
    fn remapped_chart_keeps_uncovered_lanes_in_place() {
        let mut chart = LevelChart {
            hits: vec![TimedHit {
                lane: 6,
                tick: 0,
                sample_id: 1,
                flags: 0,
            }],
            backing: Vec::new(),
            tempo: empty_tempo(),
        };
        LaneMap::identity(4).apply_to_chart(&mut chart);
        assert_eq!(chart.hits[0].lane, 6);
    }

    #[test]
    fn measure_index_groups_ticks_by_192() {
        assert_eq!(measure_index_of_tick(0), 0);
        assert_eq!(measure_index_of_tick(191), 0);
        assert_eq!(measure_index_of_tick(192), 1);
        assert_eq!(measure_index_of_tick(385), 2);
    }

    #[test]
    fn reshuffle_rebuilds_every_plain_measure() {
        let mut reshuffle = MeasureReshuffle::default();
        assert!(reshuffle.should_rebuild_map(false));
        assert!(reshuffle.should_rebuild_map(false));
        assert!(reshuffle.should_rebuild_map(false));
    }

    #[test]
    fn reshuffle_shares_one_map_across_a_hold_span() {
        let mut reshuffle = MeasureReshuffle::default();
        assert!(reshuffle.should_rebuild_map(true));
        assert!(!reshuffle.should_rebuild_map(true));
        assert!(!reshuffle.should_rebuild_map(true));
        assert!(reshuffle.should_rebuild_map(false));
    }
}
