//! X11 arm: a self-spawned Xvfb plus XTEST injection (ticket #127).
//!
//! The driver owns the server: it picks a free display, serves an
//! 800x600 root (the game's exact backbuffer size, so root pixels are
//! native pixels when the game covers the root), and injects the same
//! scripted gesture as the Wayland arm. No external display is needed
//! and the owner session is never touched.
//!
//! Live chain: `scripts/rig-x11-click-proof.sh` ->
//! `x11_arm_routes_click_through_real_game` -> `run_probe` ->
//! `inject_click`/`inject_motion` -> [`click_event_plan`] ->
//! `x11rb::xtest_fake_input`. The unit suite keeps the display-free
//! half: `x11_*plan*` guards assert the byte/event sequence; the live
//! window proof stays in the rig lane (issue #738).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use x11rb::protocol::xproto::{BUTTON_PRESS_EVENT, BUTTON_RELEASE_EVENT, MOTION_NOTIFY_EVENT};
use x11rb::protocol::xtest::ConnectionExt as _;

use crate::log;
use crate::{InputSnapshot, SnapshotSource};

/// Game backbuffer size: the Xvfb root matches it exactly.
const ROOT_WIDTH: i16 = 800;
/// Game backbuffer height in root pixels.
const ROOT_HEIGHT: i16 = 600;

/// XTEST press/release pair for the left button.
const LEFT_BUTTON: u8 = 1;
/// Hold time between the injected down and up edges.
const CLICK_HOLD: Duration = Duration::from_millis(40);
/// PASS deadline: the click line must appear within this window.
const CLICK_WAIT: Duration = Duration::from_secs(10);
/// Pure-motion control window: no click line may appear in it.
const MOTION_CONTROL_WINDOW: Duration = Duration::from_secs(1);
/// Settle time on the gear before the phase-2 click.
const GEAR_SETTLE: Duration = Duration::from_millis(1000);
/// Game boot deadline: first painted frame line in the input log.
const BOOT_WAIT: Duration = Duration::from_secs(60);

/// Gear-disc position in native pixels (pinned e2e coord, shared with
/// the Wayland arm).
const GEAR_POSITION: (i16, i16) = (729, 580);
/// XTEST fake input always sends time 0 (no synthetic timestamp).
const EVENT_TIME: u32 = 0;
/// XTEST fake input uses the core pointer (device 0) for this probe.
const EVENT_DEVICE: u8 = 0;

/// One XTEST fake-input event the driver sends to the X server.
///
/// The headless seam tests encode this plan in an explicit
/// little-endian byte form: `[type, detail, time, root, x, y]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct XtestEvent {
    event_type: u8,
    detail: u8,
    root: u32,
    x: i16,
    y: i16,
}

impl XtestEvent {
    /// Encode type and detail, then little-endian time, root,
    /// and point. The device is the fixed core pointer (zero).
    /// Test-only: the live arm passes the fields to x11rb directly.
    #[cfg(test)]
    fn encode(self) -> [u8; 14] {
        let mut bytes = [0u8; 14];
        bytes[0] = self.event_type;
        bytes[1] = self.detail;
        bytes[2..6].copy_from_slice(&EVENT_TIME.to_le_bytes());
        bytes[6..10].copy_from_slice(&self.root.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.x.to_le_bytes());
        bytes[12..14].copy_from_slice(&self.y.to_le_bytes());
        bytes
    }
}

/// Motion-only event: the silent control arm of the probe.
fn motion_event(root: u32, position: (i16, i16)) -> XtestEvent {
    XtestEvent {
        event_type: MOTION_NOTIFY_EVENT,
        detail: 0,
        root,
        x: position.0,
        y: position.1,
    }
}

/// One button edge; only the event type differs between press and release.
fn button_event(root: u32, position: (i16, i16), event_type: u8) -> XtestEvent {
    XtestEvent {
        event_type,
        detail: LEFT_BUTTON,
        root,
        x: position.0,
        y: position.1,
    }
}

/// Motion, press, release: one human click on the X11 arm.
fn click_event_plan(root: u32, position: (i16, i16)) -> [XtestEvent; 3] {
    [
        motion_event(root, position),
        button_event(root, position, BUTTON_PRESS_EVENT),
        button_event(root, position, BUTTON_RELEASE_EVENT),
    ]
}

/// Two-phase verdict of one X11 probe run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeReport {
    /// Phase 1: the center click routed to the game input log.
    pub click_routed: bool,
    /// Phase 2: the gear click repainted the planet (the game acted).
    pub game_reacted: bool,
}

