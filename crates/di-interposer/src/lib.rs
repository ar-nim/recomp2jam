//! `di-interposer` — compiles a chart key schedule into a DirectInput feed.
//!
//! The 7.55 exe reads gameplay keys from DirectInput device state and buffered
//! records, so window-message input cannot reach it. This crate owns the
//! schedule-to-DIK half: a line `<song_ms>:<dik>[,<dik>...]` becomes one press
//! per DIK and one release after the hold time. The C DLL in
//! `tools/di-interposer/` consumes the binary form at run time.
//!
//! Feed layout (all integers little-endian):
//!   header: `O2DI1701`, u32 version, u32 entry count
//!   entry:  u64 Unix-ms target, u32 (DIK | pressed<<8), u32 reserved
//!
//! The pressed flag becomes bit 7 of the 0x100-byte device state and bit 7 of
//! the buffered record flags, matching the exe's press tests.

use std::error::Error;
use std::fmt;

pub const HEADER_MAGIC: &[u8; 8] = b"O2DI1701";
pub const FEED_VERSION: u32 = 1;
pub const HEADER_BYTES: usize = 16;
pub const RECORD_BYTES: usize = 16;
pub const PRESSED_BIT: u8 = 0x80;

/// One chart moment: all listed DIKs are stroked together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyGroup {
    pub time_ms: u64,
    pub diks: Vec<u8>,
}

/// A monotonic list of chart moments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub groups: Vec<KeyGroup>,
}

/// One scheduled keyboard transition in the binary feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedEntry {
    pub time_ms: u64,
    pub dik: u8,
    pub pressed: bool,
}

/// One transition exposed through `GetDeviceData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub time_ms: u64,
    pub dik: u8,
    pub pressed: bool,
}

/// Why a schedule line cannot be compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    MissingSeparator { line: usize },
    InvalidTime { line: usize, text: String },
    InvalidDik { line: usize, text: String },
    DuplicateDik { line: usize, dik: u8 },
    EmptyGroup { line: usize },
    NonMonotonic { line: usize },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSeparator { line } => {
                write!(formatter, "line {line}: expected '<ms>:<dik>[,<dik>...]'")
            }
            Self::InvalidTime { line, text } => {
                write!(formatter, "line {line}: invalid time '{text}'")
            }
            Self::InvalidDik { line, text } => {
                write!(formatter, "line {line}: invalid DIK '{text}'")
            }
            Self::DuplicateDik { line, dik } => {
                write!(formatter, "line {line}: duplicate DIK {dik:#04x}")
            }
            Self::EmptyGroup { line } => write!(formatter, "line {line}: no DIKs"),
            Self::NonMonotonic { line } => {
                write!(formatter, "line {line}: time is before the previous group")
            }
        }
    }
}

impl Error for ScheduleError {}

/// Why a feed cannot be encoded for the C interposer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedEncodeError {
    pub entry_count: usize,
}

impl fmt::Display for FeedEncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "feed has {} entries, header allows u32::MAX",
            self.entry_count
        )
    }
}

impl Error for FeedEncodeError {}

fn parse_dik(text: &str, line: usize) -> Result<u8, ScheduleError> {
    let dik = u8::from_str_radix(text.trim(), 16).map_err(|_| ScheduleError::InvalidDik {
        line,
        text: text.into(),
    })?;
    if dik == 0 || dik & PRESSED_BIT != 0 {
        return Err(ScheduleError::InvalidDik {
            line,
            text: text.into(),
        });
    }
    Ok(dik)
}

