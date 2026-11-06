//! Core gameplay logic: chart model, song clock, judgement, score.
//!
//! Pure logic, no OS calls — the parity-test home for 7.55 behavior.
//! Timing windows VERIFIED against 7.55 (`FUN_00576540`); score bases
//! plus jam level `X` VERIFIED (`FUN_00570510` / `FUN_00570260`);
//! long-note release documented as played (head hit + tail release).
//!
//! Life bar: documented only, no model. Per-judgement life deltas are
//! UNVERIFIED (no write site; candidate table at file `0x35e1a0` has no
//! `.text` reader) so they stay out of code. VERIFIED anchors live in
//! `docs/gameplay.md`: `Life_gauge_*_bar.ojs` art, per-hit Jam (4014
//! arg 1) vs tick-driven Life (4014 arg 0), `SubmitScore` (4016) field
//! layout; the 4015 peer-life store is OPEN.

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

/// Jam threshold table: `FUN_00570260` (file `0x170260`) fills
/// `+0xb66[i] = (i + 1) * 50` for `i = 0..400` at game init — first
/// jam level needs 50 progress, then 100, 150, … cumulative with no
/// reset on level-up. Identical for every difficulty.
pub const JAM_LEVEL_COUNT: usize = 400;
/// Progress one Cool feeds (`push $0x2`, file `0x1705b0`).
pub const JAM_PROGRESS_PER_COOL: u64 = 2;
/// Progress one Good feeds (`push $0x1`, file `0x170628`).
pub const JAM_PROGRESS_PER_GOOD: u64 = 1;

/// Cumulative progress needed to reach `level` (threshold table entry).
///
/// VERIFIED (`FUN_00570260`): threshold is `(level + 1) * 50`.
#[must_use]
pub fn jam_threshold_for(level: u32) -> u64 {
    (u64::from(level) + 1) * 50
}

/// Jam level `X` (word at game object `+0x396e`, sub `+0x214e`) plus its
/// `PROGRESS` counter (`+0x396c`, sub `+0x214c`) — VERIFIED 7.55:
/// Cool feeds +2, Good feeds +1 (both via adder `0x5409f0`);
/// Bad/Miss reset progress to `0` (setter `0x540770` on the shared
/// low-branch tail, file `0x1708dd`); crossing the threshold grants
/// `X += 1` (adder `0x140a20`, no progress reset — cumulative) and
/// `X` resets to `0` on Bad/Miss (file `0x170936`, after a maxJam
/// snapshot). A pill-saved Bad takes the Cool path verbatim, so it
/// feeds like a Cool.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JamState {
    level: u32,
    progress: u64,
    max_level: u32,
}

impl JamState {
    /// Empty jam: level 0, no progress.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Current jam level `X` (score multiplier input).
    #[must_use]
    pub fn level(&self) -> u32 {
        self.level
    }

    /// Cumulative progress toward the next level (never reset on grant).
    #[must_use]
    pub fn progress(&self) -> u64 {
        self.progress
    }

    /// Highest level reached (snapshot kept across Bad/Miss resets).
    #[must_use]
    pub fn max_level(&self) -> u32 {
        self.max_level
    }

    /// Progress needed to leave the current level.
    #[must_use]
    pub fn threshold(&self) -> u64 {
        jam_threshold_for(self.level)
    }

    /// Feed one judgement into progress, granting levels while covered.
    pub fn note(&mut self, judgement: Judgement) {
        match judgement {
            Judgement::Cool => self.feed(JAM_PROGRESS_PER_COOL),
            Judgement::Good => self.feed(JAM_PROGRESS_PER_GOOD),
            Judgement::Bad | Judgement::Miss => self.reset(),
        }
    }

    /// A pill-saved Bad runs the Cool branch, so it feeds like a Cool.
    pub fn note_pill_saved(&mut self) {
        self.feed(JAM_PROGRESS_PER_COOL);
    }

    fn feed(&mut self, amount: u64) {
        self.progress = self.progress.saturating_add(amount);
        self.grant_while_covered();
    }

