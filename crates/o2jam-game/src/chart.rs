//! Chart assembly: raw OJN level events become timed lane hits,
//! backing-track notes, and a tempo map.
//!
//! Mapping verified against all 14 shipped 7.55 charts
//! (`docs/file-formats/ojn.md`): kinds 2-8 are lanes (`lane = kind -
//! 2`), kinds 9+ are autoplay/BGM, kinds 0/1 carry `f32` BPM data, and
//! nonzero values store `sample_id + 1` with flags in the high word.
//! Per-note flag semantics (hold/expression) are UNVERIFIED: flags are
//! carried opaquely and every flagged note is emitted as a tap.

use o2jam_file::{
    decode_note_value, ojn_lane_of_kind, ojn_sub_position, OjnEvent, OJN_TICKS_PER_MEASURE,
};
use serde::{Deserialize, Serialize};

use super::Lane;

/// Beats per measure when no kind-0 meter event overrides it.
const DEFAULT_BEATS_PER_MEASURE: f32 = 4.0;

/// One playable hit on a lane, at an absolute song position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedHit {
    /// Lane 0..=6 (`kind - 2`).
    pub lane: Lane,
    /// Absolute position in 192nds of a measure from song start.
    pub tick: u64,
    /// Key-sound sample id (`value - 1`; UNVERIFIED dangling refs
    /// possible where the `.ojm` pruned a sample).
    pub sample_id: u32,
    /// Raw high word of the note value (hold/expression — UNVERIFIED).
    pub flags: u16,
}

/// One automatically-played backing note (kinds 9+), same encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackingNote {
    /// Absolute position in 192nds of a measure from song start.
    pub tick: u64,
    /// Key-sound sample id (`value - 1`).
    pub sample_id: u32,
    /// Raw high word of the note value (UNVERIFIED).
    pub flags: u16,
}

/// A tempo override starting at an absolute tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TempoChange {
    /// Absolute tick (192nds) the BPM applies from.
    pub tick: u64,
    /// New tempo in BPM.
    pub bpm: f32,
}

/// A meter override: beats in the measure starting at a tick.
///
/// From kind-0 single-float events (e.g. `0.75` = 3/4 time on
/// o2ma769). TENTATIVE: only one float observed per song.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeterChange {
    /// Absolute tick (192nds) of the measure start.
    pub tick: u64,
    /// Beats in that measure.
    pub beats: f32,
}

/// Song tempo over time: header BPM plus kind-0/1 overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempoMap {
    /// Header tempo (`f32` at +16).
    pub base_bpm: f32,
    /// BPM overrides in tick order.
    pub changes: Vec<TempoChange>,
    /// Meter overrides in tick order.
    pub meters: Vec<MeterChange>,
}

impl TempoMap {
    /// Tempo active at an absolute tick.
    #[must_use]
    pub fn bpm_at(&self, tick: u64) -> f32 {
        let mut bpm = self.base_bpm;
        for change in &self.changes {
            if change.tick <= tick {
                bpm = change.bpm;
            } else {
                break;
            }
        }
        bpm
    }

    /// Beats in the measure containing an absolute tick.
    #[must_use]
    pub fn beats_at(&self, tick: u64) -> f32 {
        let mut beats = DEFAULT_BEATS_PER_MEASURE;
        for meter in &self.meters {
            if meter.tick <= tick {
                beats = meter.beats;
            } else {
                break;
            }
        }
        beats
    }

    /// Song time of an absolute tick in whole milliseconds.
    ///
    /// Accumulates whole measures with `beats * 60000 / bpm`, matching
    /// the in-exe `240000 / BPM` per-measure accumulator
    /// (`FUN_00571d60` dossier `ev_c0325c85…`; 240000 = 4 beats in ms
    /// at 1 BPM). Ticks are 192nds of a measure
    /// ([`OJN_TICKS_PER_MEASURE`]).
    #[must_use]
    pub fn ms_at_tick(&self, tick: u64) -> u64 {
        let ticks_per_measure = u64::from(OJN_TICKS_PER_MEASURE);
        let mut ms = 0.0f64;
        let mut measure_start = 0u64;
        while measure_start + ticks_per_measure <= tick {
            ms += f64::from(self.beats_at(measure_start)) * 60_000.0
                / f64::from(self.bpm_at(measure_start));
            measure_start += ticks_per_measure;
        }
        let into_measure = tick - measure_start;
        ms += f64::from(self.beats_at(measure_start)) * (into_measure as f64) * 60_000.0
            / ((ticks_per_measure as f64) * f64::from(self.bpm_at(measure_start)));
        ms.round() as u64
    }
}

