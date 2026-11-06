//! Note highway: seven lanes, judgement line, falling notes.
//!
//! Pure layout + paint over [`LevelChart`] onsets (`lane, ms` from
//! [`LevelChart::hit_onsets_ms`]); timing comes from an explicit
//! [`PlayingView::advance_to`], so tests drive the clock. Layout is
//! relative to the frame: full-width lanes, line one sixth up from
//! the bottom, `lookahead` ms of chart visible above the line.

use o2jam_render::{fill_rect, pack_xrgb, Frame, Rect};

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