    fn grant_while_covered(&mut self) {
        while usize::try_from(self.level).unwrap_or(usize::MAX) < JAM_LEVEL_COUNT
            && self.progress >= self.threshold()
        {
            self.level += 1;
            self.max_level = self.max_level.max(self.level);
        }
    }

    fn reset(&mut self) {
        self.max_level = self.max_level.max(self.level);
        self.level = 0;
        self.progress = 0;
    }
}

/// Tail release of a long note: the head must be struck, then the tail
/// MUST be released at its end — missing the release is a Miss and
/// releasing mid-hold is a Miss (player-reported; hold-bit sites pinned
/// in-exe at `0x576105`/`0x5766cb`/`0x576c42`/`0x576994` but the exact
/// release judgement was not traced, so the release reuses the struck
/// [`TimingWindows`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LongTail {
    /// Head onset in ms.
    pub head_ms: u64,
    /// Tail (release point) in ms.
    pub tail_ms: u64,
}

impl LongTail {
    /// Hold from head to tail; `None` when the span is empty (a tap).
    #[must_use]
    pub fn new(head_ms: u64, tail_ms: u64) -> Option<Self> {
        (tail_ms > head_ms).then_some(Self { head_ms, tail_ms })
    }

    /// A release before the window is mid-hold: Miss.
    #[must_use]
    pub fn is_early(&self, release_ms: u64, windows: &TimingWindows) -> bool {
        release_ms.saturating_add(windows.bad) < self.tail_ms
    }

    /// No release by the window end: Miss (caller rates it on advance).
    #[must_use]
    pub fn is_expired(&self, now_ms: u64, windows: &TimingWindows) -> bool {
        self.tail_ms.saturating_add(windows.bad) < now_ms
    }

    /// Judge a release inside the window with the strike tiers.
    #[must_use]
    pub fn judge_release(&self, release_ms: u64, windows: &TimingWindows) -> Judgement {
        windows.judge(release_ms as i64 - self.tail_ms as i64)
    }
}

/// Most combo-saving pills a player can hold (verified in-exe:
/// the grant path checks `< 5` and the spend path checks `> 0`).
pub const MAX_PILLS: u32 = 5;

/// Combo-saving pill stock (VERIFIED earn + spend, 7.55).
///
/// A pill is spent instead of breaking the combo on a combo-breaking
/// judgement (Bad — Miss spends no pill; VERIFIED absence on its path).
/// Earned via [`CoolStreak`] (15 consecutive Cools mints one, max 5);
/// mode byte 6 exempts both earning and spending (mode TBD).
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

    /// Add pills, capped at `MAX_PILLS`. Callers decide when a grant
    /// happens (no in-exe earn path is verified).
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
        if judgement != Judgement::Cool {
            self.count = 0;
            return false;
        }
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

/// Running score/combo/jam. VERIFIED 7.55 scorer `FUN_00570510`
/// (file `0x170510`): Cool `200 + 10*X`, Good `100 + 5*X`, Bad +4,
/// Miss −10 floored at zero (`-10` when the field exceeds `0x0a`).
/// Bad-with-pill takes the Cool path verbatim (BAD→COOL conversion).
/// Combo rule VERIFIED (Cool/Good continue, Bad/Miss reset —
/// `FUN_00576540` file `0x176c86` `ev_63f07dfc…` / `0x176c8c` `ev_b7f03b4d…`
/// vs Bad/Miss path `0x176d8b` `ev_55bfa19b…`; user-confirmed);
/// pill-save freezes the combo (no combo write on that path).
/// Jam feed/grant order (feed, grant, then score with the resulting
/// `X`) is an implementation choice — the exe order is untraced.
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
    /// Jam level `X` plus progress (drives the `10*X` / `5*X` terms).
    pub jam: JamState,
}

/// Count-bucket index for one tier: [cool, good, bad, miss].
fn tier_index(judgement: Judgement) -> usize {
    match judgement {
        Judgement::Cool => 0,
        Judgement::Good => 1,
        Judgement::Bad => 2,
        Judgement::Miss => 3,
    }
}

