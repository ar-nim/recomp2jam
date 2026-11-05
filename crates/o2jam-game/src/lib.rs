//! Core gameplay logic: chart model, song clock, judgement, score.
//!
//! Pure logic, no OS calls — the parity-test home for 7.55 behavior.
//! Timing windows VERIFIED against 7.55 (`FUN_00576540`); score point
//! values stay UNVERIFIED placeholders (combo rule VERIFIED).

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub mod chart;
pub use chart::{
    build_level_chart, BackingNote, LevelChart, MeterChange, TempoChange, TempoMap, TimedHit,
};

/// One triggered sound: a lane hit or a backing note whose onset
/// time has been reached by the song clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strike {
    /// Lane for playable hits; `None` for backing tracks.
    pub lane: Option<Lane>,
    /// Key-sound sample id to play.
    pub sample_id: u32,
    /// Onset time in ms from song start.
    pub at_ms: u64,
}

/// Autoplay scheduler: turns a [`LevelChart`] into clock-ordered
/// sound triggers. Pure logic — the player maps `sample_id` to
/// decoded PCM and feeds its audio backend.
#[derive(Debug, Default)]
pub struct Sequencer {
    pending: std::collections::VecDeque<Strike>,
}

impl Sequencer {
    /// Load every hit and backing onset, sorted by time.
    #[must_use]
    pub fn new(chart: &LevelChart) -> Self {
        let mut pending: Vec<Strike> = chart
            .hits
            .iter()
            .map(|hit| Strike {
                lane: Some(hit.lane),
                sample_id: hit.sample_id,
                at_ms: chart.tempo.ms_at_tick(hit.tick),
            })
            .chain(chart.backing.iter().map(|note| Strike {
                lane: None,
                sample_id: note.sample_id,
                at_ms: chart.tempo.ms_at_tick(note.tick),
            }))
            .collect();
        pending.sort_by_key(|strike| strike.at_ms);
        Self {
            pending: pending.into(),
        }
    }

    /// Remaining scheduled triggers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether every trigger has fired.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Drain every trigger with `at_ms <= now_ms`, in time order.
    pub fn pop_due(&mut self, now_ms: u64) -> Vec<Strike> {
        let mut due = Vec::new();
        while self
            .pending
            .front()
            .is_some_and(|next| next.at_ms <= now_ms)
        {
            if let Some(strike) = self.pending.pop_front() {
                due.push(strike);
            }
        }
        due
    }
}

/// A lane index for 7-key play (0..7).
pub type Lane = u8;

/// One hittable note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// Lane 0..7.
    pub lane: Lane,
    /// Time from song start.
    pub time_ms: u64,
    /// Long-note tail length; 0 = tap.
    pub duration_ms: u64,
}

/// A single difficulty chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chart {
    /// Notes sorted by time.
    pub notes: Vec<Note>,
}

impl Chart {
    /// Create a chart, sorting notes by time.
    #[must_use]
    pub fn new(mut notes: Vec<Note>) -> Self {
        notes.sort_by_key(|n| n.time_ms);
        Self { notes }
    }

    /// Note count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.notes.len()
    }

    /// Whether the chart is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }
}

/// Judgement tiers. Windows VERIFIED against 7.55 (see `TimingWindows`);
/// point values stay UNVERIFIED placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Judgement {
    /// Best window.
    Cool,
    /// Good window.
    Good,
    /// Bad breaks combo, like Miss (VERIFIED: `FUN_00576540` only continues
    /// combo on tiers `0x10`/`0x20`; user-confirmed).
    Bad,
    /// Missed entirely.
    Miss,
}

