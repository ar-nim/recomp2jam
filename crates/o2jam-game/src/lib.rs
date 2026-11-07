//! Core gameplay logic: chart model, song clock, judgement, score.
//!
//! Pure logic, no OS calls — the parity-test home for 7.55 behavior.
//! Timing windows VERIFIED against 7.55 (`FUN_00576540`); score bases
//! plus jam level `X` VERIFIED (`FUN_00570510` / `FUN_00570260`);
//! long-note hold machine traced in-exe (head hit arms the hold, bonus
//! ticks over the span, the tail rates on the release strike through the
//! armed per-lane cursor — [`HoldState`]).
//!
//! Life bar VERIFIED end to end: the difficulty switch `FUN_00570f90`
//! (files `0x170fa2`–`0x171056`, ev `feebe490…`) selects per-difficulty
//! deltas, and the scorer `FUN_00570510` applies them per judgement to
//! the u16 life word at game `+0x3970` via adder `0x540a80` — modeled as
//! [`LifeBar`]. The 4015 peer-life/rank mirror lives in [`PeerBars`]
//! (VERIFIED handler shape, dossier `rea-4015-ghidra`). VERIFIED anchors
//! live in `docs/gameplay.md`: `Life_gauge_*_bar.ojs` art, per-hit Jam
//! (4014 arg 1) vs tick-driven Life (4014 arg 0), `SubmitScore` (4016)
//! field layout.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub mod chart;
pub mod lane_effect;
pub mod peer;
pub use chart::{
    build_level_chart, BackingNote, LevelChart, MeterChange, TempoChange, TempoMap, TimedHit,
};
pub use lane_effect::{measure_index_of_tick, LaneMap, MeasureReshuffle};
pub use peer::{
    PeerBars, QueueEvent, RankDelivery, LIFE_MAX, PEER_SLOT_COUNT, QUEUE_CATEGORY_PEER, STAT_JAM,
    STAT_LIFE,
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

/// Jam threshold table: `FUN_00570260` (file `0x170260`) fills
/// `+0xb66[i] = (i + 1) * 50` for `i = 0..400` at game init — first
/// jam level needs 50 progress, then 100, 150, … cumulative with no
/// reset on level-up. Identical for every difficulty.
pub const JAM_LEVEL_COUNT: usize = 400;
/// Progress one Cool feeds (`push $0x2`, file `0x1705b0`).
pub const JAM_PROGRESS_PER_COOL: u64 = 2;
/// Progress one Good feeds (`push $0x1`, file `0x170628`).
pub const JAM_PROGRESS_PER_GOOD: u64 = 1;
/// Progress one jam level spans: successive thresholds differ by 50
/// (`(i + 1) * 50`, `FUN_00570260`), so each level needs 50 more
/// cumulative progress.
pub const JAM_PROGRESS_PER_LEVEL: u64 = 50;

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

    /// Progress inside the current level (`0..50`): cumulative progress
    /// minus the base the level started from (level * 50, since every
    /// threshold step spans [`JAM_PROGRESS_PER_LEVEL`]).
    #[must_use]
    pub fn level_progress(&self) -> u64 {
        self.progress
            .saturating_sub(u64::from(self.level) * JAM_PROGRESS_PER_LEVEL)
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

/// Chart-data span of one long note, VERIFIED 7.55 (payload-bit rules,
/// claim B14): a same-lane record pair whose head payload tests
/// [`HoldSpan::marks_hold_head`] (bit 25 set, bit 24 clear) and whose
/// paired tail payload tests [`HoldSpan::marks_hold_tail`] (bit 24).
/// The pairing is positional — the span latch advances the lane list
/// once from the head to the next record (advance call at file
/// `0x17612c`, latch stores `0x176197`/`0x1761af`) — and the load-end
/// pairing sweep re-derives it from the same bits (bit-25 tests at
/// files `0x173c7f`/`0x173e23`, bit-24 test at file `0x173fd5`,
/// dossier `ev_edc4cc84…`). Bit 26 (`0x4000000`) never reaches any
/// judge or scan: its only payload consumers are parser-side (survey
/// `0x172a4f`, fix-up `0x173061`) plus the sample-slot bank `+0x3E8`
/// (files `0x170427`/`0x170431`, claim B17), so claim B14's "bit
/// 25/26" resolves to bit 25 = head / bit 24 = tail / bit 26 = parser
/// span marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldSpan {
    /// Head onset in ms.
    pub head_ms: u64,
    /// Tail record time in ms (span end for the bonus ticker).
    pub tail_ms: u64,
}

impl HoldSpan {
    /// Hold from head to tail; `None` when the span is empty (a tap).
    #[must_use]
    pub fn new(head_ms: u64, tail_ms: u64) -> Option<Self> {
        (tail_ms > head_ms).then_some(Self { head_ms, tail_ms })
    }

    /// Head arming test, VERIFIED 7.55: bit 25 (`0x2000000`) set with
    /// bit 24 (`0x1000000`) clear. The equality form runs in the second
    /// judge (files `0x1760ff`/`0x176105`, dossier `ev_3d707204…`), the
    /// keyup branch (files `0x176c3b`/`0x176c41`), the auto judge
    /// (files `0x1771ef`/`0x1771f5`), the expiry-scan head test
    /// (`0x17746d`) and the dead lane planner (`0x1748ce`); the mode-0
    /// keydown judge tests bit 25 alone (`and ecx, 0x2000000`, file
    /// `0x1766cb`) — that divergence cannot fire, because the candidate
    /// walk excludes bit-24 records first (files
    /// `0x176837`/`0x176906`/`0x176994`).
    pub const HOLD_HEAD_TEST_MASK: u32 = 0x0300_0000;
    /// The masked payload must equal this to mark a head (see
    /// [`Self::HOLD_HEAD_TEST_MASK`]).
    pub const HOLD_HEAD_TEST_BITS: u32 = 0x0200_0000;
    /// Tail marker test, VERIFIED 7.55: bit 24 set. The expiry pairing
    /// scan marks the first bit-24 record after an unjudged head
    /// `0x180` (test file `0x1774b4`, store `0x1774e2`, dossier
    /// `ev_0ece39cb…`), a Bad keyup head hit kills it (file
    /// `0x176dab`), the strike-slot computer forks the tail spawn call
    /// on it (files `0x17043d`/`0x170487`), and the pass-event scan
    /// re-tests the same bits (files `0x177bc0`–`0x177f5b`). Shipped
    /// tails keep bit 25 set too (flag word `0x300`, B16 data walk);
    /// no exe site requires bit 25 on a tail.
    pub const HOLD_TAIL_TEST_MASK: u32 = 0x0100_0000;

    /// Whether a chart payload word marks a hold head: bit 25 set with
    /// bit 24 clear (see [`Self::HOLD_HEAD_TEST_MASK`]).
    #[must_use]
    pub fn marks_hold_head(payload: u32) -> bool {
        payload & Self::HOLD_HEAD_TEST_MASK == Self::HOLD_HEAD_TEST_BITS
    }

    /// Whether a chart payload word marks the paired hold tail: bit 24
    /// set (see [`Self::HOLD_TAIL_TEST_MASK`]).
    #[must_use]
    pub fn marks_hold_tail(payload: u32) -> bool {
        payload & Self::HOLD_TAIL_TEST_MASK != 0
    }

    /// Judge a head strike with the shared strike windows: the key-down
    /// walk branch tiers |hit − record time| against the same 6/18/25
    /// bounds as taps (compare chain at files `0x176b7f`–`0x176ba7`).
    /// The strike rides the key-down: the input forward inversion
    /// (dossier `ev_cea19f7d…`) plus the pressed-flag init to 1 at file
    /// `0x179b61` put the byte-0 walk edge on the press (see
    /// [`HoldState`] item 5 and `docs/gameplay.md`).
    #[must_use]
    pub fn judge_head(&self, hit_ms: u64, windows: &TimingWindows) -> Judgement {
        windows.judge(saturating_offset_ms(hit_ms, self.head_ms))
    }

    /// Record-expiry rule of the scan `FUN_005772f0`: an unjudged record
    /// becomes a Miss once `now > record_ms + windows.bad` (grace add
    /// at file `0x1773bf`, Miss store at file `0x17740f`). The scan
    /// applies it to the head and — while the head is still unjudged —
    /// also marks the paired tail Miss (file `0x1774e2`). Its reach is
    /// bounded by the per-lane cursor pair `+0x480[lane]`/`+0x484[lane]`,
    /// loaded once as `{list head, list head}` at chart load (files
    /// `0x172d32`/`0x172d39`) and never advanced by any site — every
    /// in-exe walk advances stack copies only. The walk therefore always
    /// grace-breaks on the list-head node itself (its payload+8 field is
    /// a link pointer, far above any song time, files `0x1773bf`–
    /// `0x1773c5`), so this Miss store is unreachable on shipped cursors
    /// and a held tail can never be expired. The walk's own stop-marker
    /// `*(header+4)` is the lane's earliest real note — insert-before-
    /// tail pins it there and no terminator record exists (fourth
    /// trace, `docs/gameplay.md`) — and every walk breaks on the bound
    /// before reading that node; see `docs/gameplay.md`.
    #[must_use]
    pub fn is_expired(record_ms: u64, now_ms: u64, windows: &TimingWindows) -> bool {
        record_ms.saturating_add(windows.bad) < now_ms
    }
}

/// Per-lane long-note hold state, VERIFIED 7.55 (`FUN_00576540` /
/// `FUN_00575fa0` / `FUN_00578110`, dossier `ev_1baf7ef1…` for the
/// scan; binary md5 `34b789ff…`; second trace evidence `ev_9a80eb3b…`
/// decompile, `ev_9faabadd…` raw instructions).
///
/// The in-exe machine has no release gate on the hold flag — releasing
/// early stops nothing — but the tail itself is player-rated:
///
/// 1. A judged head raises the per-lane hold-active flag `+0x9f8` —
///    walk-branch write at file `0x176d78` gated to the Cool/Good tiers
///    (files `0x176c86`–`0x176c90`, `FUN_00576540`), second-judge
///    write at file `0x17617b` (hold-bit test `0x176105`,
///    `FUN_00575fa0`), auto-play write at file `0x17726b`. It also
///    latches the hold span `+0x9c0`/`+0x9dc` from the paired records
///    (files `0x176197`/`0x1761af`).
/// 2. The flag never clears mid-song: its only clear sites are the
///    start/restart loops (files `0x18ec0b`/`0x18ed25`/`0x18ee3b`), and
///    its only read is the bonus scan (file `0x178194`). Early releases
///    are not detected — the bonus keeps ticking to the span end.
/// 3. The per-frame scan `FUN_00578110` ticks the +10 bonus only while
///    the flag is set and `span_start < now < span_end` (both strict:
///    files `0x1781c1`/`0x1781d7`); it issues no judgement.
/// 4. The strike walk judges the nearest unjudged non-tail record with
///    the strike windows (candidate walk files `0x1767c9`–`0x176b00`,
///    bit-24 exclusions at files `0x176837`/`0x176994`). A Cool/Good
///    hit on a hold head arms the per-lane tail cursor `+0x864[lane]`
///    at the paired tail (file `0x176e60`); a Bad hit instead kills the
///    tail `0x180` (files `0x176dab`–`0x176df6`).
/// 5. The tail rates on the release strike: the other input branch
///    judges the record at `+0x864[lane]` against the tail time with
///    the shared windows (files `0x176646`–`0x176687`) and clears the
///    cursor (file `0x176709`) — modeled by [`HoldState::judge_tail`].
///    The press/release polarity is settled statically: the drained
///    queue's kind 1 (press, pushed at files `0xe17ae`/`0xe185f`, kept
///    to the in-play states) arrives as pressed argument 1 and kind 2
///    (release, file `0xe1afa`, ungated) as argument 0 (dossier
///    `ev_c01db4a8…`); the forward `FUN_005714b0` inverts it (pressed
///    ≠ 0 forwards 0, == 0 forwards 1, dossier `ev_cea19f7d…`); the
///    edge detector copies that value into the event byte (dossier
///    `ev_248c9713…`) with pressed flags `+0x408[lane]` initialized
///    to 1 (file `0x179b61`) — 'start released' under the inversion —
///    so byte 0 rides the key-down and byte 1 the key-up. Without the
///    inversion the flag-1 init would swallow every key-down edge.
///    The raw DirectInput pressed bit reaches the drain uninverted
///    (scancode→lane poster `FUN_00544b50` → `FUN_005429b0`, keyboard
///    device dossier `ev_eb523feb…`), so the single forward inversion
///    is the polarity point.
/// 6. The expiry scan cannot Miss a held tail: its walk runs on the
///    per-lane cursor pair `+0x480[lane]`/`+0x484[lane]`, loaded once
///    as `{list head, list head}` at chart load (files `0x172d32`/
///    `0x172d39`) and never advanced by any site (the judges and the
///    scan advance stack copies only). Its first record access lands on
///    the list-head node itself, whose payload+8 field holds a link
///    pointer far above any song time, so the grace test breaks the
///    walk at once (files `0x1773bf`–`0x1773c5`); the Miss store (file
///    `0x17740f`) and the paired-tail marking (files `0x177478`–
///    `0x1774e2`) are unreachable on shipped cursors. The scan's walk
///    bound `current == *(header+4)` targets the lane's earliest real
///    note — insert-before-tail (`FUN_00578990`, file `0x178990`)
///    pins the first-inserted note in that slot and no terminator
///    record exists (fourth trace, `docs/gameplay.md`) — and the walk
///    breaks on the bound before dereferencing it (bound file
///    `0x177393`, deref file `0x1773b1`). In assist modes
///    the auto judges instead rate every due record — tails included,
///    no bit-24 exclusion — Cool `0x10` (`FUN_00577090`/`FUN_00575fa0`,
///    evidence `ev_d6802fde…`/`ev_b83c2c24…`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldState {
    span: HoldSpan,
    /// Hold-active flag, VERIFIED: game `+0x9f8[lane]` — raised by a
    /// Cool/Good head, never cleared mid-song (see item 2 above).
    armed: bool,
    /// Tail-strike cursor armed, VERIFIED: game `+0x864[lane]` — armed
    /// at the paired tail by a Cool/Good head (file `0x176e60`),
    /// consumed by the next release strike (file `0x176709`).
    tail_armed: bool,
}

