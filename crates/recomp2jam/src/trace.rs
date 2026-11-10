//! `recomp2jam-input-v1` trace model and validation.
//!
//! A trace is an ordered event list replayed at the OS input
//! boundary (real X11 key/mouse injection into the windowed binary —
//! see `fixtures/input-trace/replay_window.py`). The schema follows
//! the `recomp2jam-input-replay` skill (sequence, timestamp, control
//! and action per event; press/release pairs; units named in
//! `time_base`).
//!
//! Extensions the window harness needs, documented in
//! the fixture README:
//!
//! - `control: "click"` (`action: "press"`, `x`/`y` on the native
//!   800x600 canvas) for mouse evidence;
//! - `control: "text"` (`action: "type"`, `text`) for IME-free chat
//!   typing (injected as XTEST keysyms);
//! - `control: "await-screen"` (`action: "reach"`, `screen`) as a
//!   sync point: the harness holds the replay until the binary's
//!   capture log shows that screen, so server round trips never turn
//!   into fixed sleeps.
//!
//! Timestamps are the event's intended menu-clock time (relative to
//! the window's first paint); the harness records the applied time
//! separately in the binary's capture log (skill rule 4).

use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

/// Schema marker every trace carries.
pub const SCHEMA_VERSION: &str = "recomp2jam-input-v1";

/// One deterministic input trace.
#[derive(Debug, Clone, Deserialize)]
pub struct InputTrace {
    /// Schema marker ([`SCHEMA_VERSION`]).
    pub schema_version: String,
    /// Scenario name (the walk this trace drives).
    pub scenario: String,
    /// Clock the timestamps ride (menus: monotonic; gameplay: song
    /// position) with its unit recorded.
    pub time_base: TimeBase,
    /// Ordered events (validated: strictly increasing sequence and
    /// non-decreasing time, allowed actions, complete pairs).
    pub events: Vec<TraceEvent>,
}

/// Clock declaration for one trace.
#[derive(Debug, Clone, Deserialize)]
pub struct TimeBase {
    /// Clock name (`menu-monotonic`, `song-position`, …).
    pub name: String,
    /// Timestamp unit (`ms` here — recorded explicitly per skill).
    pub unit: String,
}

/// One trace event.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TraceEvent {
    /// Strictly increasing sequence number (replay order authority).
    pub sequence: u64,
    /// Intended timestamp on the trace's clock.
    pub time_ms: u64,
    /// Control: `lane-N` (0..=6), `menu-confirm`, `menu-back`,
    /// `menu-up`, `menu-down`, `click`, `text`, `await-screen`.
    pub control: String,
    /// Action: `press`/`release` (keys), `press` (click), `type`
    /// (text), `reach` (await-screen).
    pub action: String,
    /// Native-canvas x for `click`.
    #[serde(default)]
    pub x: Option<i32>,
    /// Native-canvas y for `click`.
    #[serde(default)]
    pub y: Option<i32>,
    /// Typed content for `text`.
    #[serde(default)]
    pub text: Option<String>,
    /// Screen name for `await-screen`.
    #[serde(default)]
    pub screen: Option<String>,
}

/// Load and validate one trace file (JSON).
///
/// # Errors
///
/// Unreadable or unparsable files, plus every validation rule
/// violation collected into one error.
pub fn load_validated(path: &Path) -> anyhow::Result<InputTrace> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading trace {}", path.display()))?;
    let trace: InputTrace =
        serde_json::from_str(&text).with_context(|| format!("parsing trace {}", path.display()))?;
    validate(&trace).map_err(|violations| {
        anyhow::anyhow!(
            "trace {} failed validation: {}",
            path.display(),
            violations.join("; ")
        )
    })?;
    Ok(trace)
}

/// Validate every replay-safety rule; all violations collected (a
/// malformed trace reports everything wrong at once, never one bite
/// at a time).
///
/// # Errors
///
/// Never: validation failures land in the returned list.
pub fn validate(trace: &InputTrace) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();
    if trace.schema_version != SCHEMA_VERSION {
        violations.push(format!(
            "schema_version {:?} != {SCHEMA_VERSION:?}",
            trace.schema_version
        ));
    }
    if trace.events.is_empty() {
        violations.push("trace has no events".to_owned());
    }
    let mut held: Vec<&str> = Vec::new();
    let mut last_sequence = None;
    let mut last_time = None;
    for event in &trace.events {
        if let Some(previous) = last_sequence {
            if event.sequence <= previous {
                violations.push(format!(
                    "event {}: sequence {} not increasing",
                    event.sequence, event.sequence
                ));
            }
        }
        last_sequence = Some(event.sequence);
        if let Some(previous) = last_time {
            if event.time_ms < previous {
                violations.push(format!(
                    "event {}: time {} goes backwards",
                    event.sequence, event.time_ms
                ));
            }
        }
        last_time = Some(event.time_ms);
        validate_event(event, &mut held, &mut violations);
    }
    for open in held {
        violations.push(format!("control {open:?} pressed without a release"));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Validate one event: known control/action pair, payload bounds,
/// press/release pairing tracked in `held`.
fn validate_event<'a>(
    event: &'a TraceEvent,
    held: &mut Vec<&'a str>,
    violations: &mut Vec<String>,
) {
    let at = format!(
        "event {} ({}/{})",
        event.sequence, event.control, event.action
    );
    match event.control.as_str() {
        control if control.starts_with("lane-") => {
            let lane: usize = control["lane-".len()..].parse().unwrap_or(usize::MAX);
            if lane > 6 {
                violations.push(format!("{at}: lane index out of 0..=6"));
            }
            validate_press_release(event, held, violations, &at);
        }
        "menu-confirm" | "menu-back" | "menu-up" | "menu-down" => {
            validate_press_release(event, held, violations, &at);
        }
        "click" => {
            if event.action != "press" {
                violations.push(format!("{at}: click only presses"));
            }
            let Some((x, y)) = event.x.zip(event.y) else {
                violations.push(format!("{at}: click needs x and y"));
                return;
            };
            if !(0..800).contains(&x) || !(0..600).contains(&y) {
                violations.push(format!("{at}: click ({x},{y}) outside the 800x600 canvas"));
            }
        }
        "text" => {
            if event.action != "type" {
                violations.push(format!("{at}: text only types"));
            }
            let Some(text) = event.text.as_ref().filter(|text| !text.is_empty()) else {
                violations.push(format!("{at}: text needs non-empty text"));
                return;
            };
            if text.chars().any(char::is_control) {
                violations.push(format!("{at}: text carries control characters"));
            }
            if !text.is_ascii() {
                violations.push(format!(
                    "{at}: text must be ASCII (the XTEST injector maps printable keysyms only)"
                ));
            }
        }
        "await-screen" => {
            if event.action != "reach" {
                violations.push(format!("{at}: await-screen only reaches"));
            }
            let Some(screen) = event.screen.as_deref() else {
                violations.push(format!("{at}: await-screen needs a screen"));
                return;
            };
            if !KNOWN_SCREENS.contains(&screen) {
                violations.push(format!("{at}: unknown screen {screen:?}"));
            }
        }
        other => violations.push(format!("{at}: unknown control {other:?}")),
    }
}

