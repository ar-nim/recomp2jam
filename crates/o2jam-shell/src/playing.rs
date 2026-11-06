//! Note highway: seven lanes, judgement line, falling notes.
//!
//! Pure layout + paint over [`LevelChart`] onsets (`lane, ms` from
//! [`LevelChart::hit_onsets_ms`]); timing comes from an explicit
//! [`PlayingView::advance_to`], so tests drive the clock. Layout is
//! relative to the frame: full-width lanes, line one sixth up from
//! the bottom, `lookahead` ms of chart visible above the line.

use o2jam_game::{
    CoolStreak, Judgement, LaneMap, LongTail, MeasureReshuffle, PillStock, Score, TimingWindows,
};
use o2jam_render::{
    draw_number, draw_text, fill_rect, pack_xrgb, text_width, visible_spans, Frame, LaneCurtain,
    Rect,
};

/// Lanes across the highway.
pub const HIGHWAY_LANES: u8 = 7;
/// How far ahead notes appear, in song ms.
pub const DEFAULT_LOOKAHEAD_MS: u64 = 2000;
/// Note block height in pixels.
pub const NOTE_H: i32 = 8;

/// Lane layout for one frame size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighwayGeometry {
    /// Left edge of lane 0.
    pub x0: i32,
    /// Lane width in pixels.
    pub lane_w: u32,
    /// Judgement-line row.
    pub line_y: i32,
    /// Frame height (notes never draw below the line).
    pub h: i32,
}

impl HighwayGeometry {
    /// Full-width lanes; line one sixth up from the bottom.
    #[must_use]
    pub fn centered(frame_w: usize, frame_h: usize) -> Self {
        let lane_w = (frame_w / HIGHWAY_LANES as usize).max(1) as u32;
        Self {
            x0: 0,
            lane_w,
            line_y: frame_h as i32 - frame_h as i32 / 6,
            h: frame_h as i32,
        }
    }

    /// Pixel column of one lane (clamped to lane count).
    #[must_use]
    pub fn lane_rect(&self, lane: u8) -> Rect {
        let lane = lane.min(HIGHWAY_LANES - 1);
        Rect {
            x: self.x0 + i32::from(lane) * self.lane_w as i32,
            y: 0,
            w: self.lane_w,
            h: self.h as u32,
        }
    }
}

/// Scrolling notes for one chart.
#[derive(Debug, Clone, Default)]
pub struct PlayingView {
    /// `(lane, onset ms)`, sorted by onset.
    onsets: Vec<(u8, u64)>,
    /// Current song position in ms.
    now_ms: u64,
    /// Visible window ahead in ms.
    lookahead_ms: u64,
}

impl PlayingView {
    /// Notes to show (empty until a chart loads).
    #[must_use]
    pub fn new(mut onsets: Vec<(u8, u64)>) -> Self {
        onsets.sort_by_key(|(_, at)| *at);
        Self {
            onsets,
            now_ms: 0,
            lookahead_ms: DEFAULT_LOOKAHEAD_MS,
        }
    }

