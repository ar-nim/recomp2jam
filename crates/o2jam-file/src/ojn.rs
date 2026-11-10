//! OJN charts: headers, level slices, note events, lanes, and the
//! new-format decrypt.
//!
//! Verified on real 7.55 files (docs/file-formats/ojn.md).

use super::ojm::{parse_m30, parse_omc, OjmSample};
use super::{
    detect_kind, read_cp949_str, read_f32le, read_str, read_u16le, read_u32le, AssetKind, FileError,
};
use crate::jpeg::JPEG_SOI;

/// Minimal parsed view of an `.ojn` chart file.
///
/// Full 300-byte header map, verified on real 7.55 data: the old-format
/// `Tutorial.ojn` (song 8888) and every decrypted new-format chart share
/// one layout, which is the `OJNList.dat` 300-byte record minus its
/// leading `u32` (all record offsets shift by 4; the list's third slice
/// end has no slot, while the header carries all three at
/// +288/+292/+296). Per-level event slices start at +284 and are
/// validated by [`parse_ojn_levels`]; playable/total/last-measure
/// triplets cross-check against the walked events
/// (docs/file-formats/ojn.md).
#[derive(Debug, Clone, PartialEq)]
pub struct OjnChartFile {
    /// Raw magic tag seen (`ojn`).
    pub magic: [u8; 4],
    /// Song id (`u32` at +0; 8888 on Tutorial.ojn).
    pub song_id: u32,
    /// Container version (`f32` at +8; 2.7 on Tutorial.ojn, 2.9 on
    /// decrypted new-format charts). The loader derives its OJM
    /// descriptor from this float instead of the file magic, so it
    /// also selects the new-variant lane count
    /// ([`ojm_descriptor_for_version`]).
    pub format_version: f32,
    /// Category id (`u32` at +12; 10 on Tutorial.ojn, single bits
    /// 1/2/4/8 on the shipped new-format songs, Tutorial 10 = 8|2).
    /// Chart path ignores it: the validator (`FUN_005718b0`, file
    /// `0x1718b0`) checks id/magic/version/last-measure/counts only,
    /// and the parser (`FUN_00571d60`, file `0x171d60`) reads slice
    /// tables/counts/BPM only. Song-select tests a song bitmask at
    /// `song+0x15c` (`and $0x7` at file `0xbe79f`, `and $0x8` at file
    /// `0x36f4f`), but the header+12 to song-slot transfer is unpinned,
    /// so the meaning stays OPEN (bitmask-shaped, unproven).
    pub genre: u32,
    /// Tempo (`f32` at +16; reads 100.0/124.0 on Tutorial/757).
    pub bpm: f32,
    /// Level-shaped flags (`u32` at +20; 1 on Tutorial.ojn, packed
    /// `(hi, lo)` halves on new songs with lo 1..5, hi 4..10).
    /// Difficulty mapping EXE-PROVEN (`AngelO2Jam.exe` 7.55, sha256
    /// `83a04541…`), no longer a positional guess: the select-entry
    /// builder reads `u16[difficulty]` over the `OJNList.dat` row
    /// words `{+24 lo, +26 hi, +28}` into `entry+0x212`, the STEP1
    /// band input (file `0x44268`, ev `b5c3094a…`). So difficulty
    /// 0 = low half (Easy), 1 = high half (Normal), 2 =
    /// [`OjnChartFile::stable_minor`] (Hard) — see
    /// [`OjnChartFile::level_for_difficulty`]. Fits 13/13 songs plus
    /// the Tutorial edge (0, 1, 0 = only Easy present); `OJNList.dat`
    /// record `@24` matches the disk header 13/13.
    pub level_flags: u32,
    /// Small stable id (`u32` at +24; 0 on Tutorial.ojn, 5..19 on the
    /// shipped new-format songs). Hard-level slot, EXE-PROVEN: it is
    /// index 2 of the same `u16[difficulty]` read that maps
    /// [`OjnChartFile::level_flags`] to Easy/Normal (file `0x44268`,
    /// ev `b5c3094a…`), so +24 completes the Easy/Normal/Hard triple
    /// (C35). 13/13 ascending; `OJNList.dat` record `@28` matches the
    /// disk header 13/13. The exe reads only the low `u16` here;
    /// observed values (0..50) fit.
    pub stable_minor: u32,
    /// Per-difficulty note totals (`u32` at +28/+32/+36): nonzero
    /// payloads on kinds 2+ (playable lanes plus autoplay). Reads
    /// 51/0/0 on Tutorial.ojn (level 0 walks 51 nonzero values).
    pub total_counts: [u32; OJN_LEVEL_COUNT],
    /// Per-difficulty playable counts (`u32` at +40/+44/+48):
    /// nonzero payloads on kinds 2..=8. Reads 38/0/0 on
    /// Tutorial.ojn (level 0 walks 38 lane notes).
    pub playable_counts: [u32; OJN_LEVEL_COUNT],
    /// Per-difficulty last measure index (`u32` at +52/+56/+60):
    /// the greatest event `measure` in the slice (102/0/0 on
    /// Tutorial.ojn; empty levels read 0 with no events).
    pub last_measures: [u32; OJN_LEVEL_COUNT],
    /// Per-difficulty event counts (`u32` at +64/+68/+72): the
    /// walked `8 + count * 4` event totals. Reads 43/0/0 on
    /// Tutorial.ojn (the 43-event walk lands exactly on the JPEG).
    pub event_counts: [u32; OJN_LEVEL_COUNT],
    /// Composite key (`u32` at +76): high `u16` tracks the song id
    /// (8888 on Tutorial.ojn, exact on all 13 new-format songs),
    /// low `u16` is 27 on old-format and 29 on new-format; TBD.
    pub key_field: u32,
    /// Spare words (`u32` at +80/+84/+88/+92; zero on every 7.55
    /// chart).
    pub pad: [u32; 4],
    /// Flag word (`u32` at +96; `0` on Tutorial.ojn and 12/13
    /// new-format songs, `0x01000000` on song 770).
    pub flag96: u32,
    /// Constant (`u32` at +100; 0 on Tutorial.ojn, 19256 on every
    /// new-format song). No `19256`/`0x4B38` immediate exists in `.text`
    /// (byte scan; the lone hit at file `0xd2094` is a `call` rel32
    /// coincidence); validator and parser never read +100. Stays OPEN.
    pub const100: u32,
    /// Small tuning float (`f32` at +104; 0.0 on Tutorial.ojn, one
    /// of 0.0/0.01/0.02/0.03 on new-format songs).
    pub tune: f32,
    /// Title area (+108, 64 B, NUL-trimmed, lossy: CP949 Korean on
    /// Tutorial.ojn, e.g. `NoteFactory` songs carry ASCII here).
    pub title: String,
    /// Artist area (+172, 32 B, NUL-trimmed, lossy).
    pub artist: String,
    /// Label area (+204, 32 B, e.g. `nao.paradigm` on Tutorial.ojn,
    /// `NoteFactory` on song 757).
    pub label: String,
    /// Container filename (+236, 32 B, e.g. `Tutorial.ojm`).
    pub ojm_file: String,
    /// Trailing word (`u32` at +268; 106954 on Tutorial.ojn; TBD).
    pub unknown1: u32,
    /// Per-difficulty durations (`u32` at +272/+276/+280,
    /// seconds-shaped; 247/0/0 on Tutorial.ojn, ~measures*4*60/BPM).
    pub durations: [u32; OJN_LEVEL_COUNT],
    /// First slice start (`u32` at +284; 300 on every 7.55 chart).
    pub slice_start: u32,
    /// Per-level slice end offsets (`u32` at +288/+292/+296; all
    /// three read 11784 on Tutorial.ojn — empty trailing slices have
    /// `start == end` — and bracket the event data on new-format
    /// songs, the last end landing on the first embedded JPEG).
    pub slice_ends: [u32; OJN_LEVEL_COUNT],
    /// Level-0 event count (`u32` at +64): kept as a named alias of
    /// `event_counts[0]` for callers that only open one difficulty.
    pub event_count_hint: u32,
}

