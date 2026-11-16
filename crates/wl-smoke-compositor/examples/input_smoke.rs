//! Headless pointer-injection probe — the input-routing arm of the #118
//! red-green (Refs: #126).
//!
//! The gesture now comes from the `input-drive` seam (ticket #127): the
//! harness drains one [`ScriptedSource`] and translates each frozen
//! snapshot into backend injection — fixture wire events on Wayland,
//! XTEST on X11. The game freezes the edges again in
//! `WindowHandler::deliver_snapshot`, sweeps them via
//! `GameLoop::ingest_snapshot`, then paints. The verdict still reads
//! the game's input log in both arms.
//!
//! Phase 1 (click routes to the input log) must stay green.
//! Phase 2 (gear click repaints the planet) must stay green.
//! Evidence of the contract: `crates/o2jam-shell/src/window.rs`
//! `deliver_snapshot` log lines (`click`, `click-up`).
//!
//! Usage: `input_smoke [--backend wayland|x11|auto] [GAME-BINARY]`.
//! Exit codes: 0 = verdict PASS, 1 = verdict FAIL, 2 = harness error.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use input_drive::{InputBackend, InputEdge, InputSnapshot, SnapshotSource};
use wl_smoke_compositor::driver;
use wl_smoke_compositor::{Compositor, FrameCallbackPolicy, PointerEvent};

const MAP_WAIT: Duration = Duration::from_secs(20);
/// Hold time between the injected button down and up edges.
const CLICK_HOLD: Duration = Duration::from_millis(40);
/// PASS deadline: the click line must appear within this window.
const CLICK_WAIT: Duration = Duration::from_secs(10);
/// The pure-motion control window: no click line may appear in it.
const MOTION_CONTROL_WINDOW: Duration = Duration::from_secs(1);
/// Linux input event code for the left mouse button.
const LEFT_BUTTON: u32 = 0x110;

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("input-smoke: harness error: {error:#}");
            std::process::exit(2);
        }
    };
    std::process::exit(code);
}

fn run() -> Result<i32> {
    let (backend_flag, game_arg) = parse_args(std::env::args().skip(1));
    let backend = match backend_flag.as_deref() {
        None | Some("auto") => select_auto(),
        Some(flag) => InputBackend::from_flag(flag)
            .with_context(|| format!("unknown --backend '{flag}' (want wayland|x11|auto)"))?,
    };
    match backend {
        InputBackend::WaylandFixture => run_wayland(game_arg.as_deref()),
        #[cfg(target_os = "linux")]
        InputBackend::X11Xvfb => run_x11(game_arg.as_deref()),
        #[cfg(not(target_os = "linux"))]
        InputBackend::X11Xvfb => bail!("the x11 arm needs Linux (self-spawned Xvfb)"),
    }
}

/// Explicit `--backend` wins; without one, `DISPLAY` selects X11,
/// else the fixture arm (it needs no display at all). The fixture arm
/// is never auto-selected from an ambient `WAYLAND_DISPLAY` (issue
/// #267); pin it with `--backend wayland` instead.
fn select_auto() -> InputBackend {
    input_drive::auto_backend().unwrap_or(InputBackend::WaylandFixture)
}

fn parse_args(args: impl Iterator<Item = String>) -> (Option<String>, Option<String>) {
    let mut backend = None;
    let mut game = None;
    let mut pending_backend = false;
    for arg in args {
        if pending_backend {
            backend = Some(arg);
            pending_backend = false;
        } else if arg == "--backend" {
            pending_backend = true;
        } else if let Some(flag) = arg.strip_prefix("--backend=") {
            backend = Some(flag.to_owned());
        } else if game.is_none() {
            game = Some(arg);
        }
    }
    (backend, game)
}

/// One frozen snapshot as fixture wire events: pointer motion first,
/// then one button event per pointer edge. Enter/leave stay with the
/// caller (one enter per gesture, one leave at the end). Keyboard
/// edges have no fixture path (OPEN-1) and are skipped.
fn wire_events_for(snapshot: &InputSnapshot) -> Vec<PointerEvent> {
    let mut events = Vec::new();
    if let Some((x, y)) = snapshot.pointer() {
        events.push(PointerEvent::Motion {
            x: f64::from(x),
            y: f64::from(y),
        });
    }
    for edge in snapshot.edges() {
        match edge {
            InputEdge::PointerDown { .. } => events.push(PointerEvent::Button {
                button: LEFT_BUTTON,
                pressed: true,
            }),
            InputEdge::PointerUp { .. } => events.push(PointerEvent::Button {
                button: LEFT_BUTTON,
                pressed: false,
            }),
            _ => {}
        }
    }
    events
}