    /// Move the song clock (tests and the frame pump call this).
    pub fn advance_to(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    /// Remove a struck note so it stops falling; false when absent.
    pub fn strike(&mut self, lane: u8, at_ms: u64) -> bool {
        if let Some(i) = self
            .onsets
            .iter()
            .position(|(hit_lane, hit_at)| *hit_lane == lane && *hit_at == at_ms)
        {
            self.onsets.remove(i);
            true
        } else {
            false
        }
    }

    /// Notes on screen as `(lane, y top)`; `y` runs from the top
    /// (lookahead ahead) to the judgement line (due now).
    #[must_use]
    pub fn visible(&self, geom: &HighwayGeometry) -> Vec<(u8, i32)> {
        let px_per_ms = geom.line_y as f32 / self.lookahead_ms as f32;
        let mut notes = Vec::new();
        for (lane, at) in &self.onsets {
            let ahead = at.saturating_sub(self.now_ms);
            if *at <= self.now_ms || ahead > self.lookahead_ms {
                continue;
            }
            notes.push((*lane, geom.line_y - (ahead as f32 * px_per_ms) as i32));
        }
        notes
    }

    /// Remap every onset through a skill-ring lane table.
    pub fn remap_lanes(&mut self, map: &LaneMap) {
        for (lane, _) in &mut self.onsets {
            if let Some(destination) = map.destination(*lane) {
                *lane = destination;
            }
        }
        self.onsets.sort_by_key(|(_, at)| *at);
    }

    /// Paint highway, line, and visible notes.
    pub fn paint(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
        self.paint_shell(frame, geom);
        for (lane, y) in self.visible(geom) {
            Self::paint_note(frame, geom, lane, y);
        }
    }

    /// Paint the highway through a skill-ring curtain: notes outside the
    /// item-text spans (hidden keeps the top 80%, sudden the bottom
    /// 20%, dark the middle 20%, reverse-dark the outer 40% pair —
    /// `o2jam-render::visible_spans`, clip math OPEN) are not drawn.
    /// `Full` paints everything, like [`PlayingView::paint`].
    pub fn paint_curtained(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        curtain: LaneCurtain,
    ) {
        self.paint_shell(frame, geom);
        if curtain == LaneCurtain::Full {
            for (lane, y) in self.visible(geom) {
                Self::paint_note(frame, geom, lane, y);
            }
            return;
        }
        let highway = Rect {
            x: 0,
            y: 0,
            w: frame.w as u32,
            h: geom.line_y.max(0) as u32,
        };
        let spans = visible_spans(curtain, &highway);
        for (lane, y) in self.visible(geom) {
            if spans
                .iter()
                .any(|span| y >= span.y && y < span.y + span.h as i32)
            {
                Self::paint_note(frame, geom, lane, y);
            }
        }
    }

    /// Background, lane separators, and the judgement line (no notes).
    fn paint_shell(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
        let w = frame.w as i32;
        fill_rect(
            frame,
            &Rect {
                x: 0,
                y: 0,
                w: w as u32,
                h: frame.h as u32,
            },
            pack_xrgb(0x0A, 0x0A, 0x0A),
        );
        for lane in 0..HIGHWAY_LANES {
            let column = geom.lane_rect(lane);
            fill_rect(
                frame,
                &Rect {
                    x: column.x + column.w as i32 - 1,
                    y: 0,
                    w: 1,
                    h: column.h,
                },
                pack_xrgb(0x30, 0x30, 0x30),
            );
        }
        fill_rect(
            frame,
            &Rect {
                x: geom.x0,
                y: geom.line_y,
                w: geom.lane_w * u32::from(HIGHWAY_LANES),
                h: 2,
            },
            pack_xrgb(0xE0, 0xE0, 0xE0),
        );
    }

    /// One falling note block.
    fn paint_note(frame: &mut Frame<'_>, geom: &HighwayGeometry, lane: u8, y: i32) {
        let column = geom.lane_rect(lane);
        fill_rect(
            frame,
            &Rect {
                x: column.x + 1,
                y,
                w: column.w.saturating_sub(2),
                h: NOTE_H as u32,
            },
            pack_xrgb(0x40, 0xC0, 0xFF),
        );
    }
}

/// Word shown for each judgement tier.
fn word_of(judgement: Judgement) -> &'static str {
    match judgement {
        Judgement::Cool => "COOL",
        Judgement::Good => "GOOD",
        Judgement::Bad => "BAD",
        Judgement::Miss => "MISS",
    }
}

/// One hittable note: rated once, by key press or by timeout.
/// Long notes carry a tail: after the head is struck the lane is
/// held until the tail, then released exactly there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingNote {
    lane: u8,
    at_ms: u64,
    tail_ms: Option<u64>,
    sample: u32,
    judged: bool,
    holding: bool,
    tail_judged: bool,
}

impl PendingNote {
    /// Tap note (no tail).
    fn tap(lane: u8, at_ms: u64, sample: u32) -> Self {
        Self {
            lane,
            at_ms,
            tail_ms: None,
            sample,
            judged: false,
            holding: false,
            tail_judged: true,
        }
    }

    /// Long note: head at `at_ms`, release at `tail_ms`.
    fn hold(lane: u8, at_ms: u64, tail_ms: u64, sample: u32) -> Self {
        Self {
            lane,
            at_ms,
            tail_ms: Some(tail_ms),
            sample,
            judged: false,
            holding: false,
            tail_judged: false,
        }
    }

    /// Done when the head is judged and the tail (if any) is judged.
    fn done(&self) -> bool {
        self.judged && self.tail_judged
    }
}