impl OjnChartFile {
    /// Chart level per difficulty slot: 0 = Easy (+20 low word),
    /// 1 = Normal (+20 high word), 2 = Hard (+24, low `u16` like the
    /// exe). `None` for out-of-range slots (7.55 charts carry
    /// [`OJN_LEVEL_COUNT`] slices).
    ///
    /// Order EXE-PROVEN (`AngelO2Jam.exe` 7.55): the entry builder
    /// indexes exactly these three words by the chosen difficulty
    /// (file `0x44268`, ev `b5c3094a…`), and the same difficulty byte
    /// (global `0x7789e7`, range-checked 0..2 at file `0x171d91`) is
    /// stored to the parser's slice index at file `0x171c18`, so it
    /// selects the per-difficulty header triplets — slice `i` =
    /// difficulty `i` (docs/file-formats/ojn.md, C34/C35). The slot
    /// *names* (Easy/Normal/Hard) stay label-level: the exe pins the
    /// ascending order via the per-difficulty life sets (file
    /// `0x170fa2`, ev `feebe490…`), not strings.
    #[must_use]
    pub fn level_for_difficulty(&self, difficulty: usize) -> Option<u32> {
        match difficulty {
            0 => Some(self.level_flags & 0xFFFF),
            1 => Some(self.level_flags >> 16),
            // The exe reads the low `u16` of +24 here (file `0x44268`).
            2 => Some(self.stable_minor & 0xFFFF),
            _ => None,
        }
    }

    /// High word of [`OjnChartFile::key_field`] (tracks the song id).
    #[must_use]
    pub fn key_hi(&self) -> u16 {
        (self.key_field >> 16) as u16
    }

    /// Low word of [`OjnChartFile::key_field`] (27 old / 29 new).
    #[must_use]
    pub fn key_lo(&self) -> u16 {
        (self.key_field & 0xFFFF) as u16
    }
}

/// Parse an `.ojm` container into its sample streams.
///
/// Dispatches on magic: `M30` and `OMC` variants are fully implemented;
/// the `OJM` variant stays [`FileError::NotImplemented`] until its
/// encrypted region is understood (docs/file-formats/ojm.md).
///
/// # Errors
///
/// `FileError::UnknownFormat` when no known container matches,
/// `FileError::NotImplemented` for recognized-but-unhandled variants.
pub fn parse_ojm(bytes: &[u8]) -> Result<Vec<OjmSample<'_>>, FileError> {
    match detect_kind(bytes)? {
        AssetKind::Ojm if bytes.starts_with(b"M30") => parse_m30(bytes),
        AssetKind::Ojm if bytes.starts_with(b"OMC") || bytes.starts_with(b"OJM") => {
            parse_omc(bytes)
        }
        AssetKind::Ojm => Err(FileError::NotImplemented(
            "M32 variant unobserved in 7.55 data",
        )),
        AssetKind::OjnList | AssetKind::Ojn => Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        }),
    }
}

/// One 300-byte song entry in `OJNList.dat`.
///
/// Full field map verified on real 7.55 data: every record (including
/// record 0, which is song 100, not a header) carries the same layout,
/// and numeric fields align 1:1 with the decrypted `.ojn` chart header
/// for the same song, shifted by 4 bytes (`OJNList.dat` has a leading
/// `u32`; the `.ojn` header is the same 296 bytes without it, plus its
/// own final slice end). Fields that match the on-disk `.ojn` files
/// exactly are version-stable; the rest drifted between the index
/// snapshot and the shipped charts (docs/file-formats/ojn.md).
/// Offsets are in-record.
#[derive(Debug, Clone, PartialEq)]
pub struct OjnListEntry<'a> {
    /// Record index in the file (0-based; 7.55 holds 719 records).
    pub index: usize,
    /// Leading word (`u32` at +0; semantics TBD: range 562..2914704
    /// over the 7.55 songs; not a file size, duration, or slice end).
    pub unknown0: u32,
    /// Song id (`u32` at +4; matches the `o2ma###` file number).
    pub song_id: u32,
    /// Tempo (`f32` at +20; reads 124.0/168.0/103.0 on songs
    /// 757/770/860, matching the decrypted `.ojn` 13/13).
    pub bpm: f32,
    /// Format marker (`f32` at +12; 2.9 on 557/562 songs, 2.0 on 5).
    pub format_version: f32,
    /// Category id (`u32` at +16; range 0..10; differs from the
    /// on-disk `.ojn` on 6/13 songs, so it was re-categorised
    /// between versions).
    pub genre: u32,
    /// Level-range-shaped flags (`u32` at +24; both `u16` halves range
    /// 1..38; matches the `.ojn` 13/13).
    pub level_flags: u32,
    /// Small stable id (`u32` at +28, range 1..50; matches the `.ojn`
    /// 13/13; semantics TBD).
    pub stable_minor: u32,
    /// Per-difficulty counts (`u32` at +32/+36/+40; drifted from the
    /// on-disk `.ojn` values on 13/13; semantics TBD).
    pub counts_a: [u32; OJN_LEVEL_COUNT],
    /// Per-difficulty counts (`u32` at +44/+48/+52; match the
    /// decrypted `.ojn` +40/+44/+48 on 13/13, so version-stable;
    /// not events, payload units, samples, or sample ids — TBD).
    pub counts_b: [u32; OJN_LEVEL_COUNT],
    /// Per-difficulty measure counts (`u32` at +56/+60/+64;
    /// positionally the `.ojn` max-tick, walk-verified there as the
    /// last measure index; values drifted between versions).
    pub measures: [u32; OJN_LEVEL_COUNT],
    /// Per-difficulty event counts (`u32` at +68/+72/+76;
    /// positionally the `.ojn` per-level event counts; values
    /// drifted between versions).
    pub events: [u32; OJN_LEVEL_COUNT],
    /// Composite key (`u32` at +80): high `u16` tracks the song id
    /// (exact on all 13 on-disk songs, else within ±13), low `u16`
    /// is 29 on new-format songs and 27 on Tutorial.ojn; TBD.
    pub key_field: u32,
    /// Spare words (+84/+88/+92/+96; zero on most songs).
    pub pad: [u32; 4],
    /// Flag word (`u32` at +100; `0` or `0x01000000`).
    pub flag100: u32,
    /// Constant (`u32` at +104; 19256 on all but two songs).
    pub const_a: u32,
    /// Small tuning float (`f32` at +108; one of 0.0/0.01/0.02/0.03).
    pub tune: f32,
    /// Title area (+112, 64 B, NUL-trimmed, lossy: Korean + tags).
    pub title: String,
    /// Artist area (+176, 32 B, NUL-trimmed, lossy).
    pub artist: String,
    /// Label area (+208, 32 B, e.g. `NoteFactory`; 43/562 songs keep
    /// stale tail bytes past the first NUL).
    pub label: String,
    /// Container filename (+240, 32 B, e.g. `o2ma757.ojm`; one song
    /// ships uppercase; 148/562 keep stale tail bytes past the NUL).
    pub ojm_file: String,
    /// Trailing word (`u32` at +272; range 0..483339; TBD).
    pub unknown1: u32,
    /// Per-difficulty durations (`u32` at +276/+280/+284, seconds-
    /// shaped, range 0..315; roughly measures*4*60/BPM).
    pub durations: [u32; OJN_LEVEL_COUNT],
    /// First slice start (`u32` at +288; 300, like `.ojn` +284).
    pub slice_start: u32,
    /// First two per-level slice ends (`u32` at +292/+296; drifted
    /// from the on-disk `.ojn`; the third end has no slot).
    pub slice_ends: [u32; 2],
    /// Raw record bytes for re-analysis.
    pub raw: &'a [u8],
}

