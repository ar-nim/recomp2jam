//! Channel dialog list state: rows, highlight, viewport, enter face
//! (#259 row cluster).

use super::super::paint::{
    channel_dialog_x_hit, channel_face_hit, channel_row_hit, channel_scroll_hit,
    channel_viewport_from_top, paint_channel_list, paint_channel_scene, ChannelScroll, SceneArt,
    StagedPlanetBoard, CHANNEL_VISIBLE_ROWS,
};
use o2jam_net::ChannelState;
use o2jam_render::Frame;

/// Scrollbar body press band, moved verbatim from
/// `ScreenGame::apply_pointer_down` (the exe's 21x45 bar body between
/// the arrows, `docs/channel-fill.md`).
const DRAG_BODY_X: std::ops::RangeInclusive<i32> = 573..=593;
const DRAG_BODY_Y: std::ops::RangeInclusive<i32> = 233..=364;

/// What a channel press asks the screen seam to do. The slash draft
/// stays owned by `ScreenGame`; `ConsumeDraft` only reports that the
/// face click landed while a draft held it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelPress {
    /// The bnd-pinned X closes the dialog to the bare board.
    Close,
    /// An active row's enter face enters the room lobby.
    Enter,
    /// The enter face landed while a slash draft owned the click.
    ConsumeDraft,
    /// A row body, scroll chrome, or off-row press: no screen step.
    Noop,
}

/// Channel list words behind one scene seam (#259: rows, highlight,
/// viewport top, and drag capture). Pure scene state — no screen flow,
/// no chat draft, no pointer store — so row logic reads and tests
/// without touching `ScreenGame`. The thin delegates on `ScreenGame`
/// keep the click seam, hover resolver, paint path, and snapshot
/// surface unchanged.
#[derive(Default)]
pub(crate) struct ChannelScene {
    /// Crate-visible words let the game-seam scenario guards assert
    /// the scene state; production reads go through the methods.
    pub(crate) rows: Vec<ChannelState>,
    pub(crate) highlight: usize,
    pub(crate) top: usize,
    pub(crate) dragging: bool,
    /// Last row the pointer visited, kept for the select bar after the
    /// pointer parks (#876 v3 oracle). The keyboard highlight is a
    /// separate Confirm target; this only holds the bar's view state.
    pub(crate) last_hovered_row: Option<usize>,
}

impl ChannelScene {
    /// Stage a fresh list: the populate zeroes the highlight and the
    /// list top beside the scrollbar reset (exe file `0x747ba`).
    pub(crate) fn set_rows(&mut self, rows: Vec<ChannelState>) {
        self.rows = rows;
        self.highlight = 0;
        self.top = 0;
        self.dragging = false;
        self.last_hovered_row = None;
    }

    /// Hold fetched rows behind the relay-unavailable notice (#244).
    pub(crate) fn clear_rows(&mut self) {
        self.rows.clear();
    }

    /// Rows currently listed (empty while the relay notice holds them).
    pub(crate) fn rows(&self) -> &[ChannelState] {
        &self.rows
    }

    /// Highlighted row: the keyboard Confirm and session-login target.
    pub(crate) fn highlighted(&self) -> Option<ChannelState> {
        self.rows.get(self.highlight).copied()
    }

    /// Row under the pointer, mapped back through the stored list top.
    pub(crate) fn hovered(&self, pointer: Option<(i32, i32)>) -> Option<usize> {
        let (x, y) = pointer?;
        let (top, shown) = channel_viewport_from_top(self.top, self.rows.len());
        let slot = channel_row_hit(x, y, shown)?;
        (top + slot < self.rows.len()).then_some(top + slot)
    }

    /// Remember the row a pointer event visited. Motion off the rows
    /// leaves the last row in place — the 7.55 select bar stays where
    /// the pointer last was (#876, `~/scratch/paired-shots-v3/states/channel`).
    /// A fresh list resets it through [`Self::set_rows`].
    pub(crate) fn remember_hover(&mut self, pointer: Option<(i32, i32)>) {
        if let Some(row) = self.hovered(pointer) {
            self.last_hovered_row = Some(row);
        }
    }

    /// Row the select bar paints: pointer first, then the last row the
    /// pointer visited. `None` lets the caller fall back to the keyboard
    /// highlight before any pointer visit.
    pub(crate) fn select_bar_row(&self, pointer: Option<(i32, i32)>) -> Option<usize> {
        self.hovered(pointer).or(self.last_hovered_row)
    }

