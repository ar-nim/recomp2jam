//! Trace evidence behind the window's input seam.
//!
//! [`CaptureAdapter`] is the only place that knows the capture log, frame
//! dumps, and the judged-strike stamp. Input routing still freezes one
//! snapshot per frame (ADR-0013); this module never drains edges, forwards
//! input, or changes the game.

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use super::super::{CaptureState, InputEdge, DEFAULT_HEIGHT, DEFAULT_WIDTH};

/// Where one capture line or frame lands. The live adapter opens files;
/// the fake adapter used by tests keeps the same ordering in memory.
pub(crate) trait EvidenceSink {
    /// Write one newline-terminated line and make it visible at once.
    fn write_line(&mut self, line: &str);
    /// Store one already-gated native-frame dump.
    fn dump_frame(&mut self, name: &str, pixels: &[u32]);
}

/// Files and the opened input log, owned by the adapter rather than by
/// the window handler.
struct LiveSink {
    dir: PathBuf,
    log: Option<std::io::BufWriter<std::fs::File>>,
}

impl EvidenceSink for LiveSink {
    fn write_line(&mut self, line: &str) {
        let Some(log) = self.log.as_mut() else {
            return;
        };
        let _ = log.write_all(line.as_bytes());
        let _ = log.write_all(b"\n");
        let _ = log.flush();
    }

    fn dump_frame(&mut self, name: &str, pixels: &[u32]) {
        // A plan armed for the input log alone carries an empty dir:
        // skip dumps, keep counting.
        if self.dir.as_os_str().is_empty() {
            return;
        }
        let rgb = xrgb_to_rgb(pixels);
        let png = o2jam_render::png::encode_png(DEFAULT_WIDTH, DEFAULT_HEIGHT, &rgb);
        let path = self.dir.join(name);
        if let Err(source) = std::fs::write(path, png) {
            tracing::warn!(%source, "frame dump failed");
        }
    }
}

/// Capture plumbing for one windowed run: log lines, dump gating, and the
/// judged-strike stamp. The wall clock starts when the capture state opens
/// (ADR-0001 keeps song time on the sample clock; the log stamp stays on
/// this delivery-boundary clock).
pub(crate) struct CaptureAdapter {
    sink: Box<dyn EvidenceSink>,
    started: Instant,
    min_paint_gap: u64,
    heartbeat_paints: u64,
    paint: u64,
    last_screen: &'static str,
    last_content_hash: u64,
    last_dump_paint: u64,
}

impl CaptureAdapter {
    /// Wrap the shell's opened capture state. Called once at handler
    /// construction; the shell no longer carries evidence state after this.
    pub(crate) fn new(state: CaptureState) -> Self {
        Self {
            sink: Box::new(LiveSink {
                dir: state.dir,
                log: state.log,
            }),
            started: state.started,
            min_paint_gap: state.min_paint_gap,
            heartbeat_paints: state.heartbeat_paints,
            paint: 0,
            last_screen: "",
            last_content_hash: 0,
            last_dump_paint: 0,
        }
    }

    /// Install a sink at the adapter's internal seam. Tests cross the
    /// same interface as the live boot, so log order needs no filesystem.
    #[cfg(test)]
    pub(crate) fn with_sink(sink: Box<dyn EvidenceSink>, started: Instant) -> Self {
        Self {
            sink,
            started,
            min_paint_gap: 1,
            heartbeat_paints: 2,
            paint: 0,
            last_screen: "",
            last_content_hash: 0,
            last_dump_paint: 0,
        }
    }

    /// Log one applied-event line. Called from key, pointer, text and
    /// erase routing; no game state is read here.
    pub(crate) fn record_event(&mut self, event: &str) {
        let wall_ms = self.started.elapsed().as_millis();
        self.sink.write_line(&format!("event {wall_ms} {event}"));
    }

    /// Stamp every lane-down edge that the caller is about to freeze,
    /// before that caller drains the pending edges (log-order contract).
    /// This borrows the queue but never removes from it: ADR-0013 keeps
    /// exactly one consumption point, in `WindowHandler::deliver_snapshot`.
    pub(crate) fn record_judged_before_drain(&mut self, pending: &[InputEdge]) {
        let judged_lanes: Vec<usize> = pending
            .iter()
            .filter_map(|edge| match edge {
                InputEdge::Lane { lane, down: true } => Some(*lane),
                _ => None,
            })
            .collect();
        let wall_ms = self.started.elapsed().as_millis();
        for lane in judged_lanes {
            self.sink
                .write_line(&format!("judged {wall_ms} lane {lane}"));
        }
    }