impl OjnListEntry<'_> {
    /// High word of [`OjnListEntry::key_field`] (tracks the song id).
    #[must_use]
    pub fn key_hi(&self) -> u16 {
        (self.key_field >> 16) as u16
    }

    /// Low word of [`OjnListEntry::key_field`] (29 / 27).
    #[must_use]
    pub fn key_lo(&self) -> u16 {
        (self.key_field & 0xFFFF) as u16
    }

    /// Whether this record looks like a real song entry rather than a
    /// dead slot: the filename is an `o2ma###.ojm` token (any ASCII
    /// case; `read_str` already cut it at the first NUL) and the tempo
    /// is sane. Dead slots (157/719 in 7.55) hold heap residue or
    /// zeros and fail one or both checks.
    #[must_use]
    pub fn is_song(&self) -> bool {
        let lower = self.ojm_file.to_ascii_lowercase();
        let numbered = lower
            .strip_prefix("o2ma")
            .and_then(|rest| rest.strip_suffix(".ojm"))
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
        numbered && (30.0..300.0).contains(&self.bpm)
    }
}

/// Parse an `OJNList.dat` song index: consecutive 300-byte records
/// (record 0 is song 100, not a header). A trailing partial record
/// (148 bytes in 7.55) is ignored. Dead slots past the song table
/// parse like any record; filter them with [`OjnListEntry::is_song`].
///
/// # Errors
///
/// `FileError::UnknownFormat` when the header magic mismatches.
pub fn parse_ojn_list(bytes: &[u8]) -> Result<Vec<OjnListEntry<'_>>, FileError> {
    if bytes.len() < 300 || bytes[8..11] != *b"ojn" {
        return Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        });
    }
    let mut out = Vec::new();
    let mut index = 0usize;
    let mut off = 0usize;
    while off + 300 <= bytes.len() {
        let rec = &bytes[off..off + 300];
        out.push(OjnListEntry {
            index,
            unknown0: read_u32le(bytes, off)?,
            song_id: read_u32le(bytes, off + 4)?,
            bpm: read_f32le(bytes, off + 20)?,
            format_version: read_f32le(bytes, off + 12)?,
            genre: read_u32le(bytes, off + 16)?,
            level_flags: read_u32le(bytes, off + 24)?,
            stable_minor: read_u32le(bytes, off + 28)?,
            counts_a: [
                read_u32le(bytes, off + 32)?,
                read_u32le(bytes, off + 36)?,
                read_u32le(bytes, off + 40)?,
            ],
            counts_b: [
                read_u32le(bytes, off + 44)?,
                read_u32le(bytes, off + 48)?,
                read_u32le(bytes, off + 52)?,
            ],
            measures: [
                read_u32le(bytes, off + 56)?,
                read_u32le(bytes, off + 60)?,
                read_u32le(bytes, off + 64)?,
            ],
            events: [
                read_u32le(bytes, off + 68)?,
                read_u32le(bytes, off + 72)?,
                read_u32le(bytes, off + 76)?,
            ],
            key_field: read_u32le(bytes, off + 80)?,
            pad: [
                read_u32le(bytes, off + 84)?,
                read_u32le(bytes, off + 88)?,
                read_u32le(bytes, off + 92)?,
                read_u32le(bytes, off + 96)?,
            ],
            flag100: read_u32le(bytes, off + 100)?,
            const_a: read_u32le(bytes, off + 104)?,
            tune: read_f32le(bytes, off + 108)?,
            title: read_cp949_str(bytes, off + 112, 64)?,
            artist: read_cp949_str(bytes, off + 176, 32)?,
            label: read_cp949_str(bytes, off + 208, 32)?,
            ojm_file: read_str(bytes, off + 240, 32)?,
            unknown1: read_u32le(bytes, off + 272)?,
            durations: [
                read_u32le(bytes, off + 276)?,
                read_u32le(bytes, off + 280)?,
                read_u32le(bytes, off + 284)?,
            ],
            slice_start: read_u32le(bytes, off + 288)?,
            slice_ends: [read_u32le(bytes, off + 292)?, read_u32le(bytes, off + 296)?],
            raw: rec,
        });
        index += 1;
        off += 300;
    }
    Ok(out)
}

/// Decrypt a new-format (`"new"` magic) `.ojn` file in place.
///
/// Verified against the in-exe loader (`FUN_00412180`, dossier
/// `ev_011eab73…`): the file is stored byte-reversed, then XORed with
/// three key bytes on a `period` stride, with keys and period taken
/// from the reversed tail (`keys` at `len-7..len-4`, period byte at
/// `len-4`). Old-format files (no `"new"` prefix) are returned
/// unchanged.
///
/// # Errors
///
/// `FileError::TooShort` on tiny input, `FileError::UnknownFormat`
/// without the `"new"` prefix.
pub fn decrypt_ojn_new_format(bytes: &[u8]) -> Result<Vec<u8>, FileError> {
    if bytes.len() < 8 {
        return Err(FileError::TooShort(bytes.len()));
    }
    if !bytes.starts_with(b"new") {
        return Ok(bytes.to_vec());
    }
    let n = bytes.len();
    let mut buf: Vec<u8> = bytes.iter().rev().copied().collect();
    let period = buf[n - 4];
    if period == 0 {
        return Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        });
    }
    let (k1, k2, k3) = (buf[n - 7], buf[n - 6], buf[n - 5]);
    let p = period as usize;
    for (i, b) in buf.iter_mut().enumerate() {
        if i.is_multiple_of(p) {
            *b ^= k1;
        } else if i % p == p / 2 {
            *b ^= k2;
        } else {
            *b ^= k3;
        }
    }
    Ok(buf)
}

/// Parse an `.ojn` chart file: decrypts new-format containers, then
/// reads the full 300-byte header map (old-format Tutorial.ojn and
/// decrypted new-format charts share one layout; per-level slices are
/// walked by [`parse_ojn_levels`] and events by
/// [`parse_ojn_level_events`], so an old-format song opens exactly
/// like a new-format one).
///
/// # Errors
///
/// `FileError::TooShort` when the header overruns, `FileError::UnknownFormat`
/// on a bad magic.
pub fn parse_ojn(bytes: &[u8]) -> Result<OjnChartFile, FileError> {
    let plain = decrypt_ojn_new_format(bytes)?;
    match detect_kind(&plain)? {
        AssetKind::Ojn => {
            if plain.len() < OJN_HEADER_LEN {
                return Err(FileError::TooShort(plain.len()));
            }
            let event_counts = [
                read_u32le(&plain, OJN_EVENT_COUNT_OFFS[0])?,
                read_u32le(&plain, OJN_EVENT_COUNT_OFFS[1])?,
                read_u32le(&plain, OJN_EVENT_COUNT_OFFS[2])?,
            ];
            Ok(OjnChartFile {
                magic: ojn_magic_tag(&plain)?,
                song_id: read_u32le(&plain, 0)?,
                format_version: read_f32le(&plain, 8)?,
                genre: read_u32le(&plain, OJN_GENRE_OFF)?,
                bpm: read_f32le(&plain, OJN_BPM_OFF)?,
                level_flags: read_u32le(&plain, OJN_LEVEL_FLAGS_OFF)?,
                stable_minor: read_u32le(&plain, OJN_STABLE_MINOR_OFF)?,
                total_counts: [
                    read_u32le(&plain, 28)?,
                    read_u32le(&plain, 32)?,
                    read_u32le(&plain, 36)?,
                ],
                playable_counts: [
                    read_u32le(&plain, 40)?,
                    read_u32le(&plain, 44)?,
                    read_u32le(&plain, 48)?,
                ],
                last_measures: [
                    read_u32le(&plain, 52)?,
                    read_u32le(&plain, 56)?,
                    read_u32le(&plain, 60)?,
                ],
                event_counts,
                key_field: read_u32le(&plain, 76)?,
                pad: [
                    read_u32le(&plain, 80)?,
                    read_u32le(&plain, 84)?,
                    read_u32le(&plain, 88)?,
                    read_u32le(&plain, 92)?,
                ],
                flag96: read_u32le(&plain, OJN_FLAG96_OFF)?,
                const100: read_u32le(&plain, OJN_CONST100_OFF)?,
                tune: read_f32le(&plain, OJN_TUNE_OFF)?,
                title: read_cp949_str(&plain, 108, 64)?,
                artist: read_cp949_str(&plain, 172, 32)?,
                label: read_cp949_str(&plain, 204, 32)?,
                ojm_file: read_str(&plain, 236, 32)?,
                unknown1: read_u32le(&plain, 268)?,
                durations: [
                    read_u32le(&plain, 272)?,
                    read_u32le(&plain, 276)?,
                    read_u32le(&plain, 280)?,
                ],
                slice_start: read_u32le(&plain, OJN_SLICES_START_OFF)?,
                slice_ends: [
                    read_u32le(&plain, OJN_SLICE_END_OFFS[0])?,
                    read_u32le(&plain, OJN_SLICE_END_OFFS[1])?,
                    read_u32le(&plain, OJN_SLICE_END_OFFS[2])?,
                ],
                event_count_hint: event_counts[0],
            })
        }
        AssetKind::Ojm | AssetKind::OjnList => Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        }),
    }
}