/// Live play: scrolling view plus verified judgement/scoring.
///
/// Key presses rate the nearest unjudged note in that lane inside
/// the bad window; notes aging past it rate Miss on advance. Presses
/// with no note nearby rate nothing (the original's empty-press
/// behavior is unverified, so nothing is guessed).
///
/// Skill rings plug in here without touching net/render: remap lanes
/// through a [`LaneMap`], hide notes behind a [`LaneCurtain`], and ask
/// [`LivePlay::should_rebuild_map`] per measure for the panic-style
/// per-measure reshuffle (`MeasureReshuffle`: rebuild every plain
/// measure, once per hold span — `FUN_00574330`).
#[derive(Debug)]
pub struct LivePlay {
    view: PlayingView,
    pending: Vec<PendingNote>,
    score: Score,
    pills: PillStock,
    streak: CoolStreak,
    windows: TimingWindows,
    reshuffle: MeasureReshuffle,
    curtain: LaneCurtain,
    now_ms: u64,
    last_word: Option<(Judgement, u64)>,
    sounds: Vec<u32>,
}

impl Default for LivePlay {
    fn default() -> Self {
        Self {
            view: PlayingView::default(),
            pending: Vec::new(),
            score: Score::default(),
            pills: PillStock::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
        }
    }
}

impl LivePlay {
    /// Notes to play (empty until a chart loads); each onset carries
    /// its key-sound sample id.
    #[must_use]
    pub fn new(onsets: &[(u8, u64, u32)]) -> Self {
        let mut pending: Vec<PendingNote> = onsets
            .iter()
            .map(|(lane, at_ms, sample)| PendingNote::tap(*lane, *at_ms, *sample))
            .collect();
        pending.sort_by_key(|note| note.at_ms);
        Self {
            view: PlayingView::new(
                onsets
                    .iter()
                    .map(|(lane, at_ms, _)| (*lane, *at_ms))
                    .collect(),
            ),
            pending,
            score: Score::default(),
            pills: PillStock::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
        }
    }

    /// Notes with long-note tails: `(lane, head ms, tail ms, sample)`.
    /// A tail equal to (or before) its head degrades to a tap.
    #[must_use]
    pub fn new_holds(entries: &[(u8, u64, u64, u32)]) -> Self {
        let mut pending: Vec<PendingNote> = entries
            .iter()
            .map(|(lane, head_ms, tail_ms, sample)| {
                LongTail::new(*head_ms, *tail_ms).map_or_else(
                    || PendingNote::tap(*lane, *head_ms, *sample),
                    |tail| PendingNote::hold(*lane, tail.head_ms, tail.tail_ms, *sample),
                )
            })
            .collect();
        pending.sort_by_key(|note| note.at_ms);
        Self {
            view: PlayingView::new(
                entries
                    .iter()
                    .map(|(lane, head_ms, _, _)| (*lane, *head_ms))
                    .collect(),
            ),
            pending,
            score: Score::default(),
            pills: PillStock::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
        }
    }

    /// Skill-ring curtain worn for this run (default: none).
    pub fn set_curtain(&mut self, curtain: LaneCurtain) {
        self.curtain = curtain;
    }

    /// Remap every pending lane through a skill-ring table
    /// (mirror / shuffle built by the `LaneMap` constructors).
    pub fn remap_lanes(&mut self, map: &LaneMap) {
        for note in &mut self.pending {
            if let Some(destination) = map.destination(note.lane) {
                note.lane = destination;
            }
        }
        self.pending.sort_by_key(|note| note.at_ms);
        self.view.remap_lanes(map);
    }

    /// Whether the lane table needs a fresh build for the next measure
    /// (panic-style ring: every plain measure, once per hold span).
    pub fn should_rebuild_map(&mut self, measure_has_hold: bool) -> bool {
        self.reshuffle.should_rebuild_map(measure_has_hold)
    }