impl HoldState {
    /// Idle hold for one chart span: not armed, nothing judged.
    #[must_use]
    pub fn new(span: HoldSpan) -> Self {
        Self {
            span,
            armed: false,
            tail_armed: false,
        }
    }

    /// Whether the per-lane hold-active flag `+0x9f8` is raised.
    #[must_use]
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// The chart span this state tracks.
    #[must_use]
    pub fn span(&self) -> HoldSpan {
        self.span
    }

    /// Apply a judged head strike. Cool/Good arm the hold (both tiers
    /// pass the verified gate) and arm the tail strike cursor at the
    /// paired tail (file `0x176e60`). Bad leaves both unarmed and — per
    /// the verified tail kill — the paired tail is rated `0x180` Miss
    /// by the caller.
    pub fn apply_head(&mut self, judgement: Judgement) {
        if matches!(judgement, Judgement::Cool | Judgement::Good) {
            self.armed = true;
            self.tail_armed = true;
        }
    }

    /// Judge the release strike on the tail, VERIFIED 7.55 (files
    /// `0x176646`–`0x176687`): while the tail cursor is armed, the next
    /// release event rates the tail record against the tail time with
    /// the shared strike windows — `Miss` once the release sits outside
    /// the Bad bound on either side — and consumes the cursor (file
    /// `0x176709`). Returns `None` when no strike is due: the head was
    /// never struck, hit Bad (tail killed `0x180`), or the strike was
    /// already consumed.
    pub fn judge_tail(&mut self, release_ms: u64, windows: &TimingWindows) -> Option<Judgement> {
        if !self.tail_armed {
            return None;
        }
        self.tail_armed = false;
        Some(windows.judge(saturating_offset_ms(release_ms, self.span.tail_ms)))
    }

