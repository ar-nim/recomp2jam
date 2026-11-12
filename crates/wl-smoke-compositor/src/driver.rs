//! Shared driver helpers for the smoke examples: game-binary
//! resolution, spawn discipline, map-wait, and PID-recorded teardown.
//!
//! Both examples (`frame_callback_smoke`, `input_smoke`) spawn the game
//! the same way: temp runtime dir, the fixture's own socket, every
//! session-display leak scrubbed (`DISPLAY`, `WAYLAND_SOCKET`,
//! `WINIT_UNIX_BACKEND`), game output into `<runtime>/game.log`. Keeping
//! that discipline in one place means the two probes can never drift on
//! how the game is launched.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::Compositor;

/// Failed-authorize login against the local Memoryer is fine: the loop
/// still maps and paints the splash, which is all the probes need.
pub const GAME_ARGS: [&str; 9] = [
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

/// Launch the game with Wayland forced: our socket, our runtime dir, and
/// no DISPLAY / WINIT_UNIX_BACKEND / inherited-session socket leaks.
/// `extra_args` (e.g. `--input-log`) append to [`GAME_ARGS`].
pub fn spawn_game(
    binary: &Path,
    runtime_dir: &Path,
    socket_name: &std::ffi::OsStr,
    extra_args: &[String],
) -> Result<Child> {
    let log = fs::File::create(runtime_dir.join("game.log"))?;
    Command::new(binary)
        .args(GAME_ARGS)
        .args(extra_args)
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

/// The game binary: explicit arg, then the example env var cargo sets when
/// a binary target is in scope, then the workspace's debug build. The
/// default path is kept honest: built when missing, rebuilt when it
/// predates the workspace sources, so a stale binary can never silently
/// test the wrong code (a real incident: a pre-fix binary reported RED
/// on the fixed tree).
pub fn resolve_game_binary(arg: Option<&str>) -> Result<PathBuf> {
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
        eprintln!("smoke-driver: game binary predates the workspace sources; rebuilding");
        build_game_binary()?;
    }
    if default.is_file() {
        Ok(default)
    } else {
        bail!(
            "game binary not found at {}; `cargo build -p recomp2jam` failed or was not run — \
             build it or pass its path as an argument",
            default.display()
        )
    }
}

pub fn workspace_root() -> PathBuf {
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

/// Pump the compositor until the game commits its first buffer, with the
/// map-wait lesson from the KWin harness applied (startup churn can take
/// a while; a failed authorize still maps and paints).
pub fn wait_for_first_paint(
    compositor: &mut Compositor,
    game: &mut Child,
    scratch: &Path,
    map_wait: Duration,
) -> Result<()> {
    let deadline = Instant::now() + map_wait;
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
        "no first paint within {map_wait:?}; game log tail:\n{}",
        log_tail(scratch)
    )
}

/// PID-recorded kill (scripts/rig-run.sh discipline, never pkill -f):
/// signal only while /proc still shows the process we started.
pub fn stop_game_verified(game: &mut Child, probe_name: &str) {
    let pid = game.id();
    let cmdline = fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    if cmdline.contains("recomp2jam") {
        let _ = game.kill();
        let _ = game.wait();
        println!("{probe_name}: stopped game (pid {pid})");
    } else {
        println!("{probe_name}: pid {pid} no longer matches 'recomp2jam' — not signalling");
    }
}

pub fn game_exit_error(status: std::process::ExitStatus, scratch: &Path) -> anyhow::Error {
    anyhow::anyhow!(
        "game exited mid-probe ({status}); log tail:\n{}",
        log_tail(scratch)
    )
}

pub fn log_tail(scratch: &Path) -> String {
    let log = scratch.join("game.log");
    let Ok(content) = fs::read_to_string(&log) else {
        return format!("(no log at {})", log.display());
    };
    let tail: Vec<&str> = content.lines().collect();
    tail[tail.len().saturating_sub(15)..].join("\n")
}