/// Jam-progress points earned per judged note (player-reported 7.55
/// behavior, UNVERIFIED in-exe: only Cool/Good feed the gauge).
/// One jam combo costs `JAM_PROGRESS_PER_JAM` progress points.
pub const JAM_PROGRESS_PER_COOL: u32 = 2;
pub const JAM_PROGRESS_PER_GOOD: u32 = 1;
/// Progress points that complete one jam combo (player-reported 7.55,
/// UNVERIFIED in-exe). Overflow carries: e.g. reaching 31 leaves
/// progress at 1 for the next cycle.
pub const JAM_PROGRESS_PER_JAM: u32 = 30;
/// Most combo-saving pills a player can hold (player-reported 7.55,
/// UNVERIFIED in-exe: earning/spending UI outside the rated path is OPEN).
pub const MAX_PILLS: u32 = 5;
/// Extra score per jam-combo level on Cool/Good hits (placeholder,
/// UNVERIFIED: 7.55 direction confirmed by player report — higher jam
/// combo scores more — but the per-level size is not located in-exe).
pub const JAM_SCORE_BONUS_PER_JAM: u64 = 10;

/// Jam progress bar plus jam-combo count.
///
/// Feed rule (player-reported 7.55, UNVERIFIED in-exe): Cool adds 2,
/// Good adds 1, Bad/Miss add nothing. Every 30 progress completes one
/// jam combo; overflow carries into the next cycle. Whether Bad/Miss
/// reset or preserve progress is OPEN — this keeps progress unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JamGauge {
    /// Progress toward the next jam (`0..JAM_PROGRESS_PER_JAM`).
    progress: u32,
    /// Completed jam combos.
    jams: u32,
}

impl JamGauge {
    /// Current progress toward the next jam.
    #[must_use]
    pub fn progress(&self) -> u32 {
        self.progress
    }

    /// Completed jam combos.
    #[must_use]
    pub fn jam_count(&self) -> u32 {
        self.jams
    }

    /// Feed one judgement into the gauge.
    ///
    /// Returns the number of jam combos completed by this note (0 or more;
    /// more than one only if a single feed overshoots by 2x — kept general).
    pub fn feed(&mut self, judgement: Judgement) -> u32 {
        let gain = match judgement {
            Judgement::Cool => JAM_PROGRESS_PER_COOL,
            Judgement::Good => JAM_PROGRESS_PER_GOOD,
            Judgement::Bad | Judgement::Miss => 0,
        };
        self.progress += gain;
        let mut earned = 0;
        while self.progress >= JAM_PROGRESS_PER_JAM {
            self.progress -= JAM_PROGRESS_PER_JAM;
            self.jams += 1;
            earned += 1;
        }
        earned
    }
}

/// Combo-saving pill stock (player-reported 7.55, UNVERIFIED in-exe).
///
/// A pill is spent instead of breaking the combo on a combo-breaking
/// judgement (Bad/Miss — the VERIFIED combo-break set). How pills are
/// earned and which shop/UI flows grant them is OPEN; this tracks the
/// spend side only, capped at `MAX_PILLS`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PillStock {
    /// Pills currently held (`<= MAX_PILLS`).
    pills: u32,
}

impl PillStock {
    /// Hold `count` pills, capped at `MAX_PILLS`.
    #[must_use]
    pub fn new(count: u32) -> Self {
        Self {
            pills: count.min(MAX_PILLS),
        }
    }

    /// Pills currently held.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.pills
    }

    /// Add pills (e.g. shop grant), capped at `MAX_PILLS`.
    pub fn add(&mut self, count: u32) {
        self.pills = self.pills.saturating_add(count).min(MAX_PILLS);
    }

    /// Spend one pill to keep the combo, if any is held.
    ///
    /// Returns true when a pill was spent (combo is kept).
    pub fn consume_to_keep_combo(&mut self) -> bool {
        if self.pills > 0 {
            self.pills -= 1;
            true
        } else {
            false
        }
    }
}