    /// Whether this hold feeds one [`LongNoteBonus::hold_tick`] now:
    /// the verified scan gate — armed, and the span window open
    /// (`span_start < now < span_end`, strict on both ends).
    #[must_use]
    pub fn hold_tick_due(&self, now_ms: u64) -> bool {
        self.armed
            && now_ms.saturating_sub(self.span.head_ms) > 0
            && self.span.tail_ms.saturating_sub(now_ms) > 0
    }
}

/// Signed ms offset `later − earlier`, saturating at `i64` bounds so a
/// strike time can never wrap when fed to [`TimingWindows::judge`].
#[must_use]
fn saturating_offset_ms(later_ms: u64, earlier_ms: u64) -> i64 {
    let later = i64::try_from(later_ms).unwrap_or(i64::MAX);
    let earlier = i64::try_from(earlier_ms).unwrap_or(i64::MAX);
    later.saturating_sub(earlier)
}

/// Deprecated pre-trace long-note model, kept only until
/// `o2jam-shell`'s release path migrates to [`HoldState`].
///
/// `is_expired` matches the verified record-expiry rule
/// ([`HoldSpan::is_expired`]). `judge_release` matches the VERIFIED
/// release-strike arithmetic — the in-exe `+0x864` branch rates the
/// armed tail with the shared windows against the tail time (files
/// `0x176646`–`0x176687`) — but lacks its verified arming gate and
/// one-shot cursor semantics; [`HoldState::judge_tail`] models both.
/// `is_early` has no in-exe counterpart: the strike Misses on |offset|
/// alone, not on release-before-tail.
#[deprecated(
    since = "0.1.0",
    note = "superseded by `HoldState`; it lacks the verified +0x864 arming gate"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LongTail {
    /// Head onset in ms.
    pub head_ms: u64,
    /// Tail (release point) in ms.
    pub tail_ms: u64,
}

