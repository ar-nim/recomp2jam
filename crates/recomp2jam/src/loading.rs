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
use o2jam_shell::avatar::{ojn_cover_figure, AvatarFigure};
use o2jam_shell::screens::paint::LoadingStatus;
use o2jam_shell::screens::ScreenGame;

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

/// Staged loading-gate phases (lifecycle §9 steps 2–5): staging
/// (`0x4f8b80`), OJN decrypt (`0x412180`), Vorbis decode
/// (`0x54d460`), and song-clock arming (`0x4e47a0`).
///
/// Staged for the windowed loader wiring (`main.rs` stays untouched
/// here): allowed dead until that follow-up calls it.
#[allow(dead_code)]
pub const LOADING_GATE_STEPS: usize = 4;

/// Walk fraction for `completed` of `total` gate units (`None` while
/// `total` is zero: archives stay read-only, unknown stays
/// indeterminate — never invents a fraction).
///
/// Staged for the windowed loader wiring: allowed dead until then.
#[allow(dead_code)]
#[must_use]
pub fn loading_fraction(completed: usize, total: usize) -> Option<f32> {
    let done = u32::try_from(completed).ok()?;
    let whole = u32::try_from(total).ok()?;
    if whole == 0 {
        return None;
    }
    let ratio = f64::from(done.min(whole)) / f64::from(whole);
    Some(ratio.clamp(0.0, 1.0) as f32)
}

/// Live loading status for a walk position plus the song being
/// loaded (OJN header / `OJNList.dat` title-artist). `None` while the
/// walk size is unknown — the shell keeps its indeterminate full bar.
///
/// Staged for the windowed loader wiring: allowed dead until then.
#[allow(dead_code)]
#[must_use]
pub fn loading_status(
    completed: usize,
    total: usize,
    title: &str,
    artist: &str,
) -> Option<LoadingStatus> {
    let progress = loading_fraction(completed, total)?;
    Some(LoadingStatus {
        progress,
        title: title.to_owned(),
        artist: artist.to_owned(),
    })
}

/// Live loading status for one gate phase (`0` = gate entered,
/// `LOADING_GATE_STEPS` = song clock armed). `None` past the gate:
/// overruns are unknown, never a fraction.
///
/// Staged for the windowed loader wiring: allowed dead until then.
#[allow(dead_code)]
#[must_use]
pub fn loading_phase_status(phase: usize, title: &str, artist: &str) -> Option<LoadingStatus> {
    if phase > LOADING_GATE_STEPS {
        return None;
    }
    loading_status(phase, LOADING_GATE_STEPS, title, artist)
}

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

/// Decode the walked chart's OJN cover section as the loading-gate song
/// illustration: one opaque 800x600 layer at anchor `(0, 0)`, staged under
/// `OJN_COVER` — never an archive body. `None` on any miss — non-chart
/// bytes, unknown magic, undecodable stream — and the loading gate keeps
/// its planet fallback then; archives stay read-only and nothing is
/// invented.
///
/// Staged for the windowed loader wiring (the `session`/`main` gate
/// follow-up feeds walked bytes through [`stage_loading_cover`]):
/// allowed dead until then.
#[allow(dead_code)]
#[must_use]
pub fn loading_cover_figure(chart_bytes: &[u8]) -> Option<AvatarFigure> {
    ojn_cover_figure(chart_bytes)
}

/// Feed walked OJN bytes into the loading scene staging: decode the chart's
/// cover section and stage it into the shell's Loading art beside the planet
/// overlay. `true` while a cover staged; any miss leaves the planet
/// fallback in place (see [`ScreenGame::stage_loading_cover`]).
///
/// Called by the chart load paths (`session::stage_chart_bytes`,
/// `main::load_level_chart`) with the walked bytes, so real loads paint
/// the cover.
pub fn stage_loading_cover(game: &mut ScreenGame, chart_bytes: &[u8]) -> bool {
    game.stage_loading_cover(chart_bytes)
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

    #[test]
    fn staging_phase_marks_a_quarter() {
        assert_eq!(loading_fraction(1, LOADING_GATE_STEPS), Some(0.25));
    }

    #[test]
    fn decrypt_phase_marks_a_half() {
        assert_eq!(loading_fraction(2, LOADING_GATE_STEPS), Some(0.5));
    }

    #[test]
    fn decode_phase_marks_three_quarters() {
        assert_eq!(loading_fraction(3, LOADING_GATE_STEPS), Some(0.75));
    }

    #[test]
    fn clock_phase_marks_full() {
        assert_eq!(loading_fraction(4, LOADING_GATE_STEPS), Some(1.0));
    }

    #[test]
    fn unknown_walk_stays_indeterminate() {
        assert_eq!(loading_fraction(0, 0), None);
    }

    #[test]
    fn phase_past_the_gate_stays_indeterminate() {
        assert_eq!(loading_phase_status(LOADING_GATE_STEPS + 1, "T", "A"), None);
    }

    #[test]
    fn phase_status_carries_title_and_artist() {
        let status = loading_phase_status(2, "First", "Band").unwrap();
        assert_eq!(status.progress, 0.5);
        assert_eq!(status.title, "First");
        assert_eq!(status.artist, "Band");
    }

    #[test]
    fn walk_status_needs_a_known_size() {
        assert_eq!(loading_status(0, 0, "First", "Band"), None);
    }

    #[test]
    fn loading_cover_rejects_non_chart_bytes() {
        assert!(loading_cover_figure(b"garbage").is_none());
        assert!(loading_cover_figure(&[]).is_none());
    }

    #[test]
    fn loading_cover_decodes_the_walked_chart() {
        // Install-guarded: song 757's OJN cover section decodes to the
        // 800x600 illustration the loading capture pins. Skips when
        // `install-root` is absent.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let figure = loading_cover_figure(&bytes).expect("the walked cover decodes");
        assert_eq!(figure.len(), 1);
        let layer = figure.layer(0).expect("one opaque layer");
        assert_eq!((layer.image.w, layer.image.h), (800, 600));
        assert_eq!((layer.dx, layer.dy), (0, 0));
    }

    #[test]
    fn loading_cover_feed_reaches_the_shell_scene() {
        // The gate step hands walked bytes to the shell staging.
        // Skips when `install-root` is absent.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let mut game = ScreenGame::new();
        assert!(stage_loading_cover(&mut game, &bytes));
        assert!(game.has_loading_cover());
    }

    #[test]
    fn loading_cover_miss_keeps_the_planet_fallback() {
        let mut game = ScreenGame::new();
        assert!(!stage_loading_cover(&mut game, b"garbage"));
        assert!(!game.has_loading_cover());
    }
}
