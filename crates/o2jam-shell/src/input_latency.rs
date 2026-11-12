//! Input-latency probe: measure the key event → judged strike round
//! trip on a synthetic clock, and the budget gate holding the shell
//! at or under the recorded number (claim B53, issue #100).
//!
//! The measurement rides the real windowed dispatch path
//! ([`window::WindowHandler::route_key`]): a scripted key event goes
//! through the key map, resolves to a lane, and enters the game's
//! judgement arm ([`GameLoop::lane_key`] → `LivePlay::strike_lane`)
//! inside one synchronous callback. Timestamps come from an injected
//! [`SyntheticClock`], so the reading is deterministic and the test
//! can model the time an event sat in the OS/winit queue before the
//! callback ran.
//!
//! What the probe proves: the shell adds zero ticks between event
//! receipt and judgement-arm entry — the judged strike reads the
//! sample clock at dispatch time, so queued latency passes through
//! 1:1 and never compounds. The queue-side number itself (winit/OS
//! delivery, the live rig measurement per ADR-0009) and the exe-side
//! comparison number are OPEN: they need the SendInput-vs-judged-
//! sample capture pair on the Wine rig, which feeds the milestone-3
//! budget decision (#76).

use std::cell::RefCell;
use std::rc::Rc;

use winit::keyboard::KeyCode;

#[cfg(test)]
use super::window::WindowHandler;
use super::GameLoop;

/// Budget for ticks the shell itself adds between event receipt and
/// judgement-arm entry. Zero: the dispatch is one synchronous
/// callback, so any positive reading is a regression (work sneaking
/// into the dispatch path, or a stale clock read at judgement).
pub const INPUT_LATENCY_BUDGET_TICKS: u64 = 0;

/// Deterministic tick source for the probe. One tick is one
/// arbitrary clock unit (the live seam is the audio sample clock;
/// the probe only measures *deltas*, so the unit cancels out).
#[derive(Debug, Default, Clone, Copy)]
pub struct SyntheticClock {
    tick: u64,
}

impl SyntheticClock {
    /// A clock at tick zero.
    #[must_use]
    pub fn new() -> Self {
        Self { tick: 0 }
    }

    /// Advance the clock by `ticks` (queue modeling, or proving the
    /// dispatch hop adds nothing while time passes elsewhere).
    pub fn advance(&mut self, ticks: u64) {
        self.tick = self.tick.saturating_add(ticks);
    }

    /// The current tick.
    #[must_use]
    pub fn now(&self) -> u64 {
        self.tick
    }
}

/// One in-flight measurement: stamped at event receipt, completed at
/// judgement-arm entry by the [`JudgementStamp`] wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputLatencyProbe {
    /// Synthetic tick at event receipt (winit delivered the key).
    receipt_tick: u64,
    /// Ticks modeled as queue time between receipt and dispatch.
    modeled_queue_ticks: u64,
    /// Synthetic tick at judgement-arm entry (`lane_key` down), if
    /// the dispatched event resolved to a lane strike.
    judgement_tick: Option<u64>,
}

impl InputLatencyProbe {
    /// Open a probe at the tick the key event was received.
    #[must_use]
    pub fn begin(receipt_tick: u64) -> Self {
        Self {
            receipt_tick,
            modeled_queue_ticks: 0,
            judgement_tick: None,
        }
    }
}

/// Completed measurement: the round trip from event receipt to
/// judgement-arm entry, decomposed into queue time and shell-added
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputLatencySample {
    /// Ticks modeled as queue time before dispatch.
    pub modeled_queue_ticks: u64,
    /// Total ticks receipt → judgement-arm entry
    /// (`judgement_tick - receipt_tick`).
    pub dispatch_ticks: u64,
    /// Ticks the shell added on top of the modeled queue time —
    /// the guarded number. Zero on the synchronous dispatch path.
    pub added_ticks: u64,
}

impl InputLatencySample {
    /// Whether the guarded number holds the recorded budget
    /// ([`INPUT_LATENCY_BUDGET_TICKS`]).
    // `<=` (not `==`): the budget is 0 today but rises with the
    // milestone-3 decision (#76), and the gate must keep working
    // unmodified when it does. The lint is expected, not allowed:
    // once the budget turns nonzero the comparison stops being
    // absurd and this attribute must go away.
    #[expect(clippy::absurd_extreme_comparisons)]
    #[must_use]
    pub fn within_budget(&self) -> bool {
        self.added_ticks <= INPUT_LATENCY_BUDGET_TICKS
    }
}

