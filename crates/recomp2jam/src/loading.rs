//! CLI side of the lifecycle §9 loading gate (7.55).
//!
//! Orchestrates the file-side gates (`o2jam_file::loading`) into the
//! `play` flow: select validation (`0x5718b0`), staging (`0x4f8b80`),
//! OJN open (`0x412180`), codec format check (`0x54d460`), and the
//! loading-screen handoff (`0x4e47a0`: `Trebuchet MS`/`Arial` fonts,
//! `MusicLoading.ojs`, song-table walk, song clock armed).
//!
//! Result grades stay OPEN and unimplemented: `gameplay.md` records
//! zero pointer refs in `.text` to any grade-string VA (file
//! `0x247363`/`0x2473db`/`0x24765b`, `JAM` strings
//! `0x24745e`/`0x24762b`/`0x2476b3`) — result-screen grades are
//! `.ojs`-driven with no rank-comparison arithmetic located in-exe.
//! See [`RESULT_GRADES_VERIFIED`].

use anyhow::Result;
use o2jam_file::{FileError, OjmEntry};

/// Whether result grade thresholds are exe-verified and implemented.
///
/// `false`: the win rule (`gameplay.md`) leaves grade thresholds
/// OPEN — negative evidence only (no in-exe refs, no comparison
/// code). Flip only with an in-exe reference to a grade string VA.
pub const RESULT_GRADES_VERIFIED: bool = false;

/// Loading-screen assets the handoff shows (`0x4e47a0`).
pub const LOADING_FONTS: [&str; 2] = ["Trebuchet MS", "Arial"];
/// Loading-screen overlay the handoff shows (`0x4e47a0`).
pub const LOADING_OVERLAY: &str = "MusicLoading.ojs";

/// A validated song select plus its staged extract name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaySelection {
    /// Song id from the song table.
    pub song_id: u32,
    /// Difficulty slot (`0..OJN_LEVEL_COUNT`).
    pub level: usize,
    /// Staged path (`MUSIC\Temp\%s%d_%d` shape).
    pub extract_name: String,
}

/// A chart opened and walked for play (OJN open + table walk).
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedChart {
    /// Song id from the chart header.
    pub song_id: u32,
    /// Tempo (`f32` at +16).
    pub bpm: f32,
    /// Container version (`f32` at +8; selects the OJM descriptor).
    pub format_version: f32,
    /// Events walked in the selected level slice.
    pub event_count: usize,
}

/// Stage an offline select and name its extract file (gates 1–2
/// without a song table: the level slot is range-checked by
/// [`prepare_chart`], the id comes from the opened header).
#[must_use]
pub fn stage_offline_select(song_id: u32, level: usize, chart_stem: &str) -> PlaySelection {
    PlaySelection {
        song_id,
        level,
        extract_name: o2jam_file::stage_extract_name(chart_stem, song_id, level),
    }
}

/// Open a chart buffer and walk one level slice (gates 3 + 5 walk).
///
/// # Errors
///
/// `FileError` on bad magic, truncation, or an out-of-range level.
pub fn prepare_chart(chart_bytes: &[u8], level: usize) -> Result<PreparedChart, FileError> {
    let (header, _) = o2jam_file::open_chart(chart_bytes)?;
    let events = o2jam_file::parse_ojn_level_events(chart_bytes, level)?;
    Ok(PreparedChart {
        song_id: header.song_id,
        bpm: header.bpm,
        format_version: header.format_version,
        event_count: events.len(),
    })
}

/// Check one bank entry's format before decode (gate 4).
///
/// # Errors
///
/// `FileError::UnknownFormat` when channels/bits fall outside the
/// exe's `1/2`-channel, `8/16`-bit checks.
pub fn check_bank_entry(entry: &OjmEntry) -> Result<(), FileError> {
    o2jam_file::validate_sample_format(entry)
}