    /// Move the song clock; notes past the bad window rate Miss.
    /// Heads missed by timeout kill their tails too, and tails with
    /// no release by their window end rate Miss (missing the release
    /// is a miss — long-note rule).
    pub fn advance_to(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        self.view.advance_to(now_ms);
        let windows = self.windows;
        let mut late = Vec::new();
        for note in &mut self.pending {
            if !note.judged && note.at_ms.saturating_add(windows.bad) < now_ms {
                note.judged = true;
                note.tail_judged = true;
                note.holding = false;
                late.push(Judgement::Miss);
            } else if let Some(tail) = note.tail_ms {
                let hold = LongTail {
                    head_ms: note.at_ms,
                    tail_ms: tail,
                };
                if note.holding && !note.tail_judged && hold.is_expired(now_ms, &windows) {
                    note.tail_judged = true;
                    note.holding = false;
                    late.push(Judgement::Miss);
                }
            }
        }
        for judgement in late {
            self.rate(judgement);
        }
    }

    /// Strike a lane: rate the nearest unjudged note in range.
    /// Returns the judgement, or `None` for an empty press.
    pub fn strike_lane(&mut self, lane: u8, now_ms: u64) -> Option<Judgement> {
        self.advance_to(now_ms);
        let mut best: Option<(usize, u64)> = None;
        for (i, note) in self.pending.iter().enumerate() {
            if note.judged || note.lane != lane {
                continue;
            }
            let gap = note.at_ms.abs_diff(now_ms);
            if gap <= self.windows.bad && best.is_none_or(|(_, seen)| gap < seen) {
                best = Some((i, gap));
            }
        }
        let (i, _) = best?;
        let offset = now_ms as i64 - self.pending[i].at_ms as i64;
        let (lane, at_ms, sample) = (
            self.pending[i].lane,
            self.pending[i].at_ms,
            self.pending[i].sample,
        );
        self.pending[i].judged = true;
        self.pending[i].holding = self.pending[i].tail_ms.is_some();
        self.view.strike(lane, at_ms);
        self.sounds.push(sample);
        let judgement = self.windows.judge(offset);
        self.rate(judgement);
        Some(judgement)
    }

    /// Release a held lane: rate the tail of the nearest held note.
    /// Releasing mid-hold (before the tail window) is a Miss;
    /// a release inside the window judges with the strike tiers.
    /// Returns the judgement, or `None` when no lane is held.
    pub fn release_lane(&mut self, lane: u8, now_ms: u64) -> Option<Judgement> {
        self.advance_to(now_ms);
        let mut best: Option<(usize, u64)> = None;
        for (i, note) in self.pending.iter().enumerate() {
            if !note.holding || note.tail_judged || note.lane != lane {
                continue;
            }
            let tail = note.tail_ms.unwrap_or(note.at_ms);
            let gap = tail.abs_diff(now_ms);
            if best.is_none_or(|(_, seen)| gap < seen) {
                best = Some((i, gap));
            }
        }
        let (i, _) = best?;
        let tail = LongTail {
            head_ms: self.pending[i].at_ms,
            tail_ms: self.pending[i].tail_ms.unwrap_or(self.pending[i].at_ms),
        };
        self.pending[i].holding = false;
        self.pending[i].tail_judged = true;
        let judgement = if tail.is_early(now_ms, &self.windows) {
            Judgement::Miss
        } else {
            tail.judge_release(now_ms, &self.windows)
        };
        self.rate(judgement);
        Some(judgement)
    }

    /// Running score and combo.
    #[must_use]
    pub fn score(&self) -> &Score {
        &self.score
    }

    /// Take queued key-sound sample ids (one per struck note).
    pub fn drain_sounds(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.sounds)
    }

    /// Song over: every head and tail judged and the tail (1.5 s) played
    /// out. Empty charts never finish on their own (nothing to play).
    #[must_use]
    pub fn is_finished(&self, now_ms: u64) -> bool {
        let Some(last) = self
            .pending
            .iter()
            .map(|note| note.tail_ms.unwrap_or(note.at_ms))
            .max()
        else {
            return false;
        };
        now_ms >= last.saturating_add(1500) && self.pending.iter().all(|note| note.done())
    }

    fn rate(&mut self, judgement: Judgement) {
        self.streak.note(judgement, &mut self.pills);
        self.score.apply_rated(judgement, &mut self.pills);
        self.last_word = Some((judgement, self.now_ms));
    }

    /// Paint highway, notes, score, combo, and the latest word.
    pub fn paint(&mut self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        self.advance_to(now_ms);
        self.view.paint_curtained(frame, geom, self.curtain);
        let points = u32::try_from(self.score.points).unwrap_or(u32::MAX);
        draw_number(frame, 16, 16, points, pack_xrgb(0xFF, 0xFF, 0xFF));
        if self.score.combo > 0 {
            draw_number(frame, 16, 32, self.score.combo, pack_xrgb(0xFF, 0xFF, 0x40));
        }
        if let Some((judgement, at)) = self.last_word {
            if now_ms.saturating_sub(at) < 600 {
                let word = word_of(judgement);
                let text_w = text_width(word) as i32;
                draw_text(
                    frame,
                    frame.w as i32 / 2 - text_w / 2,
                    geom.line_y - 40,
                    word,
                    pack_xrgb(0xFF, 0xFF, 0xFF),
                );
            }
        }
    }
}

