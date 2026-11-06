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
    /// with slot `i % width`. A zero width yields the identity map
    /// instead of dividing by zero.
    pub fn shuffled_narrow(width: usize, next_random: &mut impl FnMut() -> u32) -> Self {
        let mut map = Self::identity(NARROW_SLOT_COUNT);
        if width == 0 {
            return map;
        }
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

/// Session modifier byte default (VA `0x7789e4`), written by
/// `FUN_00495110` (file `0x95110`): the N-lane shuffle builder.
///
/// The room message `0x24200002` handler mirrors room field `+0x20d`
/// into this byte; value 2 takes a player-count gate in one room path.
pub const MODIFIER_BYTE_DEFAULT: u8 = 2;
/// Modifier byte building the N-lane swap table (`FUN_00573940`).
pub const MODIFIER_BYTE_FULL_SHUFFLE: u8 = 2;
/// Modifier byte building the 3-lane table (`FUN_00573a10`).
pub const MODIFIER_BYTE_NARROW_SHUFFLE: u8 = 0;

/// Effect ring from the avatar-shop catalog (`Itemdata_Korea.dat` in
/// `Image/avatar.opa`): mirror flips left-right, random shuffles the
/// whole chart, hidden covers the bottom fifth, sudden shows only it,
/// dark shows only the middle fifth, reverse-dark covers the middle
/// fifth, and panic (super-random) re-randomises every measure.
///
/// Which ring item selects which modifier byte is translated
/// room-side and stays OPEN; only the lane builders for bytes 0 and 2
/// plus the mirror reversal are verified in-exe, so this enum carries
/// the per-ring lane behavior and lets song setup hang the matching
/// item-text curtain without claiming wire values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRing {
    /// Left-right flip, no randomness.
    Mirror,
    /// One full shuffle for the whole chart (item text).
    Random,
    /// Bottom fifth covered (lanes untouched).
    Hidden,
    /// Only the bottom fifth visible (lanes untouched).
    Sudden,
    /// Only the middle fifth visible (lanes untouched).
    Dark,
    /// Middle fifth covered (lanes untouched).
    ReverseDark,
    /// Fresh full shuffle every plain measure (item text).
    Panic,
}

impl SkillRing {
    /// Every effect ring, in catalog order.
    #[must_use]
    pub fn all() -> [Self; 7] {
        [
            Self::Mirror,
            Self::Random,
            Self::Hidden,
            Self::Sudden,
            Self::Dark,
            Self::ReverseDark,
            Self::Panic,
        ]
    }

    /// Lane table for this ring: mirror reversal, the verified N-lane
    /// swap builder for random/panic, identity for the curtain rings.
    ///
    /// Panic shares random's builder; the per-measure rebuild (see
    /// [`Self::reshuffles_each_measure`]) is what distinguishes it.
    /// The planner re-randomises both shuffle bytes in-exe, so the
    /// item text's "whole chart" versus "every measure" split is
    /// not corroborated there — it is kept here as the catalog claims.
    #[must_use]
    pub fn build_lane_map(self, width: usize, next_random: &mut impl FnMut() -> u32) -> LaneMap {
        match self {
            Self::Mirror => LaneMap::mirrored(width),
            Self::Random | Self::Panic => LaneMap::shuffled_full(width, next_random),
            Self::Hidden | Self::Sudden | Self::Dark | Self::ReverseDark => {
                LaneMap::identity(width)
            }
        }
    }

    /// Only panic rebuilds the table every plain measure (and once per
    /// hold-note span, via [`MeasureReshuffle`]).
    #[must_use]
    pub fn reshuffles_each_measure(self) -> bool {
        matches!(self, Self::Panic)
    }
}