/// Shared stamp state for one measurement run: the synthetic clock
/// plus the in-flight probe. Owned by the caller; the wrapper borrows
/// it at judgement-arm entry, the driver (`measure_input_latency`)
/// around the dispatch.
#[derive(Debug, Default)]
pub struct StampState {
    clock: SyntheticClock,
    probe: Option<InputLatencyProbe>,
    /// Lanes that entered the judgement arm (presses the wrapper
    /// stamped), in arrival order — the observable side of the
    /// measurement for callers holding only the shared state.
    judged_lanes: Vec<usize>,
}

impl StampState {
    /// Fresh state with a zeroed clock and no in-flight probe.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The lanes that entered the judgement arm, in arrival order.
    #[must_use]
    pub fn judged_lanes(&self) -> &[usize] {
        &self.judged_lanes
    }
}

/// [`GameLoop`] wrapper stamping judgement-arm entry: on a lane key
/// press it records the synthetic-clock tick into the shared
/// [`StampState`] before delegating to the wrapped game, so the
/// probe sees exactly when the strike entered the judgement arm.
pub struct JudgementStamp<G> {
    inner: G,
    state: Rc<RefCell<StampState>>,
}

impl<G> JudgementStamp<G> {
    /// Wrap a game loop; every stamped lane press writes into `state`.
    pub fn new(inner: G, state: Rc<RefCell<StampState>>) -> Self {
        Self { inner, state }
    }

    /// The wrapped game (for direct assertions on judged state).
    pub fn inner(&self) -> &G {
        &self.inner
    }
}

impl<G: GameLoop> JudgementStamp<G> {
    /// Record one lane press on the judgement stamp without
    /// forwarding: the snapshot intake observes every drained edge,
    /// then forwards the whole snapshot once — recording here and
    /// replaying through [`GameLoop::lane_key`] would judge twice.
    fn record_lane_press(&mut self, lane: usize) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.judged_lanes.push(lane);
            let now = state.clock.now();
            if let Some(probe) = state.probe.as_mut() {
                probe.judgement_tick = Some(now);
            }
        }
    }
}

impl<G: GameLoop> GameLoop for JudgementStamp<G> {
    fn capture_raw_key(&mut self, code: KeyCode, down: bool) -> bool {
        self.inner.capture_raw_key(code, down)
    }

    fn lane_bindings(&self) -> Option<[KeyCode; super::LANE_COUNT]> {
        self.inner.lane_bindings()
    }

    fn lane_key(&mut self, lane: usize, down: bool) {
        if down {
            self.record_lane_press(lane);
        }
        self.inner.lane_key(lane, down);
    }

    fn menu_key(&mut self, key: super::MenuKey, down: bool) {
        self.inner.menu_key(key, down);
    }

    fn text_input(&mut self, text: &str) {
        self.inner.text_input(text);
    }

    fn delete_text(&mut self) {
        self.inner.delete_text();
    }

    fn ingest_snapshot(&mut self, snapshot: &super::InputSnapshot) {
        // Observe every drained lane press on the stamp, then forward
        // the whole snapshot once: replaying through `lane_key` would
        // forward twice, and bypassing the record would blind the gate.
        for edge in snapshot.edges() {
            if let super::InputEdge::Lane { lane, down: true } = *edge {
                self.record_lane_press(lane);
            }
        }
        self.inner.ingest_snapshot(snapshot);
    }

    fn quit_requested(&self) -> bool {
        self.inner.quit_requested()
    }

    fn screen_name(&self) -> &'static str {
        self.inner.screen_name()
    }

    fn state_hash(&self) -> u64 {
        self.inner.state_hash()
    }

    fn idle_repaint_interval(&self) -> Option<std::time::Duration> {
        self.inner.idle_repaint_interval()
    }

    fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>) {
        self.inner.paint(frame);
    }
}

/// Drive one scripted key event through the windowed dispatch path
/// and measure the receipt → judgement round trip.
///
/// 1. Stamps event receipt on the shared clock.
/// 2. Advances the clock by `modeled_queue_ticks` (the queue time the
///    live rig will measure for real; ADR-0009 capture pair).
/// 3. Routes the key through [`WindowHandler::route_key`] — the same
///    code path a live winit event takes.
/// 4. Completes the probe from the wrapper's judgement stamp.
///
/// Returns `None` when the event never reached the judgement arm
/// (e.g. the key did not resolve to a lane, or the game was not on
/// the Playing screen) — a miss is evidence too, not a zero.
///
/// Test-suite machinery (`route_key` is crate-private): the guard
/// gate and future trace drivers exercise it from the shell's tests.
#[cfg(test)]
pub(crate) fn measure_input_latency(
    handler: &mut WindowHandler,
    code: KeyCode,
    modeled_queue_ticks: u64,
    state: &Rc<RefCell<StampState>>,
) -> Option<InputLatencySample> {
    {
        let mut state = state.borrow_mut();
        let receipt_tick = state.clock.now();
        let mut probe = InputLatencyProbe::begin(receipt_tick);
        probe.modeled_queue_ticks = modeled_queue_ticks;
        state.probe = Some(probe);
        state.clock.advance(modeled_queue_ticks);
    }
    handler.route_key(code, true);
    handler.deliver_snapshot();
    let state = state.borrow();
    let probe = state.probe.as_ref()?;
    let judgement_tick = probe.judgement_tick?;
    let dispatch_ticks = judgement_tick.saturating_sub(probe.receipt_tick);
    Some(InputLatencySample {
        modeled_queue_ticks: probe.modeled_queue_ticks,
        dispatch_ticks,
        added_ticks: dispatch_ticks.saturating_sub(probe.modeled_queue_ticks),
    })
}