impl Score {
    /// Score one tier with the current jam level (`200 + 10*X` /
    /// `100 + 5*X` / `+4` / `−10` floored at zero).
    fn add_tier_points(&mut self, judgement: Judgement) {
        let level = u64::from(self.jam.level());
        match judgement {
            Judgement::Cool => self.points += 200 + 10 * level,
            Judgement::Good => self.points += 100 + 5 * level,
            Judgement::Bad => self.points += 4,
            Judgement::Miss => self.points = self.points.saturating_sub(10),
        }
    }

    /// Apply one judgement.
    pub fn apply(&mut self, j: Judgement) {
        self.counts[tier_index(j)] += 1;
        self.jam.note(j);
        match j {
            Judgement::Cool | Judgement::Good => self.combo += 1,
            Judgement::Bad | Judgement::Miss => self.combo = 0,
        }
        self.add_tier_points(j);
        self.max_combo = self.max_combo.max(self.combo);
    }

    /// Apply one judgement through the jam + pill path.
    ///
    /// Verified 7.55 (`FUN_00570510`, file `0x170510`): Cool
    /// `200 + 10*X`, Good `100 + 5*X`, Bad-with-pill the Cool value
    /// verbatim, Bad-without-pill +4, Miss −10 floored at zero.
    /// Bad with a pill held keeps the combo frozen (no increment, no
    /// reset — verified absence of combo writes on the pill-save
    /// path) and feeds jam like a Cool; Bad with no pill and Miss
    /// break combo and reset jam (Miss spends no pill — verified
    /// absence of pill logic on its path).
    pub fn apply_rated(&mut self, judgement: Judgement, pills: &mut PillStock) {
        self.counts[tier_index(judgement)] += 1;
        if judgement == Judgement::Bad && pills.consume_to_keep_combo() {
            self.jam.note_pill_saved();
            let level = u64::from(self.jam.level());
            self.points += 200 + 10 * level;
            self.max_combo = self.max_combo.max(self.combo);
            return;
        }
        self.jam.note(judgement);
        match judgement {
            Judgement::Cool | Judgement::Good => self.combo += 1,
            Judgement::Bad | Judgement::Miss => self.combo = 0,
        }
        self.add_tier_points(judgement);
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

    /// Elapsed song time in ms (saturates instead of wrapping).
    #[must_use]
    pub fn now_ms(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .saturating_add(u64::try_from(self.offset.as_millis()).unwrap_or(u64::MAX))
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
        let mut pills = PillStock::new(1);
        s.apply_rated(Judgement::Cool, &mut pills);
        assert_eq!((s.combo, s.points), (1, 200));
        s.apply_rated(Judgement::Bad, &mut pills);
        assert_eq!(pills.count(), 0);
        assert_eq!((s.combo, s.points), (1, 400));
        s.apply_rated(Judgement::Bad, &mut pills);
        assert_eq!((s.combo, s.points), (0, 404));
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
        let mut pills = PillStock::new(1);
        score.apply_rated(Judgement::Cool, &mut pills);
        score.apply_rated(Judgement::Bad, &mut pills);
        assert_eq!(score.combo, 1);
        assert_eq!(pills.count(), 0);
        // Miss spends no pill (verified absence) but still breaks.
        score.apply_rated(Judgement::Cool, &mut pills);
        score.apply_rated(Judgement::Miss, &mut pills);
        assert_eq!(score.combo, 0);
        assert_eq!(pills.count(), 0);
        assert_eq!(score.counts, [2, 0, 1, 1]);
    }

    #[test]
    fn rated_uses_verified_score_bases() {
        let mut score = Score::default();
        let mut pills = PillStock::default();
        score.apply_rated(Judgement::Cool, &mut pills);
        score.apply_rated(Judgement::Good, &mut pills);
        assert_eq!((score.combo, score.points), (2, 300));
        score.apply_rated(Judgement::Bad, &mut pills);
        assert_eq!((score.combo, score.points), (0, 304));
    }

    #[test]
    fn rated_combo_breaks_without_pills() {
        let mut score = Score::default();
        let mut pills = PillStock::default();
        score.apply_rated(Judgement::Cool, &mut pills);
        score.apply_rated(Judgement::Miss, &mut pills);
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
    fn jam_threshold_table_matches_verified_fifties() {
        assert_eq!(jam_threshold_for(0), 50);
        assert_eq!(jam_threshold_for(1), 100);
        assert_eq!(jam_threshold_for(2), 150);
        assert_eq!(jam_threshold_for(399), 20_000);
        assert_eq!(JAM_LEVEL_COUNT, 400);
    }

    #[test]
    fn jam_feeds_two_per_cool_and_one_per_good() {
        let mut jam = JamState::new();
        jam.note(Judgement::Cool);
        assert_eq!((jam.level(), jam.progress()), (0, 2));
        jam.note(Judgement::Good);
        assert_eq!((jam.level(), jam.progress()), (0, 3));
    }

    #[test]
    fn jam_grants_level_without_resetting_progress() {
        let mut jam = JamState::new();
        for _ in 0..25 {
            jam.note(Judgement::Cool);
        }
        assert_eq!((jam.level(), jam.progress()), (1, 50));
        assert_eq!(jam.threshold(), 100);
        assert_eq!(jam.max_level(), 1);
    }

    #[test]
    fn jam_resets_level_and_progress_on_bad() {
        let mut jam = JamState::new();
        for _ in 0..25 {
            jam.note(Judgement::Cool);
        }
        jam.note(Judgement::Bad);
        assert_eq!((jam.level(), jam.progress()), (0, 0));
        assert_eq!(jam.max_level(), 1);
    }

    #[test]
    fn jam_resets_level_and_progress_on_miss() {
        let mut jam = JamState::new();
        jam.note(Judgement::Good);
        jam.note(Judgement::Miss);
        assert_eq!((jam.level(), jam.progress()), (0, 0));
    }

    #[test]
    fn rated_scores_jam_multiplier_on_cool_and_good() {
        let mut score = Score::default();
        let mut pills = PillStock::default();
        for _ in 0..25 {
            score.apply_rated(Judgement::Cool, &mut pills);
        }
        assert_eq!(score.jam.level(), 1);
        let before = score.points;
        score.apply_rated(Judgement::Cool, &mut pills);
        assert_eq!(score.points, before + 200 + 10);
        let before = score.points;
        score.apply_rated(Judgement::Good, &mut pills);
        assert_eq!(score.points, before + 100 + 5);
    }

    #[test]
    fn rated_pill_saved_bad_feeds_jam_like_cool() {
        let mut score = Score::default();
        let mut pills = PillStock::new(1);
        score.apply_rated(Judgement::Cool, &mut pills);
        let progress = score.jam.progress();
        score.apply_rated(Judgement::Bad, &mut pills);
        assert_eq!(score.jam.progress(), progress + JAM_PROGRESS_PER_COOL);
        assert_eq!(score.combo, 1);
    }

    #[test]
    fn long_tail_rejects_empty_spans() {
        assert!(LongTail::new(1000, 1000).is_none());
        assert!(LongTail::new(1000, 999).is_none());
        assert!(LongTail::new(1000, 1500).is_some());
    }

    #[test]
    fn long_tail_release_before_window_is_early() {
        let windows = TimingWindows::default();
        let tail = LongTail::new(1000, 2000).expect("hold");
        assert!(tail.is_early(2000 - windows.bad - 1, &windows));
        assert!(!tail.is_early(2000 - windows.bad, &windows));
        assert_eq!(tail.judge_release(2000, &windows), Judgement::Cool);
    }

    #[test]
    fn long_tail_missing_release_expires_after_window() {
        let windows = TimingWindows::default();
        let tail = LongTail::new(1000, 2000).expect("hold");
        assert!(!tail.is_expired(2000 + windows.bad, &windows));
        assert!(tail.is_expired(2000 + windows.bad + 1, &windows));
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
}