/// One difficulty level as timed notes: playable hits, backing notes,
/// and the tempo map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelChart {
    /// Playable lane hits, sorted by tick.
    pub hits: Vec<TimedHit>,
    /// Autoplay/backing notes (kinds 9+), sorted by tick.
    pub backing: Vec<BackingNote>,
    /// Tempo/meter map for tick-to-time conversion.
    pub tempo: TempoMap,
}

impl LevelChart {
    /// Playable hit count (mirrors header +40/+44/+48 on real charts).
    #[must_use]
    pub fn hit_count(&self) -> usize {
        self.hits.len()
    }

    /// Total note count incl. backing (mirrors header +28/+32/+36).
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.hits.len() + self.backing.len()
    }

    /// Playable onsets as `(lane, ms, sample_id)` for autoplay parity.
    #[must_use]
    pub fn hit_onsets_ms(&self) -> Vec<(Lane, u64, u32)> {
        self.hits
            .iter()
            .map(|hit| (hit.lane, self.tempo.ms_at_tick(hit.tick), hit.sample_id))
            .collect()
    }
}

/// Absolute tick of a payload slot: measure base plus subdivision offset.
fn absolute_tick(measure: i32, sub_index: usize, sub_count: usize) -> Option<u64> {
    if measure < 0 {
        return None;
    }
    (measure as u64)
        .checked_mul(u64::from(OJN_TICKS_PER_MEASURE))?
        .checked_add(u64::from(ojn_sub_position(sub_index, sub_count)?))
}