#[allow(deprecated)]
impl LongTail {
    /// Hold from head to tail; `None` when the span is empty (a tap).
    #[must_use]
    pub fn new(head_ms: u64, tail_ms: u64) -> Option<Self> {
        (tail_ms > head_ms).then_some(Self { head_ms, tail_ms })
    }

    /// A release before the window is mid-hold: Miss.
    /// REFUTED for 7.55 — kept for `o2jam-shell` compatibility only.
    #[must_use]
    pub fn is_early(&self, release_ms: u64, windows: &TimingWindows) -> bool {
        release_ms.saturating_add(windows.bad) < self.tail_ms
    }

    /// No release by the window end: Miss (caller rates it on advance).
    #[must_use]
    pub fn is_expired(&self, now_ms: u64, windows: &TimingWindows) -> bool {
        HoldSpan::is_expired(self.tail_ms, now_ms, windows)
    }

    /// Judge a release inside the window with the strike tiers.
    /// REFUTED for 7.55 — kept for `o2jam-shell` compatibility only.
    #[must_use]
    pub fn judge_release(&self, release_ms: u64, windows: &TimingWindows) -> Judgement {
        windows.judge(saturating_offset_ms(release_ms, self.tail_ms))
    }
}

/// Bonus one held long-note tick feeds (VERIFIED 7.55: `+10` to the hold
/// accumulator at game `+0x3978` on every tick while held — write at file
/// `0x14163c` via the combo helper, called at file `0x178200`).
pub const HOLD_TICK_BONUS_POINTS: u64 = 10;

/// Accumulated bonus for held long notes — VERIFIED 7.55.
///
/// While a hold stays active (per-lane flag at game `+0x9f8`, raised at
/// the Cool/Good hold head), the per-frame hold scan `FUN_00578110`
/// (file `0x178110`, dossier `ev_737dfe7b…`) advances each held lane
/// cursor by `+6` (file `0x1781f6`) and runs the combo helper
/// `FUN_00541620` (call at file `0x178200`), which adds
/// [`HOLD_TICK_BONUS_POINTS`] to the hold accumulator (game `+0x3978`,
/// sub-object `+0x2158`, write at file `0x14163c`) plus `+1` combo
/// (write at file `0x141652`; mode byte `6` exempts only the `+10`,
/// mode TBD so the exemption is not modeled — same as pills).
/// The accumulator is the lead candidate for the dedicated
/// `longNoteScore` slot in `SubmitScore` (4016) / `UpdateGameStats`
/// (4014) — layout in `net-protocol.md`; the submit-side read runs
/// through a vtable path, so that link is corroborated, not proven.
///
/// Gate [`LongNoteBonus::hold_tick`] with [`HoldState::hold_tick_due`],
/// mirroring the verified scan gate (flag check at file `0x178194`, span
/// window at files `0x1781c1`/`0x1781d7`): one call per held tick,
/// nothing on taps, before the head, or past the tail.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LongNoteBonus {
    points: u64,
    ticks: u64,
}

impl LongNoteBonus {
    /// Empty bonus: no held ticks yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Bonus points accumulated (`ticks * 10`).
    #[must_use]
    pub fn points(&self) -> u64 {
        self.points
    }

    /// Held ticks counted.
    #[must_use]
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Record one tick of an active hold: `+10` bonus points.
    pub fn hold_tick(&mut self) {
        self.ticks = self.ticks.saturating_add(1);
        self.points = self.points.saturating_add(HOLD_TICK_BONUS_POINTS);
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

/// Per-difficulty per-judgement life deltas, VERIFIED 7.55: the
/// difficulty switch `FUN_00570f90` (files `0x170fa2`–`0x171056`, ev
/// `feebe490…`, called at game init right after the difficulty byte is
/// stored to `this+0x34`, files `0x1701f2`/`0x170206`) loads one set of
/// four i32 immediates into the delta fields `+0xe88` (Cool), `+0xe8c`
/// (Good), `+0xe90` (Bad), `+0xe94` (Miss): Easy {+3, +2, −10, −50}
/// (files `0x170fbc`/`0x170fc9`/`0x170fd6`/`0x170fe3`), Normal {+2, +1,
/// −7, −40} (files `0x170ff2`/`0x170fff`/`0x17100c`/`0x171019`), Hard
/// {+1, 0, −5, −30} (files `0x171028`/`0x171035`/`0x171042`/`0x17104f`).
/// A difficulty outside 0..=2 takes the switch fallthrough and selects
/// nothing (file `0x170fb4`). Bad takes −10/−7/−5 — the player-reported
/// Bad=0 row was wrong (see `docs/gameplay.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifeDeltas {
    /// Per-Cool change (field `+0xe88`).
    cool: i32,
    /// Per-Good change (field `+0xe8c`).
    good: i32,
    /// Per-Bad change (field `+0xe90`).
    bad: i32,
    /// Per-Miss change (field `+0xe94`).
    miss: i32,
}

impl LifeDeltas {
    /// Easy arm {+3, +2, −10, −50} (files `0x170fbc`/`0x170fc9`/
    /// `0x170fd6`/`0x170fe3`).
    pub const EASY: Self = Self {
        cool: 3,
        good: 2,
        bad: -10,
        miss: -50,
    };