#[cfg(test)]
mod live_play_tests {
    use super::*;

    fn geom() -> HighwayGeometry {
        HighwayGeometry::centered(70, 60)
    }

    #[test]
    fn strike_rates_cool_on_exact_hit() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 1);
        assert_eq!(live.score().points, 200);
        assert!(live.view.visible(&geom()).is_empty());
    }

    #[test]
    fn struck_notes_queue_their_samples() {
        let mut live = LivePlay::new(&[(2, 1000, 7), (2, 1100, 9)]);
        assert_eq!(live.drain_sounds(), Vec::<u32>::new());
        live.strike_lane(2, 1010);
        assert_eq!(live.drain_sounds(), vec![7]);
        assert_eq!(live.drain_sounds(), Vec::<u32>::new());
    }

    #[test]
    fn strike_rates_good_on_early_hit() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 990), Some(Judgement::Good));
        assert_eq!(live.score().points, 100);
    }

    #[test]
    fn strike_rates_bad_on_late_hit() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1020), Some(Judgement::Bad));
        assert_eq!(live.score().combo, 0);
        assert_eq!(live.score().points, 4);
    }

    #[test]
    fn late_notes_rate_miss_on_advance() {
        let mut live = LivePlay::new(&[(1, 1000, 7)]);
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.score().counts[3], 1);
        assert_eq!(live.score().points, 0);
    }

    #[test]
    fn empty_press_rates_nothing() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(0, 1000), None);
        assert_eq!(live.score().counts, [0, 0, 0, 0]);
    }

    #[test]
    fn nearest_note_wins_in_lane() {
        let mut live = LivePlay::new(&[(2, 1000, 7), (2, 1100, 9)]);
        assert_eq!(live.strike_lane(2, 1010), Some(Judgement::Good));
        assert_eq!(live.strike_lane(2, 1100), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 2);
    }

    #[test]
    fn paint_shows_score_combo_and_word() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1100,
        );
        // Score "200" top-left, combo "1" below, "COOL" centered.
        assert_eq!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
        assert_eq!(buf[32 * 70 + 17], pack_xrgb(0xFF, 0xFF, 0x40));
        let word_x = 35 - text_width("COOL") as i32 / 2;
        assert_eq!(
            buf[10 * 70 + word_x as usize + 2],
            pack_xrgb(0xFF, 0xFF, 0xFF)
        );
    }

    #[test]
    fn finishes_after_tail_with_all_judged() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        assert!(!live.is_finished(3000));
        live.strike_lane(0, 1000);
        assert!(!live.is_finished(2000));
        assert!(live.is_finished(2500));
    }

    #[test]
    fn empty_chart_never_finishes() {
        let live = LivePlay::default();
        assert!(!live.is_finished(u64::MAX));
    }

    #[test]
    fn hold_rates_head_then_tail_release() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 2000), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 2);
        assert_eq!(live.score().points, 400);
        assert!(live.is_finished(3500));
    }

    #[test]
    fn hold_early_release_is_a_miss() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 1500), Some(Judgement::Miss));
        assert_eq!(live.score().combo, 0);
    }

    #[test]
    fn hold_missing_release_rates_miss_on_advance() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        live.advance_to(2000 + 25 + 1);
        assert_eq!(live.release_lane(2, 2000 + 25 + 1), None);
        assert_eq!(live.score().counts, [1, 0, 0, 1]);
        assert_eq!(live.score().combo, 0);
    }

    #[test]
    fn hold_missing_head_kills_the_tail() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.release_lane(2, 2000), None);
        assert_eq!(live.score().counts[3], 1);
        assert!(live.is_finished(3500));
    }

    #[test]
    fn empty_tail_degrades_to_tap() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 1000), None);
        assert!(live.is_finished(2500));
    }

    #[test]
    fn lane_remap_moves_pending_and_view() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.remap_lanes(&LaneMap::mirrored(7));
        assert_eq!(live.strike_lane(6, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(0, 1000), None);
    }

    #[test]
    fn lane_remap_keeps_time_order() {
        let mut live = LivePlay::new(&[(0, 1100, 7), (6, 1000, 9)]);
        live.remap_lanes(&LaneMap::mirrored(7));
        assert_eq!(live.strike_lane(0, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(6, 1100), Some(Judgement::Cool));
    }

    #[test]
    fn sudden_curtain_hides_notes_above_the_bottom_fifth() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 1500), (0, 2900)]);
        view.advance_to(1000);
        assert_eq!(view.visible(&geom).len(), 2);
        let mut full = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut full,
                w: 70,
                h: 60,
            },
            &geom,
            LaneCurtain::Full,
        );
        let mut sudden = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut sudden,
                w: 70,
                h: 60,
            },
            &geom,
            LaneCurtain::Sudden,
        );
        let note = pack_xrgb(0x40, 0xC0, 0xFF);
        assert!(full.contains(&note));
        assert!(!sudden.contains(&note));
    }

    #[test]
    fn hidden_curtain_keeps_notes_at_the_top() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 2900)]);
        view.advance_to(1000);
        let mut hidden = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut hidden,
                w: 70,
                h: 60,
            },
            &geom,
            LaneCurtain::Hidden,
        );
        assert!(hidden.contains(&pack_xrgb(0x40, 0xC0, 0xFF)));
    }

    #[test]
    fn reshuffle_rebuilds_every_plain_measure() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(true));
        assert!(!live.should_rebuild_map(true));
    }

    #[test]
    fn word_fades_after_timeout() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1700,
        );
        let word_x = 35 - text_width("COOL") as i32 / 2;
        assert_eq!(
            buf[10 * 70 + word_x as usize + 2],
            pack_xrgb(0x0A, 0x0A, 0x0A)
        );
    }
}