    /// Row whose enter face covers (`x`, `y`): the session bridge's
    /// side-effect-free twin of the click path's enter arm (#30).
    /// A slash draft owns the click instead of entering (#163).
    pub(crate) fn enter_at(&self, x: i32, y: i32, draft_busy: bool) -> Option<ChannelState> {
        if channel_dialog_x_hit(x, y) || channel_scroll_hit(x, y).is_some() {
            return None;
        }
        let (top, shown) = channel_viewport_from_top(self.top, self.rows.len());
        let slot = channel_row_hit(x, y, shown)?;
        let index = top + slot;
        if index >= self.rows.len() || !channel_face_hit(x, y, slot) || draft_busy {
            return None;
        }
        self.rows.get(index).filter(|row| row.active).copied()
    }

    /// Step the list top one row toward `scroll`, clamped to
    /// `[0, len - shown]`. The highlight never moves on this path
    /// (exe handlers `0x4749a0`/`0x4749f0`, ticket #130).
    pub(crate) fn step(&mut self, scroll: ChannelScroll) {
        let max_top = self.rows.len().saturating_sub(CHANNEL_VISIBLE_ROWS);
        match scroll {
            ChannelScroll::Up => self.top = self.top.saturating_sub(1),
            ChannelScroll::Down => self.top = (self.top + 1).min(max_top),
        }
    }

    /// Resolve one press: X closes, scroll arrows step the viewport,
    /// a row body moves the highlight, and only the enter face of an
    /// active row enters. A slash draft consumes a face click before
    /// it can enter (the game seam takes the draft on `ConsumeDraft`).
    pub(crate) fn press(&mut self, x: i32, y: i32, draft_busy: bool) -> ChannelPress {
        if channel_dialog_x_hit(x, y) {
            return ChannelPress::Close;
        }
        if let Some(scroll) = channel_scroll_hit(x, y) {
            self.step(scroll);
            return ChannelPress::Noop;
        }
        let (top, shown) = channel_viewport_from_top(self.top, self.rows.len());
        let Some(slot) = channel_row_hit(x, y, shown) else {
            return ChannelPress::Noop;
        };
        let index = top + slot;
        self.highlight = index;
        if !channel_face_hit(x, y, slot) {
            return ChannelPress::Noop;
        }
        if draft_busy {
            return ChannelPress::ConsumeDraft;
        }
        if self.rows.get(index).is_some_and(|row| row.active) {
            ChannelPress::Enter
        } else {
            ChannelPress::Noop
        }
    }

    /// Arm drag capture when the press lands on the scrollbar body
    /// between the arrows. Returns whether the press belonged to the
    /// body (the game seam then skips its normal click dispatch).
    pub(crate) fn arm_drag(&mut self, x: i32, y: i32) -> bool {
        let on_body = DRAG_BODY_X.contains(&x) && DRAG_BODY_Y.contains(&y);
        if on_body {
            self.dragging = true;
        }
        on_body
    }

    /// Release drag capture (the pointer-up edge on the Channel screen).
    pub(crate) fn end_drag(&mut self) {
        self.dragging = false;
    }

    /// Absolute-set the list top from a drag motion, clamped to the
    /// track. Motion without capture changes nothing; the casts are
    /// the pinned shell arithmetic (ticket #174).
    pub(crate) fn drag_to(&mut self, y: i32) {
        if !self.dragging {
            return;
        }
        let max_top = self.rows.len().saturating_sub(CHANNEL_VISIBLE_ROWS);
        let track_top = 233i32;
        let track_bot_origin = 364i32;
        let frac = ((y.clamp(track_top, track_bot_origin) - track_top) as f32
            / (track_bot_origin - track_top) as f32)
            .clamp(0.0, 1.0);
        self.top = (frac * max_top as f32).round() as usize;
    }

    /// Paint the live rows, highlight, and enter faces: the staged
    /// dialog path with art, or the flat fallback without it. The
    /// screen seam still chooses the surrounding scene art.
    /// Test-only shortcut for a scene painting its own interactive rows.
    #[cfg(test)]
    pub(crate) fn paint(
        &self,
        frame: &mut Frame<'_>,
        art: Option<&SceneArt>,
        board: Option<StagedPlanetBoard<'_>>,
        hovered_row: Option<usize>,
    ) {
        self.paint_rows(&self.rows, frame, art, board, hovered_row);
    }

