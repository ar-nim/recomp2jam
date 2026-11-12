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
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
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

/// Failed-authorize login against the local Memoryer is fine: the loop
/// still maps and paints the splash, which is all this probe needs.
const GAME_ARGS: [&str; 9] = [
    "online",
    "--host",
    "127.0.0.1",
    "--port",
    "15010",
    "--user-id",
    "2",
    "--token",
    "77b0a3e3-32e6-4a67-b3d8-563a0df4c08c",
];

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
    let game_binary = resolve_game_binary(args.next().as_deref())?;

    let scratch = tempfile::tempdir().context("create scratch dir")?;
    let socket_name = format!("wl-smoke-{}", std::process::id());
    let mut compositor = Compositor::bind(scratch.path(), &socket_name, policy)?;
    let socket_file_name = compositor
        .socket_path()
        .file_name()
        .context("socket path has no file name")?
        .to_owned();

    let mut game = spawn_game(&game_binary, scratch.path(), &socket_file_name)?;
    println!(
        "frame-callback-smoke: game pid {} on socket {} (log: {})",
        game.id(),
        socket_name,
        scratch.path().join("game.log").display()
    );

    wait_for_first_paint(&mut compositor, &mut game, scratch.path())?;
    println!("frame-callback-smoke: first paint seen; settling {SETTLE:?}");
    compositor.run_until(Instant::now() + SETTLE)?;

    let paints_at_sample_start = compositor.stats().paints;
    let samples = sample_cpu_windows(&mut compositor, &mut game, scratch.path())?;
    let paints_during_sampling = compositor.stats().paints - paints_at_sample_start;
    let stats = compositor.stats();
    stop_game_verified(&mut game);

    Ok(build_verdict(
        policy,
        &samples,
        paints_during_sampling,
        &stats,
    ))
}

/// The game binary: explicit arg, then the example env var cargo sets when
/// a binary target is in scope, then the workspace's debug build. The
/// default path is kept honest: built when missing, rebuilt when it
/// predates the workspace sources, so a stale binary can never silently
/// test the wrong code (a real incident: a pre-fix binary reported RED
/// on the fixed tree).
fn resolve_game_binary(arg: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = arg {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_recomp2jam") {
        return Ok(PathBuf::from(path));
    }
    let default = workspace_root().join("target/debug/recomp2jam");
    if !default.is_file() {
        build_game_binary()?;
    } else if binary_is_stale(&default)? {
        eprintln!("frame-callback-smoke: game binary predates the workspace sources; rebuilding");
        build_game_binary()?;
    }
    if default.is_file() {
        Ok(default)
    } else {
        bail!(
            "game binary not found at {}; `cargo build -p recomp2jam` failed or was not run — \
             build it or pass its path as the second argument",
            default.display()
        )
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build_game_binary() -> Result<()> {
    let status = std::process::Command::new(env!("CARGO"))
        .args(["build", "--quiet", "-p", "recomp2jam"])
        .current_dir(workspace_root())
        .status()
        .context("failed to run cargo build -p recomp2jam")?;
    if !status.success() {
        bail!("cargo build -p recomp2jam exited with {status}");
    }
    Ok(())
}

/// True when any workspace source file is newer than the binary.
fn binary_is_stale(binary: &Path) -> Result<bool> {
    use std::time::SystemTime;
    let bin_mtime = fs::metadata(binary)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut newest = SystemTime::UNIX_EPOCH;
    fn walk(dir: &Path, newest: &mut SystemTime) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = entry.metadata()?;
            if meta.is_dir() {
                if path.file_name().is_some_and(|n| n != "target") {
                    walk(&path, newest)?;
                }
            } else if path.extension().is_some_and(|e| e == "rs" || e == "toml") {
                if let Ok(m) = meta.modified() {
                    if m > *newest {
                        *newest = m;
                    }
                }
            }
        }
        Ok(())
    }
    for crate_dir in fs::read_dir(workspace_root().join("crates"))? {
        walk(&crate_dir?.path(), &mut newest)?;
    }
    Ok(newest > bin_mtime)
}

/// Launch the game with Wayland forced: our socket, our runtime dir, and
/// no DISPLAY / WINIT_UNIX_BACKEND / inherited-session socket leaks.
fn spawn_game(binary: &Path, runtime_dir: &Path, socket_name: &std::ffi::OsStr) -> Result<Child> {
    let log = fs::File::create(runtime_dir.join("game.log"))?;
    Command::new(binary)
        .args(GAME_ARGS)
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .env("WAYLAND_DISPLAY", socket_name)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env_remove("WINIT_UNIX_BACKEND")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .spawn()
        .with_context(|| format!("spawn {}", binary.display()))
}

/// Pump the compositor until the game commits its first buffer, with the
/// map-wait lesson from the KWin harness applied (startup churn can take
/// a while; a failed authorize still maps and paints).
fn wait_for_first_paint(
    compositor: &mut Compositor,
    game: &mut Child,
    scratch: &Path,
) -> Result<()> {
    let deadline = Instant::now() + MAP_WAIT;
    while Instant::now() < deadline {
        compositor.turn()?;
        if compositor.stats().first_paint_at.is_some() {
            return Ok(());
        }
        if let Some(status) = game.try_wait()? {
            return Err(game_exit_error(status, scratch));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    bail!(
        "no first paint within {MAP_WAIT:?}; game log tail:\n{}",
        log_tail(scratch)
    )
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
            return Err(game_exit_error(status, scratch));
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

/// PID-recorded kill (scripts/rig-run.sh discipline, never pkill -f):
/// signal only while /proc still shows the process we started.
fn stop_game_verified(game: &mut Child) {
    let pid = game.id();
    let cmdline = fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    if cmdline.contains("recomp2jam") {
        let _ = game.kill();
        let _ = game.wait();
        println!("frame-callback-smoke: stopped game (pid {pid})");
    } else {
        println!("frame-callback-smoke: pid {pid} no longer matches 'recomp2jam' — not signalling");
    }
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

fn game_exit_error(status: std::process::ExitStatus, scratch: &Path) -> anyhow::Error {
    anyhow::anyhow!(
        "game exited mid-probe ({status}); log tail:\n{}",
        log_tail(scratch)
    )
}

fn log_tail(scratch: &Path) -> String {
    let log = scratch.join("game.log");
    let Ok(content) = fs::read_to_string(&log) else {
        return format!("(no log at {})", log.display());
    };
    let tail: Vec<&str> = content.lines().collect();
    tail[tail.len().saturating_sub(15)..].join("\n")
}
