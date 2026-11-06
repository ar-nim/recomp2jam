//! File-side of the lifecycle §9 loading gate (7.55).
//!
//! Thin wrappers over the verified parsers, one per exe gate step:
//! select validation (`0x5718b0`, file `0x1718b0`, ev `66e60d16…`),
//! staging (`0x4f8b80`, file `0xf8b80`, ev `d2a69b45…`),
//! OJN open/decrypt (`0x412180`, file `0x12180`, ev `011eab73…`),
//! codec format check (part of `0x54db40`/`0x54d460`, ev `0ee1373d…`/
//! `3f154198…`). The loading-screen handoff (`0x4e47a0`) lives in the
//! CLI layer (`recomp2jam::loading`), which arms the song clock.
//! Behavior only; no disassembly reproduced.

use std::path::{Path, PathBuf};

use super::{FileError, OjmEntry, OjnChartFile, OjnLevelSlice, OjnListEntry, OJN_LEVEL_COUNT};

/// In-exe song-table stride walked by the loading screen
/// (`0x4e47a0` song vector stride `0x1b4`).
pub const SONG_TABLE_STRIDE: usize = 0x1b4;

/// Staging root the exe creates (`.\Music`).
pub const MUSIC_DIR_NAME: &str = "Music";

/// Staging temp dir the exe creates (`.\Music\TEMP`).
pub const MUSIC_TEMP_DIR_NAME: &str = "TEMP";

/// Validate a song select against the song table.
///
/// Mirrors `0x5718b0`: the candidate must name a real song entry
/// (id mask) and a real difficulty slot; anything else is rejected
/// before any file opens.
///
/// # Errors
///
/// `FileError::UnknownFormat` for song id `0` or no table match,
/// `FileError::Truncated` for a difficulty slot outside
/// `0..OJN_LEVEL_COUNT`.
pub fn validate_selection<'a>(
    song_id: u32,
    level: usize,
    catalog: &'a [OjnListEntry<'a>],
) -> Result<&'a OjnListEntry<'a>, FileError> {
    if level >= OJN_LEVEL_COUNT {
        return Err(FileError::Truncated {
            need: level,
            have: OJN_LEVEL_COUNT,
        });
    }
    if song_id == 0 {
        return Err(FileError::UnknownFormat {
            magic: song_id.to_le_bytes().to_vec(),
        });
    }
    catalog
        .iter()
        .find(|entry| entry.song_id == song_id && entry.is_song())
        .ok_or_else(|| FileError::UnknownFormat {
            magic: song_id.to_le_bytes().to_vec(),
        })
}

/// Build the staged extract name for a selected song.
///
/// Mirrors `0x4f8b80` formatting `MUSIC\Temp\%s%d_%d`: the `.ojm`
/// stem plus song id plus difficulty slot, under the temp dir.
#[must_use]
pub fn stage_extract_name(ojm_file: &str, song_id: u32, level: usize) -> String {
    let stem = ojm_file.rsplit_once('.').map_or(ojm_file, |(stem, _)| stem);
    format!("{MUSIC_DIR_NAME}\\Temp\\{stem}{song_id}_{level}")
}

/// Staging directories the exe ensures exist (`.\Music` +
/// `.\Music\TEMP`, `0x4f8b80`). Pure paths; creation stays caller-side.
#[must_use]
pub fn stage_dirs(base: &Path) -> (PathBuf, PathBuf) {
    let music = base.join(MUSIC_DIR_NAME);
    let temp = music.join(MUSIC_TEMP_DIR_NAME);
    (music, temp)
}

/// Open an `.ojn` chart buffer: decrypt plus header plus slice walk.
///
/// Mirrors `0x412180` with its guarded wrappers (`0x412890` /
/// `0x412a50`): the `parse_ojn` header read must agree with the
/// `parse_ojn_levels` slice walk or the open fails.
///
/// # Errors
///
/// Whatever `parse_ojn` / `parse_ojn_levels` report on bad input.
pub fn open_chart(bytes: &[u8]) -> Result<(OjnChartFile, Vec<OjnLevelSlice>), FileError> {
    let header = super::parse_ojn(bytes)?;
    let slices = super::parse_ojn_levels(bytes)?;
    Ok((header, slices))
}

