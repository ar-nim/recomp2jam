//! Headless frame-callback starvation probe — the L2-Wayland-CI arm of the
//! #118 red-green, runnable anywhere (Refs: #126).
//!
//! Spawns the game binary against this crate's in-process compositor on a
//! temp-dir socket, waits for map + first paint, then samples
//! `/proc/<pid>/stat` utime+stime over 5 x 6 s windows:
//!
//! - `starve`: the compositor never sends `wl_callback.done`. PASS: every
//!   window < 20% CPU (the #118 wedge pins a core). FAIL: any window
//!   >= 60%.
//! - `flow`: callbacks complete at ~60 Hz. Sanity arm: the game must keep
//!   painting (paints counted by the compositor must grow during sampling)
//!   and stay below the 60% fail band — so an idle verdict can never pass
//!   by the game simply never painting. Under repaint-on-demand (#138)
//!   the pump (not the assertion) moved: the probe injects one pointer
//!   motion per half-window, and every motion owes exactly one repaint
//!   through the hover-dirty path.
//! - `idle`: callbacks flow at ~60 Hz against an EXPOSED window (no
//!   occlusion) while the game sits untouched after the first paint:
//!   the #138 repaint-on-demand guard. PASS: every window < 20% CPU
//!   (a repaint-on-idle loop pins ~60%+ here, like the pre-fix wedge).
//!   FAIL otherwise — no inconclusive band: the window stays exposed
//!   with flowing callbacks, so any sample above the pass band is
//!   repaint waste. Liveness rides the first-paint gate
//!   (the game mapped and painted), so — unlike `flow` — this arm
//!   expects paints-during-sampling near zero and asserts no floor.
//!
//! Usage: `frame_callback_smoke [starve|flow|idle] [GAME-BINARY]`.
//! Exit codes: 0 = verdict PASS, 1 = verdict FAIL, 2 = harness error.

use std::fs;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use wl_smoke_compositor::driver;
use wl_smoke_compositor::{Compositor, CompositorStats, FrameCallbackPolicy, PointerEvent};

// Acceptance bands from issue #118 (same as scripts/wayland-loop-smoke.sh).
const MAP_WAIT: Duration = Duration::from_secs(20);
const SETTLE: Duration = Duration::from_secs(3);
const WINDOW_COUNT: usize = 5;
const WINDOW_SECS: u64 = 6;
const PASS_MAX_PERCENT: f64 = 20.0;
const FAIL_MIN_PERCENT: f64 = 60.0;
/// Flow-mode sanity band: a debug-profile game repainting at ~60 Hz runs
/// its software raster nearly every frame (~62% of one core measured,
/// 2026-11-12), which is healthy — liveness is what this arm asserts, so
/// only a wedge-class window (no frames would ever pace that high and
/// still complete) fails it.
const FLOW_FAIL_MIN_PERCENT: f64 = 90.0;
/// Fewer paints than this across 30 s of flowing callbacks means the loop
/// is not actually repainting.
const FLOW_MIN_PAINTS: u64 = 10;
/// Linux `USER_HZ`; /proc utime/stime are ticks of this clock (100 on
/// every Linux target this repo runs on).
const USER_HZ: u64 = 100;

fn main() {
    let verdict = match run() {
        Ok(verdict) => verdict,
        Err(error) => {
            eprintln!("frame-callback-smoke: harness error: {error:#}");
            std::process::exit(2);
        }
    };
    println!("frame-callback-smoke: {}", verdict.line);
    std::process::exit(verdict.code);
}

/// What the probe feeds the game and which bands judge it: `Idle` rides
/// flowing callbacks like `Flow` (the window stays exposed) but judges
/// like `Starve` (the untouched game must idle near 0% CPU).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeMode {
    Starve,
    Flow,
    Idle,
}

impl ProbeMode {
    /// Compositor callback behavior for this mode.
    fn frame_policy(self) -> FrameCallbackPolicy {
        match self {
            ProbeMode::Starve => FrameCallbackPolicy::Starve,
            ProbeMode::Flow | ProbeMode::Idle => FrameCallbackPolicy::Flow,
        }
    }