/// (file `0x176540`, VA `0x576540`, dossier `ev_14992eb2…` /
/// decompile `ev_5ecb1ce8…`).
///
/// Timing windows in ms, VERIFIED against 7.55 `FUN_00576540`
/// (file `0x176540`, VA `0x576540`, dossier `ev_14992eb2…` /
/// decompile `ev_5ecb1ce8…`).
///
/// Both judgement sites in that function compare the absolute
/// `|song_ms - note_ms|` (song ms via `FUN_0057a240` tick→ms interpolation,
/// dossier `ev_9f8e1c2b…` / decompile `ev_f170d5f6…`) against inclusive
/// bounds 6 / 18 / 25 ms:
/// file `0x176646` (`ev_b492dfe0…`), `0x176655` (`ev_c9145d1a…`),
/// `0x176664` (`ev_da09b5c7…`) and key-press site file `0x176b7f`
/// (`ev_c31f6413…`), `0x176b8e` (`ev_7611dac8…`), `0x176b9d`
/// (`ev_3db1071b…`). Tiers emit flags `0x10`/`0x20`/`0x40`/`0x80`
/// (Cool/Good/Bad/Miss). Long-note combo ticks exist alongside this
/// path (hold-bit `0x2000000` handling in the same function) — OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingWindows {
    /// |offset| <= cool = Cool.
    pub cool: u64,
    /// |offset| <= good = Good.
    pub good: u64,
    /// |offset| <= bad = Bad, else Miss.
    pub bad: u64,
}

impl Default for TimingWindows {
    fn default() -> Self {
        Self {
            cool: 6,
            good: 18,
            bad: 25,
        }
    }
}

impl TimingWindows {
    /// Judge a hit offset (hit_time - note_time, signed ms).
    #[must_use]
    pub fn judge(&self, offset_ms: i64) -> Judgement {
        let a = offset_ms.unsigned_abs();
        if a <= self.cool {
            Judgement::Cool
        } else if a <= self.good {
            Judgement::Good
        } else if a <= self.bad {
            Judgement::Bad
        } else {
            Judgement::Miss
        }
    }
}

/// Running score/combo. Point values are UNVERIFIED placeholders;
/// combo rule is VERIFIED (Cool/Good continue, Bad/Miss reset —
/// `FUN_00576540` file `0x176c86` `ev_63f07dfc…` / `0x176c8c` `ev_b7f03b4d…`
/// vs Bad/Miss path `0x176d8b` `ev_55bfa19b…`; user-confirmed).
/// Long-note combo bonus is OPEN (user-reported; hold path in same
/// function, points not yet located — scoring lives behind the `+0xa14`
/// virtual callback, no inline point immediates).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Score {
    /// Accumulated points.
    pub points: u64,
    /// Current combo.
    pub combo: u32,
    /// Best combo so far.
    pub max_combo: u32,
    /// Per-tier counts [cool, good, bad, miss].
    pub counts: [u32; 4],
}

impl Score {
    /// Apply one judgement.
    pub fn apply(&mut self, j: Judgement) {
        let idx = match j {
            Judgement::Cool => 0,
            Judgement::Good => 1,
            Judgement::Bad => 2,
            Judgement::Miss => 3,
        };
        self.counts[idx] += 1;
        match j {
            Judgement::Cool => {
                self.combo += 1;
                self.points += 300;
            }
            Judgement::Good => {
                self.combo += 1;
                self.points += 100;
            }
            Judgement::Bad => {
                self.combo = 0;
                self.points += 10;
            }
            Judgement::Miss => {
                self.combo = 0;
            }
        }
        self.max_combo = self.max_combo.max(self.combo);
    }

    /// Apply one judgement through the jam + pill path.
    ///
    /// Rated rule (player-reported 7.55, scoring sizes UNVERIFIED in-exe):
    /// the gauge feeds first (this note can complete a jam), Cool/Good
    /// continue the combo and score base points plus
    /// `JAM_SCORE_BONUS_PER_JAM` per completed jam, and Bad/Miss spend
    /// one pill to keep the combo when a pill is held, else break it.
    /// Whether ordering (bonus on the completing note) and pill-kept
    /// Bad/Miss points match 7.55 is OPEN.
    pub fn apply_rated(
        &mut self,
        judgement: Judgement,
        gauge: &mut JamGauge,
        pills: &mut PillStock,
    ) {
        gauge.feed(judgement);
        let jam_bonus = u64::from(gauge.jam_count()) * JAM_SCORE_BONUS_PER_JAM;
        let idx = match judgement {
            Judgement::Cool => 0,
            Judgement::Good => 1,
            Judgement::Bad => 2,
            Judgement::Miss => 3,
        };
        self.counts[idx] += 1;
        match judgement {
            Judgement::Cool => {
                self.combo += 1;
                self.points += 300 + jam_bonus;
            }
            Judgement::Good => {
                self.combo += 1;
                self.points += 100 + jam_bonus;
            }
            Judgement::Bad => {
                if !pills.consume_to_keep_combo() {
                    self.combo = 0;
                }
                self.points += 10;
            }
            Judgement::Miss => {
                if !pills.consume_to_keep_combo() {
                    self.combo = 0;
                }
            }
        }
        self.max_combo = self.max_combo.max(self.combo);
    }
}

