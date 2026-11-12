//! Headless pointer-injection probe — the input-routing arm of the #118
//! red-green (Refs: #126).
//!
//! The #118 no-input leg is OPEN in every live environment (owner desktop
//! KWin, uinput rigs): real clicks never reach the game on any backend —
//! suspected environment routing, not client code. This probe
//! discriminates exactly that. The fixture *is* the compositor, so it
//! sends the protocol-level wire sequence of a real click
//! (`wl_pointer.enter` → `motion` → `button` down/up → `leave`) directly
//! into the client. No KWin, no uinput, no XTEST, no seat — the
//! environment is bypassed by construction:
//!
//! - A `click x y` line in the game's input log proves the full client
//!   chain (winit Wayland backend → `CursorMoved`/`MouseInput` →
//!   `WindowHandler` routing → game dispatch). Client routing broken ⇒
//!   FAIL, the guard bites.
//! - Green here plus red live isolates the environment path as the
//!   fault (see `docs/input-testing-matrix.md`).
//!
//! The probe runs in the fixture's normal callback-flow regime: input
//! routing does not involve the frame-callback gate.
//!
//! Observed game log contract (crates/o2jam-shell/src/window.rs): the log
//! records `event <wall_ms> click <x> <y>` on the press edge and
//! `event <wall_ms> click-up <x> <y>` on release. Motion routes to the
//! game as hover but is deliberately NOT logged (it would flood the
//! trace), so the PASS assertion is the click line only — a paint or
//! content reaction to motion is not required or expected.
//!
//! Usage: `input_smoke [GAME-BINARY]`.
//! Exit codes: 0 = verdict PASS, 1 = verdict FAIL, 2 = harness error.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use wl_smoke_compositor::driver;
use wl_smoke_compositor::{Compositor, FrameCallbackPolicy, PointerEvent};

const MAP_WAIT: Duration = Duration::from_secs(20);
/// Hold time between the injected button down and up edges.
const CLICK_HOLD: Duration = Duration::from_millis(40);
/// PASS deadline: the click line must appear within this window.
const CLICK_WAIT: Duration = Duration::from_secs(10);
/// The pure-motion control window: no click line may appear in it.
const MOTION_CONTROL_WINDOW: Duration = Duration::from_secs(1);

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
    let mut args = std::env::args().skip(1);
    let game_binary = driver::resolve_game_binary(args.next().as_deref())?;

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

    // Control: one pure motion must NOT produce a click line (the game's
    // log contract: motion routes as hover, stays silent). Any click
    // line here means the injection misfired before the click.
    inject_motion_control(&mut compositor, center_x, center_y, &input_log)?;

    // The click: enter + motion + down, hold, up + leave — the exact
    // sequence a human click produces on the wire.
    for event in PointerEvent::click_at(center_x, center_y) {
        if matches!(event, PointerEvent::Button { pressed: false, .. }) {
            std::thread::sleep(CLICK_HOLD);
        }
        compositor.inject_pointer(event);
        // Flush each step so the client sees enter before the button,
        // and the press before the release.
        compositor.turn()?;
    }

    let verdict = match wait_for_click_line(&input_log, CLICK_WAIT)? {
        Some((x, y)) => {
            println!(
                "input-smoke: verdict=PASS routed click at native ({x}, {y}), injected center ({center_x:.0}, {center_y:.0})"
            );
            0
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

/// Inject one motion away from the center and assert the game stays
/// silent about clicks for the control window: motion alone routes as
/// hover and never logs (documented contract), and the PASS criterion
/// must not depend on any motion-driven paint reaction.
fn inject_motion_control(
    compositor: &mut Compositor,
    center_x: f64,
    center_y: f64,
    input_log: &Path,
) -> Result<()> {
    // Away from the center so a mis-tracked press position would show
    // up in the coords of any wrongly logged click. Enter first: a bare
    // motion before enter is off-protocol and clients may ignore it.
    for event in [
        PointerEvent::Enter {
            x: center_x + 37.0,
            y: center_y - 23.0,
        },
        PointerEvent::Motion {
            x: center_x + 37.0,
            y: center_y - 23.0,
        },
    ] {
        compositor.inject_pointer(event);
        compositor.turn()?;
    }
    let control_end = Instant::now() + MOTION_CONTROL_WINDOW;
    while Instant::now() < control_end {
        compositor.turn()?;
        if let Some((x, y)) = latest_click_line(input_log)? {
            bail!(
                "control motion produced a click line ({x}, {y}); injection misfired before the click"
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    compositor.inject_pointer(PointerEvent::Leave);
    compositor.turn()?;
    println!("input-smoke: control motion silent (no click line), as the log contract requires");
    Ok(())
}

/// Poll the input log for a routed click line until the deadline.
/// Returns the click's native-frame coordinates.
fn wait_for_click_line(input_log: &Path, wait: Duration) -> Result<Option<(i32, i32)>> {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if let Some(click) = latest_click_line(input_log)? {
            return Ok(Some(click));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(None)
}

/// The last `click x y` line in the input log (press edge; `click-up`
/// lines are releases and do not count), parsed to native coordinates.
fn latest_click_line(input_log: &Path) -> Result<Option<(i32, i32)>> {
    let Ok(content) = fs::read_to_string(input_log) else {
        return Ok(None);
    };
    for line in content.lines().rev() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // The press line's shape: `event <wall_ms> click <x> <y>`.
        if fields.len() == 5 && fields[0] == "event" && fields[2] == "click" {
            let x: i32 = fields[3].parse().context("parse click x")?;
            let y: i32 = fields[4].parse().context("parse click y")?;
            return Ok(Some((x, y)));
        }
    }
    Ok(None)
}

fn log_tail(input_log: &Path) -> String {
    let Ok(content) = fs::read_to_string(input_log) else {
        return format!("(no log at {})", input_log.display());
    };
    let tail: Vec<&str> = content.lines().collect();
    tail[tail.len().saturating_sub(15)..].join("\n")
}