    /// Paint an explicit list while keeping the interactive rows (`self.rows`)
    /// unchanged. The relay notice uses this to show boot-held rows behind
    /// its sheet without making them clickable (#348 composition arm).
    pub(crate) fn paint_rows(
        &self,
        rows: &[ChannelState],
        frame: &mut Frame<'_>,
        art: Option<&SceneArt>,
        board: Option<StagedPlanetBoard<'_>>,
        hovered_row: Option<usize>,
    ) {
        match art {
            Some(art) => paint_channel_scene(
                frame,
                rows,
                self.top,
                self.highlight,
                art,
                board,
                hovered_row,
            ),
            None => paint_channel_list(frame, rows, self.top, self.highlight),
        }
    }
}

#[cfg(test)]
mod channel_tests {
    use super::*;
    use o2jam_render::{near_xrgb, pack_xrgb};

    /// Active row 2 and inactive row 9, the game-seam fixture used
    /// by the live-chain click and face guards.
    fn two_channels() -> Vec<ChannelState> {
        vec![
            ChannelState {
                gateway_id: 0,
                channel_id: 2,
                capacity: 100,
                population: 5,
                active: true,
            },
            ChannelState {
                gateway_id: 0,
                channel_id: 9,
                capacity: 0,
                population: 0,
                active: false,
            },
        ]
    }

    /// Six active rows: the viewport shows four, so the list top has
    /// room to move (the scrolled-viewport guards).
    fn six_channels() -> Vec<ChannelState> {
        (0u16..6)
            .map(|id| ChannelState {
                gateway_id: 0,
                channel_id: id,
                capacity: 100,
                population: 5,
                active: true,
            })
            .collect()
    }

