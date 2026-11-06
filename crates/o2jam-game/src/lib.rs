//! Core gameplay logic: chart model, song clock, judgement, score.
//!
//! Pure logic, no OS calls — the parity-test home for 7.55 behavior.
//! Timing windows VERIFIED against 7.55 (`FUN_00576540`); score point
//! values stay UNVERIFIED placeholders (combo rule VERIFIED).

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub mod chart;
pub mod lane_effect;
pub use chart::{
    build_level_chart, BackingNote, LevelChart, MeterChange, TempoChange, TempoMap, TimedHit,
};
pub use lane_effect::{measure_index_of_tick, LaneMap, MeasureReshuffle};

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
/// Most combo-saving pills a player can hold (verified in-exe:
/// the grant path checks `< 5` and the spend path checks `> 0`).
pub const MAX_PILLS: u32 = 5;

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

/// Consecutive-Cool streak that earns pills, exactly as the 7.55
/// client runs it (`FUN_00576540` score path, file `0x176540`,
/// dossier `ev_c168abac…`): while pills aren't full, each Cool adds
/// 1; crossing `> 14` (the 15th consecutive Cool) grants a pill
/// and resets; Good/Bad/Miss reset the streak. Mode byte `6`
/// exempts both earning and spending (mode TBD).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoolStreak {
    count: u32,
}

impl CoolStreak {
    /// Longest streak that still grants nothing (grant at 15).
    pub const GRANT_AFTER: u32 = 14;

    /// Record one judgement for the streak; returns `true` when a
    /// pill is granted (caller adds it to [`PillStock`]).
    pub fn note(&mut self, judgement: Judgement, pills: &mut PillStock) -> bool {
        match judgement {
            Judgement::Cool => {
                if pills.count() < MAX_PILLS {
                    self.count += 1;
                    if self.count > Self::GRANT_AFTER {
                        self.count = 0;
                        pills.add(1);
                        return true;
                    }
                }
                false
            }
            _ => {
                self.count = 0;
                false
            }
        }
    }
}

/// A waiting-room seat count, VERIFIED against 7.55: eight team buttons
/// `TEAMA..TEAMH` (`0x16201004..0x1620100B`) plus eight avatar covers and
/// eight extend slots in `STATE_WAITING`, and the S→C team-change handler
/// `FUN_0055f470` guards its slot index with `< 8`.
///
/// Assignment is per-seat: the client maps the pressed button to an index
/// (`button − 0x16201004`, see `FUN_004b22d0` / `FUN_0048c810`) and sends
/// `SetRoomTeam` (4004) with that index; the server rebroadcasts and each
/// client refreshes its 8-slot UI. Score aggregation and the winner live
/// server-side (per-member `UpdateGameStats` 4014 / `SubmitScore` 4016;
/// `Winner_image.ojs` in play, `Result_WinLose.ojs` on the result screen).
/// This type mirrors that room state: per-seat teams plus per-seat
/// submitted totals, so tests can pin the client-visible behavior
/// (assignment, totals, winner pick) without a server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomTeams {
    /// Team per seat (`None` = seat empty), index = seat 0..8.
    seats: [Option<u8>; MAX_SEATS],
    /// Submitted score total per seat.
    totals: [u64; MAX_SEATS],
}

/// Seats in a 7.55 waiting room (eight team buttons, eight avatar covers).
pub const MAX_SEATS: usize = 8;
/// Team ids are the 0..8 button indices (`TEAMA` = 0 … `TEAMH` = 7).
pub const MAX_TEAMS: u8 = 8;

impl Default for RoomTeams {
    fn default() -> Self {
        Self::new()
    }
}

impl RoomTeams {
    /// Empty room: every seat vacant, every total zero.
    #[must_use]
    pub fn new() -> Self {
        Self {
            seats: [None; MAX_SEATS],
            totals: [0; MAX_SEATS],
        }
    }

    /// Seat count (always [`MAX_SEATS`]).
    #[must_use]
    pub fn seats(&self) -> usize {
        MAX_SEATS
    }