/// Check one bank entry's sample format before decode.
///
/// Mirrors the `0x54d460` gate: only 8/16-bit mono/stereo streams
/// enter the decoder (`DISTRIBUTOR`-tagged path).
///
/// # Errors
///
/// `FileError::UnknownFormat` (entry bytes as context) when the
/// channel count or bit depth falls outside the exe's `1/2` and
/// `8/16` checks.
pub fn validate_sample_format(entry: &OjmEntry) -> Result<(), FileError> {
    if entry.num_channels != 1 && entry.num_channels != 2 {
        return Err(FileError::UnknownFormat {
            magic: entry.name.bytes().take(4).collect(),
        });
    }
    if entry.bits_per_sample != 8 && entry.bits_per_sample != 16 {
        return Err(FileError::UnknownFormat {
            magic: entry.name.bytes().take(4).collect(),
        });
    }
    Ok(())
}

/// Check decoded PCM channel count before it enters the bank.
///
/// Mirrors the `0x54d460` post-decode gate: only mono/stereo
/// streams play. Mode-3 splits and OGG bodies carry no header
/// params, so they gate here instead of at [`validate_sample_format`].
///
/// # Errors
///
/// `FileError::UnknownFormat` when `channels` is not 1 or 2.
pub fn validate_decoded_channels(channels: u8) -> Result<(), FileError> {
    if channels == 1 || channels == 2 {
        Ok(())
    } else {
        Err(FileError::UnknownFormat {
            magic: vec![channels],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Leak-free: parse from a leaked blob so entries borrow static.
        let owned: &'static [u8] = Box::leak(catalog_blob().into_boxed_slice());
        super::super::parse_ojn_list(owned).unwrap()
    }

    #[test]
    fn validation_accepts_a_known_song_and_slot() {
        let entries = catalog();
        let entry = validate_selection(757, 2, &entries).unwrap();
        assert_eq!(entry.song_id, 757);
    }

    #[test]
    fn validation_rejects_a_bad_difficulty_slot() {
        let entries = catalog();
        assert!(matches!(
            validate_selection(757, 3, &entries),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn validation_rejects_an_unknown_song_id() {
        let entries = catalog();
        assert!(matches!(
            validate_selection(9999, 0, &entries),
            Err(FileError::UnknownFormat { .. })
        ));
        assert!(matches!(
            validate_selection(0, 0, &entries),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn staging_formats_the_temp_extract_name() {
        assert_eq!(
            stage_extract_name("o2ma757.ojm", 757, 2),
            "Music\\Temp\\o2ma757757_2"
        );
    }

    #[test]
    fn staging_dirs_nest_temp_under_music() {
        let (music, temp) = stage_dirs(Path::new("base"));
        assert_eq!(music, Path::new("base").join("Music"));
        assert_eq!(temp, music.join("TEMP"));
    }

    #[test]
    fn open_parses_a_minimal_old_format_chart() {
        let mut blob = vec![0u8; 300];
        blob[0..4].copy_from_slice(&8888u32.to_le_bytes());
        blob[4..8].copy_from_slice(b"ojn\0");
        blob[8..12].copy_from_slice(&2.7f32.to_le_bytes());
        blob[64..68].copy_from_slice(&0u32.to_le_bytes());
        blob[68..72].copy_from_slice(&0u32.to_le_bytes());
        blob[72..76].copy_from_slice(&0u32.to_le_bytes());
        blob[284..288].copy_from_slice(&300u32.to_le_bytes());
        blob[288..292].copy_from_slice(&300u32.to_le_bytes());
        blob[292..296].copy_from_slice(&300u32.to_le_bytes());
        blob[296..300].copy_from_slice(&300u32.to_le_bytes());
        let (header, slices) = open_chart(&blob).unwrap();
        assert_eq!(header.song_id, 8888);
        assert_eq!(slices.len(), OJN_LEVEL_COUNT);
    }

    #[test]
    fn decoded_channels_accept_mono_and_stereo() {
        assert!(validate_decoded_channels(1).is_ok());
        assert!(validate_decoded_channels(2).is_ok());
        assert!(validate_decoded_channels(6).is_err());
    }

    #[test]
    fn codec_check_accepts_shipped_pcm_shapes() {
        let entry = OjmEntry {
            name: "s1".to_owned(),
            size: 8,
            data_offset: 0,
            audio_format: 1,
            num_channels: 2,
            sample_rate: 44100,
            bit_rate: 176400,
            block_align: 4,
            bits_per_sample: 16,
        };
        assert!(validate_sample_format(&entry).is_ok());
    }

    #[test]
    fn codec_check_rejects_surround_and_float_depths() {
        let base = OjmEntry {
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
        let surround = OjmEntry {
            num_channels: 6,
            ..base.clone()
        };
        assert!(validate_sample_format(&surround).is_err());
        let depth = OjmEntry {
            bits_per_sample: 24,
            ..base
        };
        assert!(validate_sample_format(&depth).is_err());
    }
}
