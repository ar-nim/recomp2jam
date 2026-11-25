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

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use o2jam_audio::PcmSamples;
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

/// Last frame of Waiting after the host's `4010` on the qualified solo
/// wall trace (`solo-gate-02` pack, 10 fps watcher).
pub const LOADING_COVER_VISIBLE_EARLIEST: Duration = Duration::from_millis(198);
/// First full-bleed Loading frame after `4010` on the same trace.
pub const LOADING_COVER_VISIBLE_LATEST: Duration = Duration::from_millis(291);
/// Wall span from the `4011` observation to C2S `4012` (660.488 ms in the
/// pack; the earlier pack measured 606.051 ms). The cover is already up at
/// the lower bracket, so the confirm never creates the frame.
pub const LOADING_CONFIRM_AFTER_START: Duration = Duration::from_millis(660);
/// Lower bracket for the handoff: the last Loading frame still paints at
/// 904.191 ms, i.e. 232 ms after the 672.192 ms confirm observation.
pub const LOADING_PLAYING_EARLIEST_AFTER_CONFIRM: Duration = Duration::from_millis(232);
/// Upper bracket: the first Playing frame sits at 1,068.015 ms, i.e.
/// 396 ms after the confirm observation.
pub const LOADING_PLAYING_LATEST_AFTER_CONFIRM: Duration = Duration::from_millis(396);
/// Handoff point inside the observed 232-396 ms bracket. The exact
/// in-process instant stays OPEN; no exe pin picks one value in the range.
pub const LOADING_PLAYING_AFTER_CONFIRM: Duration = Duration::from_millis(300);

// Bracket pins for the #598 wall-trace guard; the pacing state machine
// uses the measured spans, not the bracket ends.
#[allow(dead_code)]
const _: [Duration; 4] = [
    LOADING_COVER_VISIBLE_EARLIEST,
    LOADING_COVER_VISIBLE_LATEST,
    LOADING_PLAYING_EARLIEST_AFTER_CONFIRM,
    LOADING_PLAYING_LATEST_AFTER_CONFIRM,
];

/// Outward effect of one clocked Loading-gate advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingGateCue {
    /// Keep painting the staged cover; nothing goes on the wire.
    None,
    /// Post `ConfirmMusicLoaded` 4012; stay in Loading for now.
    SendConfirm,
    /// Enter Playing only after the confirm has been posted.
    EnterPlaying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadingGatePhase {
    Idle,
    CoverUp {
        entered_at: Instant,
    },
    Ready {
        entered_at: Instant,
        armed_at: Instant,
    },
    ConfirmSent {
        sent_at: Instant,
    },
}

/// Observable arm state of the joined Loading gate (#608). The gate
/// walks cover -> chart armed -> 4012 posted; the exe pins the wire
/// correlate (4011, then C2S 4012, then S2C 4013, P3/P13) but no
/// stopped-state read of its arm word exists yet, so this is our
/// state, never an exe memory claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingGateArm {
    /// No start broadcast has armed the gate.
    Idle,
    /// Cover is up; the chart gate has not armed yet.
    AwaitingChart,
    /// Chart staged and audio ready; confirm waits for the wall span.
    ChartArmed {
        entered_at: Instant,
        armed_at: Instant,
    },
    /// 4012 posted; the Playing leg still waits.
    ConfirmPosted { sent_at: Instant },
}

/// Wall pacing of the joined Loading gate, driven by explicit paint
/// instants. The exe's four internal legs have no in-process timing pin,
/// so these spans are wire/frame proxies from two qualified solo boots;
/// the exact handoff inside the last bracket stays OPEN.
#[derive(Debug)]
pub struct LoadingGatePacing {
    phase: LoadingGatePhase,
}

impl Default for LoadingGatePacing {
    fn default() -> Self {
        Self {
            phase: LoadingGatePhase::Idle,
        }
    }
}

impl LoadingGatePacing {
    /// Arm the gate when the shell enters Loading on a start broadcast.
    pub fn enter_at(&mut self, now: Instant) {
        self.phase = LoadingGatePhase::CoverUp { entered_at: now };
    }

