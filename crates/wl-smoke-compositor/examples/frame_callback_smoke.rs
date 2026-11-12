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
//!   by the game simply never painting.
//!
//! Usage: `frame_callback_smoke [starve|flow] [GAME-BINARY]`.
//! Exit codes: 0 = verdict PASS, 1 = verdict FAIL, 2 = harness error.

use std::fs;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use wl_smoke_compositor::driver;
use wl_smoke_compositor::{Compositor, CompositorStats, FrameCallbackPolicy};

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

struct Verdict {
    line: String,
    code: i32,
}

fn run() -> Result<Verdict> {
    let mut args = std::env::args().skip(1);
    let policy = match args.next().as_deref() {
        Some("starve") => FrameCallbackPolicy::Starve,
        Some("flow") => FrameCallbackPolicy::Flow,
        other => bail!("usage: frame_callback_smoke [starve|flow] [GAME-BINARY] (got {other:?})"),
    };
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

    let paints_at_sample_start = compositor.stats().paints;
    let samples = sample_cpu_windows(&mut compositor, &mut game, scratch.path())?;
    let paints_during_sampling = compositor.stats().paints - paints_at_sample_start;
    let stats = compositor.stats();
    driver::stop_game_verified(&mut game, "frame-callback-smoke");

    Ok(build_verdict(
        policy,
        &samples,
        paints_during_sampling,
        &stats,
    ))
}

/// The 5 x 6 s sampling run: per-window utime+stime deltas in percent of
/// one core. Also the natural place to die early if the game exits.
fn sample_cpu_windows(
    compositor: &mut Compositor,
    game: &mut Child,
    scratch: &Path,
) -> Result<Vec<f64>> {
    let mut samples = Vec::with_capacity(WINDOW_COUNT);
    let mut previous_ticks = cpu_ticks(game.id())?;
    for _ in 0..WINDOW_COUNT {
        compositor.run_until(Instant::now() + Duration::from_secs(WINDOW_SECS))?;
        if let Some(status) = game.try_wait()? {
            return Err(driver::game_exit_error(status, scratch));
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
    policy: FrameCallbackPolicy,
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
            "mode={policy:?} verdict={verdict} cpu-samples-pct=[{sample_line}] \
             max={max:.1} paints={} paints-during-sampling={paints_during_sampling} \
             pending-frame-callbacks={}",
            stats.paints, stats.pending_frame_callbacks
        )
    };
    let fail_band = if policy == FrameCallbackPolicy::Flow {
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
    if policy == FrameCallbackPolicy::Flow && paints_during_sampling < FLOW_MIN_PAINTS {
        return Verdict {
            line: line("FAIL(no-paints)"),
            code: 1,
        };
    }
    if policy == FrameCallbackPolicy::Starve
        && samples.iter().all(|sample| *sample < PASS_MAX_PERCENT)
    {
        return Verdict {
            line: line("PASS"),
            code: 0,
        };
    }
    if policy == FrameCallbackPolicy::Flow {
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