    /// Normal arm {+2, +1, −7, −40} (files `0x170ff2`/`0x170fff`/
    /// `0x17100c`/`0x171019`).
    pub const NORMAL: Self = Self {
        cool: 2,
        good: 1,
        bad: -7,
        miss: -40,
    };

    /// Hard arm {+1, 0, −5, −30} (files `0x171028`/`0x171035`/
    /// `0x171042`/`0x17104f`).
    pub const HARD: Self = Self {
        cool: 1,
        good: 0,
        bad: -5,
        miss: -30,
    };

    /// The set the switch arm selects; `None` outside 0..=2.
    #[must_use]
    pub fn for_difficulty(difficulty: u8) -> Option<Self> {
        match difficulty {
            0 => Some(Self::EASY),
            1 => Some(Self::NORMAL),
            2 => Some(Self::HARD),
            _ => None,
        }
    }

    /// The delta the scorer reads for one judgement tier.
    #[must_use]
    fn delta_for(&self, judgement: Judgement) -> i32 {
        match judgement {
            Judgement::Cool => self.cool,
            Judgement::Good => self.good,
            Judgement::Bad => self.bad,
            Judgement::Miss => self.miss,
        }
    }
}

/// Local life bar, VERIFIED 7.55 end to end.
///
/// Field: the u16 life word at game `+0x3970`, reached as word `+0x2150`
/// on the score sub-object `this+0x1820` (`0x1820 + 0x2150 = 0x3970`) —
/// which is why earlier whole-offset scans of `+0x3970` found reads
/// only. Application: the scorer `FUN_00570510` (file `0x170510`)
/// selects the tier's delta field per judgement (Cool/Good/Bad/Miss →
/// `+0xe88`/`+0xe8c`/`+0xe90`/`+0xe94`, files `0x17056c`/`0x1705e6`/
/// `0x170806`/`0x170861`) and calls adder `0x540a80` (files
/// `0x17057c`/`0x1705f6`/`0x170816`/`0x170871`).
///
/// Adder `0x540a80` (file `0x140a80`) adds the i32 delta into the u16
/// word (write `0x140a97`), clamps to [`LIFE_MAX`] when a positive delta
/// lands above 1000 (files `0x140ab6`–`0x140abf`), and snaps to 0 when a
/// negative delta's u16 wrap lands above 1000 — the wrap only happens on
/// underflow, so the range is saturating over `0..=[LIFE_MAX]`.
///
/// Death gate: the scorer snapshots the life word at entry (file
/// `0x17051c`) and skips every tier block while it is 0 (files
/// `0x170557`/`0x1705d1`/`0x1707f1`/`0x17083c`), so a dead bar ignores
/// further judgements; the killing judgement still applies its delta.
/// On any change the scorer fires the gauge UI event (files
/// `0x17097e`–`0x170997`). The bar starts full: 1000 is pushed into the
/// life setter `0x5407b0` on the global game object (file `0x4dae7c`,
/// re-verified from bytes; setter at file `0x1407b0`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifeBar {
    /// Life remaining (`0..=LIFE_MAX`; the exe's u16 field `+0x3970`).
    life: u16,
    /// Difficulty-selected per-judgement deltas.
    deltas: LifeDeltas,
}

impl LifeBar {
    /// A full bar on the Easy set — the exe's init state (difficulty
    /// byte defaults to 0, file `0x9515c`; life set to 1000, file
    /// `0x4dae7c`).
    #[must_use]
    pub fn starting() -> Self {
        Self {
            life: LIFE_MAX,
            deltas: LifeDeltas::EASY,
        }
    }

    /// A full bar on `difficulty`'s delta set (0 = Easy, 1 = Normal,
    /// 2 = Hard); `None` outside 0..=2, where the switch selects
    /// nothing (file `0x170fb4`).
    #[must_use]
    pub fn new(difficulty: u8) -> Option<Self> {
        Some(Self {
            life: LIFE_MAX,
            deltas: LifeDeltas::for_difficulty(difficulty)?,
        })
    }

    /// Life remaining (`0..=[LIFE_MAX]`).
    #[must_use]
    pub fn life(&self) -> u16 {
        self.life
    }

    /// Whether the bar is empty (the scorer's dead state).
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.life == 0
    }

    /// Apply one judgement's delta. No-op while dead, mirroring the
    /// scorer's entry gate (file `0x17051c`, tests `0x170557`/`0x1705d1`/
    /// `0x1707f1`/`0x17083c`) so the type is safe to drive directly.
    pub fn apply(&mut self, judgement: Judgement) {
        if self.is_dead() {
            return;
        }
        self.add(self.deltas.delta_for(judgement));
    }

    /// One pass of adder `0x540a80` (file `0x140a80`): absorb the i32
    /// delta into the u16 word, clamp positive overflow at [`LIFE_MAX`]
    /// (file `0x140ab9`), snap negative underflow to 0 (file `0x140adf`).
    fn add(&mut self, delta: i32) {
        let summed = i32::from(self.life) + delta;
        if delta > 0 && summed > i32::from(LIFE_MAX) {
            self.life = LIFE_MAX;
        } else if delta < 0 && summed < 0 {
            self.life = 0;
        } else {
            self.life = u16::try_from(summed).unwrap_or(LIFE_MAX);
        }
    }
}

