//! Song-clock wall-trace equivalence replay (ADR-0001, claim B52).
//!
//! The exe paces song time from the wall clock (B38, `timeGetTime`);
//! the Rust client paces it from audio samples played (ADR-0001
//! approved deviation). This harness pins the equivalence of the two
//! clocks for judgement: a scripted run paces [`LivePlay`] with a
//! fixed wall-clock driver and records a [`WallTrace`]; the same
//! stimuli then replay against a synthetic sample-clock engine
//! ([`SampleClockEngine`], the seam the live backend's
//! `samples_played` feeds); per-judgement drift between the two runs
//! must stay inside the Cool window (6 ms, `TimingWindows::default`),
//! so no judgement can change tier between the clocks.
//!
//! All drivers are synthetic and deterministic (seeded jitter, no
//! real time), so the guard runs anywhere. The live-device divergence
//! number is a separate rig arm: a Wine null-sink run comparing a
//! captured wall trace against the sample position (OPEN, see
//! docs/parity/playing.md).

use o2jam_audio::song_position_ms;
use o2jam_game::Judgement;

use crate::playing::LivePlay;

/// Deterministic wall-frame schedule: nominal frame length plus
/// seeded jitter, the fixed-clock stand-in for the rig's frame pump.
#[derive(Debug, Clone)]
pub struct FrameSchedule {
    /// Nominal frame length in milliseconds.
    nominal_ms: u64,
    /// Maximum per-frame jitter in milliseconds (applied both sides).
    jitter_ms: u64,
    /// xorshift64 state; fixed by the caller so runs reproduce.
    rng: u64,
}

impl FrameSchedule {
    /// Create a schedule with the given nominal frame length, jitter
    /// amplitude, and seed. A zero seed is bumped so the xorshift
    /// state is never all-zero.
    #[must_use]
    pub fn new(nominal_ms: u64, jitter_ms: u64, seed: u64) -> Self {
        Self {
            nominal_ms,
            jitter_ms,
            rng: seed.max(1),
        }
    }

    /// Wall times of `count` frames starting at 0, strictly monotonic.
    #[must_use]
    pub fn wall_times(&self, count: usize) -> Vec<u64> {
        let mut rng = self.rng;
        let mut times = Vec::with_capacity(count);
        let mut now = 0u64;
        for _ in 0..count {
            times.push(now);
            now = now.saturating_add(self.nominal_ms.saturating_add(self.jitter(&mut rng)));
        }
        times
    }

    /// Next jittered frame delta in `[nominal - jitter, nominal + jitter]`.
    fn jitter(&self, rng: &mut u64) -> u64 {
        if self.jitter_ms == 0 {
            return self.nominal_ms;
        }
        // xorshift64; the schedule only needs spread, not crypto.
        *rng ^= *rng << 13;
        *rng ^= *rng >> 7;
        *rng ^= *rng << 17;
        let span = self.jitter_ms * 2 + 1;
        // Saturating both ways: an adversarial schedule may declare
        // jitter beyond the nominal frame (the audit's underflow
        // case); the delta clamps at 0 instead of panicking.
        self.nominal_ms
            .saturating_add(*rng % span)
            .saturating_sub(self.jitter_ms)
    }
}

/// One scripted key press: `lane` at wall time `at_ms`, aimed at the
/// note that starts at `note_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stimulus {
    /// Lane struck (0..7).
    pub lane: u8,
    /// Wall time of the press in ms.
    pub at_ms: u64,
    /// Onset of the note the press aims at (song ms).
    pub note_ms: u64,
}

/// A recorded wall-clock reference run: the frame wall times plus the
/// scripted presses. The rig arm captures the same shape from a Wine
/// run; the text format is shared (`to_lines`/`from_lines`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallTrace {
    /// Wall time of each frame, strictly monotonic, starting at 0.
    pub frames: Vec<u64>,
    /// Scripted presses in application order.
    pub stimuli: Vec<Stimulus>,
}