    /// Same FNV-1a byte fold as the game's observable-state hash,
    /// here over every pixel of a rendered frame.
    fn frame_hash(buf: &[u32]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for pixel in buf {
            for byte in u64::from(*pixel).to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
            }
        }
        hash
    }

    /// True while ink nears `ink` inside the `probe` box at `origin`
    /// (the game-seam probe helper, needed by the row-ink guard).
    fn ink_near(
        buf: &[u32],
        w: usize,
        h: usize,
        origin: (i32, i32),
        probe: (i32, i32),
        ink: u32,
    ) -> bool {
        for row in origin.1..origin.1 + probe.1 {
            for col in origin.0..origin.0 + probe.0 {
                let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
                    continue;
                };
                if r < h && c < w && near_xrgb(buf[r * w + c], ink) {
                    return true;
                }
            }
        }
        false
    }

    /// Golden renders, captured before the #259 move: the flat row
    /// painter and the art-less staged path must stay byte-identical.
    #[test]
    fn channel_rows_render_golden() {
        let mut scene = ChannelScene::default();
        scene.set_rows(two_channels());
        scene.highlight = 1;
        let mut buf = vec![0u32; 800 * 600];
        scene.paint(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            None,
            None,
        );
        assert_eq!(frame_hash(&buf), 0xdaceb980f6a6da6f);

        // Scrolled viewport plus a hovered row on the art-less staged
        // path (no install-root read).
        let mut scene = ChannelScene::default();
        scene.set_rows(six_channels());
        scene.top = 2;
        scene.highlight = 5;
        let mut buf = vec![0u32; 800 * 600];
        scene.paint(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            Some(&SceneArt::default()),
            None,
            Some(3),
        );
        assert_eq!(frame_hash(&buf), 0x8b70961739689077);
    }

    /// Highlighted row (the keyboard Confirm and session-login
    /// target): a fresh list opens on row 0; only a body click moves
    /// the highlight — arrow steps and keys leave it alone (#189).
    #[test]
    fn highlighted_channel_names_the_confirm_target() {
        let mut scene = ChannelScene::default();
        assert_eq!(scene.highlighted(), None);
        scene.set_rows(two_channels());
        assert_eq!(scene.highlighted().map(|row| row.channel_id), Some(2));
        scene.step(ChannelScroll::Down);
        assert_eq!(scene.highlighted().map(|row| row.channel_id), Some(2));
        assert_eq!(scene.press(300, 290, false), ChannelPress::Noop);
        assert_eq!(scene.highlighted().map(|row| row.channel_id), Some(9));
    }

    /// Mouse entry target: only an enter-face click names its row.
    #[test]
    fn channel_enter_at_names_only_face_clicks() {
        let mut scene = ChannelScene::default();
        scene.set_rows(two_channels());
        // Row 0 enter face (ticket #56: (502,236)-(546,257)).
        assert_eq!(
            scene.enter_at(524, 246, false).map(|row| row.channel_id),
            Some(2)
        );
        // Row bodies highlight only, never enter.
        assert_eq!(scene.enter_at(300, 240, false), None);
        assert_eq!(scene.enter_at(300, 290, false), None);
        // Off the rows entirely.
        assert_eq!(scene.enter_at(10, 10, false), None);
        // The X chrome closes instead of entering.
        assert_eq!(scene.enter_at(577, 171, false), None);
        // A slash draft consumes the face click instead of entering.
        assert_eq!(scene.enter_at(524, 246, true), None);
    }

    /// Row ink on a native frame: the pinned fallback label still
    /// lands at the measured native tops (re-measured 2026-11-11 with
    /// the `Ch. NN` label, #93).
    #[test]
    fn channel_rows_ink_at_measured_native_tops() {
        let mut scene = ChannelScene::default();
        scene.set_rows(two_channels());
        let mut buf = vec![0u32; 800 * 600];
        scene.paint(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            None,
            None,
        );
        assert!(ink_near(
            &buf,
            800,
            600,
            (200, 224),
            (64, 9),
            pack_xrgb(0xFF, 0xFF, 0xFF)
        ));
        assert_eq!(buf[240 * 800 + 230], pack_xrgb(19, 20, 21));
        assert_eq!(buf[229 * 800 + 350], pack_xrgb(230, 227, 230));
        assert!(ink_near(
            &buf,
            800,
            600,
            (502, 236),
            (24, 12),
            pack_xrgb(150, 148, 151)
        ));
        assert!(ink_near(
            &buf,
            800,
            600,
            (200, 282),
            (64, 9),
            pack_xrgb(0x70, 0x70, 0x70)
        ));
    }

    /// Exe-pinned slot rects (CStatePlanet dialog+0x94):
    /// (216,224,572,275), (216,275,572,324), (216,324,572,375),
    /// (216,375,572,436) — left/top inclusive, right/bottom
    /// exclusive. x = 300 sits inside every band's x range.
    #[test]
    fn channel_row_hit_maps_the_exe_pinned_rects() {
        assert_eq!(channel_row_hit(300, 224, 10), Some(0));
        assert_eq!(channel_row_hit(300, 240, 10), Some(0));
        assert_eq!(channel_row_hit(300, 274, 10), Some(0));
        assert_eq!(channel_row_hit(300, 275, 10), Some(1));
        assert_eq!(channel_row_hit(300, 282, 10), Some(1));
        assert_eq!(channel_row_hit(300, 375, 10), Some(3));
        assert_eq!(channel_row_hit(300, 435, 10), Some(3));
        assert_eq!(channel_row_hit(300, 436, 10), None);
        assert_eq!(channel_row_hit(300, 100, 10), None);
        assert_eq!(channel_row_hit(300, 732, 10), None);
        assert_eq!(channel_row_hit(300, 240, 0), None);
        // Issue #29: the y band alone is not a hit — x must sit
        // inside the pinned rect too. The scroll track (x573-593)
        // and the dialog margins share the bands' y range.
        assert_eq!(channel_row_hit(583, 240, 10), None);
        assert_eq!(channel_row_hit(100, 240, 10), None);
        assert_eq!(channel_row_hit(700, 290, 10), None);
        assert_eq!(channel_row_hit(216, 240, 10), Some(0));
        assert_eq!(channel_row_hit(571, 240, 10), Some(0));
        assert_eq!(channel_row_hit(572, 240, 10), None);
    }
}

#[cfg(test)]
mod channel_dispatch_tests {
    use super::super::planet::board_dispatch_model::*;

    #[test]
    fn channel_hosted_dialog_receives_dispatch_before_the_global_listener() {
        // seam: the call order is the whole behavior at this seam.
        let hosted = HostedDialog;
        let global = GlobalListener;
        let mut trace = Vec::new();

        route_board_message(Some(&hosted), &global, &mut trace);

        assert_eq!(
            trace,
            [MessageSink::HostDialog, MessageSink::GlobalListener],
            "the +0xF6C dialog's +0x50 call must precede the listener's +0x5C call"
        );
    }
}