    /// Assign `seat` to `team` (both range-checked; out-of-range is ignored).
    ///
    /// Mirrors the client send path: pressing `TEAMx` yields index
    /// `x − 'A'` which the client ships as `SetRoomTeam` (4004).
    pub fn set_team(&mut self, seat: usize, team: u8) {
        if seat < MAX_SEATS && team < MAX_TEAMS {
            self.seats[seat] = Some(team);
        }
    }

    /// Vacate `seat` (clears team and total; out-of-range is ignored).
    pub fn vacate(&mut self, seat: usize) {
        if seat < MAX_SEATS {
            self.seats[seat] = None;
            self.totals[seat] = 0;
        }
    }

    /// Team of `seat`, or `None` when the seat is empty/out of range.
    #[must_use]
    pub fn team_of(&self, seat: usize) -> Option<u8> {
        self.seats.get(seat).copied().flatten()
    }

    /// Record a member's submitted total (as the server would accumulate
    /// from `UpdateGameStats`/`SubmitScore`); ignored for empty seats.
    pub fn submit(&mut self, seat: usize, points: u64) {
        if seat < MAX_SEATS && self.seats[seat].is_some() {
            self.totals[seat] = self.totals[seat].saturating_add(points);
        }
    }

    /// Combined total of every seated member on `team`.
    #[must_use]
    pub fn team_total(&self, team: u8) -> u64 {
        self.seats
            .iter()
            .zip(self.totals.iter())
            .filter(|(seat, _)| **seat == Some(team))
            .map(|(_, total)| *total)
            .sum()
    }

    /// Winning team: highest combined total, `None` when no seat is filled.
    ///
    /// Ties break toward the lower team id, matching the
    /// first-seat-wins order of an 8-slot scan. Which side the client
    /// renders (`Winner_image.ojs`, `Result_WinLose.ojs`) is decided by
    /// this same comparison server-side.
    #[must_use]
    pub fn winner(&self) -> Option<u8> {
        (0..MAX_TEAMS)
            .filter(|team| self.seats.contains(&Some(*team)))
            .max_by_key(|team| (self.team_total(*team), MAX_TEAMS - team))
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
                self.points += 200;
            }
            Judgement::Good => {
                self.combo += 1;
                self.points += 100;
            }
            Judgement::Bad => {
                self.combo = 0;
                self.points += 4;
            }
            Judgement::Miss => {
                self.combo = 0;
                self.points = self.points.saturating_sub(10);
            }
        }
        self.max_combo = self.max_combo.max(self.combo);
    }

    /// Apply one judgement through the jam + pill path.
    ///
    /// Verified 7.55 score bases (`FUN_00570510`, file `0x170540`):
    /// Cool +200, Good +100, Bad +4, Miss −10 floored at zero.
    /// Variable `+10*X`/`+5*X` terms use a word at game object
    /// `+0x396e` whose identity is NOT established (no in-exe
    /// writer found), so they are not implemented yet.
    /// Bad with a pill held scores the full Cool base (+200) and
    /// keeps the combo frozen (no increment, no reset — verified
    /// absence of combo writes on the pill-save path); Bad with
    /// no pill and Miss break it (Miss spends no pill — verified
    /// absence of pill logic on its path).
    pub fn apply_rated(
        &mut self,
        judgement: Judgement,
        gauge: &mut JamGauge,
        pills: &mut PillStock,
    ) {
        gauge.feed(judgement);
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
                self.points += 200;
            }
            Judgement::Good => {
                self.combo += 1;
                self.points += 100;
            }
            Judgement::Bad => {
                if pills.consume_to_keep_combo() {
                    self.points += 200;
                } else {
                    self.combo = 0;
                    self.points += 4;
                }
            }
            Judgement::Miss => {
                self.combo = 0;
                self.points = self.points.saturating_sub(10);
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

/// Difficulty tier selecting a life-delta row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    /// Easy (+3/+2/0/−50).
    Easy,
    /// Normal (+2/+1/0/−40).
    #[default]
    Normal,
    /// Hard (+1/+0/0/−30).
    Hard,
}