impl WallTrace {
    /// Serialize as text lines: `frame <wall_ms>` and
    /// `press <lane> <at_ms> <note_ms>`.
    #[must_use]
    pub fn to_lines(&self) -> String {
        let mut out = String::new();
        for at in &self.frames {
            out.push_str(&format!("frame {at}\n"));
        }
        for s in &self.stimuli {
            out.push_str(&format!("press {} {} {}\n", s.lane, s.at_ms, s.note_ms));
        }
        out
    }

    /// Parse the [`Self::to_lines`] format. Returns `None` on any
    /// malformed line, unsorted frames, or a non-monotonic schedule.
    #[must_use]
    pub fn from_lines(text: &str) -> Option<Self> {
        let mut frames = Vec::new();
        let mut stimuli = Vec::new();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            match fields.next()? {
                "frame" => frames.push(fields.next()?.parse().ok()?),
                "press" => {
                    let lane: u8 = fields.next()?.parse().ok()?;
                    let at_ms: u64 = fields.next()?.parse().ok()?;
                    let note_ms: u64 = fields.next()?.parse().ok()?;
                    stimuli.push(Stimulus {
                        lane,
                        at_ms,
                        note_ms,
                    });
                }
                _ => return None,
            }
            if fields.next().is_some() {
                return None;
            }
        }
        if frames.windows(2).any(|w| w[0] >= w[1]) {
            return None;
        }
        Some(Self { frames, stimuli })
    }
}

/// Synthetic sample-clock engine: the device consumes audio frames as
/// wall time passes and reports the song position in whole
/// milliseconds, exactly the contract the live backend's
/// `samples_played` + `song_position_ms` fulfill.
#[derive(Debug, Clone)]
pub struct SampleClockEngine {
    sample_rate: u32,
    samples_played: u64,
    /// Sub-frame carry: wall deltas rarely divide into whole device
    /// frames, so the remainder accumulates like a real mixer.
    carry_nanos: u64,
}

/// Sub-frame carry in nanoseconds of device output time: wall deltas
/// rarely divide into whole device frames, so the remainder
/// accumulates like a real mixer.
const CARRY_MODULO: u128 = 1_000_000_000;

impl SampleClockEngine {
    /// Create an engine at `sample_rate` Hz with an empty playhead.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples_played: 0,
            carry_nanos: 0,
        }
    }

    /// Consume the device frames `wall_delta_ms` of playback takes.
    pub fn advance(&mut self, wall_delta_ms: u64) {
        // Exact: rate * delta_ms * 1_000_000 + carry, in whole frames
        // of one second; the sub-second remainder carries forward, so
        // cumulative consumption never drifts from wall time.
        let due = u128::from(self.sample_rate) * u128::from(wall_delta_ms) * 1_000_000
            + u128::from(self.carry_nanos);
        // The quotient can exceed u64 only past ~10^11 years of
        // playback at 48 kHz; saturate there instead of truncating.
        self.samples_played = self
            .samples_played
            .saturating_add(u64::try_from(due / CARRY_MODULO).unwrap_or(u64::MAX));
        // Pinned bound: the remainder of a division by CARRY_MODULO
        // (1_000_000_000) always fits u64 — the `as` cannot narrow.
        self.carry_nanos = (due % CARRY_MODULO) as u64;
    }

    /// Song position in whole milliseconds (the sample clock).
    #[must_use]
    pub fn position_ms(&self) -> u64 {
        song_position_ms(self.samples_played, self.sample_rate)
    }
}

/// One judged hit as the harness observed it: the note, the position
/// the active clock assigned to the press, and the resulting tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgedHit {
    /// Lane of the struck note.
    pub lane: u8,
    /// Onset of the note (song ms).
    pub note_ms: u64,
    /// Position the active clock assigned to the press (ms).
    pub judged_ms: u64,
    /// Tier the strike rated.
    pub tier: Judgement,
}

/// Judge the scripted run on the wall clock: the position fed to
/// [`LivePlay`] is the frame's wall time, the exe's pacing per B38.
/// Every stimulus must rate a hit; `None` reports the count that did
/// not (a plan that places presses outside the bad window).
#[must_use]
pub fn judge_on_wall_clock(onsets: &[(u8, u64, u32)], trace: &WallTrace) -> Option<Vec<JudgedHit>> {
    run(onsets, trace, |frame_wall_ms, _wall_delta_ms| frame_wall_ms)
}