    /// Record one painted frame; dump only on the gated transitions.
    pub(crate) fn record_frame(&mut self, pixels: &[u32], screen: &'static str, state_hash: u64) {
        self.paint = self.paint.saturating_add(1);
        let content = content_hash(pixels);
        let wall_ms = self.started.elapsed().as_millis();
        self.sink.write_line(&format!(
            "frame {0} wall_ms={wall_ms} screen={screen} state={state_hash:016x} content={content:016x}",
            self.paint
        ));
        let since_dump = self.paint.saturating_sub(self.last_dump_paint);
        let screen_changed = screen != self.last_screen;
        let content_changed = content != self.last_content_hash;
        let due = (screen_changed || (content_changed && since_dump >= self.min_paint_gap))
            || since_dump >= self.heartbeat_paints;
        self.last_screen = screen;
        self.last_content_hash = content;
        if !due {
            return;
        }
        self.last_dump_paint = self.paint;
        self.sink
            .dump_frame(&format!("frame-{:06}-{screen}.png", self.paint), pixels);
    }
}

/// FNV-1a over the backbuffer words: the content fingerprint gating dumps.
fn content_hash(pixels: &[u32]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for word in pixels {
        for shift in (0..32).step_by(8) {
            hash ^= u64::from((*word >> shift) as u8);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
    }
    hash
}

/// Strip XRGB backbuffer words down to RGB triplets for PNG dumps.
fn xrgb_to_rgb(buf: &[u32]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(buf.len() * 3);
    for pixel in buf {
        rgb.push((pixel >> 16) as u8);
        rgb.push((pixel >> 8) as u8);
        rgb.push(*pixel as u8);
    }
    rgb
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Instant;

    use super::super::super::{InputEdge, InputSnapshot};
    use super::{CaptureAdapter, EvidenceSink};
    use crate::{GameLoop, Shell};

    /// In-memory sink: every line is visible to the reader at write time,
    /// the same contract the live writer gets from its per-line flush.
    #[derive(Clone, Default)]
    pub(crate) struct FakeSink {
        lines: Rc<RefCell<Vec<String>>>,
        dumps: Rc<RefCell<Vec<String>>>,
        flushes: Rc<RefCell<usize>>,
    }

    impl FakeSink {
        pub(crate) fn lines(&self) -> Vec<String> {
            self.lines.borrow().clone()
        }

        pub(crate) fn dumps(&self) -> Vec<String> {
            self.dumps.borrow().clone()
        }

        pub(crate) fn flushes(&self) -> usize {
            *self.flushes.borrow()
        }
    }

    impl EvidenceSink for FakeSink {
        fn write_line(&mut self, line: &str) {
            self.lines.borrow_mut().push(line.to_owned());
            *self.flushes.borrow_mut() += 1;
        }

        fn dump_frame(&mut self, name: &str, _pixels: &[u32]) {
            self.dumps.borrow_mut().push(name.to_owned());
        }
    }

    /// One recording frame boundary for the log-order guards.
    struct RecordingGame {
        ingests: Rc<RefCell<Vec<InputSnapshot>>>,
    }

    impl GameLoop for RecordingGame {
        fn lane_key(&mut self, _lane: usize, _down: bool) {}

        fn ingest_snapshot(&mut self, snapshot: &InputSnapshot) {
            self.ingests.borrow_mut().push(snapshot.clone());
        }

        fn paint(&mut self, _frame: &mut o2jam_render::Frame<'_>) {}
    }

    /// Handler with the fake sink installed at the capture seam.
    fn handler_with_sink(
        sink: FakeSink,
    ) -> (super::super::WindowHandler, Rc<RefCell<Vec<InputSnapshot>>>) {
        let ingests = Rc::new(RefCell::new(Vec::new()));
        let shell = Shell::new(Box::new(RecordingGame {
            ingests: ingests.clone(),
        }));
        let mut handler = super::super::WindowHandler::new(shell);
        handler.capture = Some(CaptureAdapter::with_sink(Box::new(sink), Instant::now()));
        (handler, ingests)
    }

    #[test]
    fn capture_log_records_events_frames_and_dumps() {
        let sink = FakeSink::default();
        let (mut handler, _) = handler_with_sink(sink.clone());
        handler.log_event("key-down KeyS");
        handler.capture_frame();
        handler.capture_frame();

        let lines = sink.lines();
        assert!(
            lines.iter().any(|line| line.contains("key-down KeyS")),
            "routed key: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.starts_with("frame 1 ")),
            "first paint: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.starts_with("frame 2 ")),
            "second paint: {lines:?}"
        );
        assert!(
            !sink.dumps().is_empty(),
            "a content change after one paint owes one dump"
        );
    }

    #[test]
    fn capture_log_records_judged_strikes_on_delivery() {
        let sink = FakeSink::default();
        let (mut handler, ingests) = handler_with_sink(sink.clone());
        handler.route_key(winit::keyboard::KeyCode::KeyS, true);
        handler.deliver_snapshot();

        let lines = sink.lines();
        assert!(
            lines.iter().any(|line| line.contains("key-down KeyS")),
            "routed key: {lines:?}"
        );
        let judged: Vec<_> = lines
            .iter()
            .filter(|line| line.starts_with("judged "))
            .collect();
        assert_eq!(judged.len(), 1, "one lane-down stamp: {lines:?}");
        assert!(judged[0].ends_with("lane 0"), "stamped lane: {lines:?}");
        assert_eq!(
            ingests.borrow().len(),
            1,
            "the judged stamp never adds a second intake"
        );
        assert_eq!(
            ingests.borrow()[0].edges(),
            &[InputEdge::Lane {
                lane: 0,
                down: true
            }],
            "the single drain still delivers the edge once"
        );
    }

    #[test]
    fn judged_lines_precede_drained_edges_in_fake_sink() {
        // Live caller chain: winit KeyboardInput -> WindowHandler::route_key
        // -> deliver_snapshot -> CaptureAdapter::record_judged_before_drain
        // -> the one pending_edges.drain in deliver_snapshot -> game intake.
        // A second drain or a moved stamp would break this line order.
        let sink = FakeSink::default();
        let (mut handler, ingests) = handler_with_sink(sink.clone());
        handler.route_key(winit::keyboard::KeyCode::KeyS, true);
        handler.deliver_snapshot();
        handler.route_key(winit::keyboard::KeyCode::Space, true);
        handler.deliver_snapshot();

        let lines = sink.lines();
        let first_key = lines
            .iter()
            .position(|line| line.ends_with("key-down KeyS"))
            .expect("first routed key");
        let first_judged = lines
            .iter()
            .position(|line| line.ends_with("lane 0"))
            .expect("first judged strike");
        let second_key = lines
            .iter()
            .position(|line| line.ends_with("key-down Space"))
            .expect("second routed key");
        let second_judged = lines
            .iter()
            .position(|line| line.ends_with("lane 3"))
            .expect("second judged strike");
        assert!(
            first_key < first_judged && first_judged < second_key && second_key < second_judged,
            "judged stamps before each drain: {lines:?}"
        );
        let snapshots = ingests.borrow();
        assert_eq!(snapshots.len(), 2, "one intake per delivered frame");
        assert_eq!(
            snapshots[0].edges(),
            &[InputEdge::Lane {
                lane: 0,
                down: true
            }]
        );
        assert_eq!(
            snapshots[1].edges(),
            &[InputEdge::Lane {
                lane: 3,
                down: true
            }]
        );
    }

    #[test]
    fn capture_log_flushes_each_line_for_concurrent_readers() {
        let sink = FakeSink::default();
        let (mut handler, _) = handler_with_sink(sink.clone());
        handler.log_event("click 1 2");

        let lines = sink.lines();
        assert_eq!(lines.len(), 1, "the line exists before the writer drops");
        assert!(
            lines[0].contains("click 1 2"),
            "the visible line is the routed event: {lines:?}"
        );
        assert_eq!(sink.flushes(), 1, "each write is immediately visible");
    }
}