/// Life bar: 0..=1000, dead at 0 (game over → back to waiting).
///
/// Deltas are player-reported 7.55 (UNVERIFIED in-exe): matching
/// `[3,2,1]`/`[2,1,0]` fragments exist in-shipped-bytes but are not
/// tied to this code path, and the unique `[-10,-7,-5]` triple
/// nearby contradicts the reported Bad=no-change, so it belongs to
/// an unidentified system until proven otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifeGauge {
    hp: u32,
}

impl LifeGauge {
    /// Full life.
    pub const MAX: u32 = 1000;

    /// Create a full life bar.
    #[must_use]
    pub fn new() -> Self {
        Self { hp: Self::MAX }
    }

    /// Current life.
    #[must_use]
    pub fn hp(&self) -> u32 {
        self.hp
    }

    /// Whether the player is dead (game over).
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.hp == 0
    }

    /// Apply one judgement's life delta for a difficulty.
    pub fn apply(&mut self, judgement: Judgement, difficulty: Difficulty) {
        let delta: i32 = match (judgement, difficulty) {
            (Judgement::Cool, Difficulty::Easy) => 3,
            (Judgement::Cool, Difficulty::Normal) => 2,
            (Judgement::Cool, Difficulty::Hard) => 1,
            (Judgement::Good, Difficulty::Easy) => 2,
            (Judgement::Good, Difficulty::Normal) => 1,
            (Judgement::Good, Difficulty::Hard) => 0,
            (Judgement::Bad, _) => 0,
            (Judgement::Miss, Difficulty::Easy) => -50,
            (Judgement::Miss, Difficulty::Normal) => -40,
            (Judgement::Miss, Difficulty::Hard) => -30,
        };
        self.hp = (self.hp as i32 + delta).clamp(0, Self::MAX as i32) as u32;
    }
}

impl Default for LifeGauge {
    fn default() -> Self {
        Self::new()
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
        assert_eq!(s.points, 300);
        s.apply(Judgement::Bad);
        assert_eq!(s.combo, 0);
        assert_eq!(s.points, 304);
        s.apply(Judgement::Cool);
        s.apply(Judgement::Miss);
        assert_eq!(s.combo, 0);
        assert_eq!(s.max_combo, 2);
        assert_eq!(s.points, 504 - 10);
        assert_eq!(s.counts, [2, 1, 1, 1]);
    }

    #[test]
    fn pill_saved_bad_scores_cool_base_and_freezes_combo() {
        let mut s = Score::default();
        let mut gauge = JamGauge::default();
        let mut pills = PillStock::new(1);
        s.apply_rated(Judgement::Cool, &mut gauge, &mut pills);
        assert_eq!((s.combo, s.points), (1, 200));
        s.apply_rated(Judgement::Bad, &mut gauge, &mut pills);
        assert_eq!(pills.count(), 0);
        assert_eq!((s.combo, s.points), (1, 400));
        s.apply_rated(Judgement::Bad, &mut gauge, &mut pills);
        assert_eq!((s.combo, s.points), (0, 404));
    }

