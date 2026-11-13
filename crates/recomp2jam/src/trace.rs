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
//! separately in the binary's capture log (skill rule 4). The
//! legacy `time_ms` pacing still drives the windowed XTEST rig;
//! `time_us` (ticket #208) is the microsecond authority and must
//! agree with it (`time_ms == time_us / 1000`, whole-millisecond
//! floor). Runs order on the microsecond clock.
//!
//! Pointer gestures pair explicitly: a `click` `press` opens the
//! gesture at `(x, y)` and a `release` at the same point closes it
//! (a release with no matching press, or at another point,
//! rejects). A lone press stays a tap shorthand for the windowed
//! driver (the committed U1 trace clicks this way). Optional
//! `control_id` (the exe staged control, e.g.
//! `IDC_BUTTON_SSHOP_BUYITEM`) and `label` (the human step name)
//! name the gesture without changing replay.

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
    /// Intended timestamp on the trace's clock (legacy millisecond
    /// pacing: the windowed XTEST driver still reads this field).
    pub time_ms: u64,
    /// Microsecond timestamp authority (ticket #208): when present
    /// it orders the run on the microsecond clock and must agree
    /// with the legacy pacing (`time_ms == time_us / 1000`, the
    /// driver whole-millisecond floor). New traces always carry it.
    #[serde(default)]
    pub time_us: Option<u64>,
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
    /// Exe staged control id for `click` (e.g.
    /// `IDC_BUTTON_SSHOP_BUYITEM`): names the dispatched control
    /// beside the portable `control` name. Informational — dispatch
    /// still resolves the staged code tables by coordinate.
    #[serde(default)]
    pub control_id: Option<String>,
    /// Human step name for `click` (e.g. `row-1-toggle`): keeps the
    /// trace readable without changing replay.
    #[serde(default)]
    pub label: Option<String>,
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
    let mut open_clicks: Vec<(i32, i32)> = Vec::new();
    let mut last_sequence = None;
    let mut last_time = None;
    let mut last_time_us = None;
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
        let time_us = effective_time_us(event);
        if let Some(previous) = last_time_us {
            if time_us < previous {
                violations.push(format!(
                    "event {}: time_us {time_us} goes backwards",
                    event.sequence
                ));
            }
        }
        last_time_us = Some(time_us);
        if let Some(stamp) = event.time_us {
            if stamp / 1000 != event.time_ms {
                violations.push(format!(
                    "event {}: time_us {stamp} disagrees with time_ms {}",
                    event.sequence, event.time_ms
                ));
            }
        }
        validate_event(event, &mut held, &mut open_clicks, &mut violations);
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

/// Effective microsecond stamp: the `time_us` authority when the
/// trace carries it, else the legacy millisecond pacing in
/// microseconds (saturating — a hostile `u64::MAX` pacing clamps
/// instead of wrapping the order check).
fn effective_time_us(event: &TraceEvent) -> u64 {
    event
        .time_us
        .unwrap_or_else(|| event.time_ms.saturating_mul(1000))
}

/// Validate one event: known control/action pair, payload bounds,
/// press/release pairing tracked in `held`, click gestures tracked
/// in `open_clicks`.
fn validate_event<'a>(
    event: &'a TraceEvent,
    held: &mut Vec<&'a str>,
    open_clicks: &mut Vec<(i32, i32)>,
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
            let Some((x, y)) = event.x.zip(event.y) else {
                violations.push(format!("{at}: click needs x and y"));
                return;
            };
            if !(0..800).contains(&x) || !(0..600).contains(&y) {
                violations.push(format!("{at}: click ({x},{y}) outside the 800x600 canvas"));
            }
            match event.action.as_str() {
                // A lone press stays a tap shorthand for the
                // windowed XTEST driver (the committed U1 trace
                // clicks this way); an explicit release closes the
                // gesture at the same point.
                "press" => open_clicks.push((x, y)),
                "release" => match open_clicks.pop() {
                    Some(open) if open == (x, y) => {}
                    _ => {
                        violations.push(format!("{at}: release at ({x},{y}) closes no open press"))
                    }
                },
                _ => violations.push(format!("{at}: click presses or releases")),
            }
            validate_name(event.control_id.as_deref(), "control_id", &at, violations);
            validate_name(event.label.as_deref(), "label", &at, violations);
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