fn parse_group(text: &str, line: usize) -> Result<KeyGroup, ScheduleError> {
    let (time_text, dik_text) = text
        .trim()
        .split_once(':')
        .ok_or(ScheduleError::MissingSeparator { line })?;
    let time_ms = time_text
        .trim()
        .parse::<u64>()
        .map_err(|_| ScheduleError::InvalidTime {
            line,
            text: time_text.into(),
        })?;
    let mut diks = Vec::new();
    for code in dik_text.split(',') {
        let dik = parse_dik(code, line)?;
        if diks.contains(&dik) {
            return Err(ScheduleError::DuplicateDik { line, dik });
        }
        diks.push(dik);
    }
    if diks.is_empty() {
        return Err(ScheduleError::EmptyGroup { line });
    }
    Ok(KeyGroup { time_ms, diks })
}

/// Parse the pinned `<song_ms>:<dik>[,<dik>...]` schedule format.
pub fn parse_schedule(text: &str) -> Result<Schedule, ScheduleError> {
    let mut groups = Vec::new();
    let mut previous_ms = None;
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let group = parse_group(line, index + 1)?;
        if previous_ms.is_some_and(|previous| group.time_ms < previous) {
            return Err(ScheduleError::NonMonotonic { line: index + 1 });
        }
        previous_ms = Some(group.time_ms);
        groups.push(group);
    }
    if groups.is_empty() {
        return Err(ScheduleError::EmptyGroup { line: 0 });
    }
    Ok(Schedule { groups })
}

/// Expand chart moments into press/release pairs on an absolute clock.
pub fn build_feed(schedule: &Schedule, epoch_ms: u64, hold_ms: u32) -> Vec<FeedEntry> {
    let mut entries = Vec::with_capacity(schedule.groups.len() * 2);
    for group in &schedule.groups {
        let stroke_ms = epoch_ms.saturating_add(group.time_ms);
        let release_ms = stroke_ms.saturating_add(u64::from(hold_ms));
        for &dik in &group.diks {
            entries.push(FeedEntry {
                time_ms: stroke_ms,
                dik,
                pressed: true,
            });
            entries.push(FeedEntry {
                time_ms: release_ms,
                dik,
                pressed: false,
            });
        }
    }
    entries.sort_by_key(|entry| (entry.time_ms, !entry.pressed));
    entries
}

/// Reproduce the exe's 0x100-byte keyboard state at `now_ms`.
pub fn keyboard_state_at(entries: &[FeedEntry], now_ms: u64) -> [u8; 256] {
    let mut state = [0_u8; 256];
    for entry in entries {
        if entry.time_ms > now_ms {
            break;
        }
        state[usize::from(entry.dik)] = if entry.pressed { PRESSED_BIT } else { 0 };
    }
    state
}

/// Buffered events whose target time is in `(since_ms, now_ms]`.
pub fn events_until(entries: &[FeedEntry], since_ms: u64, now_ms: u64) -> Vec<InputEvent> {
    entries
        .iter()
        .filter(|entry| entry.time_ms > since_ms && entry.time_ms <= now_ms)
        .map(|entry| InputEvent {
            time_ms: entry.time_ms,
            dik: entry.dik,
            pressed: entry.pressed,
        })
        .collect()
}

/// Flags placed in the second DWORD of one buffered record.
pub fn buffered_flags(event: InputEvent) -> u8 {
    if event.pressed {
        PRESSED_BIT
    } else {
        0
    }
}

/// Serialize the feed for the C interposer.
pub fn encode_feed(entries: &[FeedEntry]) -> Result<Vec<u8>, FeedEncodeError> {
    let count = u32::try_from(entries.len()).map_err(|_| FeedEncodeError {
        entry_count: entries.len(),
    })?;
    let mut bytes = Vec::with_capacity(HEADER_BYTES + entries.len() * RECORD_BYTES);
    bytes.extend_from_slice(HEADER_MAGIC);
    bytes.extend_from_slice(&FEED_VERSION.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    for entry in entries {
        bytes.extend_from_slice(&entry.time_ms.to_le_bytes());
        let code = u32::from(entry.dik) | (u32::from(entry.pressed) << 8);
        bytes.extend_from_slice(&code.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
    }
    Ok(bytes)
}