/// Number of difficulty slices per `.ojn` chart. Every 7.55 chart
/// carries three concatenated level slices (verified on all 13
/// new-format files plus the old-format Tutorial.ojn; empty trailing
/// slices have `data_start == data_end` and zero events).
/// See `docs/file-formats/ojn.md`.
pub const OJN_LEVEL_COUNT: usize = 3;

/// Shared `.ojn` header length: slice tables live inside it and level
/// data starts right after it (`u32` at +284 reads 300 on every file).
pub const OJN_HEADER_LEN: usize = 300;

/// Header offsets of the per-level event counts (`u32` each).
pub const OJN_EVENT_COUNT_OFFS: [usize; OJN_LEVEL_COUNT] = [64, 68, 72];

/// Header offsets of the per-level slice end offsets (`u32` each).
pub const OJN_SLICE_END_OFFS: [usize; OJN_LEVEL_COUNT] = [288, 292, 296];

/// Header offset of the first slice's start (`u32`).
pub const OJN_SLICES_START_OFF: usize = 284;

/// Header offset of the category id (`u32` at +12). See
/// [`OjnChartFile::genre`]: bitmask-shaped, consumer-open.
pub const OJN_GENRE_OFF: usize = 12;

/// Header offset of the tempo (`f32` at +16).
pub const OJN_BPM_OFF: usize = 16;

/// Header offset of the packed level flags (`u32` at +20). See
/// [`OjnChartFile::level_flags`]: Easy = low half, Normal = high
/// half (exe-pinned, file `0x44268`).
pub const OJN_LEVEL_FLAGS_OFF: usize = 20;

/// Header offset of the small stable id (`u32` at +24). See
/// [`OjnChartFile::stable_minor`]: Hard level, exe-pinned slot 2 of
/// the difficulty-indexed read (file `0x44268`).
pub const OJN_STABLE_MINOR_OFF: usize = 24;

/// Header offset of the flag word (`u32` at +96).
pub const OJN_FLAG96_OFF: usize = 96;

/// Header offset of the new-format constant (`u32` at +100, 19256).
/// See [`OjnChartFile::const100`]: no consumer, open.
pub const OJN_CONST100_OFF: usize = 100;

/// Header offset of the small tuning float (`f32` at +104).
pub const OJN_TUNE_OFF: usize = 104;

/// Raw magic tag seen on an `.ojn` image: the old-format tag lives at
/// file offset 4 (right after the song id: Tutorial.ojn opens
/// `B8 22 00 00 6F 6A 6E 00`), while prefix-form synthetics carry it at
/// 0. Reports whichever tag the sniff matched so real headers decode
/// to `ojn` instead of the song-id bytes.
fn ojn_magic_tag(plain: &[u8]) -> Result<[u8; 4], FileError> {
    let at = if plain.len() >= 8 && plain[4..7] == *b"ojn" {
        4
    } else {
        0
    };
    plain[at..at + 4]
        .try_into()
        .map_err(|_| FileError::Truncated {
            need: at + 4,
            have: plain.len(),
        })
}

/// Byte length of one event head (`u32` tick + `u16` kind + `u16` count).
const OJN_EVENT_HEAD_LEN: usize = 8;

/// Byte length of one event payload unit (one `u32` sample/timing value).
const OJN_EVENT_UNIT_LEN: usize = 4;

/// One difficulty slice of an `.ojn` chart: the event range for a single
/// level. Header tables locate each slice; the constructor walks the
/// `8 + count * 4` event framing and only succeeds when the walk lands
/// exactly on `data_end` with `event_count` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnLevelSlice {
    /// Difficulty slot 0/1/2 = Easy/Normal/Hard: the exe indexes the
    /// per-difficulty header triplets with the chosen difficulty
    /// (stored to its slice index at file `0x171c18`), so slice `i`
    /// belongs to difficulty `i` (docs/file-formats/ojn.md, C34/C35).
    pub level: usize,
    /// Event count from the header table (+64/+68/+72).
    pub event_count: u32,
    /// Byte offset where this level's events start.
    pub data_start: usize,
    /// Byte offset where this level's events end (next slice or images).
    pub data_end: usize,
}

fn walk_level_events(
    plain: &[u8],
    start: usize,
    end: usize,
    event_count: u32,
) -> Result<(), FileError> {
    let mut off = start;
    let mut walked = 0u32;
    while walked < event_count {
        let head_end = off
            .checked_add(OJN_EVENT_HEAD_LEN)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: plain.len(),
            })?;
        if head_end > end {
            return Err(FileError::Truncated {
                need: head_end,
                have: end,
            });
        }
        let units = u32::from(read_u16le(plain, off + 6)?) as usize;
        let payload = units
            .checked_mul(OJN_EVENT_UNIT_LEN)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: plain.len(),
            })?;
        let step = OJN_EVENT_HEAD_LEN
            .checked_add(payload)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: plain.len(),
            })?;
        let next = off.checked_add(step).ok_or(FileError::Truncated {
            need: usize::MAX,
            have: plain.len(),
        })?;
        if next > end {
            return Err(FileError::Truncated {
                need: next,
                have: end,
            });
        }
        off = next;
        walked += 1;
    }
    if off != end {
        return Err(FileError::Truncated {
            need: end,
            have: off,
        });
    }
    Ok(())
}

/// Split an `.ojn` chart into its per-level event slices: decrypts
/// new-format containers, reads the counts at +64/+68/+72 and the slice
/// ends at +288/+292/+296 (first start at +284), then validates each
/// slice by walking its events to land exactly on its end.
/// Verified on all 13 new-format 7.55 files plus Tutorial.ojn, with the
/// last slice ending exactly on the first embedded JPEG and
/// file bytes +40/+44/+48 matching `OJNList.dat` +44/+48/+52
/// (docs/file-formats/ojn.md).
///
/// # Errors
///
/// `FileError::TooShort`/`Truncated` when slices overrun the buffer,
/// `FileError::UnknownFormat` on a bad header.
pub fn parse_ojn_levels(bytes: &[u8]) -> Result<Vec<OjnLevelSlice>, FileError> {
    let plain = decrypt_ojn_new_format(bytes)?;
    if plain.len() < OJN_HEADER_LEN {
        return Err(FileError::TooShort(plain.len()));
    }
    match detect_kind(&plain)? {
        AssetKind::Ojn => {}
        AssetKind::Ojm | AssetKind::OjnList => {
            return Err(FileError::UnknownFormat {
                magic: bytes[..4.min(bytes.len())].to_vec(),
            });
        }
    }
    let mut start = read_u32le(&plain, OJN_SLICES_START_OFF)? as usize;
    let mut out = Vec::with_capacity(OJN_LEVEL_COUNT);
    for level in 0..OJN_LEVEL_COUNT {
        let end = read_u32le(&plain, OJN_SLICE_END_OFFS[level])? as usize;
        let event_count = read_u32le(&plain, OJN_EVENT_COUNT_OFFS[level])?;
        if start < OJN_HEADER_LEN || end < start || end > plain.len() {
            return Err(FileError::Truncated {
                need: end,
                have: plain.len(),
            });
        }
        walk_level_events(&plain, start, end, event_count)?;
        out.push(OjnLevelSlice {
            level,
            event_count,
            data_start: start,
            data_end: end,
        });
        start = end;
    }
    Ok(out)
}