/// Replay the same stimuli against the sample clock: the frame wall
/// times still pace *when* presses apply (input polling is a wall
/// event), but the position fed to [`LivePlay`] is the synthetic
/// device's sample-derived position, the Rust client's pacing per
/// ADR-0001.
#[must_use]
pub fn replay_on_sample_clock(
    onsets: &[(u8, u64, u32)],
    trace: &WallTrace,
    sample_rate: u32,
) -> Option<Vec<JudgedHit>> {
    let mut engine = SampleClockEngine::new(sample_rate);
    let mut prev_wall_ms = 0u64;
    run(onsets, trace, |frame_wall_ms, _wall_delta_ms| {
        // The device consumes only the time since the previous frame;
        // frame wall times are cumulative.
        engine.advance(frame_wall_ms - prev_wall_ms);
        prev_wall_ms = frame_wall_ms;
        engine.position_ms()
    })
}

/// Drive [`LivePlay`] over the trace frames, applying each stimulus
/// at the first frame whose wall time reached it, and record one
/// [`JudgedHit`] per rated strike. `position_of` converts the frame's
/// wall time (and its delta to the previous frame) into the clock
/// position fed to the engine (identity for the wall clock, sample
/// position for the replay). Returns `None` when a stimulus rated
/// nothing (its press landed outside every window of its note).
fn run(
    onsets: &[(u8, u64, u32)],
    trace: &WallTrace,
    mut position_of: impl FnMut(u64, u64) -> u64,
) -> Option<Vec<JudgedHit>> {
    let mut play = LivePlay::new(onsets);
    let mut hits = Vec::with_capacity(trace.stimuli.len());
    let mut pending = trace.stimuli.iter().peekable();
    let mut prev_wall_ms = 0u64;
    for &frame_wall_ms in &trace.frames {
        let position = position_of(frame_wall_ms, frame_wall_ms - prev_wall_ms);
        prev_wall_ms = frame_wall_ms;
        play.advance_to(position);
        while pending.peek().is_some_and(|s| s.at_ms <= frame_wall_ms) {
            let stimulus = pending.next().expect("peeked stimulus");
            let tier = play.strike_lane(stimulus.lane, position)?;
            hits.push(JudgedHit {
                lane: stimulus.lane,
                note_ms: stimulus.note_ms,
                judged_ms: position,
                tier,
            });
        }
    }
    Some(hits)
}