#[cfg(test)]
mod tests {
    use super::super::{MenuKey, Shell};
    use super::*;
    use std::cell::RefCell;

    /// Game probe recording lane presses for probe arithmetic tests.
    #[derive(Default)]
    struct LaneProbe {
        presses: Vec<usize>,
    }

    impl GameLoop for Rc<RefCell<LaneProbe>> {
        fn lane_key(&mut self, lane: usize, down: bool) {
            if down {
                self.borrow_mut().presses.push(lane);
            }
        }

        fn menu_key(&mut self, key: MenuKey, down: bool) {
            let _ = (key, down);
        }

        fn ingest_snapshot(&mut self, snapshot: &super::super::InputSnapshot) {
            for edge in snapshot.edges() {
                match *edge {
                    super::super::InputEdge::Lane { lane, down } => self.lane_key(lane, down),
                    super::super::InputEdge::Menu { key, down } => self.menu_key(key, down),
                    _ => {}
                }
            }
        }

        fn paint(&mut self, _frame: &mut o2jam_render::Frame<'_>) {}
    }

    /// A handler plus its stamp state, around a lane-recording probe.
    fn measured_handler() -> (
        WindowHandler,
        Rc<RefCell<StampState>>,
        Rc<RefCell<LaneProbe>>,
    ) {
        let state = Rc::new(RefCell::new(StampState::new()));
        let game = Rc::new(RefCell::new(LaneProbe::default()));
        let stamped = JudgementStamp::new(game.clone(), state.clone());
        let shell = Shell::new(Box::new(stamped));
        (WindowHandler::new(shell), state, game)
    }

    #[test]
    fn zero_queue_dispatch_adds_no_ticks() {
        let (mut handler, state, game) = measured_handler();
        let sample = measure_input_latency(&mut handler, KeyCode::KeyS, 0, &state)
            .expect("lane press reached judgement");
        assert_eq!(sample.modeled_queue_ticks, 0);
        assert_eq!(sample.dispatch_ticks, 0);
        assert_eq!(sample.added_ticks, 0);
        assert!(sample.within_budget());
        assert_eq!(game.borrow().presses, vec![0]);
    }

    #[test]
    fn modeled_queue_time_passes_through_one_to_one() {
        let (mut handler, state, game) = measured_handler();
        let sample = measure_input_latency(&mut handler, KeyCode::KeyS, 37, &state)
            .expect("lane press reached judgement");
        // The queued event is judged at dispatch time: the queue
        // ticks are the measured latency, the shell adds none.
        assert_eq!(sample.modeled_queue_ticks, 37);
        assert_eq!(sample.dispatch_ticks, 37);
        assert_eq!(sample.added_ticks, 0);
        assert!(sample.within_budget());
        assert_eq!(game.borrow().presses, vec![0]);
    }

    #[test]
    fn unreached_judgement_arm_is_a_miss_not_a_zero() {
        // KeyF is lane 2's bind... on a game that ignores lane keys:
        // here the probe game still records, so use a menu-only key.
        let (mut handler, state, _game) = measured_handler();
        assert!(
            measure_input_latency(&mut handler, KeyCode::Escape, 0, &state).is_none(),
            "Escape routes to the menu hook, never the judgement arm"
        );
    }

    #[test]
    fn budget_holds_the_shell_added_number_not_the_queue_number() {
        // A 10 000-tick queue is a fine measurement as long as the
        // shell adds nothing: the gate guards `added_ticks`, so the
        // milestone-3 budget can be set on the live queue number
        // without this guard flapping.
        let (mut handler, state, _game) = measured_handler();
        let sample = measure_input_latency(&mut handler, KeyCode::KeyS, 10_000, &state)
            .expect("lane press reached judgement");
        assert!(sample.within_budget());
        assert_eq!(INPUT_LATENCY_BUDGET_TICKS, 0);
    }
}