    /// Drop the arm when Loading ends (or a user Confirm owns the send).
    pub fn clear(&mut self) {
        self.phase = LoadingGatePhase::Idle;
    }

    /// `true` while the wall-paced confirm still needs paint clocks.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !matches!(self.phase, LoadingGatePhase::Idle)
    }

    /// Latch the chart arm at one explicit paint instant. Only the
    /// cover phase accepts it; a failed or still-staged chart never
    /// opens the 4012 send.
    pub fn arm_chart_at(&mut self, now: Instant) {
        if let LoadingGatePhase::CoverUp { entered_at } = self.phase {
            self.phase = LoadingGatePhase::Ready {
                entered_at,
                armed_at: now,
            };
        }
    }

    /// Expose the arm latch for guards and gate telemetry.
    #[must_use]
    pub const fn arm_state(&self) -> LoadingGateArm {
        match self.phase {
            LoadingGatePhase::Idle => LoadingGateArm::Idle,
            LoadingGatePhase::CoverUp { .. } => LoadingGateArm::AwaitingChart,
            LoadingGatePhase::Ready {
                entered_at,
                armed_at,
            } => LoadingGateArm::ChartArmed {
                entered_at,
                armed_at,
            },
            LoadingGatePhase::ConfirmSent { sent_at } => LoadingGateArm::ConfirmPosted { sent_at },
        }
    }

    /// Re-arm a failed 4012 at this instant, so the retry keeps the
    /// same hold and never enters Playing on a dead socket.
    pub fn retry_confirm_at(&mut self, now: Instant) {
        if matches!(self.phase, LoadingGatePhase::ConfirmSent { .. }) {
            self.phase = LoadingGatePhase::CoverUp { entered_at: now };
        }
    }

    /// Keep the heartbeat after a refused handoff. The 4012 is already
    /// latched, so this re-arms only the Playing leg; the shell's gate
    /// gets another paint clock to finish its own chart work.
    pub fn retry_playing_at(&mut self, now: Instant) {
        if matches!(self.phase, LoadingGatePhase::Idle) {
            self.phase = LoadingGatePhase::ConfirmSent { sent_at: now };
        }
    }

    /// One clocked advance. Each outward cue fires once; a later advance
    /// in the same phase is a no-op.
    #[must_use]
    pub fn advance_at(&mut self, now: Instant) -> LoadingGateCue {
        match self.phase {
            LoadingGatePhase::Idle | LoadingGatePhase::CoverUp { .. } => LoadingGateCue::None,
            LoadingGatePhase::Ready { entered_at, .. } => {
                if now.saturating_duration_since(entered_at) >= LOADING_CONFIRM_AFTER_START {
                    self.phase = LoadingGatePhase::ConfirmSent { sent_at: now };
                    LoadingGateCue::SendConfirm
                } else {
                    LoadingGateCue::None
                }
            }
            LoadingGatePhase::ConfirmSent { sent_at } => {
                if now.saturating_duration_since(sent_at) >= LOADING_PLAYING_AFTER_CONFIRM {
                    self.phase = LoadingGatePhase::Idle;
                    LoadingGateCue::EnterPlaying
                } else {
                    LoadingGateCue::None
                }
            }
        }
    }
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

/// Playable onsets of the shipped `Tutorial.ojn` Easy slice.
///
/// The exe's tutorial entry writes song id 8888 (`S-PLAY-B3`) and its
/// Init constructs the same `CNewNote` highway as Playing (`0x4f0950`
/// → `0x56f6a0`/`0x56ff30`). Routing these onsets through
/// [`ScreenGame::load_chart`] gives the Tutorial painter the shared
/// staged-note seam. `None` keeps a missing or wrong chart silent.
#[must_use]
pub fn tutorial_chart_onsets(bytes: &[u8]) -> Option<Vec<(u8, u64, u32)>> {
    let prepared = prepare_chart(bytes, 0).ok()?;
    if prepared.song_id != 8_888 {
        return None;
    }
    let events = o2jam_file::parse_ojn_level_events(bytes, 0).ok()?;
    let chart = o2jam_game::build_level_chart(prepared.bpm, &events);
    Some(chart.hit_onsets_ms())
}