/// Album cover of one `.ojn` chart (docs/file-formats/ojn.md): the
/// embedded JPEG after the last level slice. Verified on all 14
/// shipped 7.55 charts: a level walk lands exactly on a `FF D8` SOI
/// at the third slice-end word (`u32` at +296; the last slice ends
/// exactly on the first embedded JPEG), and the stream runs to end of
/// file — the shipped covers carry no EOI marker, the file just ends
/// (decode with [`crate::decode_ojn_cover`]). This is the album art
/// the select/album panels show, not chart data.
///
/// # Errors
///
/// `FileError::TooShort` when the header overruns,
/// `FileError::UnknownFormat` on a non-`.ojn` magic or a cover offset
/// that misses the SOI marker, `FileError::Truncated` when the cover
/// offset points outside the file (the [`parse_ojn_levels`]
/// slice-geometry convention).
pub fn ojn_cover_jpeg(bytes: &[u8]) -> Result<Vec<u8>, FileError> {
    let plain = decrypt_ojn_new_format(bytes)?;
    match detect_kind(&plain)? {
        AssetKind::Ojn => {}
        AssetKind::Ojm | AssetKind::OjnList => {
            return Err(FileError::UnknownFormat {
                magic: bytes[..4.min(bytes.len())].to_vec(),
            });
        }
    }
    if plain.len() < OJN_HEADER_LEN {
        return Err(FileError::TooShort(plain.len()));
    }
    let offset = read_u32le(&plain, OJN_SLICE_END_OFFS[OJN_LEVEL_COUNT - 1])? as usize;
    if offset < OJN_HEADER_LEN || offset > plain.len() {
        return Err(FileError::Truncated {
            need: offset,
            have: plain.len(),
        });
    }
    if !plain[offset..].starts_with(&JPEG_SOI) {
        let magic_end = offset.saturating_add(JPEG_SOI.len()).min(plain.len());
        return Err(FileError::UnknownFormat {
            magic: plain[offset..magic_end].to_vec(),
        });
    }
    Ok(plain[offset..].to_vec())
}

/// First note-event kind: kinds 2..=8 are the seven playable lanes.
/// Verified: per-level playable-note totals (kinds 2-8, nonzero values)
/// equal the header counts at +40/+44/+48 on all 13 new-format 7.55
/// charts x 3 levels (39/39; docs/file-formats/ojn.md).
/// Songs use either lanes {0, 2, 3} or all seven.
/// See `docs/file-formats/ojn.md`.
pub const OJN_LANE_KIND_FIRST: u16 = 2;

/// Last note-event kind (inclusive).
pub const OJN_LANE_KIND_LAST: u16 = 8;

/// Playable lane count (kinds 2..=8).
pub const OJN_LANE_COUNT: usize = 7;

/// Subdivisions per measure: event payload index `i` of `count` sits at
/// `i * OJN_TICKS_PER_MEASURE / count` (observed counts 1/16/32/192
/// all divide 192; Tutorial.ojn uses count 192 for its BGM channel).
pub const OJN_TICKS_PER_MEASURE: u32 = 192;

/// Lane index for a note-event kind, if it is a playable lane.
///
/// Verified: kinds 2..=8 map to lanes 0..=6 (`lane = kind - 2`);
/// kinds 0/1 carry BPM floats and kinds 9+ carry autoplay/BGM notes
/// in the same value encoding (docs/file-formats/ojn.md).
#[must_use]
pub fn ojn_lane_of_kind(kind: u16) -> Option<u8> {
    if (OJN_LANE_KIND_FIRST..=OJN_LANE_KIND_LAST).contains(&kind) {
        // Range is 2..=8, so the difference fits `u8` (PERF-07: no `as`).
        u8::try_from(kind - OJN_LANE_KIND_FIRST).ok()
    } else {
        None
    }
}

/// One decoded chart event: measure tick, channel kind, raw payload.
///
/// The head word is a signed measure index (not the float beat an
/// early dossier guessed: Tutorial.ojn ticks read 0/12/17/... as ints
/// and new-format ticks run 1..~100 per slice). Kinds 0/1 hold `f32`
/// BPM data; kinds 2+ hold `u32` note values (see [`decode_note_value`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjnEvent {
    /// Measure index (`i32` head word).
    pub measure: i32,
    /// Channel kind (`u16`).
    pub kind: u16,
    /// Payload words (`count` x `u32`).
    pub values: Vec<u32>,
}

/// Decode the events of one level slice into [`OjnEvent`]s.
///
/// Decrypts new-format containers, locates the slice via
/// [`parse_ojn_levels`], then collects the `8 + count * 4` framed
/// events. Fails like [`parse_ojn_levels`] on truncated input;
/// an out-of-range `level` is a [`FileError::Truncated`] mismatch.
/// Verified framing on all 14 shipped 7.55 charts
/// (docs/file-formats/ojn.md).
///
/// # Errors
///
/// `FileError::TooShort`/`Truncated` when events overrun the buffer
/// or the level index is out of range.
pub fn parse_ojn_level_events(bytes: &[u8], level: usize) -> Result<Vec<OjnEvent>, FileError> {
    let plain = decrypt_ojn_new_format(bytes)?;
    if plain.len() < OJN_HEADER_LEN {
        return Err(FileError::TooShort(plain.len()));
    }
    let slices = parse_ojn_levels(bytes)?;
    let slice = slices.get(level).ok_or(FileError::Truncated {
        need: level + 1,
        have: slices.len(),
    })?;
    let mut events = Vec::with_capacity(slice.event_count as usize);
    let mut off = slice.data_start;
    for _ in 0..slice.event_count {
        // Head word is a signed measure index; values that do not fit
        // `i32` are malformed input (ERRORS-01), not wraparound data.
        let measure =
            i32::try_from(read_u32le(&plain, off)?).map_err(|_| FileError::Truncated {
                need: off + 4,
                have: plain.len(),
            })?;
        let kind = read_u16le(&plain, off + 4)?;
        let count = read_u16le(&plain, off + 6)? as usize;
        let mut values = Vec::with_capacity(count);
        for i in 0..count {
            values.push(read_u32le(&plain, off + 8 + i * OJN_EVENT_UNIT_LEN)?);
        }
        off += OJN_EVENT_HEAD_LEN + count * OJN_EVENT_UNIT_LEN;
        events.push(OjnEvent {
            measure,
            kind,
            values,
        });
    }
    Ok(events)
}

/// One nonzero note payload split into sample id + channel flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnNoteValue {
    /// Key-sound sample id: raw low word minus one (`0` = empty,
    /// so stored values run `sample_id + 1`).
    ///
    /// Verified: `value - 1` lands in the song `.ojm` sample-id set
    /// for every lane note on 12/13 new-format charts (one dangling
    /// ref on o2ma765: value 35, id 34 absent) and for autoplay
    /// notes modulo a few dangling high refs (~999/1000).
    pub sample_id: u32,
    /// Raw high word: per-note flags (hold/expression).
    /// Semantics UNVERIFIED (observed `0xC0..0xF0`, `0x200/0x300`,
    /// `0x400` on Tutorial BGM; the in-exe parser tests payload bit
    /// 26, which shipped note payloads never set — see
    /// docs/file-formats/ojn.md).
    pub flags: u16,
}