impl Default for LifeBar {
    fn default() -> Self {
        Self::starting()
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
    /// Life bar (u16 `+0x3970`, per-difficulty deltas — VERIFIED).
    pub life: LifeBar,
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
    ///
    /// Dead gate VERIFIED (`FUN_00570510` files `0x170557`/`0x1705d1`/
    /// `0x1707f1`/`0x17083c`): once the life word reads 0, every tier
    /// block is skipped, so counts, jam, combo, score and life all
    /// freeze; only a per-note display word clears (file `0x17099b`,
    /// out of model scope). The killing judgement still applies.
    pub fn apply(&mut self, j: Judgement) {
        if self.life.is_dead() {
            return;
        }
        self.counts[tier_index(j)] += 1;
        self.life.apply(j);
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
    /// path) and feeds jam AND life like a Cool (the dispatch calls
    /// the scorer with tier arg 4, file `0x16e880`, dossier
    /// `ev_c168abac…`, so the scorer's Cool life add runs verbatim);
    /// Bad with no pill and Miss break combo and reset jam (Miss
    /// spends no pill — verified absence of pill logic on its path).
    /// The same dead gate as [`Score::apply`] freezes everything; the
    /// pill dispatch runs upstream of the scorer and its dead-state
    /// behavior is untraced, so the model conservatively freezes pill
    /// spend too — an implementation choice, not an exe claim.
    pub fn apply_rated(&mut self, judgement: Judgement, pills: &mut PillStock) {
        if self.life.is_dead() {
            return;
        }
        self.counts[tier_index(judgement)] += 1;
        if judgement == Judgement::Bad && pills.consume_to_keep_combo() {
            self.life.apply(Judgement::Cool);
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
        self.life.apply(judgement);
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
    fn jam_level_progress_tracks_position_inside_the_level() {
        let mut jam = JamState::new();
        assert_eq!(jam.level_progress(), 0);
        jam.note(Judgement::Cool);
        assert_eq!(jam.level_progress(), JAM_PROGRESS_PER_COOL);
        for _ in 0..24 {
            jam.note(Judgement::Cool);
        }
        assert_eq!((jam.level(), jam.level_progress()), (1, 0));
        jam.note(Judgement::Good);
        assert_eq!((jam.level(), jam.level_progress()), (1, 1));
    }

    #[test]
    fn jam_level_span_matches_verified_threshold_step() {
        assert_eq!(JAM_PROGRESS_PER_LEVEL, 50);
        assert_eq!(jam_threshold_for(1) - jam_threshold_for(0), 50);
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
    fn hold_span_rejects_empty_spans() {
        assert!(HoldSpan::new(1000, 1000).is_none());
        assert!(HoldSpan::new(1000, 999).is_none());
        assert!(HoldSpan::new(1000, 1500).is_some());
    }

    #[test]
    fn bit_25_only_payload_marks_hold_head() {
        // Head flag word `0x200` plus sample ref (B16 layout), bit 24 clear.
        let payload = 0x0200_0000 | 0x11;
        assert!(HoldSpan::marks_hold_head(payload));
        assert!(!HoldSpan::marks_hold_tail(payload));
    }

    #[test]
    fn bits_25_and_24_payload_marks_tail_not_head() {
        // Shipped tails keep the head bit set (flag word `0x300`); the
        // bit-24 test still claims it and the head equality rejects it.
        let payload = 0x0300_0000 | 0x22;
        assert!(HoldSpan::marks_hold_tail(payload));
        assert!(!HoldSpan::marks_hold_head(payload));
    }

    #[test]
    fn bit_24_only_payload_marks_tail() {
        let payload = 0x0100_0000 | 0x33;
        assert!(HoldSpan::marks_hold_tail(payload));
        assert!(!HoldSpan::marks_hold_head(payload));
    }

    #[test]
    fn bit_26_payload_marks_neither_head_nor_tail() {
        // Bit 26 is judge-invisible (negative sweep 0x5703f0–0x578300):
        // its only payload consumers are parser-side (survey 0x172a4f,
        // fix-up 0x173061) and the sample bank +0x3E8 (0x170427).
        let payload = 0x0400_0000;
        assert!(!HoldSpan::marks_hold_head(payload));
        assert!(!HoldSpan::marks_hold_tail(payload));
    }

    #[test]
    fn tap_payload_marks_neither_head_nor_tail() {
        assert!(!HoldSpan::marks_hold_head(0));
        assert!(!HoldSpan::marks_hold_tail(0));
    }

    #[test]
    fn head_strike_uses_shared_strike_windows() {
        let windows = TimingWindows::default();
        let span = HoldSpan::new(10_000, 12_000).expect("hold");
        assert_eq!(
            span.judge_head(10_000 + windows.cool, &windows),
            Judgement::Cool
        );
        assert_eq!(
            span.judge_head(10_000 + windows.good, &windows),
            Judgement::Good
        );
        assert_eq!(
            span.judge_head(10_000 + windows.bad, &windows),
            Judgement::Bad
        );
        assert_eq!(
            span.judge_head(10_000 + windows.bad + 1, &windows),
            Judgement::Miss
        );
        assert_eq!(
            span.judge_head(10_000 - windows.cool, &windows),
            Judgement::Cool
        );
    }

    #[test]
    fn cool_or_good_head_arms_hold_and_bad_does_not() {
        let span = HoldSpan::new(1000, 2000).expect("hold");
        let mut cool = HoldState::new(span);
        cool.apply_head(Judgement::Cool);
        assert!(cool.is_armed());
        let mut good = HoldState::new(span);
        good.apply_head(Judgement::Good);
        assert!(good.is_armed());
        let mut bad = HoldState::new(span);
        bad.apply_head(Judgement::Bad);
        assert!(!bad.is_armed());
        let mut missed = HoldState::new(span);
        missed.apply_head(Judgement::Miss);
        assert!(!missed.is_armed());
    }

    #[test]
    fn hold_tick_due_needs_armed_hold_and_open_span() {
        let span = HoldSpan::new(1000, 2000).expect("hold");
        let mut hold = HoldState::new(span);
        assert!(!hold.hold_tick_due(1500));
        hold.apply_head(Judgement::Good);
        assert!(!hold.hold_tick_due(999));
        assert!(!hold.hold_tick_due(1000));
        assert!(hold.hold_tick_due(1001));
        assert!(hold.hold_tick_due(1999));
        assert!(!hold.hold_tick_due(2000));
        assert!(!hold.hold_tick_due(2001));
    }

    #[test]
    fn tail_strike_needs_a_cool_or_good_head() {
        let span = HoldSpan::new(1000, 2000).expect("hold");
        let windows = TimingWindows::default();
        let mut fresh = HoldState::new(span);
        assert_eq!(fresh.judge_tail(2000, &windows), None);
        let mut missed = HoldState::new(span);
        missed.apply_head(Judgement::Miss);
        assert_eq!(missed.judge_tail(2000, &windows), None);
        let mut bad = HoldState::new(span);
        bad.apply_head(Judgement::Bad);
        assert_eq!(bad.judge_tail(2000, &windows), None);
    }

    #[test]
    fn tail_strike_rates_release_offset_vs_tail_time_then_disarms() {
        let span = HoldSpan::new(1000, 2000).expect("hold");
        let windows = TimingWindows::default();
        let mut hold = HoldState::new(span);
        hold.apply_head(Judgement::Cool);
        assert_eq!(
            hold.judge_tail(2000 - windows.cool, &windows),
            Some(Judgement::Cool)
        );
        assert_eq!(hold.judge_tail(2000, &windows), None);
        let mut early = HoldState::new(span);
        early.apply_head(Judgement::Good);
        assert_eq!(
            early.judge_tail(2000 - windows.bad - 1, &windows),
            Some(Judgement::Miss)
        );
        let mut late = HoldState::new(span);
        late.apply_head(Judgement::Good);
        assert_eq!(
            late.judge_tail(2000 + windows.bad + 1, &windows),
            Some(Judgement::Miss)
        );
        let mut borderline = HoldState::new(span);
        borderline.apply_head(Judgement::Good);
        assert_eq!(
            borderline.judge_tail(2000 + windows.bad, &windows),
            Some(Judgement::Bad)
        );
    }

    #[test]
    fn tail_strike_keeps_the_hold_flag_and_bonus_ticking() {
        // +0x9f8 never clears mid-song; only the +0x864 cursor is
        // consumed by the release strike (files 0x176709 / 0x178194).
        let span = HoldSpan::new(1000, 2000).expect("hold");
        let windows = TimingWindows::default();
        let mut hold = HoldState::new(span);
        hold.apply_head(Judgement::Good);
        assert_eq!(hold.judge_tail(2000, &windows), Some(Judgement::Cool));
        assert!(hold.is_armed());
        assert!(hold.hold_tick_due(1500));
    }

    #[test]
    fn unjudged_record_expires_past_bad_window() {
        let windows = TimingWindows::default();
        assert!(!HoldSpan::is_expired(2000, 2000 + windows.bad, &windows));
        assert!(HoldSpan::is_expired(2000, 2000 + windows.bad + 1, &windows));
    }

    #[test]
    fn fresh_hold_bonus_holds_nothing() {
        let bonus = LongNoteBonus::new();
        assert_eq!(bonus.ticks(), 0);
        assert_eq!(bonus.points(), 0);
    }

    #[test]
    fn hold_tick_grants_ten_bonus_points() {
        let mut bonus = LongNoteBonus::new();
        bonus.hold_tick();
        assert_eq!(bonus.ticks(), 1);
        assert_eq!(bonus.points(), HOLD_TICK_BONUS_POINTS);
    }

    #[test]
    fn hold_bonus_accumulates_while_held() {
        let mut bonus = LongNoteBonus::new();
        for _ in 0..5 {
            bonus.hold_tick();
        }
        assert_eq!(bonus.ticks(), 5);
        assert_eq!(bonus.points(), 5 * HOLD_TICK_BONUS_POINTS);
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
    fn life_delta_sets_match_the_verified_switch_arms() {
        let easy = LifeDeltas::for_difficulty(0).expect("easy");
        assert_eq!((easy.cool, easy.good), (3, 2));
        assert_eq!((easy.bad, easy.miss), (-10, -50));
        let normal = LifeDeltas::for_difficulty(1).expect("normal");
        assert_eq!((normal.cool, normal.good), (2, 1));
        assert_eq!((normal.bad, normal.miss), (-7, -40));
        let hard = LifeDeltas::for_difficulty(2).expect("hard");
        assert_eq!((hard.cool, hard.good), (1, 0));
        assert_eq!((hard.bad, hard.miss), (-5, -30));
    }

    #[test]
    fn unknown_difficulty_selects_no_life_set() {
        assert!(LifeDeltas::for_difficulty(3).is_none());
        assert!(LifeDeltas::for_difficulty(u8::MAX).is_none());
        assert!(LifeBar::new(3).is_none());
    }

    #[test]
    fn bar_starts_full_on_every_difficulty() {
        assert_eq!(LifeBar::default().life(), LIFE_MAX);
        for difficulty in 0..3 {
            let bar = LifeBar::new(difficulty).expect("set");
            assert_eq!(bar.life(), LIFE_MAX);
            assert!(!bar.is_dead());
        }
    }

    #[test]
    fn cool_on_a_full_bar_clamps_at_1000() {
        let mut bar = LifeBar::starting();
        bar.apply(Judgement::Cool);
        assert_eq!(bar.life(), LIFE_MAX);
    }

    #[test]
    fn hard_good_changes_nothing() {
        let mut bar = LifeBar::new(2).expect("hard");
        bar.apply(Judgement::Good);
        assert_eq!(bar.life(), LIFE_MAX);
    }

    #[test]
    fn easy_misses_drain_and_floor_at_zero() {
        let mut bar = LifeBar::starting();
        for _ in 0..19 {
            bar.apply(Judgement::Miss);
        }
        assert_eq!(bar.life(), 50);
        bar.apply(Judgement::Miss);
        assert_eq!(bar.life(), 0);
        assert!(bar.is_dead());
    }

    #[test]
    fn dead_bar_ignores_further_deltas() {
        let mut bar = LifeBar::starting();
        for _ in 0..20 {
            bar.apply(Judgement::Miss);
        }
        bar.apply(Judgement::Cool);
        assert_eq!(bar.life(), 0);
    }

    #[test]
    fn normal_bar_drains_at_its_verified_rate() {
        let mut bar = LifeBar::new(1).expect("normal");
        for _ in 0..24 {
            bar.apply(Judgement::Miss);
        }
        assert_eq!(bar.life(), 40);
        bar.apply(Judgement::Good);
        assert_eq!(bar.life(), 41);
        bar.apply(Judgement::Miss);
        assert_eq!(bar.life(), 1);
        bar.apply(Judgement::Miss);
        assert_eq!(bar.life(), 0);
    }

    #[test]
    fn score_freezes_once_life_hits_zero() {
        let mut score = Score::default();
        for _ in 0..20 {
            score.apply(Judgement::Miss);
        }
        assert!(score.life.is_dead());
        let frozen = (score.points, score.combo, score.counts);
        score.apply(Judgement::Cool);
        score.apply(Judgement::Miss);
        assert_eq!((score.points, score.combo, score.counts), frozen);
    }

    #[test]
    fn killing_miss_still_applies_its_accounting() {
        let mut score = Score::default();
        for _ in 0..19 {
            score.apply(Judgement::Miss);
        }
        assert_eq!(score.life.life(), 50);
        let misses = score.counts[3];
        score.apply(Judgement::Miss);
        assert_eq!(score.life.life(), 0);
        assert_eq!(score.counts[3], misses + 1);
    }

    #[test]
    fn rated_pill_saved_bad_takes_the_cool_life_delta() {
        let mut score = Score::default();
        let mut pills = PillStock::new(1);
        for _ in 0..18 {
            score.apply_rated(Judgement::Miss, &mut pills);
        }
        assert_eq!(score.life.life(), 100);
        score.apply_rated(Judgement::Cool, &mut pills);
        assert_eq!((score.life.life(), score.combo), (103, 1));
        score.apply_rated(Judgement::Bad, &mut pills);
        assert_eq!(pills.count(), 0);
        assert_eq!((score.life.life(), score.combo), (106, 1));
        assert_eq!(score.counts, [1, 0, 1, 18]);
    }

    /// Parity model of the exe's per-lane note-list discipline (fourth
    /// trace, `docs/gameplay.md` item 6): `FUN_00578990` (file
    /// `0x178990`) links every record immediately before the current
    /// `*(header+4)` slot, and the loader walks chart events in
    /// ascending measure order — so the first-inserted record of a lane
    /// (its earliest note) is pinned in the tail slot forever, the
    /// remaining records keep chart order in front of it, and iteration
    /// from the header runs ascending in time ending at that pinned
    /// note. The slot vector stores records in walk order; its last
    /// element mirrors `*(header+4)`. No terminator record exists.
    #[derive(Default)]
    struct LaneListParity {
        walk_order: Vec<u64>,
    }

    impl LaneListParity {
        /// `FUN_00578990`: link one record in front of the tail slot;
        /// the first insert into an empty list takes the tail slot.
        fn insert_before_tail(&mut self, record_ms: u64) {
            let tail_slot = self.walk_order.len().saturating_sub(1);
            self.walk_order.insert(tail_slot, record_ms);
        }

        /// A candidate walk (`FUN_00576540` loop bound at file
        /// `0x1767e5`, expiry scan bound at file `0x177393`): both test
        /// `cursor == *(header+4)` before their first dereference, so
        /// the tail-slot record is never visited.
        fn walked_records(&self) -> &[u64] {
            let bound_slot = self.walk_order.len().saturating_sub(1);
            &self.walk_order[..bound_slot]
        }
    }

    #[test]
    fn lane_list_walk_bound_excludes_the_first_inserted_note() {
        // The stop-marker *(header+4) is the lane's earliest real note,
        // not a terminator: inserting 4000/8000/12000 in chart order
        // pins 4000 in the tail slot and the walk visits only 8000 and
        // 12000, in ascending time order.
        let mut lane = LaneListParity::default();
        lane.insert_before_tail(4_000);
        lane.insert_before_tail(8_000);
        lane.insert_before_tail(12_000);
        assert_eq!(lane.walked_records(), [8_000, 12_000]);
    }

    #[test]
    fn lane_list_single_note_lives_only_in_the_excluded_tail_slot() {
        // A one-note lane: the sole record is the bound slot itself, so
        // a candidate walk visits nothing rather than reading it.
        let mut lane = LaneListParity::default();
        lane.insert_before_tail(2_000);
        assert_eq!(lane.walked_records(), [] as [u64; 0]);
    }
}