    /// Whether this mode floors paints-during-sampling (only `Flow`
    /// proves liveness by repaint volume; `Idle` proves it by the
    /// first-paint gate and expects near-zero paints after).
    fn floors_paints(self) -> bool {
        self == ProbeMode::Flow
    }
}

struct Verdict {
    line: String,
    code: i32,
}

fn run() -> Result<Verdict> {
    let mut args = std::env::args().skip(1);
    let mode = match args.next().as_deref() {
        Some("starve") => ProbeMode::Starve,
        Some("flow") => ProbeMode::Flow,
        Some("idle") => ProbeMode::Idle,
        other => {
            bail!("usage: frame_callback_smoke [starve|flow|idle] [GAME-BINARY] (got {other:?})")
        }
    };
    let policy = mode.frame_policy();
    let game_binary = driver::resolve_game_binary(args.next().as_deref())?;

    let scratch = tempfile::tempdir().context("create scratch dir")?;
    let socket_name = format!("wl-smoke-{}", std::process::id());
    let mut compositor = Compositor::bind(scratch.path(), &socket_name, policy)?;
    let socket_file_name = compositor
        .socket_path()
        .file_name()
        .context("socket path has no file name")?
        .to_owned();

    let mut game = driver::spawn_game(&game_binary, scratch.path(), &socket_file_name, &[])?;
    println!(
        "frame-callback-smoke: game pid {} on socket {} (log: {})",
        game.id(),
        socket_name,
        scratch.path().join("game.log").display()
    );

    driver::wait_for_first_paint(&mut compositor, &mut game, scratch.path(), MAP_WAIT)?;
    println!("frame-callback-smoke: first paint seen; settling {SETTLE:?}");
    compositor.run_until(Instant::now() + SETTLE)?;

    // The flow pump: one pointer motion per half-window owes one
    // hover-dirty repaint each, so liveness still shows up as paint
    // volume under repaint-on-demand. Enter first: a bare motion
    // before enter is off-protocol and clients may ignore it.
    let motion_target = if mode.floors_paints() {
        let (x, y) = wait_for_motion_target(&mut compositor, &mut game, scratch.path())?;
        compositor.inject_pointer(PointerEvent::Enter { x, y });
        compositor.turn()?;
        Some((x, y))
    } else {
        None
    };

    let paints_at_sample_start = compositor.stats().paints;
    let samples = sample_cpu_windows(&mut compositor, &mut game, scratch.path(), motion_target)?;
    let paints_during_sampling = compositor.stats().paints - paints_at_sample_start;
    let stats = compositor.stats();
    driver::stop_game_verified(&mut game, "frame-callback-smoke");

    Ok(build_verdict(
        mode,
        &samples,
        paints_during_sampling,
        &stats,
    ))
}