/// Decode one OGG body into the bank, skipping undecodable streams.
///
/// Loading gate 4 (`0x54d460`): only mono/stereo decodes enter the bank.
fn insert_ogg(bank: &mut HashMap<u32, PcmSamples>, slot: u32, ogg: &[u8]) {
    if let Ok(pcm) = o2jam_audio::decode_vorbis(ogg) {
        if o2jam_file::validate_decoded_channels(pcm.channels).is_ok() {
            bank.insert(slot, pcm);
        }
    }
}

/// Wrap decoded PCM frames with the entry header's rate/channels.
///
/// Loading gate 4 (`0x54d460`): only mono/stereo streams enter the bank.
fn insert_pcm(bank: &mut HashMap<u32, PcmSamples>, slot: u32, entry: &OjmEntry, samples: Vec<i16>) {
    let Ok(channels) = u8::try_from(entry.num_channels) else {
        return;
    };
    if o2jam_file::validate_decoded_channels(channels).is_err() {
        return;
    }
    if entry.sample_rate > 0 && entry.num_channels > 0 && !samples.is_empty() {
        bank.insert(
            slot,
            PcmSamples {
                samples,
                sample_rate: entry.sample_rate,
                channels,
            },
        );
    }
}

/// Build the strike-slot -> decoded-PCM bank for one OJM image.
///
/// Shared by offline autoplay (`main.rs`) and online play
/// (`session.rs`): M30 entries key by their sample id (verified exe
/// behavior); id-less entries (old-variant OMC streams, new-variant
/// mode-2/mode-3 entries) key by fetch order via
/// [`o2jam_file::OjmTables::resolve_order_slot`] — the documented
/// reconstruction fallback (`docs/file-formats/ojm.md`).
/// `format_version` is the chart's `+8` float: it selects the
/// new-variant lane count ([`o2jam_file::ojm_descriptor_for_version`]);
/// OGG bodies (mode-3 splits) decode with Vorbis, encrypted bodies
/// with [`o2jam_file::decode_new_variant_pcm`] into the entry
/// header's rate/channels. Garbage decodes to an empty bank — silent,
/// never invented.
#[must_use]
pub fn load_sound_bank(bank_bytes: &[u8], format_version: f32) -> HashMap<u32, PcmSamples> {
    use o2jam_file::{OjmEntry, NEW_VARIANT_HIGH_BANK};
    let mut bank = HashMap::new();
    if bank_bytes.starts_with(b"M30") {
        if let Ok(samples) = o2jam_file::parse_m30(bank_bytes) {
            for sample in samples {
                if let Some(id) = sample.sample_id {
                    insert_ogg(&mut bank, id, sample.data);
                }
            }
        }
        return bank;
    }
    // New-variant first: its entry walk is bounds-checked, and each
    // body must decode (OGG checksum or PCM frames), so garbage
    // headers fall through to the old-variant walk below.
    if let Ok(tables) = o2jam_file::parse_new_variant_entries(bank_bytes) {
        let descriptor = o2jam_file::ojm_descriptor_for_version(format_version);
        let flat: Vec<&OjmEntry> = tables.mode2.iter().chain(tables.mode3.iter()).collect();
        let mut decoded_any = false;
        for (order, entry) in flat.iter().enumerate() {
            // Codec gate (`0x54d460`): mode-2 entries carry header
            // WAV params, so reject bad shapes before decode;
            // mode-3 splits carry none and gate post-decode instead.
            if order < tables.mode2.len() && check_bank_entry(entry).is_err() {
                continue;
            }
            let Ok(body) = entry.sample_bytes(bank_bytes) else {
                continue;
            };
            let slot = order as u32;
            if body.starts_with(b"OggS") {
                insert_ogg(&mut bank, slot, body);
            } else if let Ok(frames) = o2jam_file::decode_new_variant_pcm(body, descriptor) {
                insert_pcm(&mut bank, slot, entry, frames);
            }
            decoded_any = decoded_any || bank.contains_key(&slot);
        }
        if decoded_any {
            // High-bank aliases serve bit26-origin strikes, which name
            // the same fetch-order entries (see `resolve_order_slot`).
            let slots: Vec<u32> = bank.keys().copied().collect();
            for slot in slots {
                if let Some(pcm) = bank.get(&slot).cloned() {
                    bank.insert(NEW_VARIANT_HIGH_BANK + slot, pcm);
                }
            }
            return bank;
        }
    }
    if let Ok(samples) = o2jam_file::parse_omc(bank_bytes) {
        for (order, sample) in samples.iter().enumerate() {
            insert_ogg(&mut bank, order as u32, sample.data);
        }
    }
    bank
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
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn unknown_walk_stays_indeterminate() {
        assert_eq!(loading_fraction(0, 0), None);
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
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

    #[test]
    fn bank_decode_rejects_garbage() {
        assert!(load_sound_bank(b"garbage", 2.9).is_empty());
        assert!(load_sound_bank(&[], 2.9).is_empty());
        assert!(load_sound_bank(&[0xFFu8; 64], 2.9).is_empty());
    }

    #[test]
    fn null_backend_emits_banked_sample() {
        use o2jam_audio::{AudioBackend, NullBackend};
        let mut backend = NullBackend::new();
        let pcm = PcmSamples {
            samples: vec![1000i16; 100],
            sample_rate: 44100,
            channels: 2,
        };
        backend.play_pcm(&pcm).unwrap();
        assert_eq!(backend.samples_played(), 50);
    }

    /// RED proof for #581/#598: the exe's solo gate paints the cover
    /// before the confirm, sends 4012 at the 660.5 ms wall-trace span,
    /// and only enters Playing 232-396 ms after that send.
    #[test]
    fn solo_gate_pacing_holds_through_the_wall_trace_windows() {
        use std::time::{Duration, Instant};

        use super::{
            LoadingGateCue, LoadingGatePacing, LOADING_CONFIRM_AFTER_START,
            LOADING_COVER_VISIBLE_LATEST, LOADING_PLAYING_AFTER_CONFIRM,
        };

        let start = Instant::now();
        let mut gate = LoadingGatePacing::default();
        gate.enter_at(start);

        assert_eq!(
            gate.advance_at(start + LOADING_COVER_VISIBLE_LATEST),
            LoadingGateCue::None,
            "the cover is up before the confirm on the solo gate"
        );
        gate.arm_chart_at(start + LOADING_COVER_VISIBLE_LATEST);
        assert_eq!(
            gate.advance_at(start + LOADING_CONFIRM_AFTER_START - Duration::from_millis(1)),
            LoadingGateCue::None
        );
        assert_eq!(
            gate.advance_at(start + LOADING_CONFIRM_AFTER_START),
            LoadingGateCue::SendConfirm
        );
        let confirmed = start + LOADING_CONFIRM_AFTER_START;
        assert_eq!(
            gate.advance_at(confirmed + LOADING_PLAYING_AFTER_CONFIRM - Duration::from_millis(1)),
            LoadingGateCue::None
        );
        assert_eq!(
            gate.advance_at(confirmed + LOADING_PLAYING_AFTER_CONFIRM),
            LoadingGateCue::EnterPlaying
        );
        assert_eq!(
            gate.advance_at(confirmed + Duration::from_secs(1)),
            LoadingGateCue::None
        );
    }

    /// #727: a shell that refuses the first handoff still gets paint
    /// clocks. Only the Playing leg re-arms, so the 4012 stays single.
    #[test]
    fn refused_handoff_rearms_only_the_playing_leg() {
        use std::time::{Duration, Instant};

        use super::{
            LoadingGateCue, LoadingGatePacing, LOADING_CONFIRM_AFTER_START,
            LOADING_PLAYING_AFTER_CONFIRM,
        };

        let start = Instant::now();
        let mut gate = LoadingGatePacing::default();
        gate.enter_at(start);
        gate.arm_chart_at(start);
        let confirmed = start + LOADING_CONFIRM_AFTER_START;
        assert_eq!(gate.advance_at(confirmed), LoadingGateCue::SendConfirm);
        let handoff = confirmed + LOADING_PLAYING_AFTER_CONFIRM;
        assert_eq!(gate.advance_at(handoff), LoadingGateCue::EnterPlaying);
        assert!(!gate.is_active());

        gate.retry_playing_at(handoff);
        assert!(gate.is_active());
        assert_eq!(
            gate.advance_at(handoff + LOADING_PLAYING_AFTER_CONFIRM - Duration::from_millis(1)),
            LoadingGateCue::None
        );
        assert_eq!(
            gate.advance_at(handoff + LOADING_PLAYING_AFTER_CONFIRM),
            LoadingGateCue::EnterPlaying
        );
    }

    /// #608: the arm latch is observable, and an unarmed gate never
    /// posts 4012 even after the wall span has passed.
    #[test]
    fn chart_arm_state_walks_cover_ready_and_confirm() {
        use std::time::{Duration, Instant};

        use super::{LoadingGateArm, LoadingGateCue, LoadingGatePacing};

        let start = Instant::now();
        let mut gate = LoadingGatePacing::default();
        assert_eq!(gate.arm_state(), LoadingGateArm::Idle);

        gate.enter_at(start);
        assert_eq!(gate.arm_state(), LoadingGateArm::AwaitingChart);
        assert_eq!(
            gate.advance_at(start + Duration::from_secs(1)),
            LoadingGateCue::None,
            "a wall span never replaces the chart arm"
        );

        let armed_at = start + Duration::from_millis(50);
        gate.arm_chart_at(armed_at);
        assert_eq!(
            gate.arm_state(),
            LoadingGateArm::ChartArmed {
                entered_at: start,
                armed_at,
            }
        );

        let confirmed = start + Duration::from_millis(700);
        assert_eq!(gate.advance_at(confirmed), LoadingGateCue::SendConfirm);
        assert_eq!(
            gate.arm_state(),
            LoadingGateArm::ConfirmPosted { sent_at: confirmed }
        );
    }

    /// #598 wall-trace pin: two qualified solo boots bracket the gate
    /// entirely before 4012 creates the cover, then hand off inside a
    /// 232-396 ms window after the confirm. The guard fails if a pacing
    /// constant drifts off the cited observation.
    #[test]
    fn solo_gate_wall_trace_brackets_bound_the_pacing_spans() {
        use super::{
            LOADING_CONFIRM_AFTER_START, LOADING_COVER_VISIBLE_EARLIEST,
            LOADING_COVER_VISIBLE_LATEST, LOADING_PLAYING_AFTER_CONFIRM,
            LOADING_PLAYING_EARLIEST_AFTER_CONFIRM, LOADING_PLAYING_LATEST_AFTER_CONFIRM,
        };

        assert_eq!(LOADING_COVER_VISIBLE_EARLIEST.as_millis(), 198);
        assert_eq!(LOADING_COVER_VISIBLE_LATEST.as_millis(), 291);
        assert_eq!(LOADING_CONFIRM_AFTER_START.as_millis(), 660);
        assert_eq!(LOADING_PLAYING_EARLIEST_AFTER_CONFIRM.as_millis(), 232);
        assert_eq!(LOADING_PLAYING_LATEST_AFTER_CONFIRM.as_millis(), 396);
        assert!(
            LOADING_COVER_VISIBLE_LATEST < LOADING_CONFIRM_AFTER_START,
            "the cover paints before the confirm"
        );
        assert!(
            LOADING_PLAYING_EARLIEST_AFTER_CONFIRM <= LOADING_PLAYING_AFTER_CONFIRM
                && LOADING_PLAYING_AFTER_CONFIRM <= LOADING_PLAYING_LATEST_AFTER_CONFIRM,
            "the handoff stays inside the observed bracket"
        );
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn null_backend_silent_gain_leaves_the_clock() {
        use o2jam_audio::{AudioBackend, NullBackend};
        let mut backend = NullBackend::new();
        let pcm = PcmSamples {
            samples: vec![1000i16; 100],
            sample_rate: 44100,
            channels: 2,
        };
        backend.play_key(&pcm, 0.0).unwrap();
        assert_eq!(backend.samples_played(), 0);
        backend.play_key(&pcm, 1.0).unwrap();
        assert_eq!(backend.samples_played(), 50);
    }
}