/// Inject one snapshot's wire events, holding before each release so
/// the client sees the press before the release.
fn inject_snapshot(compositor: &mut Compositor, snapshot: &InputSnapshot) -> Result<()> {
    for event in wire_events_for(snapshot) {
        if matches!(event, PointerEvent::Button { pressed: false, .. }) {
            std::thread::sleep(CLICK_HOLD);
        }
        compositor.inject_pointer(event);
        // Flush each step so the client sees motion before the button,
        // and the press before the release.
        compositor.turn()?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_x11(game_arg: Option<&str>) -> Result<i32> {
    let game_binary = driver::resolve_game_binary(game_arg)?;
    let scratch = tempfile::tempdir().context("create scratch dir")?;
    let report = input_drive::x11::run_probe(&game_binary, scratch.path())?;
    if !report.click_routed {
        eprintln!("input-smoke: verdict=FAIL (x11) click never routed to the input log");
        return Ok(1);
    }
    if !report.game_reacted {
        eprintln!("input-smoke: verdict=FAIL (x11) click routed but the planet never repainted");
        return Ok(1);
    }
    println!("input-smoke: verdict=PASS (x11) routed and the planet repainted");
    Ok(0)
}

fn run_wayland(game_arg: Option<&str>) -> Result<i32> {
    let game_binary = driver::resolve_game_binary(game_arg)?;

    let scratch = tempfile::tempdir().context("create scratch dir")?;
    let socket_name = format!("wl-smoke-input-{}", std::process::id());
    // Normal callback-flow regime: input routing does not involve the
    // frame-callback gate, but a wedged loop would starve everything, so
    // callbacks flow as in the healthy arm of the CPU probe.
    let mut compositor = Compositor::bind(scratch.path(), &socket_name, FrameCallbackPolicy::Flow)?;
    let socket_file_name = compositor
        .socket_path()
        .file_name()
        .context("socket path has no file name")?
        .to_owned();

    let input_log = scratch.path().join("input.log");
    let extra_args = vec![
        "--frame-dump".to_owned(),
        scratch.path().join("frames").display().to_string(),
        "--input-log".to_owned(),
        input_log.display().to_string(),
    ];
    let mut game =
        driver::spawn_game(&game_binary, scratch.path(), &socket_file_name, &extra_args)?;
    println!(
        "input-smoke: game pid {} on socket {} (game log: {}, input log: {})",
        game.id(),
        socket_name,
        scratch.path().join("game.log").display(),
        input_log.display()
    );

    wait_for_paint_and_pointer(&mut compositor, &mut game, scratch.path())?;
    let Some((center_x, center_y)) = compositor.toplevel_center() else {
        driver::stop_game_verified(&mut game, "input-smoke");
        bail!("toplevel center unknown after first paint");
    };
    println!(
        "input-smoke: first paint + pointer bound; toplevel center ({center_x:.0}, {center_y:.0})"
    );

    // The case both Linux arms drive: silent control motion, center
    // click, gear hover, gear click.
    let mut script = input_drive::probe_gesture(
        (center_x.round() as i32, center_y.round() as i32),
        (729, 580),
    );
    let control = script.snapshot();
    let center_click = script.snapshot();
    let gear_hover = script.snapshot();
    let gear_click = script.snapshot();
    assert!(
        control.edges().is_empty(),
        "the scripted control must carry no edges"
    );
    assert_eq!(
        center_click.edges().len(),
        2,
        "the scripted click must freeze down+up in one frame"
    );

    // Control: one pure motion must NOT produce a click line (the game's
    // log contract: motion routes as hover, stays silent). Any click
    // line here means the injection misfired before the click.
    compositor.inject_pointer(PointerEvent::Enter {
        x: control.pointer().map_or(center_x, |(x, _)| f64::from(x)),
        y: control.pointer().map_or(center_y, |(_, y)| f64::from(y)),
    });
    compositor.turn()?;
    inject_snapshot(&mut compositor, &control)?;
    let control_end = Instant::now() + MOTION_CONTROL_WINDOW;
    while Instant::now() < control_end {
        compositor.turn()?;
        if let Some((x, y)) = latest_click_line(&input_log)? {
            bail!(
                "control motion produced a click line ({x}, {y}); injection misfired before the click"
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    compositor.inject_pointer(PointerEvent::Leave);
    compositor.turn()?;
    println!("input-smoke: control motion silent (no click line), as the log contract requires");

    // The click: enter + motion + down, hold, up + leave — the exact
    // sequence a human click produces on the wire.
    compositor.inject_pointer(PointerEvent::Enter {
        x: center_x,
        y: center_y,
    });
    compositor.turn()?;
    inject_snapshot(&mut compositor, &center_click)?;
    compositor.inject_pointer(PointerEvent::Leave);
    compositor.turn()?;

    let routed = wait_for_click_line(&mut compositor, &input_log, CLICK_WAIT)?;
    let verdict = match routed {
        Some((x, y)) => {
            println!(
                "input-smoke: phase1=PASS routed click at native ({x}, {y}), injected center ({center_x:.0}, {center_y:.0})"
            );
            // Phase 2 — the GAME must act. Click the gear disc (pinned
            // e2e coord); the options dialog must paint, which changes
            // the planet frame's content hash. (A screen-name line is
            // not usable: the modal keeps screen=planet, and a banded
            // emblem would need a lit-state seed this harness lacks.)
            // This is the arm that catches the #118 forwarding class:
            // routing logs the click BEFORE the (then no-op) session
            // call, so phase 1 alone stayed green while live input died.
            // Settle hover first: sweep the pointer onto the gear so the
            // hover repaint happens BEFORE the settled hash; then the
            // click's required change can only be the dialog.
            compositor.inject_pointer(PointerEvent::Enter { x: 729.0, y: 580.0 });
            compositor.turn()?;
            inject_snapshot(&mut compositor, &gear_hover)?;
            std::thread::sleep(Duration::from_millis(1000));
            let settled = last_content_hash(&input_log, "planet");
            inject_snapshot(&mut compositor, &gear_click)?;
            compositor.inject_pointer(PointerEvent::Leave);
            compositor.turn()?;
            let reacted = wait_for_content_change(
                &mut compositor,
                &input_log,
                "planet",
                settled,
                CLICK_WAIT,
            )?;
            match reacted {
                true => {
                    println!(
                        "input-smoke: phase2=PASS gear click repainted the planet (dialog acted)"
                    );
                    0
                }
                false => {
                    eprintln!(
                        "input-smoke: verdict=FAIL click routed (phase1) but the game never acted — planet content hash never changed; input log tail:\n{}",
                        log_tail(&input_log)
                    );
                    1
                }
            }
        }
        None => {
            eprintln!(
                "input-smoke: verdict=FAIL no click line within {CLICK_WAIT:?} of injection; input log tail:\n{}",
                log_tail(&input_log)
            );
            1
        }
    };
    driver::stop_game_verified(&mut game, "input-smoke");
    Ok(verdict)
}

/// First paint plus a bound pointer and a created toplevel: the
/// preconditions for aiming an injection at the surface.
fn wait_for_paint_and_pointer(
    compositor: &mut Compositor,
    game: &mut std::process::Child,
    scratch: &Path,
) -> Result<()> {
    let deadline = Instant::now() + MAP_WAIT;
    while Instant::now() < deadline {
        compositor.turn()?;
        if compositor.stats().first_paint_at.is_some() && compositor.pointer_ready() {
            return Ok(());
        }
        if let Some(status) = game.try_wait()? {
            return Err(driver::game_exit_error(status, scratch));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    bail!(
        "no first paint + pointer within {MAP_WAIT:?}; game log tail:\n{}",
        driver::log_tail(scratch)
    )
}

/// Poll the input log for a routed click line until the deadline.
/// Returns the click's native-frame coordinates. The compositor keeps
/// turning while polling: under repaint-on-demand (#138) the reaction
/// paint rides a frame callback, and a stalled compositor would wedge
/// the client mid-gesture (the flake that bit the on-demand input
/// probe — the paint, never the routing, stopped).
fn wait_for_click_line(
    compositor: &mut Compositor,
    input_log: &Path,
    wait: Duration,
) -> Result<Option<(i32, i32)>> {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        compositor.turn()?;
        if let Some(click) = latest_click_line(input_log)? {
            return Ok(Some(click));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(None)
}

/// The content hash of the newest frame line for the given screen
/// (`frame <n> wall_ms=<ms> screen=<name> state=<h> content=<h>`).
fn last_content_hash(input_log: &Path, screen: &str) -> Option<String> {
    let Ok(content) = fs::read_to_string(input_log) else {
        return None;
    };
    input_drive::log::last_content_hash(&content, screen)
}

/// Poll until a frame line for the given screen carries a content hash
/// different from the settled one — the game visibly reacted.
fn wait_for_content_change(
    compositor: &mut Compositor,
    input_log: &Path,
    screen: &str,
    settled: Option<String>,
    wait: Duration,
) -> Result<bool> {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        // Keep the frame callbacks flowing: the reaction paint the
        // probe waits for needs one (see `wait_for_click_line`).
        compositor.turn()?;
        if let Some(now) = last_content_hash(input_log, screen) {
            if settled.as_deref() != Some(now.as_str()) {
                return Ok(true);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(false)
}

/// The last `click x y` line in the input log (press edge; `click-up`
/// lines are releases and do not count), parsed to native coordinates.
fn latest_click_line(input_log: &Path) -> Result<Option<(i32, i32)>> {
    let Ok(content) = fs::read_to_string(input_log) else {
        return Ok(None);
    };
    Ok(input_drive::log::latest_click_line(&content))
}

fn log_tail(input_log: &Path) -> String {
    let Ok(content) = fs::read_to_string(input_log) else {
        return format!("(no log at {})", input_log.display());
    };
    input_drive::log::log_tail(&content, 15)
}