/// Lane table for one raw session modifier byte (VA `0x7789e4`), the
/// verified song-setup branch (`FUN_00571c30`, file `0x171c30`): byte 2
/// builds the N-lane swap table, byte 0 the 3-lane table, anything else
/// plays straight through.
///
/// The mirror reversal's selector byte is room-side OPEN, so mirror is
/// worn through [`SkillRing::Mirror`] instead of here.
#[must_use]
pub fn lane_map_for_modifier_byte(
    modifier: u8,
    width: usize,
    next_random: &mut impl FnMut() -> u32,
) -> LaneMap {
    match modifier {
        MODIFIER_BYTE_FULL_SHUFFLE => LaneMap::shuffled_full(width, next_random),
        MODIFIER_BYTE_NARROW_SHUFFLE => LaneMap::shuffled_narrow(width, next_random),
        _ => LaneMap::identity(width),
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
    fn narrow_shuffle_with_zero_width_stays_identity() {
        let mut random = stub_random(vec![]);
        let map = LaneMap::shuffled_narrow(0, &mut random);
        assert_eq!(map, LaneMap::identity(NARROW_SLOT_COUNT));
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
    fn mirror_ring_reverses_without_randomness() {
        let mut never_called = || -> u32 { panic!("mirror draws no randomness") };
        let map = SkillRing::Mirror.build_lane_map(FULL_WIDTH_LANES, &mut never_called);
        assert_eq!(map, LaneMap::mirrored(FULL_WIDTH_LANES));
        assert!(!SkillRing::Mirror.reshuffles_each_measure());
    }

    #[test]
    fn random_ring_shuffles_once_for_the_chart() {
        let mut random = stub_random(vec![3, 1, 4, 8, 5, 9, 6]);
        let map = SkillRing::Random.build_lane_map(FULL_WIDTH_LANES, &mut random);
        let destinations: Vec<Lane> = (0..7)
            .map(|lane| map.destination(lane).expect("covered lane"))
            .collect();
        assert_eq!(destinations, vec![3, 0, 2, 1, 5, 4, 6]);
        assert!(!SkillRing::Random.reshuffles_each_measure());
    }

    #[test]
    fn hidden_ring_leaves_lanes_straight_through() {
        let mut random = stub_random(vec![]);
        let map = SkillRing::Hidden.build_lane_map(FULL_WIDTH_LANES, &mut random);
        assert_eq!(map, LaneMap::identity(FULL_WIDTH_LANES));
        assert!(!SkillRing::Hidden.reshuffles_each_measure());
    }

    #[test]
    fn sudden_ring_leaves_lanes_straight_through() {
        let mut random = stub_random(vec![]);
        let map = SkillRing::Sudden.build_lane_map(FULL_WIDTH_LANES, &mut random);
        assert_eq!(map, LaneMap::identity(FULL_WIDTH_LANES));
        assert!(!SkillRing::Sudden.reshuffles_each_measure());
    }

    #[test]
    fn dark_ring_leaves_lanes_straight_through() {
        let mut random = stub_random(vec![]);
        let map = SkillRing::Dark.build_lane_map(FULL_WIDTH_LANES, &mut random);
        assert_eq!(map, LaneMap::identity(FULL_WIDTH_LANES));
        assert!(!SkillRing::Dark.reshuffles_each_measure());
    }

    #[test]
    fn reverse_dark_ring_leaves_lanes_straight_through() {
        let mut random = stub_random(vec![]);
        let map = SkillRing::ReverseDark.build_lane_map(FULL_WIDTH_LANES, &mut random);
        assert_eq!(map, LaneMap::identity(FULL_WIDTH_LANES));
        assert!(!SkillRing::ReverseDark.reshuffles_each_measure());
    }

    #[test]
    fn panic_ring_rebuilds_a_fresh_shuffle_each_measure() {
        assert!(SkillRing::Panic.reshuffles_each_measure());
        let mut first = stub_random(vec![3, 1, 4, 8, 5, 9, 6]);
        let mut second = stub_random(vec![6, 6, 6, 6, 6, 6, 0]);
        let one = SkillRing::Panic.build_lane_map(FULL_WIDTH_LANES, &mut first);
        let two = SkillRing::Panic.build_lane_map(FULL_WIDTH_LANES, &mut second);
        assert_ne!(one, two);
    }

    #[test]
    fn modifier_byte_two_builds_the_full_shuffle() {
        let mut random = stub_random(vec![3, 1, 4, 8, 5, 9, 6]);
        let map =
            lane_map_for_modifier_byte(MODIFIER_BYTE_FULL_SHUFFLE, FULL_WIDTH_LANES, &mut random);
        assert_eq!(map.width(), FULL_WIDTH_LANES);
        let destinations: Vec<Lane> = (0..7)
            .map(|lane| map.destination(lane).expect("covered lane"))
            .collect();
        assert_eq!(destinations, vec![3, 0, 2, 1, 5, 4, 6]);
    }

    #[test]
    fn modifier_byte_zero_builds_the_narrow_table() {
        let mut random = stub_random(vec![2, 0, 1]);
        let map =
            lane_map_for_modifier_byte(MODIFIER_BYTE_NARROW_SHUFFLE, FULL_WIDTH_LANES, &mut random);
        assert_eq!(map.width(), NARROW_SLOT_COUNT);
        assert_eq!(map.destination(NARROW_UNUSED_SLOT as Lane), Some(1));
    }

    #[test]
    fn unknown_modifier_byte_plays_straight_through() {
        let mut random = stub_random(vec![]);
        let map = lane_map_for_modifier_byte(9, FULL_WIDTH_LANES, &mut random);
        assert_eq!(map, LaneMap::identity(FULL_WIDTH_LANES));
    }

    #[test]
    fn every_ring_lists_exactly_once_in_catalog_order() {
        assert_eq!(
            SkillRing::all(),
            [
                SkillRing::Mirror,
                SkillRing::Random,
                SkillRing::Hidden,
                SkillRing::Sudden,
                SkillRing::Dark,
                SkillRing::ReverseDark,
                SkillRing::Panic,
            ]
        );
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