/// Describe the loading-screen handoff for logs (gate 5).
#[must_use]
pub fn handoff_note(selection: &PlaySelection, chart: &PreparedChart) -> String {
    format!(
        "{} over {}+{} (select {}: {} events, {:.1} BPM) -> clock armed",
        LOADING_OVERLAY,
        LOADING_FONTS[0],
        LOADING_FONTS[1],
        selection.extract_name,
        chart.event_count,
        chart.bpm,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use o2jam_file::OjnListEntry;

    fn catalog_blob() -> Vec<u8> {
        let mut record = vec![0u8; 300];
        record[4..8].copy_from_slice(&757u32.to_le_bytes());
        record[8..11].copy_from_slice(b"ojn");
        record[12..16].copy_from_slice(&2.9f32.to_le_bytes());
        record[20..24].copy_from_slice(&124f32.to_le_bytes());
        record[240..251].copy_from_slice(b"o2ma757.ojm");
        record
    }

    fn catalog() -> Vec<OjnListEntry<'static>> {
        let owned: &'static [u8] = Box::leak(catalog_blob().into_boxed_slice());
        o2jam_file::parse_ojn_list(owned).unwrap()
    }

    fn chart_blob() -> Vec<u8> {
        let mut blob = vec![0u8; 300];
        blob[0..4].copy_from_slice(&757u32.to_le_bytes());
        blob[4..8].copy_from_slice(b"ojn\0");
        blob[8..12].copy_from_slice(&2.9f32.to_le_bytes());
        blob[16..20].copy_from_slice(&124f32.to_le_bytes());
        blob[64..68].copy_from_slice(&0u32.to_le_bytes());
        blob[68..72].copy_from_slice(&0u32.to_le_bytes());
        blob[72..76].copy_from_slice(&0u32.to_le_bytes());
        blob[284..288].copy_from_slice(&300u32.to_le_bytes());
        blob[288..292].copy_from_slice(&300u32.to_le_bytes());
        blob[292..296].copy_from_slice(&300u32.to_le_bytes());
        blob[296..300].copy_from_slice(&300u32.to_le_bytes());
        blob
    }

    #[test]
    fn offline_select_stages_the_extract_name() {
        let selection = stage_offline_select(757, 1, "o2ma757.ojm");
        assert_eq!(selection.song_id, 757);
        assert_eq!(selection.level, 1);
        assert_eq!(selection.extract_name, "Music\\Temp\\o2ma757757_1");
    }

    #[test]
    fn catalog_select_still_validates_against_the_table() {
        let entries = catalog();
        let entry = o2jam_file::validate_selection(757, 1, &entries).unwrap();
        assert_eq!(entry.song_id, 757);
        assert!(o2jam_file::validate_selection(9999, 0, &entries).is_err());
        assert!(o2jam_file::validate_selection(757, 9, &entries).is_err());
    }

    #[test]
    fn prepare_rejects_bad_slots() {
        assert!(prepare_chart(&chart_blob(), 9).is_err());
    }

    #[test]
    fn prepare_opens_and_walks_the_slice() {
        let chart = prepare_chart(&chart_blob(), 0).unwrap();
        assert_eq!(chart.song_id, 757);
        assert_eq!(chart.event_count, 0);
    }

    #[test]
    fn bank_check_mirrors_codec_gate() {
        let entry = OjmEntry {
            name: "s1".to_owned(),
            size: 8,
            data_offset: 0,
            audio_format: 1,
            num_channels: 1,
            sample_rate: 44100,
            bit_rate: 88200,
            block_align: 2,
            bits_per_sample: 16,
        };
        assert!(check_bank_entry(&entry).is_ok());
        let bad = OjmEntry {
            num_channels: 6,
            ..entry
        };
        assert!(check_bank_entry(&bad).is_err());
    }

    #[test]
    fn handoff_names_overlay_fonts_and_clock() {
        let selection = stage_offline_select(757, 0, "o2ma757.ojm");
        let chart = prepare_chart(&chart_blob(), 0).unwrap();
        let note = handoff_note(&selection, &chart);
        assert!(note.contains(LOADING_OVERLAY));
        assert!(note.contains("clock armed"));
        assert!(note.contains(&selection.extract_name));
    }

    #[test]
    fn grades_stay_open_until_exe_verified() {
        const { assert!(!RESULT_GRADES_VERIFIED) };
    }
}