/// Split a raw note payload word (kinds 2+) into sample id + flags.
///
/// Returns `None` for empty slots (`0`, or a zero low word). Lanes
/// and autoplay channels share the encoding; kinds 0/1 payloads are
/// `f32` BPM data and must not go through here.
#[must_use]
pub fn decode_note_value(raw: u32) -> Option<OjnNoteValue> {
    if raw == 0 {
        return None;
    }
    let low = raw & 0xFFFF;
    if low == 0 {
        return None;
    }
    Some(OjnNoteValue {
        sample_id: low - 1,
        // `raw >> 16` is at most 0xFFFF, so this always fits (PERF-07).
        flags: u16::try_from(raw >> 16).ok()?,
    })
}

/// Position of payload index `i` of `count` within its measure, in
/// [`OJN_TICKS_PER_MEASURE`]ths. Returns `None` for empty events
/// (`count == 0`) or grids that do not divide 192 exactly.
#[must_use]
pub fn ojn_sub_position(sub_index: usize, sub_count: usize) -> Option<u32> {
    if sub_count == 0 {
        return None;
    }
    let scaled = (sub_index as u32).checked_mul(OJN_TICKS_PER_MEASURE)?;
    if scaled % sub_count as u32 != 0 {
        return None;
    }
    Some(scaled / sub_count as u32)
}

/// Walked note totals for one level's event stream.
///
/// Counts nonzero payloads ([`decode_note_value`] is `Some`): kinds 2+
/// are the header total triplet at +28/+32/+36, kinds 2..=8 the
/// playable triplet at +40/+44/+48. Kinds 0/1 carry `f32` meter/tempo
/// data and never count as notes. Shipped 7.55 walks land exactly on
/// both triplets (39/39 levels); the total-minus-playable autoplay gap
/// ranges 13..2160 across the 14 charts and never equals the grade
/// reference's 26, so the unscored 26 are not a file-level event
/// class (see `docs/cresult-anchors.md` OPEN 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnLevelNoteCounts {
    /// Nonzero payloads on kinds 2+ (header +28/+32/+36).
    pub total_notes: usize,
    /// Nonzero payloads on playable lanes 2..=8 (header +40/+44/+48).
    pub playable_notes: usize,
    /// Backing/autoplay notes: `total_notes - playable_notes`.
    pub autoplay_notes: usize,
}

