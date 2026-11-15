//! Shared input-log assertions: both Linux arms read the same game
//! log lines (`click x y` on press, `click-up x y` on release, one
//! `frame` line per paint), so the polling lives here once.

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

/// Last routed press line in already-read log text: `event <wall_ms>
/// click <x> <y>`. Release (`click-up`) lines never count.
#[must_use]
pub fn latest_click_line(content: &str) -> Option<(i32, i32)> {
    for line in content.lines().rev() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() == 5 && fields[0] == "event" && fields[2] == "click" {
            let parsed = fields[3].parse().ok().zip(fields[4].parse().ok());
            if let Some((x, y)) = parsed {
                return Some((x, y));
            }
        }
    }
    None
}

/// Content hash of the newest `frame` line for one screen, if any.
#[must_use]
pub fn last_content_hash(content: &str, screen: &str) -> Option<String> {
    let marker = format!("screen={screen}");
    content
        .lines()
        .filter(|line| line.split_whitespace().nth(3) == Some(marker.as_str()))
        .filter_map(|line| line.split("content=").nth(1).map(str::to_owned))
        .next_back()
}

/// Poll until `settled` reads true, pumping the backend between reads.
/// Returns false on deadline expiry (verdict FAIL, not a harness error).
pub fn wait_for_condition(
    pump: &mut dyn FnMut() -> Result<()>,
    settled: &mut dyn FnMut() -> Result<bool>,
    wait: Duration,
) -> Result<bool> {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        pump()?;
        if settled()? {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(false)
}

/// Read the whole input log, tolerating a not-yet-created file.
pub fn read_log(input_log: &Path) -> Result<String> {
    match std::fs::read_to_string(input_log) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error).with_context(|| format!("read {}", input_log.display())),
    }
}

/// Last `tail_lines` lines of already-read log text.
#[must_use]
pub fn log_tail(content: &str, tail_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    lines[lines.len().saturating_sub(tail_lines)..].join("\n")
}

#[cfg(test)]
mod log_tests {
    use super::*;

    const SAMPLE: &str = "event 12 click 400 300\nevent 13 click-up 400 300\nframe 7 wall_ms=99 screen=planet state=0000000000000001 content=aaa\nframe 8 wall_ms=100 screen=splash state=0000000000000002 content=bbb\n";

    #[test]
    fn press_line_parses_while_release_does_not_count() {
        assert_eq!(latest_click_line(SAMPLE), Some((400, 300)));
        assert_eq!(latest_click_line("event 13 click-up 400 300\n"), None);
        assert_eq!(latest_click_line(""), None);
    }

    #[test]
    fn content_hash_reads_the_newest_frame_for_the_screen() {
        assert_eq!(last_content_hash(SAMPLE, "planet").as_deref(), Some("aaa"));
        assert_eq!(last_content_hash(SAMPLE, "menu"), None);
    }

    #[test]
    fn condition_wait_returns_true_once_settled() {
        use std::cell::Cell;
        use std::rc::Rc;
        let calls = Rc::new(Cell::new(0));
        let pumping = calls.clone();
        let mut pump = move || -> Result<()> {
            pumping.set(pumping.get() + 1);
            Ok(())
        };
        let checking = calls.clone();
        let mut settled = move || -> Result<bool> { Ok(checking.get() >= 2) };
        let done = wait_for_condition(&mut pump, &mut settled, Duration::from_secs(5)).unwrap();
        assert!(done);
    }

    #[test]
    fn condition_wait_expires_quietly_on_deadline() {
        let settled = wait_for_condition(
            &mut || Ok(()),
            &mut || Ok(false),
            Duration::from_millis(120),
        )
        .unwrap();
        assert!(!settled);
    }

    #[test]
    fn missing_log_reads_as_empty() {
        let content = read_log(Path::new("/nonexistent-dir/input.log")).unwrap();
        assert!(content.is_empty());
    }
}
