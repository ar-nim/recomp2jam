//! Note highway: seven lanes, judgement line, falling notes.
//!
//! Pure layout + paint over [`LevelChart`] onsets (`lane, ms` from
//! [`LevelChart::hit_onsets_ms`]); timing comes from an explicit
//! [`PlayingView::advance_to`], so tests drive the clock. Layout is
//! relative to the frame: full-width lanes, line one sixth up from
//! the bottom, `lookahead` ms of chart visible above the line.

use o2jam_game::{CoolStreak, Judgement, PillStock, Score, TimingWindows};
use o2jam_render::{draw_number, draw_text, fill_rect, pack_xrgb, text_width, Frame, Rect};

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
            x: self.x0 + lane as i32 * self.lane_w as i32,
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

    /// Paint highway, line, and visible notes.
    pub fn paint(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
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
        for (lane, y) in self.visible(geom) {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingNote {
    lane: u8,
    at_ms: u64,
    sample: u32,
    judged: bool,
}

/// Live play: scrolling view plus verified judgement/scoring.
///
/// Key presses rate the nearest unjudged note in that lane inside
/// the bad window; notes aging past it rate Miss on advance. Presses
/// with no note nearby rate nothing (the original's empty-press
/// behavior is unverified, so nothing is guessed).
#[derive(Debug, Default)]
pub struct LivePlay {
    view: PlayingView,
    pending: Vec<PendingNote>,
    score: Score,
    pills: PillStock,
    streak: CoolStreak,
    windows: TimingWindows,
    now_ms: u64,
    last_word: Option<(Judgement, u64)>,
    sounds: Vec<u32>,
}

impl LivePlay {
    /// Notes to play (empty until a chart loads); each onset carries
    /// its key-sound sample id.
    #[must_use]
    pub fn new(onsets: Vec<(u8, u64, u32)>) -> Self {
        let mut pending: Vec<PendingNote> = onsets
            .iter()
            .map(|(lane, at_ms, sample)| PendingNote {
                lane: *lane,
                at_ms: *at_ms,
                sample: *sample,
                judged: false,
            })
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
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
        }
    }

    /// Move the song clock; notes past the bad window rate Miss.
    pub fn advance_to(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        self.view.advance_to(now_ms);
        let bad = self.windows.bad;
        let mut late = Vec::new();
        for note in &mut self.pending {
            if !note.judged && note.at_ms.saturating_add(bad) < now_ms {
                note.judged = true;
                late.push(Judgement::Miss);
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
        self.view.strike(lane, at_ms);
        self.sounds.push(sample);
        let judgement = self.windows.judge(offset);
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

    /// Song over: every note judged and the tail (1.5 s) played out.
    /// Empty charts never finish on their own (nothing to play).
    #[must_use]
    pub fn is_finished(&self, now_ms: u64) -> bool {
        let Some(last) = self.pending.iter().map(|note| note.at_ms).max() else {
            return false;
        };
        now_ms >= last.saturating_add(1500) && self.pending.iter().all(|note| note.judged)
    }

    fn rate(&mut self, judgement: Judgement) {
        self.streak.note(judgement, &mut self.pills);
        self.score.apply_rated(judgement, &mut self.pills);
        self.last_word = Some((judgement, self.now_ms));
    }

    /// Paint highway, notes, score, combo, and the latest word.
    pub fn paint(&mut self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        self.advance_to(now_ms);
        self.view.paint(frame, geom);
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
        let mut live = LivePlay::new(vec![(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 1);
        assert_eq!(live.score().points, 200);
        assert!(live.view.visible(&geom()).is_empty());
    }

    #[test]
    fn struck_notes_queue_their_samples() {
        let mut live = LivePlay::new(vec![(2, 1000, 7), (2, 1100, 9)]);
        assert_eq!(live.drain_sounds(), Vec::<u32>::new());
        live.strike_lane(2, 1010);
        assert_eq!(live.drain_sounds(), vec![7]);
        assert_eq!(live.drain_sounds(), Vec::<u32>::new());
    }

    #[test]
    fn strike_rates_good_on_early_hit() {
        let mut live = LivePlay::new(vec![(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 990), Some(Judgement::Good));
        assert_eq!(live.score().points, 100);
    }

    #[test]
    fn strike_rates_bad_on_late_hit() {
        let mut live = LivePlay::new(vec![(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1020), Some(Judgement::Bad));
        assert_eq!(live.score().combo, 0);
        assert_eq!(live.score().points, 4);
    }

    #[test]
    fn late_notes_rate_miss_on_advance() {
        let mut live = LivePlay::new(vec![(1, 1000, 7)]);
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.score().counts[3], 1);
        assert_eq!(live.score().points, 0);
    }

    #[test]
    fn empty_press_rates_nothing() {
        let mut live = LivePlay::new(vec![(2, 1000, 7)]);
        assert_eq!(live.strike_lane(0, 1000), None);
        assert_eq!(live.score().counts, [0, 0, 0, 0]);
    }

    #[test]
    fn nearest_note_wins_in_lane() {
        let mut live = LivePlay::new(vec![(2, 1000, 7), (2, 1100, 9)]);
        assert_eq!(live.strike_lane(2, 1010), Some(Judgement::Good));
        assert_eq!(live.strike_lane(2, 1100), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 2);
    }

    #[test]
    fn paint_shows_score_combo_and_word() {
        let mut live = LivePlay::new(vec![(0, 1000, 7)]);
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
        let mut live = LivePlay::new(vec![(0, 1000, 7)]);
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
    fn word_fades_after_timeout() {
        let mut live = LivePlay::new(vec![(0, 1000, 7)]);
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
