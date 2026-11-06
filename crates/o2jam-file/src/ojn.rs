//! OJN charts: headers, level slices, note events, lanes, and the
//! new-format decrypt.
//!
//! Verified on real 7.55 files (docs/file-formats/ojn.md).

use super::ojm::{parse_m30, parse_omc, OjmSample};
use super::{detect_kind, read_f32le, read_str, read_u16le, read_u32le, AssetKind, FileError};

/// Minimal parsed view of an `.ojn` chart file.
///
/// Header field map verified on real 7.55 data (Tutorial.ojn and
/// decrypted o2ma757.ojn agree on id/magic/BPM positions).
#[derive(Debug, Clone, PartialEq)]
pub struct OjnChartFile {
    /// Raw magic tag seen (`ojn`).
    pub magic: [u8; 4],
    /// Song id (`u32` at +0).
    pub song_id: u32,
    /// Container version (`f32` at +8; reads 2.7 on Tutorial.ojn,
    /// 2.9 on decrypted o2ma757.ojn). The loader derives its OJM
    /// descriptor from this float instead of the file magic, so it
    /// also selects the new-variant lane count
    /// ([`ojm_descriptor_for_version`]).
    pub format_version: f32,
    /// Tempo (`f32` at +16; reads 100.0/124.0 on Tutorial/757).
    pub bpm: f32,
    /// Count field (`u32` at +64): equals the walked event total on
    /// Tutorial.ojn (43) but not on decrypted o2ma757.ojn (208 vs a
    /// 580-event run) — semantics TBD, exposed raw.
    pub event_count_hint: u32,
}

/// Parse an `.ojm` container into its sample streams.
///
/// Dispatches on magic: `M30` and `OMC` variants are fully implemented;
/// the `OJM` variant stays [`FileError::NotImplemented`] until its
/// encrypted region is understood (docs/file-formats/ojm.md).
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
            title: read_str(bytes, off + 112, 64)?,
            artist: read_str(bytes, off + 176, 32)?,
            label: read_str(bytes, off + 208, 32)?,
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
/// (old-format header parsing lands next; note-table work is tracked
/// in docs/file-formats/ojn.md).
pub fn parse_ojn(bytes: &[u8]) -> Result<OjnChartFile, FileError> {
    let plain = decrypt_ojn_new_format(bytes)?;
    match detect_kind(&plain)? {
        AssetKind::Ojn => Ok(OjnChartFile {
            magic: plain[..4].try_into().map_err(|_| FileError::Truncated {
                need: 4,
                have: plain.len(),
            })?,
            song_id: read_u32le(&plain, 0)?,
            format_version: read_f32le(&plain, 8)?,
            bpm: read_f32le(&plain, 16)?,
            event_count_hint: read_u32le(&plain, 64)?,
        }),
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
    /// Difficulty level (0, 1, 2).
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
        Some((kind - OJN_LANE_KIND_FIRST) as u8)
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
        let measure = read_u32le(&plain, off)? as i32;
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
        flags: (raw >> 16) as u16,
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