/// The pointer position flow motions aim at: first paint plus a bound
/// pointer and a committed toplevel (the input probe's preconditions).
fn wait_for_motion_target(
    compositor: &mut Compositor,
    game: &mut Child,
    scratch: &Path,
) -> Result<(f64, f64)> {
    let deadline = Instant::now() + MAP_WAIT;
    while Instant::now() < deadline {
        compositor.turn()?;
        if compositor.pointer_ready() {
            if let Some(center) = compositor.toplevel_center() {
                return Ok(center);
            }
        }
        if let Some(status) = game.try_wait()? {
            return Err(driver::game_exit_error(status, scratch));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    bail!("no motion target (pointer + toplevel) within {MAP_WAIT:?}");
}

/// The 5 x 6 s sampling run: per-window utime+stime deltas in percent of
/// one core. Also the natural place to die early if the game exits.
/// With a motion target (flow mode), each half-window opens with one
/// pointer motion — one hover-dirty repaint per motion — so the liveness
/// floor keeps meaning paint volume without ever spinning the loop.
fn sample_cpu_windows(
    compositor: &mut Compositor,
    game: &mut Child,
    scratch: &Path,
    motion_target: Option<(f64, f64)>,
) -> Result<Vec<f64>> {
    const HALVES: u64 = 2;
    let mut samples = Vec::with_capacity(WINDOW_COUNT);
    let mut previous_ticks = cpu_ticks(game.id())?;
    for window in 0..WINDOW_COUNT {
        for half in 0..HALVES {
            if let Some((x, y)) = motion_target {
                // Alternate a pixel sideways so consecutive motions
                // always move the hover point, never re-arm the same.
                let nudge = f64::from(((window * HALVES as usize + half as usize) % 2) as u8);
                compositor.inject_pointer(PointerEvent::Motion { x: x + nudge, y });
                compositor.turn()?;
            }
            compositor
                .run_until(Instant::now() + Duration::from_secs(WINDOW_SECS) / HALVES as u32)?;
            if let Some(status) = game.try_wait()? {
                return Err(driver::game_exit_error(status, scratch));
            }
        }
        let ticks = cpu_ticks(game.id())?;
        let window_ticks = ticks.saturating_sub(previous_ticks);
        previous_ticks = ticks;
        let capacity = WINDOW_SECS * USER_HZ;
        samples.push(100.0 * window_ticks as f64 / capacity as f64);
        println!(
            "frame-callback-smoke: window {} -> {:.1}% cpu, {} paints total",
            samples.len(),
            samples.last().expect("just pushed"),
            compositor.stats().paints
        );
    }
    Ok(samples)
}

/// utime + stime of `pid` in clock ticks (fields 14+15 of /proc/<pid>/stat).
fn cpu_ticks(pid: u32) -> Result<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))
        .with_context(|| format!("read /proc/{pid}/stat"))?;
    let after_comm = stat
        .rfind(')')
        .map(|index| &stat[index + 1..])
        .context("malformed /proc stat: no comm field")?;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    // fields[0] is state (proc field 3); utime is field 14, stime 15.
    let utime: u64 = fields[11].parse().context("parse utime")?;
    let stime: u64 = fields[12].parse().context("parse stime")?;
    Ok(utime + stime)
}

fn build_verdict(
    mode: ProbeMode,
    samples: &[f64],
    paints_during_sampling: u64,
    stats: &CompositorStats,
) -> Verdict {
    let max = samples.iter().copied().fold(0.0, f64::max);
    let sample_line = samples
        .iter()
        .map(|sample| format!("{sample:.1}"))
        .collect::<Vec<_>>()
        .join(", ");
    let line = |verdict: &str| {
        format!(
            "mode={mode:?} verdict={verdict} cpu-samples-pct=[{sample_line}] \
             max={max:.1} paints={} paints-during-sampling={paints_during_sampling} \
             pending-frame-callbacks={}",
            stats.paints, stats.pending_frame_callbacks
        )
    };
    let fail_band = if mode == ProbeMode::Flow {
        FLOW_FAIL_MIN_PERCENT
    } else {
        FAIL_MIN_PERCENT
    };
    if samples.iter().any(|sample| *sample >= fail_band) {
        return Verdict {
            line: line("FAIL"),
            code: 1,
        };
    }
    if mode.floors_paints() && paints_during_sampling < FLOW_MIN_PAINTS {
        return Verdict {
            line: line("FAIL(no-paints)"),
            code: 1,
        };
    }
    if mode != ProbeMode::Flow && samples.iter().all(|sample| *sample < PASS_MAX_PERCENT) {
        return Verdict {
            line: line("PASS"),
            code: 0,
        };
    }
    // `Idle` has no inconclusive band: the window stays exposed with
    // flowing callbacks, so any sample above the pass band is repaint
    // waste, never occlusion-timing noise (`Starve` keeps its middle
    // band: a minimized window can take a turn to go quiet).
    if mode == ProbeMode::Idle {
        return Verdict {
            line: line("FAIL(idle-waste)"),
            code: 1,
        };
    }
    if mode == ProbeMode::Flow {
        return Verdict {
            line: line("PASS"),
            code: 0,
        };
    }
    Verdict {
        line: line("INCONCLUSIVE"),
        code: 2,
    }
}