    #[test]
    fn rated_scores_match_verified_bases_without_bonus() {
        // Variable +10*X/+5*X terms are pending X's identity, so a
        // jammed run scores the same verified bases as a fresh one.
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
        assert_eq!(jammed_score.points, fresh_score.points);
        assert_eq!(jammed_score.points, 200);
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
    fn pill_saves_combo_from_bad_only() {
        let mut score = Score::default();
        let mut gauge = JamGauge::default();
        let mut pills = PillStock::new(1);
        score.apply_rated(Judgement::Cool, &mut gauge, &mut pills);
        score.apply_rated(Judgement::Bad, &mut gauge, &mut pills);
        assert_eq!(score.combo, 1);
        assert_eq!(pills.count(), 0);
        // Miss spends no pill (verified absence) but still breaks.
        score.apply_rated(Judgement::Cool, &mut gauge, &mut pills);
        score.apply_rated(Judgement::Miss, &mut gauge, &mut pills);
        assert_eq!(score.combo, 0);
        assert_eq!(pills.count(), 0);
        assert_eq!(score.counts, [2, 0, 1, 1]);
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
    fn eight_seats_take_teams_a_through_h() {
        let mut room = RoomTeams::new();
        assert_eq!(room.seats(), 8);
        assert_eq!(room.winner(), None);
        for seat in 0..8 {
            #[allow(clippy::cast_possible_truncation)]
            room.set_team(seat, seat as u8);
        }
        assert_eq!(room.team_of(0), Some(0));
        assert_eq!(room.team_of(7), Some(7));
        assert_eq!(room.team_of(8), None);
        room.set_team(8, 0);
        room.set_team(0, 8);
        assert_eq!(room.team_of(0), Some(0));
    }

    #[test]
    fn team_totals_sum_only_their_own_seats() {
        let mut room = RoomTeams::new();
        room.set_team(0, 0);
        room.set_team(1, 1);
        room.set_team(2, 0);
        room.submit(0, 100_000);
        room.submit(1, 150_000);
        room.submit(2, 50_000);
        room.submit(3, 999_999);
        assert_eq!(room.team_total(0), 150_000);
        assert_eq!(room.team_total(1), 150_000);
        assert_eq!(room.winner(), Some(0));
    }

    #[test]
    fn vacated_seat_loses_team_and_total() {
        let mut room = RoomTeams::new();
        room.set_team(0, 3);
        room.submit(0, 10_000);
        room.vacate(0);
        assert_eq!(room.team_of(0), None);
        assert_eq!(room.team_total(3), 0);
        assert_eq!(room.winner(), None);
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

    #[test]
    fn cool_streak_grants_pill_every_fifteen_cools() {
        let mut pills = PillStock::new(0);
        let mut streak = CoolStreak::default();
        for _ in 0..14 {
            assert!(!streak.note(Judgement::Cool, &mut pills));
        }
        assert_eq!(pills.count(), 0);
        assert!(streak.note(Judgement::Cool, &mut pills));
        assert_eq!(pills.count(), 1);
        for _ in 0..14 {
            assert!(!streak.note(Judgement::Cool, &mut pills));
        }
        assert!(streak.note(Judgement::Cool, &mut pills));
        assert_eq!(pills.count(), 2);
    }

    #[test]
    fn cool_streak_breaks_on_good() {
        let mut pills = PillStock::new(0);
        let mut streak = CoolStreak::default();
        for _ in 0..14 {
            streak.note(Judgement::Cool, &mut pills);
        }
        streak.note(Judgement::Good, &mut pills);
        assert_eq!(pills.count(), 0);
        for _ in 0..15 {
            streak.note(Judgement::Cool, &mut pills);
        }
        assert_eq!(pills.count(), 1);
    }

    #[test]
    fn cool_streak_freezes_when_pills_full() {
        let mut pills = PillStock::new(MAX_PILLS);
        let mut streak = CoolStreak::default();
        for _ in 0..30 {
            assert!(!streak.note(Judgement::Cool, &mut pills));
        }
        assert_eq!(pills.count(), MAX_PILLS);
        assert!(pills.consume_to_keep_combo());
        for _ in 0..14 {
            assert!(!streak.note(Judgement::Cool, &mut pills));
        }
        assert!(streak.note(Judgement::Cool, &mut pills));
        assert_eq!(pills.count(), MAX_PILLS);
    }

    #[test]
    fn life_gauge_applies_difficulty_table_and_dies_at_zero() {
        let mut life = LifeGauge::new();
        assert_eq!(life.hp(), 1000);
        life.apply(Judgement::Cool, Difficulty::Normal);
        assert_eq!(life.hp(), 1000);
        life.apply(Judgement::Miss, Difficulty::Hard);
        assert_eq!(life.hp(), 970);
        life.apply(Judgement::Good, Difficulty::Easy);
        assert_eq!(life.hp(), 972);
        life.apply(Judgement::Bad, Difficulty::Easy);
        assert_eq!(life.hp(), 972);
        for _ in 0..40 {
            life.apply(Judgement::Miss, Difficulty::Easy);
        }
        assert_eq!(life.hp(), 0);
        assert!(life.is_dead());
    }
}