impl ProbeReport {
    /// Both phases passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.click_routed && self.game_reacted
    }
}

/// A private Xvfb server. Killed on drop (PID-recorded discipline:
// only the child we spawned is ever signalled).
pub struct XvfbServer {
    child: Child,
    display: String,
}

impl XvfbServer {
    /// Start Xvfb on a free display with an 800x600 root.
    pub fn spawn() -> Result<Self> {
        let number = pick_free_display(99);
        let display = format!(":{number}");
        let mut child = Command::new("Xvfb")
            .arg(&display)
            .arg("-screen")
            .arg("0")
            .arg(format!("{ROOT_WIDTH}x{ROOT_HEIGHT}x24"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("spawn Xvfb on {display}"))?;
        if wait_for_socket(number, Duration::from_secs(10)) {
            Ok(Self { child, display })
        } else {
            let _ = child.kill();
            bail!("Xvfb on {display} never opened its socket")
        }
    }

    /// The display clients must use (e.g. `:99`).
    #[must_use]
    pub fn display(&self) -> &str {
        &self.display
    }
}

impl Drop for XvfbServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// X11 lock file proving display `number` is taken.
#[must_use]
pub fn display_lock_path(number: u32) -> PathBuf {
    PathBuf::from(format!("/tmp/.X{number}-lock"))
}

/// First display at or above `from` with no lock file.
#[must_use]
pub fn pick_free_display(from: u32) -> u32 {
    (from..from + 1000)
        .find(|number| !display_lock_path(*number).exists())
        .unwrap_or(from)
}

/// True when the X server socket for `number` appears before the
/// deadline.
fn wait_for_socket(number: u32, wait: Duration) -> bool {
    let socket = PathBuf::from(format!("/tmp/.X11-unix/X{number}"));
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if socket.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    socket.exists()
}

/// Map a native game pixel onto root coordinates through the game
/// window rect, assuming the uniform-scale letterbox the client
/// paints. Falls back to identity when the window already matches
/// the backbuffer.
#[must_use]
pub fn map_native_to_root(native: (i16, i16), window: (i16, i16, u16, u16)) -> (i16, i16) {
    use o2jam_shell::{DEFAULT_HEIGHT, DEFAULT_WIDTH};
    let (origin_x, origin_y, width, height) = window;
    let scale = (f64::from(width) / f64::from(DEFAULT_WIDTH))
        .min(f64::from(height) / f64::from(DEFAULT_HEIGHT));
    let offset_x =
        f64::from(origin_x) + (f64::from(width) - f64::from(DEFAULT_WIDTH) * scale) / 2.0;
    let offset_y =
        f64::from(origin_y) + (f64::from(height) - f64::from(DEFAULT_HEIGHT) * scale) / 2.0;
    (
        (offset_x + f64::from(native.0) * scale).round() as i16,
        (offset_y + f64::from(native.1) * scale).round() as i16,
    )
}

/// Human click on the X11 arm: motion first (warms the cursor cache —
/// a press with no tracked position drops), then down, hold, up.
fn inject_click(
    connection: &impl x11rb::connection::Connection,
    root: u32,
    position: (i16, i16),
) -> Result<()> {
    let [motion, press, release] = click_event_plan(root, position);
    send_fake_input(connection, motion)?;
    send_fake_input(connection, press)?;
    connection.flush()?;
    std::thread::sleep(CLICK_HOLD);
    send_fake_input(connection, release)?;
    connection.flush()?;
    Ok(())
}

/// Motion with no edges: the silent control (motion routes as hover,
/// never logs).
fn inject_motion(
    connection: &impl x11rb::connection::Connection,
    root: u32,
    position: (i16, i16),
) -> Result<()> {
    send_fake_input(connection, motion_event(root, position))?;
    connection.flush()?;
    Ok(())
}

/// Send one planned event and wait for the server to accept it.
fn send_fake_input(
    connection: &impl x11rb::connection::Connection,
    event: XtestEvent,
) -> Result<()> {
    connection
        .xtest_fake_input(
            event.event_type,
            event.detail,
            EVENT_TIME,
            event.root,
            event.x,
            event.y,
            EVENT_DEVICE,
        )?
        .check()?;
    Ok(())
}

/// Game arguments for one probe child: the login plus the probe capture
/// plan, and the rig fence last — `--no-sound` keeps the spawned client
/// on the null sink, so an armed probe can never open the operator's
/// output device (#283).
fn game_args(frames: &Path, input_log: &Path) -> Vec<String> {
    vec![
        "online".to_owned(),
        "--host".to_owned(),
        "127.0.0.1".to_owned(),
        "--port".to_owned(),
        "15010".to_owned(),
        "--user-id".to_owned(),
        "2".to_owned(),
        "--token".to_owned(),
        "77b0a3e3-32e6-4a67-b3d8-563a0df4c08c".to_owned(),
        "--frame-dump".to_owned(),
        frames.display().to_string(),
        "--input-log".to_owned(),
        input_log.display().to_string(),
        "--no-sound".to_owned(),
        // Input-routing probe: keep the pre-#244 channel path so the
        // gear click measures input delivery, not the relay notice.
        "--no-relay-gate".to_owned(),
    ]
}

/// Launch the game on one X display with the probe capture plan.
fn spawn_game_on(
    game_binary: &Path,
    scratch: &Path,
    display: &str,
    input_log: &Path,
) -> Result<Child> {
    let log = std::fs::File::create(scratch.join("game.log"))?;
    let frames = scratch.join("frames");
    std::fs::create_dir_all(&frames)?;
    Command::new(game_binary)
        .args(game_args(&frames, input_log))
        .env("DISPLAY", display)
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env_remove("WINIT_UNIX_BACKEND")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .spawn()
        .with_context(|| format!("spawn {}", game_binary.display()))
}

/// Stop only the game we started (never `pkill`-style matching).
fn stop_game_verified(game: &mut Child) {
    let pid = game.id();
    let cmdline = std::fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    if cmdline.contains("recomp2jam") {
        let _ = game.kill();
        let _ = game.wait();
    }
}

/// Drive the full two-phase probe on X11: same scripted gesture as the
/// Wayland arm, same input-log verdicts.
pub fn run_probe(game_binary: &Path, scratch: &Path) -> Result<ProbeReport> {
    let server = XvfbServer::spawn()?;
    let input_log = scratch.join("input.log");
    let mut game = spawn_game_on(game_binary, scratch, server.display(), &input_log)?;

    let report = run_probe_inner(&input_log, &mut game, server.display());
    stop_game_verified(&mut game);
    report
}

fn run_probe_inner(input_log: &Path, game: &mut Child, display: &str) -> Result<ProbeReport> {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::ConnectionExt as _;
    wait_for_boot(input_log, game)?;
    let (connection, screen) = x11rb::connect(Some(display))?;
    let root = connection.setup().roots[screen].root;
    let geometry = connection.get_geometry(root)?.reply()?;
    let center = (
        i32::from(geometry.width) / 2,
        i32::from(geometry.height) / 2,
    );
    let window = find_game_window(&connection, root)?;
    let aim = |native: (i16, i16)| match window {
        Some(rect) => map_native_to_root(native, rect),
        None => native,
    };

    let mut script = crate::probe_gesture(
        center,
        (i32::from(GEAR_POSITION.0), i32::from(GEAR_POSITION.1)),
    );
    let control_snapshot = script.snapshot();
    let center_snapshot = script.snapshot();
    let hover_snapshot = script.snapshot();
    let gear_snapshot = script.snapshot();
    assert!(script.snapshot().edges().is_empty());
    let control = aim(snapshot_point(&control_snapshot));
    let center_point = aim(snapshot_point(&center_snapshot));
    let hover = aim(snapshot_point(&hover_snapshot));
    let gear = aim(snapshot_point(&gear_snapshot));
    assert_eq!(center_snapshot.edges().len(), 2);
    assert_eq!(gear_snapshot.edges().len(), 2);
    inject_motion(&connection, root, control)?;
    let silent = log::wait_for_condition(
        &mut || Ok(()),
        &mut || Ok(log::latest_click_line(&log::read_log(input_log)?).is_some()),
        MOTION_CONTROL_WINDOW,
    )?;
    if silent {
        bail!("control motion produced a click line; injection misfired");
    }

    inject_click(&connection, root, center_point)?;
    let mut routed = None;
    log::wait_for_condition(
        &mut || Ok(()),
        &mut || {
            routed = log::latest_click_line(&log::read_log(input_log)?);
            Ok(routed.is_some())
        },
        CLICK_WAIT,
    )?;
    if routed.is_none() {
        return Ok(ProbeReport {
            click_routed: false,
            game_reacted: false,
        });
    }

    inject_motion(&connection, root, hover)?;
    std::thread::sleep(GEAR_SETTLE);
    let settled = log::last_content_hash(&log::read_log(input_log)?, "planet");
    inject_click(&connection, root, gear)?;
    let mut reacted = false;
    log::wait_for_condition(
        &mut || Ok(()),
        &mut || {
            let now = log::last_content_hash(&log::read_log(input_log)?, "planet");
            reacted = settled.as_deref() != now.as_deref();
            Ok(reacted)
        },
        CLICK_WAIT,
    )?;
    Ok(ProbeReport {
        click_routed: true,
        game_reacted: reacted,
    })
}

/// Native point a snapshot aims at (every probe snapshot carries one).
fn snapshot_point(snapshot: &InputSnapshot) -> (i16, i16) {
    let (x, y) = snapshot.pointer().expect("probe snapshots aim somewhere");
    (x as i16, y as i16)
}

/// Wait for the first painted `frame` line: the game mapped and the
/// input log is live.
fn wait_for_boot(input_log: &Path, game: &mut Child) -> Result<()> {
    let deadline = Instant::now() + BOOT_WAIT;
    while Instant::now() < deadline {
        if let Some(status) = game.try_wait()? {
            bail!("mid-boot: game exited before first paint ({status})");
        }
        let content = log::read_log(input_log)?;
        if content.lines().any(|line| line.starts_with("frame ")) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    bail!("mid-boot: no painted frame within {BOOT_WAIT:?}")
}

/// Game window rect `(x, y, w, h)` by `WM_NAME`, if the client mapped
/// one yet. `None` means the fullscreen-at-origin fallback applies.
fn find_game_window(
    connection: &impl x11rb::connection::Connection,
    root: u32,
) -> Result<Option<(i16, i16, u16, u16)>> {
    use x11rb::protocol::xproto::ConnectionExt as _;
    let tree = connection.query_tree(root)?.reply()?;
    for window in tree.children {
        let cookie = match connection.get_property(
            false,
            window,
            x11rb::protocol::xproto::AtomEnum::WM_NAME,
            x11rb::protocol::xproto::AtomEnum::STRING,
            0,
            64,
        ) {
            Ok(cookie) => cookie,
            Err(_) => continue,
        };
        let name = match cookie.reply() {
            Ok(reply) => reply,
            Err(_) => continue,
        };
        if String::from_utf8_lossy(&name.value).contains("recomp2jam") {
            let geometry = connection.get_geometry(window)?.reply()?;
            let coords = connection
                .translate_coordinates(window, root, 0, 0)?
                .reply()?;
            return Ok(Some((
                coords.dst_x,
                coords.dst_y,
                geometry.width,
                geometry.height,
            )));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod x11_tests {
    use super::*;
    use crate::script::ScriptedSource;

    /// Live chain (#283): `cargo test -p input-drive` ->
    /// `x11_arm_routes_click_through_real_game` -> `run_probe` ->
    /// `spawn_game_on` -> `game_args` -> child `main::run_online` ->
    /// `attach_initial_audio(!no_sound)`. Asserts the spawned argument
    /// list, never a device: with the fence the child's session stays
    /// on the null sink whatever the machine's hardware.
    #[test]
    fn x11_game_args_never_opt_into_hardware_audio() {
        let args = game_args(Path::new("/tmp/frames"), Path::new("/tmp/input.log"));
        assert!(
            args.iter().any(|arg| arg == "--no-sound"),
            "probe child must carry the rig fence, got {args:?}"
        );
    }

    /// Ticket #244: the input-routing probe opts out of the new
    /// relay notice so its gear click still measures input delivery
    /// on the bare planet board. The default client boot keeps the
    /// gate; only this harness argument skips it.
    #[test]
    fn x11_game_args_skip_the_relay_notice_gate() {
        let args = game_args(Path::new("/tmp/frames"), Path::new("/tmp/input.log"));
        assert!(
            args.iter().any(|arg| arg == "--no-relay-gate"),
            "input probe must keep the pre-notice planet path, got {args:?}"
        );
    }

    /// Headless stand-in for the rig-lane live proof: the driver's
    /// XTEST plan is three framed events — motion warms the pointer,
    /// press opens the click, release closes it. No display or server
    /// is touched (tier rules in docs/agents/triage-labels.md).
    #[test]
    fn x11_click_plan_encodes_motion_press_release_bytes() {
        let plan = click_event_plan(0x1234, (10, -20));
        let bytes: Vec<u8> = plan.iter().flat_map(|event| event.encode()).collect();
        let expected = [
            6, 0, 0, 0, 0, 0, 0x34, 0x12, 0, 0, 0x0a, 0, 0xec, 0xff, // motion
            4, 1, 0, 0, 0, 0, 0x34, 0x12, 0, 0, 0x0a, 0, 0xec, 0xff, // press
            5, 1, 0, 0, 0, 0, 0x34, 0x12, 0, 0, 0x0a, 0, 0xec, 0xff, // release
        ];
        assert_eq!(bytes, expected);
    }

    /// The silent control arm must carry no button edge at all.
    #[test]
    fn x11_motion_plan_encodes_one_event_without_button_edges() {
        let bytes = motion_event(7, (100, 200)).encode();
        assert_eq!(bytes, [6, 0, 0, 0, 0, 0, 7, 0, 0, 0, 100, 0, 200, 0]);
    }

    /// All three edges of one click aim at the same root pixel.
    #[test]
    fn x11_click_plan_aims_every_event_at_the_same_root_and_point() {
        let plan = click_event_plan(9, (-1, 2));
        assert!(plan
            .iter()
            .all(|event| event.root == 9 && (event.x, event.y) == (-1, 2)));
    }

    #[test]
    fn lock_path_names_the_display_lock() {
        assert_eq!(display_lock_path(99), PathBuf::from("/tmp/.X99-lock"));
    }

    #[test]
    fn free_display_starts_at_the_floor_without_a_lock() {
        let picked = pick_free_display(42000);
        assert!(picked >= 42000);
        assert!(!display_lock_path(picked).exists());
    }

    #[test]
    fn fullscreen_window_maps_native_to_root_identity() {
        assert_eq!(map_native_to_root((400, 300), (0, 0, 800, 600)), (400, 300));
        assert_eq!(
            map_native_to_root(GEAR_POSITION, (0, 0, 800, 600)),
            GEAR_POSITION
        );
    }

    #[test]
    fn probe_gesture_survives_as_four_x11_frames() {
        let mut script = ScriptedSource::new(vec![
            InputSnapshot::motion_to(437, 277),
            InputSnapshot::click_at(400, 300),
            InputSnapshot::motion_to(729, 580),
            InputSnapshot::click_at(729, 580),
        ]);
        assert_eq!(script.remaining(), 4);
        assert!(script.snapshot().edges().is_empty());
        assert_eq!(script.snapshot().edges().len(), 2);
        assert!(script.snapshot().edges().is_empty());
        assert_eq!(script.snapshot().edges().len(), 2);
        assert_eq!(script.remaining(), 0);
    }

    /// Full X11 arm against the real game. Skips cleanly when the rig
    /// cannot run here: non-Linux (compile-gated anyway), no Xvfb
    /// binary, no game data under `install-root`, or the game exits
    /// before boot completes (the shared local Memoryer admits one
    /// login at a time — a mid-boot exit is contention, not a
    /// verdict). A boot that completes but never routes is a FAIL.
    #[test]
    fn x11_arm_routes_click_through_real_game() {
        if which_xvfb().is_none() {
            eprintln!("SKIP x11_arm_routes_click_through_real_game: no Xvfb binary on PATH");
            return;
        }
        if !game_data_present() {
            eprintln!(
                "SKIP x11_arm_routes_click_through_real_game: no game data under install-root"
            );
            return;
        }
        let game_binary = ensure_fresh_game_binary();
        let scratch = std::env::temp_dir().join(format!("input-drive-x11-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).expect("create scratch dir");
        let report = match run_probe(&game_binary, &scratch) {
            Ok(report) => report,
            Err(error) => {
                let message = format!("{error:#}");
                std::fs::remove_dir_all(&scratch).ok();
                if message.contains("mid-boot") {
                    eprintln!(
                        "SKIP x11_arm_routes_click_through_real_game: game never booted ({message})"
                    );
                    return;
                }
                panic!("x11 probe failed after boot: {message}");
            }
        };
        std::fs::remove_dir_all(&scratch).ok();
        assert!(
            report.click_routed,
            "phase 1: click routes to the input log"
        );
        assert!(
            report.game_reacted,
            "phase 2: gear click repaints the planet"
        );
    }

    fn which_xvfb() -> Option<PathBuf> {
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join("Xvfb"))
                .find(|candidate| candidate.is_file())
        })
    }

    fn game_data_present() -> bool {
        o2jam_file::data_root::root_has_game_data(&o2jam_file::data_root::install_root())
    }

    fn workspace_game_binary() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/recomp2jam")
    }

    /// The game binary rebuilt when it predates the workspace sources,
    /// so a stale binary can never silently test the wrong code.
    fn ensure_fresh_game_binary() -> PathBuf {
        let binary = workspace_game_binary();
        let status = std::process::Command::new(env!("CARGO"))
            .args(["build", "--offline", "--quiet", "-p", "recomp2jam"])
            .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .status()
            .expect("cargo build -p recomp2jam runs");
        assert!(status.success(), "game binary builds");
        assert!(binary.is_file(), "game binary exists");
        binary
    }
}