/// Collect kind-0/1 tempo and meter overrides from raw events.
fn collect_tempo(base_bpm: f32, events: &[OjnEvent]) -> TempoMap {
    let mut changes = Vec::new();
    let mut meters = Vec::new();
    for event in events {
        let measure_tick = (event.measure.max(0) as u64) * u64::from(OJN_TICKS_PER_MEASURE);
        match event.kind {
            0 => {
                for raw in &event.values {
                    meters.push(MeterChange {
                        tick: measure_tick,
                        beats: f32::from_le_bytes(raw.to_le_bytes()),
                    });
                }
            }
            1 => {
                for (i, raw) in event.values.iter().enumerate() {
                    let bpm = f32::from_le_bytes(raw.to_le_bytes());
                    if bpm > 0.0 {
                        let slots = event.values.len().max(1) as u32;
                        let offset =
                            (i as u64) * u64::from(OJN_TICKS_PER_MEASURE) / u64::from(slots);
                        changes.push(TempoChange {
                            tick: measure_tick + offset,
                            bpm,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    changes.sort_by_key(|change| change.tick);
    meters.sort_by_key(|meter| meter.tick);
    TempoMap {
        base_bpm,
        changes,
        meters,
    }
}

/// Build a timed level chart from decoded events plus header tempo.
///
/// Kinds 2-8 with nonzero values become [`TimedHit`]s; kinds 9+
/// become [`BackingNote`]s; kinds 0/1 feed the [`TempoMap`].
/// Flagged notes are emitted as taps (hold semantics UNVERIFIED).
#[must_use]
pub fn build_level_chart(header_bpm: f32, events: &[OjnEvent]) -> LevelChart {
    let mut hits = Vec::new();
    let mut backing = Vec::new();
    for event in events {
        for (i, raw) in event.values.iter().enumerate() {
            let Some(value) = decode_note_value(*raw) else {
                continue;
            };
            let Some(tick) = absolute_tick(event.measure, i, event.values.len()) else {
                continue;
            };
            if let Some(lane) = ojn_lane_of_kind(event.kind) {
                hits.push(TimedHit {
                    lane,
                    tick,
                    sample_id: value.sample_id,
                    flags: value.flags,
                });
            } else if event.kind >= 9 {
                backing.push(BackingNote {
                    tick,
                    sample_id: value.sample_id,
                    flags: value.flags,
                });
            }
        }
    }
    hits.sort_by_key(|hit| (hit.tick, hit.lane));
    backing.sort_by_key(|note| note.tick);
    LevelChart {
        hits,
        backing,
        tempo: collect_tempo(header_bpm, events),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use o2jam_file::OjnEvent;

    /// Synthetic two-measure level: lane taps, a tempo lift, a meter
    /// cut, and a flagged backing note.
    fn synthetic_events() -> Vec<OjnEvent> {
        vec![
            OjnEvent {
                measure: 0,
                kind: 2,
                values: vec![3, 0, 0, 0],
            },
            OjnEvent {
                measure: 0,
                kind: 4,
                values: vec![0, 0, 5, 0],
            },
            OjnEvent {
                measure: 1,
                kind: 1,
                values: vec![200f32.to_bits(), 0, 0, 0],
            },
            OjnEvent {
                measure: 1,
                kind: 0,
                values: vec![0.75f32.to_bits()],
            },
            OjnEvent {
                measure: 1,
                kind: 10,
                values: vec![0x00D0_0002, 0],
            },
        ]
    }

    #[test]
    fn build_level_chart_splits_lanes_from_backing() {
        let chart = build_level_chart(100.0, &synthetic_events());
        assert_eq!(chart.hit_count(), 2);
        assert_eq!(chart.total_count(), 3);
        assert_eq!(
            chart.hits[0],
            TimedHit {
                lane: 0,
                tick: 0,
                sample_id: 2,
                flags: 0,
            }
        );
        assert_eq!(chart.hits[1].lane, 2);
        assert_eq!(chart.hits[1].tick, 2 * 48);
        assert_eq!(chart.hits[1].sample_id, 4);
        assert_eq!(
            chart.backing[0],
            BackingNote {
                tick: 192,
                sample_id: 1,
                flags: 0xD0,
            }
        );
    }

    #[test]
    fn tempo_map_applies_bpm_and_meter_overrides_in_order() {
        let chart = build_level_chart(100.0, &synthetic_events());
        assert_eq!(chart.tempo.bpm_at(0), 100.0);
        assert_eq!(chart.tempo.bpm_at(192), 200.0);
        assert_eq!(chart.tempo.beats_at(0), 4.0);
        assert_eq!(chart.tempo.beats_at(192), 0.75);
    }

    #[test]
    fn ms_at_tick_accumulates_whole_measures_first() {
        let tempo = TempoMap {
            base_bpm: 120.0,
            changes: Vec::new(),
            meters: Vec::new(),
        };
        assert_eq!(tempo.ms_at_tick(0), 0);
        assert_eq!(tempo.ms_at_tick(192), 2000);
        assert_eq!(tempo.ms_at_tick(768), 8000);
        assert_eq!(tempo.ms_at_tick(96), 1000);
    }

    #[test]
    fn hit_onsets_convert_ticks_through_tempo_map() {
        let chart = build_level_chart(120.0, &synthetic_events());
        let onsets = chart.hit_onsets_ms();
        assert_eq!(onsets[0], (0, 0, 2));
        assert_eq!(onsets[1].0, 2);
        assert!(onsets[1].1 > 0);
    }

    #[test]
    fn build_level_chart_ignores_empty_and_unaligned_slots() {
        let events = vec![
            OjnEvent {
                measure: -1,
                kind: 2,
                values: vec![3],
            },
            OjnEvent {
                measure: 0,
                kind: 3,
                values: vec![0, 0],
            },
            OjnEvent {
                measure: 0,
                kind: 1,
                values: vec![120f32.to_bits()],
            },
        ];
        let chart = build_level_chart(120.0, &events);
        assert!(chart.hits.is_empty());
        assert!(chart.backing.is_empty());
        assert_eq!(chart.tempo.bpm_at(0), 120.0);
    }
}