/// Screens the harness can sync on (the `ScreenGame::screen_name`
/// vocabulary).
const KNOWN_SCREENS: [&str; 12] = [
    "avi",
    "planet",
    "channel",
    "room",
    "waiting",
    "loading",
    "playing",
    "result",
    "shop",
    "musicshop",
    "myroom",
    "lanking",
];

/// Press/release pairing: a press holds the control, a release frees
/// it; anything else is invalid.
fn validate_press_release<'a>(
    event: &'a TraceEvent,
    held: &mut Vec<&'a str>,
    violations: &mut Vec<String>,
    at: &str,
) {
    match event.action.as_str() {
        "press" => {
            if held.contains(&event.control.as_str()) {
                violations.push(format!("{at}: double press without a release"));
            }
            held.push(event.control.as_str());
        }
        "release" => {
            if held
                .iter()
                .position(|held| *held == event.control)
                .is_none()
            {
                violations.push(format!("{at}: release without a press"));
            } else {
                held.remove(
                    held.iter()
                        .position(|held| *held == event.control)
                        .unwrap_or_default(),
                );
            }
        }
        other => violations.push(format!("{at}: unknown action {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(sequence: u64, time_ms: u64, control: &str, action: &str) -> TraceEvent {
        TraceEvent {
            sequence,
            time_ms,
            control: control.to_owned(),
            action: action.to_owned(),
            x: None,
            y: None,
            text: None,
            screen: None,
        }
    }

    fn trace(events: Vec<TraceEvent>) -> InputTrace {
        InputTrace {
            schema_version: SCHEMA_VERSION.to_owned(),
            scenario: "test".to_owned(),
            time_base: TimeBase {
                name: "menu-monotonic".to_owned(),
                unit: "ms".to_owned(),
            },
            events,
        }
    }

    #[test]
    fn the_committed_u1_fixture_validates() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("fixtures/input-trace/u1-windowed-online.json");
        if !path.exists() {
            println!("skip: missing {}", path.display());
            return;
        }
        let loaded = load_validated(&path).unwrap();
        assert!(loaded.events.len() >= 10);
    }

    #[test]
    fn press_release_pairs_must_close() {
        let mut open = event(0, 0, "lane-1", "press");
        let bad = trace(vec![open.clone()]);
        assert!(validate(&bad).is_err());

        open.sequence = 0;
        let mut close = event(1, 40, "lane-1", "release");
        let good = trace(vec![open, close.clone()]);
        assert!(validate(&good).is_ok());

        close.sequence = 2;
        let double = trace(vec![
            event(0, 0, "menu-confirm", "press"),
            event(1, 20, "menu-confirm", "press"),
            close,
        ]);
        assert!(validate(&double).is_err());
    }

    #[test]
    fn sequence_and_time_must_move_forward() {
        let bad_time = trace(vec![
            event(0, 100, "menu-confirm", "press"),
            event(1, 50, "menu-confirm", "release"),
        ]);
        assert!(validate(&bad_time).is_err());
        let bad_seq = trace(vec![
            event(1, 0, "menu-confirm", "press"),
            event(1, 10, "menu-confirm", "release"),
        ]);
        assert!(validate(&bad_seq).is_err());
    }

    #[test]
    fn clicks_stay_on_the_native_canvas() {
        let mut click = event(0, 0, "click", "press");
        click.x = Some(136);
        click.y = Some(88);
        assert!(validate(&trace(vec![click.clone()])).is_ok());
        click.x = Some(900);
        assert!(validate(&trace(vec![click])).is_err());
    }

    #[test]
    fn unknown_controls_and_screens_reject() {
        let mut text = event(0, 0, "text", "type");
        text.text = Some("hi".to_owned());
        assert!(validate(&trace(vec![text])).is_ok());
        let mut bogus_screen = event(0, 0, "await-screen", "reach");
        bogus_screen.screen = Some("nowhere".to_owned());
        assert!(validate(&trace(vec![bogus_screen])).is_err());
        assert!(validate(&trace(vec![event(0, 0, "telepathy", "press")])).is_err());
    }
}