/// Count scored notes in a decoded level event slice.
///
/// Pure walk over [`parse_ojn_level_events`] output; empty and
/// zero-slot payloads contribute nothing.
#[must_use]
pub fn count_level_notes(events: &[OjnEvent]) -> OjnLevelNoteCounts {
    let mut total_notes = 0usize;
    let mut playable_notes = 0usize;
    for event in events {
        if event.kind < OJN_LANE_KIND_FIRST {
            continue;
        }
        for raw in &event.values {
            if decode_note_value(*raw).is_none() {
                continue;
            }
            total_notes = total_notes.saturating_add(1);
            if ojn_lane_of_kind(event.kind).is_some() {
                playable_notes = playable_notes.saturating_add(1);
            }
        }
    }
    OjnLevelNoteCounts {
        total_notes,
        playable_notes,
        autoplay_notes: total_notes.saturating_sub(playable_notes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic 300-byte header with a distinctive value at every
    /// mapped offset (scenario: each field parses from its own slot).
    fn full_header() -> Vec<u8> {
        let mut h = vec![0u8; OJN_HEADER_LEN];
        h[0..4].copy_from_slice(&8888u32.to_le_bytes());
        h[4..8].copy_from_slice(b"ojn\0");
        h[8..12].copy_from_slice(&2.7f32.to_le_bytes());
        h[12..16].copy_from_slice(&10u32.to_le_bytes());
        h[16..20].copy_from_slice(&100f32.to_le_bytes());
        h[20..24].copy_from_slice(&1u32.to_le_bytes());
        for (i, off) in [28, 32, 36].iter().enumerate() {
            h[*off..*off + 4].copy_from_slice(&(51u32 + i as u32).to_le_bytes());
        }
        for (i, off) in [40, 44, 48].iter().enumerate() {
            h[*off..*off + 4].copy_from_slice(&(38u32 + i as u32).to_le_bytes());
        }
        for (i, off) in [52, 56, 60].iter().enumerate() {
            h[*off..*off + 4].copy_from_slice(&(102u32 + i as u32).to_le_bytes());
        }
        for (i, off) in OJN_EVENT_COUNT_OFFS.iter().enumerate() {
            h[*off..*off + 4].copy_from_slice(&(43u32 + i as u32).to_le_bytes());
        }
        h[76..80].copy_from_slice(&0x22B8_001Bu32.to_le_bytes());
        h[104..108].copy_from_slice(&0f32.to_le_bytes());
        h[108..120].copy_from_slice(b"Test Title\0\0");
        h[172..183].copy_from_slice(b"TestArtist\0");
        h[204..217].copy_from_slice(b"nao.paradigm\0");
        h[236..249].copy_from_slice(b"Tutorial.ojm\0");
        h[268..272].copy_from_slice(&106954u32.to_le_bytes());
        h[272..276].copy_from_slice(&247u32.to_le_bytes());
        h[284..288].copy_from_slice(&(OJN_HEADER_LEN as u32).to_le_bytes());
        for off in OJN_SLICE_END_OFFS {
            h[off..off + 4].copy_from_slice(&11784u32.to_le_bytes());
        }
        h
    }

    #[test]
    fn header_fields_parse_from_their_own_slots() {
        let chart = parse_ojn(&full_header()).unwrap();
        assert_eq!(chart.song_id, 8888);
        assert_eq!(chart.magic, *b"ojn\0");
        assert_eq!(chart.format_version, 2.7);
        assert_eq!(chart.genre, 10);
        assert_eq!(chart.bpm, 100.0);
        assert_eq!(chart.level_flags, 1);
        assert_eq!(chart.stable_minor, 0);
        assert_eq!(chart.total_counts, [51, 52, 53]);
        assert_eq!(chart.playable_counts, [38, 39, 40]);
        assert_eq!(chart.last_measures, [102, 103, 104]);
        assert_eq!(chart.event_counts, [43, 44, 45]);
        assert_eq!(chart.event_count_hint, 43);
        assert_eq!(chart.key_field, 0x22B8_001B);
        assert_eq!(chart.key_hi(), 8888);
        assert_eq!(chart.key_lo(), 27);
        assert_eq!(chart.pad, [0, 0, 0, 0]);
        assert_eq!(chart.flag96, 0);
        assert_eq!(chart.const100, 0);
        assert_eq!(chart.tune, 0.0);
        assert_eq!(chart.title, "Test Title");
        assert_eq!(chart.artist, "TestArtist");
        assert_eq!(chart.label, "nao.paradigm");
        assert_eq!(chart.ojm_file, "Tutorial.ojm");
        assert_eq!(chart.unknown1, 106954);
        assert_eq!(chart.durations, [247, 0, 0]);
        assert_eq!(chart.slice_start, OJN_HEADER_LEN as u32);
        assert_eq!(chart.slice_ends, [11784, 11784, 11784]);
    }

    #[test]
    fn tutorial_real_header_values_parse() {
        // Pinned from the real 7.55 `Tutorial.ojn` (118738 bytes;
        // header dumped byte-exact in docs/file-formats/ojn.md):
        // every numeric field plus the ASCII areas. The Korean
        // title/artist areas hold CP949 bytes, decoded like the
        // song-select rows read them.
        let mut h = vec![0u8; OJN_HEADER_LEN];
        h[0..4].copy_from_slice(&8888u32.to_le_bytes());
        h[4..8].copy_from_slice(b"ojn\0");
        h[8..12].copy_from_slice(&2.7f32.to_le_bytes());
        h[12..16].copy_from_slice(&10u32.to_le_bytes());
        h[16..20].copy_from_slice(&100f32.to_le_bytes());
        h[20..24].copy_from_slice(&1u32.to_le_bytes());
        h[28..32].copy_from_slice(&51u32.to_le_bytes());
        h[40..44].copy_from_slice(&38u32.to_le_bytes());
        h[52..56].copy_from_slice(&102u32.to_le_bytes());
        h[64..68].copy_from_slice(&43u32.to_le_bytes());
        h[76..80].copy_from_slice(&0x22B8_001Bu32.to_le_bytes());
        h[108..116].copy_from_slice(&[0xC6, 0xA9, 0xC5, 0xE4, 0xB8, 0xAE, 0xBE, 0xF3]);
        h[172..182].copy_from_slice(&[0xBA, 0xE4, 0xC6, 0xBC, 0xC7, 0xAE, 0xB5, 0xA5, 0xC0, 0xCC]);
        h[204..217].copy_from_slice(b"nao.paradigm\0");
        h[236..249].copy_from_slice(b"Tutorial.ojm\0");
        h[268..272].copy_from_slice(&106954u32.to_le_bytes());
        h[272..276].copy_from_slice(&247u32.to_le_bytes());
        h[284..288].copy_from_slice(&300u32.to_le_bytes());
        for off in OJN_SLICE_END_OFFS {
            h[off..off + 4].copy_from_slice(&11784u32.to_le_bytes());
        }
        let chart = parse_ojn(&h).unwrap();
        assert_eq!(chart.song_id, 8888);
        assert_eq!(chart.format_version, 2.7);
        assert_eq!(chart.genre, 10);
        assert_eq!(chart.bpm, 100.0);
        assert_eq!(chart.level_flags, 1);
        assert_eq!(chart.total_counts, [51, 0, 0]);
        assert_eq!(chart.playable_counts, [38, 0, 0]);
        assert_eq!(chart.last_measures, [102, 0, 0]);
        assert_eq!(chart.event_counts, [43, 0, 0]);
        assert_eq!(chart.key_hi(), 8888);
        assert_eq!(chart.key_lo(), 27);
        assert_eq!(chart.title, "\u{D29C}\u{D1A0}\u{B9AC}\u{C5BC}");
        assert_eq!(chart.artist, "\u{BDF0}\u{D2F0}\u{D480}\u{B370}\u{C774}");
        assert_eq!(chart.label, "nao.paradigm");
        assert_eq!(chart.ojm_file, "Tutorial.ojm");
        assert_eq!(chart.durations, [247, 0, 0]);
    }

    #[test]
    fn ojn_list_korean_title_decodes_cp949() {
        // 7.55 text bodies are CP949 (the MSVC2005 Korean-locale
        // codepage): EUC-KR bytes must decode to Korean glyphs,
        // never to U+FFFD replacements — the song-select rows read
        // these titles.
        let mut rec = vec![0u8; 300];
        rec[8..11].copy_from_slice(b"ojn");
        rec[4..8].copy_from_slice(&757u32.to_le_bytes());
        rec[20..24].copy_from_slice(&124.0f32.to_le_bytes());
        rec[112..120].copy_from_slice(&[0xC6, 0xA9, 0xC5, 0xE4, 0xB8, 0xAE, 0xBE, 0xF3]);
        rec[240..251].copy_from_slice(b"o2ma757.ojm");
        let entries = parse_ojn_list(&rec).unwrap();
        assert_eq!(entries[0].title, "\u{D29C}\u{D1A0}\u{B9AC}\u{C5BC}");
    }

    #[test]
    fn difficulty_slots_read_low_high_and_hard_levels() {
        // C34/C35 verdict scenario: the exe indexes {+20 lo, +20 hi,
        // +24} by the chosen difficulty (entry builder file `0x44268`,
        // slice index via file `0x171c18`), so slot 0/1/2 must read
        // low half / high half / +24.
        let mut h = full_header();
        h[20..24].copy_from_slice(&((7u32 << 16) | 3).to_le_bytes());
        h[24..28].copy_from_slice(&13u32.to_le_bytes());
        let chart = parse_ojn(&h).unwrap();
        assert_eq!(chart.level_for_difficulty(0), Some(3));
        assert_eq!(chart.level_for_difficulty(1), Some(7));
        assert_eq!(chart.level_for_difficulty(2), Some(13));
        assert_eq!(chart.level_for_difficulty(3), None);
    }

    #[test]
    fn tutorial_edge_maps_only_easy() {
        // Tutorial.ojn edge: level flags 1, +24 = 0 — only Easy has a
        // level; Normal and Hard read 0.
        let mut h = full_header();
        h[20..24].copy_from_slice(&1u32.to_le_bytes());
        h[24..28].copy_from_slice(&0u32.to_le_bytes());
        let chart = parse_ojn(&h).unwrap();
        assert_eq!(chart.level_for_difficulty(0), Some(1));
        assert_eq!(chart.level_for_difficulty(1), Some(0));
        assert_eq!(chart.level_for_difficulty(2), Some(0));
    }

    #[test]
    fn header_rejects_short_and_bad_magic() {
        assert!(matches!(parse_ojn(b"short"), Err(FileError::TooShort(_))));
        let mut bad = full_header();
        bad[4..8].copy_from_slice(b"XXXX");
        assert!(matches!(
            parse_ojn(&bad),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn framed_event(measure: i32, kind: u16, payload: &[u32]) -> Vec<u8> {
        let mut event = Vec::with_capacity(OJN_EVENT_HEAD_LEN + payload.len() * 4);
        event.extend_from_slice(&(measure as u32).to_le_bytes());
        event.extend_from_slice(&kind.to_le_bytes());
        event.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        for unit in payload {
            event.extend_from_slice(&unit.to_le_bytes());
        }
        event
    }

    /// Tutorial-shaped level 0: one BGM event (kind 9, 192 slots),
    /// lane taps on kinds 2/5, a meter float, a BPM lift, and the
    /// closing 1-slot event — then two empty trailing levels.
    fn tutorial_shaped_blob() -> Vec<u8> {
        let level0 = [
            framed_event(0, 9, &vec![0u32; 192]),
            framed_event(12, 2, &[7, 0, 0, 0]),
            framed_event(12, 1, &[150f32.to_bits()]),
            framed_event(17, 0, &[0.75f32.to_bits()]),
            framed_event(20, 5, &[0, 12, 0, 0]),
            framed_event(102, 9, &[0x1F]),
        ]
        .concat();
        let mut blob = full_header();
        blob[28..32].copy_from_slice(&3u32.to_le_bytes());
        blob[32..36].copy_from_slice(&0u32.to_le_bytes());
        blob[36..40].copy_from_slice(&0u32.to_le_bytes());
        blob[40..44].copy_from_slice(&2u32.to_le_bytes());
        blob[44..48].copy_from_slice(&0u32.to_le_bytes());
        blob[48..52].copy_from_slice(&0u32.to_le_bytes());
        blob[52..56].copy_from_slice(&102u32.to_le_bytes());
        blob[56..60].copy_from_slice(&0u32.to_le_bytes());
        blob[60..64].copy_from_slice(&0u32.to_le_bytes());
        blob[64..68].copy_from_slice(&6u32.to_le_bytes());
        blob[68..72].copy_from_slice(&0u32.to_le_bytes());
        blob[72..76].copy_from_slice(&0u32.to_le_bytes());
        let end0 = OJN_HEADER_LEN + level0.len();
        for off in OJN_SLICE_END_OFFS {
            blob[off..off + 4].copy_from_slice(&(end0 as u32).to_le_bytes());
        }
        blob.extend_from_slice(&level0);
        blob
    }

    #[test]
    fn old_format_levels_walk_with_exact_ranges() {
        let blob = tutorial_shaped_blob();
        let header = parse_ojn(&blob).unwrap();
        let slices = parse_ojn_levels(&blob).unwrap();
        assert_eq!(slices.len(), OJN_LEVEL_COUNT);
        assert_eq!(slices[0].event_count, 6);
        assert_eq!(slices[0].data_start, OJN_HEADER_LEN);
        assert_eq!(slices[1].event_count, 0);
        assert_eq!(slices[2].event_count, 0);
        assert_eq!(slices[1].data_start, slices[0].data_end);
        assert_eq!(slices[2].data_start, slices[2].data_end);
        assert_eq!(header.event_counts[0], slices[0].event_count);
        assert_eq!(header.slice_ends[2] as usize, slices[2].data_end);
        let events = parse_ojn_level_events(&blob, 0).unwrap();
        assert_eq!(events.len(), 6);
        assert_eq!(events[0].kind, 9);
        assert_eq!(events[0].values.len(), 192);
        assert_eq!(events[1].measure, 12);
        assert_eq!(events[5].measure, 102);
        assert_eq!(header.last_measures[0], 102);
        let playable = events
            .iter()
            .filter(|e| ojn_lane_of_kind(e.kind).is_some())
            .flat_map(|e| e.values.iter())
            .filter(|v| decode_note_value(**v).is_some())
            .count();
        let total = events
            .iter()
            .filter(|e| e.kind >= 2)
            .flat_map(|e| e.values.iter())
            .filter(|v| decode_note_value(**v).is_some())
            .count();
        assert_eq!(playable, header.playable_counts[0] as usize);
        assert_eq!(total, header.total_counts[0] as usize);
        assert!(parse_ojn_level_events(&blob, 1).unwrap().is_empty());
    }

    #[test]
    fn event_kinds_cover_meter_tempo_lanes_and_backing() {
        let meter = framed_event(21, 0, &[0.75f32.to_bits()]);
        assert_eq!(f32::from_le_bytes(meter[8..12].try_into().unwrap()), 0.75);
        let tempo = framed_event(62, 1, &[290f32.to_bits(), 0, 145f32.to_bits()]);
        let words: Vec<f32> = tempo[8..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|w| f32::from_le_bytes(*w))
            .collect();
        assert_eq!(words, [290.0, 0.0, 145.0]);
        for kind in OJN_LANE_KIND_FIRST..=OJN_LANE_KIND_LAST {
            assert_eq!(ojn_lane_of_kind(kind), Some((kind - 2) as u8));
        }
        assert_eq!(ojn_lane_of_kind(0), None);
        assert_eq!(ojn_lane_of_kind(1), None);
        assert_eq!(ojn_lane_of_kind(9), None);
        assert_eq!(ojn_lane_of_kind(19), None);
        assert_eq!(decode_note_value(0), None);
        assert_eq!(decode_note_value(0x0200_0000), None);
        assert_eq!(
            decode_note_value(3),
            Some(OjnNoteValue {
                sample_id: 2,
                flags: 0,
            })
        );
        assert_eq!(
            decode_note_value(0x00D0_0002),
            Some(OjnNoteValue {
                sample_id: 1,
                flags: 0xD0,
            })
        );
        assert_eq!(ojn_sub_position(0, 16), Some(0));
        assert_eq!(ojn_sub_position(4, 16), Some(48));
        assert_eq!(ojn_sub_position(0, 0), None);
        assert_eq!(ojn_sub_position(1, 7), None);
    }

    #[test]
    fn level_note_counts_split_lanes_from_backing() {
        let events = vec![
            OjnEvent {
                measure: 0,
                kind: 2,
                values: vec![3, 0, 0x0200_0000],
            },
            OjnEvent {
                measure: 1,
                kind: 9,
                values: vec![5, 7],
            },
            OjnEvent {
                measure: 2,
                kind: 1,
                values: vec![150f32.to_bits()],
            },
        ];
        let counts = count_level_notes(&events);
        assert_eq!(counts.total_notes, 3);
        assert_eq!(counts.playable_notes, 1);
        assert_eq!(counts.autoplay_notes, 2);
    }

    #[test]
    fn level_note_counts_empty_stream_is_zero() {
        let counts = count_level_notes(&[]);
        assert_eq!(
            counts,
            OjnLevelNoteCounts {
                total_notes: 0,
                playable_notes: 0,
                autoplay_notes: 0,
            }
        );
    }

    #[test]
    fn level_note_counts_match_tutorial_shaped_blob() {
        let blob = tutorial_shaped_blob();
        let events = parse_ojn_level_events(&blob, 0).unwrap();
        let counts = count_level_notes(&events);
        assert_eq!(counts.total_notes, 3);
        assert_eq!(counts.playable_notes, 2);
        assert_eq!(counts.autoplay_notes, 1);
    }

    /// SOI-prefixed stand-in cover body: [`ojn_cover_jpeg`] frames the
    /// stream by slice ends and the SOI sniff; pixel decoding lives in
    /// [`crate::decode_ojn_cover`] (tested in `jpeg.rs`).
    fn fake_cover() -> Vec<u8> {
        let mut body = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        body.extend_from_slice(b"JFIF\0");
        body.extend_from_slice(&[0x01, 0x02, 0x03, 0xFF, 0xD9]);
        body
    }

    /// Chart blob with every slice ending on an appended cover body
    /// (the shipped shape: the last slice ends exactly on the JPEG).
    fn chart_with_cover() -> Vec<u8> {
        let mut blob = tutorial_shaped_blob();
        let cover_offset = blob.len() as u32;
        for off in OJN_SLICE_END_OFFS {
            blob[off..off + 4].copy_from_slice(&cover_offset.to_le_bytes());
        }
        blob.extend_from_slice(&fake_cover());
        blob
    }

    #[test]
    fn cover_runs_from_the_last_slice_end_to_eof() {
        let blob = chart_with_cover();
        let cover = ojn_cover_jpeg(&blob).unwrap();
        let offset = blob.len() - cover.len();
        assert_eq!(
            offset,
            usize::try_from(parse_ojn(&blob).unwrap().slice_ends[2]).unwrap()
        );
        assert_eq!(cover, fake_cover());
    }

    #[test]
    fn cover_extracts_after_the_new_format_decrypt() {
        // Encoder contract of FUN_00412180 (see the lib.rs round-trip
        // test): XOR with (k1,k2,k3) on period p over the plaintext,
        // key slots at plaintext tail, `new` magic after reversal.
        let plain = chart_with_cover();
        let (k1, k2, k3, p): (u8, u8, u8, usize) = (0x97, 0xE0, 0x34, 6);
        let key = |i: usize| {
            if i.is_multiple_of(p) {
                k1
            } else if i % p == p / 2 {
                k2
            } else {
                k3
            }
        };
        let n = plain.len();
        let mut buf: Vec<u8> = plain.iter().enumerate().map(|(i, &b)| b ^ key(i)).collect();
        buf[n - 7] = k1;
        buf[n - 6] = k2;
        buf[n - 5] = k3;
        buf[n - 4] = u8::try_from(p).unwrap();
        // Tail bytes become the "new" magic after reversal.
        buf[n - 3] = b'w';
        buf[n - 2] = b'e';
        buf[n - 1] = b'n';
        let encrypted: Vec<u8> = buf.iter().rev().copied().collect();
        assert!(encrypted.starts_with(b"new"));
        // Key slots and magic tail are clobbered by the encoder
        // contract (lib.rs round-trip test); the body round-trips.
        let cover = ojn_cover_jpeg(&encrypted).unwrap();
        let intact = fake_cover().len() - 7;
        assert_eq!(&cover[..intact], &fake_cover()[..intact]);
    }

    #[test]
    fn cover_rejects_offsets_outside_the_file() {
        // full_header() names 11784 with no body behind the header.
        assert!(matches!(
            ojn_cover_jpeg(&full_header()),
            Err(FileError::Truncated { .. })
        ));
        let mut early = full_header();
        for off in OJN_SLICE_END_OFFS {
            early[off..off + 4].copy_from_slice(&10u32.to_le_bytes());
        }
        assert!(matches!(
            ojn_cover_jpeg(&early),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn cover_rejects_an_offset_that_misses_the_soi() {
        let mut blob = full_header();
        let cover_offset = u32::try_from(blob.len()).unwrap();
        for off in OJN_SLICE_END_OFFS {
            blob[off..off + 4].copy_from_slice(&cover_offset.to_le_bytes());
        }
        blob.extend_from_slice(b"not a jpeg");
        assert!(matches!(
            ojn_cover_jpeg(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }
}