#[cfg(test)]
mod highway_tests {
    use super::*;

    fn geom() -> HighwayGeometry {
        HighwayGeometry::centered(70, 60)
    }

    #[test]
    fn lanes_tile_full_width() {
        let geom = geom();
        assert_eq!(geom.lane_w, 10);
        assert_eq!(geom.lane_rect(0).x, 0);
        assert_eq!(geom.lane_rect(6).x, 60);
        assert_eq!(geom.line_y, 50);
    }

    #[test]
    fn due_note_sits_on_line_and_future_note_tops_out() {
        let mut view = PlayingView::new(vec![(0, 1000), (3, 3000)]);
        let geom = geom();
        view.advance_to(1000);
        // Onset == now is due (struck), not falling.
        assert_eq!(view.visible(&geom), vec![(3, 0)]);
        view.advance_to(2000);
        assert_eq!(view.visible(&geom), vec![(3, 25)]);
    }

    #[test]
    fn old_and_distant_notes_stay_hidden() {
        let mut view = PlayingView::new(vec![(1, 500), (2, 9999)]);
        view.advance_to(1000);
        assert!(view.visible(&geom()).is_empty());
    }

    #[test]
    fn paint_draws_line_and_note_pixels() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 1500)]);
        view.advance_to(1000);
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom,
        );
        // Judgement line spans the lanes; the note sits above it.
        assert_eq!(buf[50 * 70], pack_xrgb(0xE0, 0xE0, 0xE0));
        assert_eq!(buf[50 * 70 + 69], pack_xrgb(0xE0, 0xE0, 0xE0));
        let note_y = 50 - (500.0 * 50.0 / 2000.0) as i32;
        assert_eq!(buf[note_y as usize * 70 + 2], pack_xrgb(0x40, 0xC0, 0xFF));
        assert_eq!(buf[0], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn empty_view_paints_highway_without_notes() {
        let geom = HighwayGeometry::centered(21, 12);
        let view = PlayingView::default();
        assert!(view.visible(&geom).is_empty());
        let mut buf = vec![0u32; 21 * 12];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 21,
                h: 12,
            },
            &geom,
        );
        assert_eq!(buf[10 * 21], pack_xrgb(0xE0, 0xE0, 0xE0));
    }
}