/// Optional human/exe name on a click: present means visible —
/// blank or control-character content rejects.
fn validate_name(name: Option<&str>, field: &str, at: &str, violations: &mut Vec<String>) {
    let Some(name) = name else { return };
    if name.trim().is_empty() || name.chars().any(char::is_control) {
        violations.push(format!("{at}: {field} must be visible text"));
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
            time_us: None,
            control: control.to_owned(),
            action: action.to_owned(),
            x: None,
            y: None,
            text: None,
            screen: None,
            control_id: None,
            label: None,
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

    fn click_at(sequence: u64, time_us: u64, action: &str, x: i32, y: i32) -> TraceEvent {
        TraceEvent {
            sequence,
            time_ms: time_us / 1000,
            time_us: Some(time_us),
            control: "click".to_owned(),
            action: action.to_owned(),
            x: Some(x),
            y: Some(y),
            text: None,
            screen: None,
            control_id: Some("IDC_BUTTON_SSHOP_BUYITEM".to_owned()),
            label: Some("cart-commit".to_owned()),
        }
    }

    #[test]
    fn time_us_must_agree_with_time_ms() {
        // Ticket #208: the microsecond stamp is the authority; a
        // trace carrying both clocks must name the same instant.
        let mut press = click_at(0, 1_000_000, "press", 630, 510);
        press.time_ms = 999;
        assert!(validate(&trace(vec![press])).is_err());
    }

    #[test]
    fn time_us_orders_events_on_the_microsecond_clock() {
        // Millisecond pacing ties (same `time_ms`, 1 ms apart in
        // reality) still move forward on the microsecond clock;
        // backwards microsecond time rejects even when `time_ms`
        // never moves backwards.
        let forward = trace(vec![
            click_at(0, 1_000_000, "press", 630, 510),
            click_at(1, 1_000_400, "release", 630, 510),
        ]);
        assert!(validate(&forward).is_ok());
        let mut back = click_at(1, 999_900, "release", 630, 510);
        back.time_ms = 1000;
        let backward = trace(vec![click_at(0, 1_000_000, "press", 630, 510), back]);
        assert!(validate(&backward).is_err());
    }

    #[test]
    fn click_release_needs_a_matching_press() {
        // Press/release pairs pin the gesture: a lone release and a
        // release at another point both reject. A lone press stays a
        // tap shorthand for the windowed XTEST driver (the committed
        // U1 trace clicks this way), so only the release arm is
        // strict.
        let lone_release = trace(vec![click_at(0, 1_000_000, "release", 630, 510)]);
        assert!(validate(&lone_release).is_err());
        let moved_release = trace(vec![
            click_at(0, 1_000_000, "press", 630, 510),
            click_at(1, 1_000_400, "release", 100, 100),
        ]);
        assert!(validate(&moved_release).is_err());
        let pair = trace(vec![
            click_at(0, 1_000_000, "press", 630, 510),
            click_at(1, 1_000_400, "release", 630, 510),
        ]);
        assert!(validate(&pair).is_ok());
        let tap = trace(vec![click_at(0, 1_000_000, "press", 630, 510)]);
        assert!(validate(&tap).is_ok());
    }

    #[test]
    fn control_id_and_label_reject_blanks() {
        let mut blank_id = click_at(0, 1_000_000, "press", 630, 510);
        blank_id.control_id = Some(String::new());
        assert!(validate(&trace(vec![blank_id])).is_err());
        let mut blank_label = click_at(0, 1_000_000, "press", 630, 510);
        blank_label.label = Some("  ".to_owned());
        assert!(validate(&trace(vec![blank_label])).is_err());
        assert!(validate(&trace(vec![click_at(0, 1_000_000, "press", 630, 510)])).is_ok());
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

    /// Fixture paths for the ticket #208 shop/MyRoom menu traces
    /// (synthetic clicks, committed beside the driver traces).
    const MENU_TRACES: [&str; 5] = [
        "shop-entry-from-room.json",
        "shop-goods-browse-pager.json",
        "shop-row-cart-buyitem.json",
        "myroom-bag-slot-equip.json",
        "giftbox-claim.json",
    ];

    #[test]
    fn committed_menu_traces_validate() {
        // Ticket #208: every committed menu trace validates —
        // monotonic microsecond stamps, closed press/release pairs,
        // canvas bounds, named controls.
        for name in MENU_TRACES {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/input-trace")
                .join(name);
            let loaded = load_validated(&path)
                .unwrap_or_else(|_| panic!("menu trace must validate: {name}"));
            assert!(
                loaded.events.iter().any(|event| event.time_us.is_some()),
                "{name} carries microsecond stamps"
            );
            assert!(
                loaded.events.iter().any(|event| event.control_id.is_some()),
                "{name} names its controls"
            );
        }
    }
}