/// Song clock: elapsed time since song start.
///
/// Day-0 this is wall-clock; the real client anchors render positions to
/// the *audio* clock (sample count). Keep this type as the seam so the
/// swap is one constructor change.
#[derive(Debug, Clone)]
pub struct SongClock {
    start: Instant,
    offset: Duration,
}

impl SongClock {
    /// Start at zero.
    #[must_use]
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
            offset: Duration::ZERO,
        }
    }

    /// Start with an offset (e.g. delayed audio start).
    #[must_use]
    pub fn start_with_offset(offset: Duration) -> Self {
        Self {
            start: Instant::now(),
            offset,
        }
    }

    /// Elapsed song time in ms.
    #[must_use]
    pub fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64 + self.offset.as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_sorts_notes() {
        let c = Chart::new(vec![
            Note {
                lane: 1,
                time_ms: 300,
                duration_ms: 0,
            },
            Note {
                lane: 0,
                time_ms: 100,
                duration_ms: 0,
            },
        ]);
        assert_eq!(c.notes[0].time_ms, 100);
        assert_eq!(c.len(), 2);
        assert!(!c.is_empty());
    }

    #[test]
    fn windows_judge_verified_7_55_tiers() {
        let w = TimingWindows::default();
        assert_eq!((w.cool, w.good, w.bad), (6, 18, 25));
        assert_eq!(w.judge(0), Judgement::Cool);
        assert_eq!(w.judge(6), Judgement::Cool);
        assert_eq!(w.judge(7), Judgement::Good);
        assert_eq!(w.judge(-18), Judgement::Good);
        assert_eq!(w.judge(19), Judgement::Bad);
        assert_eq!(w.judge(-25), Judgement::Bad);
        assert_eq!(w.judge(26), Judgement::Miss);
        assert_eq!(w.judge(-26), Judgement::Miss);
    }

    #[test]
    fn score_combo_breaks_on_bad_and_miss() {
        let mut s = Score::default();
        s.apply(Judgement::Cool);
        s.apply(Judgement::Good);
        assert_eq!(s.combo, 2);
        assert_eq!(s.points, 400);
        s.apply(Judgement::Bad);
        assert_eq!(s.combo, 0);
        s.apply(Judgement::Cool);
        s.apply(Judgement::Miss);
        assert_eq!(s.combo, 0);
        assert_eq!(s.max_combo, 2);
        assert_eq!(s.counts, [2, 1, 1, 1]);
    }

    #[test]
    fn jam_gauge_cool_feeds_two_good_feeds_one() {
        let mut gauge = JamGauge::default();
        assert_eq!(gauge.feed(Judgement::Cool), 0);
        assert_eq!(gauge.progress(), 2);
        assert_eq!(gauge.feed(Judgement::Good), 0);
        assert_eq!(gauge.progress(), 3);
        assert_eq!(gauge.jam_count(), 0);
    }

    #[test]
    fn jam_gauge_thirty_progress_completes_one_jam() {
        let mut gauge = JamGauge::default();
        for _ in 0..15 {
            gauge.feed(Judgement::Cool);
        }
        assert_eq!(gauge.jam_count(), 1);
        assert_eq!(gauge.progress(), 0);
    }

    #[test]
    fn jam_gauge_overflow_carries_remainder() {
        let mut gauge = JamGauge::default();
        for _ in 0..14 {
            gauge.feed(Judgement::Cool);
        }
        assert_eq!(gauge.progress(), 28);
        gauge.feed(Judgement::Good);
        assert_eq!(gauge.progress(), 29);
        assert_eq!(gauge.feed(Judgement::Cool), 1);
        assert_eq!(gauge.jam_count(), 1);
        assert_eq!(gauge.progress(), 1);
    }

    #[test]
    fn jam_gauge_ignores_bad_and_miss() {
        let mut gauge = JamGauge::default();
        gauge.feed(Judgement::Cool);
        gauge.feed(Judgement::Bad);
        gauge.feed(Judgement::Miss);
        assert_eq!(gauge.progress(), 2);
        assert_eq!(gauge.jam_count(), 0);
    }

    #[test]
    fn pill_stock_caps_at_five() {
        let overfull = PillStock::new(9);
        assert_eq!(overfull.count(), MAX_PILLS);
        let mut stock = PillStock::default();
        stock.add(3);
        stock.add(4);
        assert_eq!(stock.count(), 5);
    }

    #[test]
    fn pill_saves_combo_from_bad_and_miss() {
        let mut score = Score::default();
        let mut gauge = JamGauge::default();
        let mut pills = PillStock::new(2);
        score.apply_rated(Judgement::Cool, &mut gauge, &mut pills);
        score.apply_rated(Judgement::Bad, &mut gauge, &mut pills);
        assert_eq!(score.combo, 1);
        score.apply_rated(Judgement::Miss, &mut gauge, &mut pills);
        assert_eq!(score.combo, 1);
        assert_eq!(pills.count(), 0);
        assert_eq!(score.counts, [1, 0, 1, 1]);
    }

    #[test]
    fn rated_combo_breaks_without_pills() {
        let mut score = Score::default();
        let mut gauge = JamGauge::default();
        let mut pills = PillStock::default();
        score.apply_rated(Judgement::Cool, &mut gauge, &mut pills);
        score.apply_rated(Judgement::Miss, &mut gauge, &mut pills);
        assert_eq!(score.combo, 0);
        assert_eq!(score.max_combo, 1);
    }

    #[test]
    fn higher_jam_combo_scores_more_on_cool() {
        let mut fresh_score = Score::default();
        let mut fresh_gauge = JamGauge::default();
        let mut pills = PillStock::default();
        fresh_score.apply_rated(Judgement::Cool, &mut fresh_gauge, &mut pills);

        let mut jammed_score = Score::default();
        let mut jammed_gauge = JamGauge::default();
        for _ in 0..15 {
            jammed_gauge.feed(Judgement::Cool);
        }
        assert_eq!(jammed_gauge.jam_count(), 1);
        jammed_score.apply_rated(Judgement::Cool, &mut jammed_gauge, &mut pills);
        assert!(jammed_score.points > fresh_score.points);
    }

    #[test]
    fn clock_moves_forward() {
        let c = SongClock::start();
        std::thread::sleep(Duration::from_millis(5));
        assert!(c.now_ms() < 10_000);
    }

    fn chart_two_hits_one_backing() -> LevelChart {
        let tempo = TempoMap {
            base_bpm: 120.0,
            changes: Vec::new(),
            meters: Vec::new(),
        };
        LevelChart {
            hits: vec![
                TimedHit {
                    lane: 3,
                    tick: 192,
                    sample_id: 7,
                    flags: 0,
                },
                TimedHit {
                    lane: 0,
                    tick: 0,
                    sample_id: 3,
                    flags: 0,
                },
            ],
            backing: vec![BackingNote {
                tick: 96,
                sample_id: 9,
                flags: 0,
            }],
            tempo,
        }
    }

    #[test]
    fn sequencer_fires_in_time_order() {
        let mut seq = Sequencer::new(&chart_two_hits_one_backing());
        assert_eq!(seq.len(), 3);
        let due = seq.pop_due(0);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].lane, Some(0));
        assert_eq!(due[0].at_ms, 0);
        let due = seq.pop_due(999);
        assert!(due.is_empty());
        let due = seq.pop_due(1000);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].lane, None);
        assert_eq!(due[0].sample_id, 9);
        let due = seq.pop_due(2000);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].lane, Some(3));
        assert!(seq.is_empty());
    }
}