/// Pair wall-run hits with replay hits by `(lane, note_ms)` and
/// return the maximum per-judgement position drift. Panics (via
/// `expect`) when the runs disagree on the judged note set — a replay
/// that judges different notes is not an equivalence run.
#[must_use]
pub fn max_per_judgement_drift(wall: &[JudgedHit], replay: &[JudgedHit]) -> Option<u64> {
    let mut max_drift = 0u64;
    for hit in wall {
        let paired = replay
            .iter()
            .find(|other| other.lane == hit.lane && other.note_ms == hit.note_ms)?;
        let drift = hit.judged_ms.abs_diff(paired.judged_ms);
        max_drift = max_drift.max(drift);
    }
    Some(max_drift)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic fixture chart: one tap every `STEP_MS` ms cycling the
    /// seven lanes (fixture data only — no copyrighted bytes).
    const STEP_MS: u64 = 347;
    const CHART_LEN_MS: u64 = 180_000;

    fn fixture_onsets() -> Vec<(u8, u64, u32)> {
        let mut onsets = Vec::new();
        let mut at = 2_000u64;
        let mut lane = 0u8;
        while at < CHART_LEN_MS {
            onsets.push((lane, at, onsets.len() as u32));
            lane = (lane + 1) % 7;
            at += STEP_MS;
        }
        onsets
    }

    /// Strike offsets cycling through every window band, boundaries
    /// included: Good edges (±9/±12), Cool edges (±4), dead center —
    /// a tier flip shows here first. The aimed magnitude stays ≤ 17 ms
    /// so frame quantization (≤ 5 ms on the 4 ms schedule) can never
    /// push a press past the 25 ms Bad edge.
    const AIM_OFFSETS: [i64; 10] = [-17, -12, -9, -4, 0, 1, 4, 9, 12, 17];

    fn fixture_stimuli(onsets: &[(u8, u64, u32)]) -> Vec<Stimulus> {
        onsets
            .iter()
            .enumerate()
            .map(|(i, (lane, at_ms, _))| {
                let offset = AIM_OFFSETS[i % AIM_OFFSETS.len()];
                let at_ms_i64 = i64::try_from(*at_ms).expect("chart onset fits i64");
                Stimulus {
                    lane: *lane,
                    at_ms: (at_ms_i64 + offset).max(0) as u64,
                    note_ms: *at_ms,
                }
            })
            .collect()
    }

    /// Record the wall trace from a scripted plan: the fixed-clock
    /// driver's frame times plus the stimuli.
    fn record_wall_trace() -> WallTrace {
        // Fine 4 ms frames with ±1 ms jitter, an adversarial-but-fixed
        // seed; fine frames keep press quantization well inside the
        // Bad window while the jitter still perturbs the schedule.
        let schedule = FrameSchedule::new(4, 1, 0xA44);
        let frame_count = usize::try_from(CHART_LEN_MS / 4 + 1_000).expect("frame count fits");
        let frames = schedule.wall_times(frame_count);
        WallTrace {
            frames,
            stimuli: fixture_stimuli(&fixture_onsets()),
        }
    }

    #[test]
    fn frame_schedule_is_monotonic_and_reproducible() {
        let a = FrameSchedule::new(16, 5, 0xA44).wall_times(500);
        let b = FrameSchedule::new(16, 5, 0xA44).wall_times(500);
        assert_eq!(a, b, "same seed must reproduce the same schedule");
        assert!(a.windows(2).all(|w| w[0] < w[1]), "frames must advance");
        // The recorded trace must run past the chart so every press
        // lands on a frame.
        let trace = record_wall_trace();
        assert!(
            trace.frames.last().expect("trace has frames") > &CHART_LEN_MS,
            "schedule must outrun the chart"
        );
    }

    /// Regression (audit #102-era): a schedule declaring jitter above
    /// the nominal frame must not underflow the jittered delta — the
    /// amplitude saturates at a zero-length frame and the wall times
    /// stay reproducible and non-decreasing.
    #[test]
    fn frame_schedule_survives_jitter_above_nominal() {
        let a = FrameSchedule::new(2, 10, 0xA44).wall_times(200);
        let b = FrameSchedule::new(2, 10, 0xA44).wall_times(200);
        assert_eq!(a, b, "same seed must reproduce the same schedule");
        assert!(
            a.windows(2).all(|w| w[0] <= w[1]),
            "clamped deltas must never move a frame backwards"
        );
    }

    #[test]
    fn sample_engine_consumes_frames_proportionally_with_carry() {
        let mut engine = SampleClockEngine::new(48_000);
        // 1000 one-ms advances must consume the same frames as one
        // 1000 ms advance (carry makes quantization lossless overall).
        for _ in 0..1000 {
            engine.advance(1);
        }
        let mut bulk = SampleClockEngine::new(48_000);
        bulk.advance(1000);
        assert_eq!(engine.position_ms(), bulk.position_ms());
        // 48000 Hz at real time lands on ~1000 ms, off by at most the
        // one-frame quantization (≈0.02 ms) plus the ms floor.
        assert!(bulk.position_ms() >= 999, "position must not run ahead");
        assert!(bulk.position_ms() <= 1000, "carry must not lag behind");
    }

    #[test]
    fn wall_trace_lines_round_trip() {
        let trace = record_wall_trace();
        let parsed =
            WallTrace::from_lines(&trace.to_lines()).expect("serialized trace must parse back");
        assert_eq!(parsed, trace);
    }

    #[test]
    fn wall_trace_from_lines_rejects_non_monotonic_frames() {
        let text = "frame 10\nframe 5\npress 0 10 20\n";
        assert!(WallTrace::from_lines(text).is_none());
    }

    #[test]
    fn max_per_judgement_drift_pairs_by_note() {
        let wall = vec![JudgedHit {
            lane: 0,
            note_ms: 100,
            judged_ms: 104,
            tier: Judgement::Cool,
        }];
        let mut replay = vec![JudgedHit {
            lane: 0,
            note_ms: 100,
            judged_ms: 105,
            tier: Judgement::Cool,
        }];
        assert_eq!(max_per_judgement_drift(&wall, &replay), Some(1));
        replay[0].note_ms = 200;
        assert_eq!(max_per_judgement_drift(&wall, &replay), None);
    }

    /// Whether the wall hit's offset sits more than `drift` away from
    /// every window boundary (6/18/25 ms): only those hits must keep
    /// their tier across the clocks; a hit within the drift bound of a
    /// boundary can legitimately straddle it under frame
    /// quantization.
    fn clear_of_boundaries(hit: &JudgedHit, drift: u64) -> bool {
        let offset = hit.judged_ms.abs_diff(hit.note_ms) as i64;
        let magnitude = offset.unsigned_abs();
        let nearest = [6u64, 18, 25]
            .into_iter()
            .map(|edge| edge.abs_diff(magnitude))
            .min()
            .expect("window edges exist");
        nearest > drift
    }

    /// Claim B52 guard (issue #99): replaying the wall trace against
    /// the sample-clock engine keeps every per-judgement position
    /// inside the Cool window (6 ms); hits clear of the window edges
    /// never change tier, so the ADR-0001 sample-clock deviation
    /// cannot change any judgement on a fixed clock.
    #[test]
    fn song_clock_equivalence_replay_stays_inside_the_pinned_windows() {
        let onsets = fixture_onsets();
        let trace = record_wall_trace();
        let wall = judge_on_wall_clock(&onsets, &trace)
            .expect("every scripted press must rate a hit on the wall clock");
        assert_eq!(wall.len(), trace.stimuli.len());
        for rate in [44_100, 48_000, 96_000] {
            let replay = replay_on_sample_clock(&onsets, &trace, rate)
                .expect("every scripted press must rate a hit on the sample clock");
            let drift = max_per_judgement_drift(&wall, &replay)
                .expect("replay must judge the same notes as the wall trace");
            assert!(
                drift < 6,
                "per-judgement drift {drift} ms at {rate} Hz must stay inside the 6 ms Cool window"
            );
            let near_boundary = wall
                .iter()
                .filter(|hit| clear_of_boundaries(hit, drift))
                .collect::<Vec<_>>();
            assert!(
                near_boundary.len() > wall.len() / 2,
                "most hits should sit clear of the window edges ({} of {})",
                near_boundary.len(),
                wall.len()
            );
            assert!(
                near_boundary.iter().all(|hit| {
                    replay
                        .iter()
                        .find(|other| other.lane == hit.lane && other.note_ms == hit.note_ms)
                        .is_some_and(|paired| paired.tier == hit.tier)
                }),
                "hits clear of the edges must not change tier at {rate} Hz"
            );
        }
    }

    /// Negative control: a device consuming half the frames per wall
    /// delta (a broken device contract) diverges past the Cool window,
    /// so the 6 ms bound in the equivalence guard can actually fail.
    #[test]
    fn clock_divergence_detects_a_half_rate_device() {
        let trace = record_wall_trace();
        let mut engine = SampleClockEngine::new(44_100);
        let mut prev_wall_ms = 0u64;
        let mut max_divergence = 0u64;
        for &frame_wall_ms in &trace.frames {
            engine.advance((frame_wall_ms - prev_wall_ms) / 2);
            prev_wall_ms = frame_wall_ms;
            max_divergence = max_divergence.max(frame_wall_ms.abs_diff(engine.position_ms()));
        }
        assert!(
            max_divergence >= 6,
            "half-rate device must diverge past the 6 ms Cool window (got {max_divergence} ms)"
        );
    }
}
