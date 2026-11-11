//! The `OJNList.dat` patch index and patch section.
//!
//! Verified on real 7.55 files (docs/file-formats/ojn.md).

use crate::{read_u32le, unknown_format, FileError};

/// Magic heading each `OJNList.dat` patch record (`;HD\0`).
pub const OJN_PATCH_MAGIC: &[u8; 4] = b";HD\0";

/// Bytes per patch record and per patch-section header.
pub const OJN_PATCH_RECORD_LEN: usize = 28;

/// Constant word at patch-record offset 4 (48401 on the real file).
pub const OJN_PATCH_CONST: u32 = 0xbd11;

/// Constant 5-byte tag closing the header and every patch record.
pub const OJN_PATCH_TAG: [u8; 5] = [0x77, 0x30, 0xf7, 0x12, 0x00];

/// Marker words heading patch-index pairs (both seen on the real file).
pub const OJN_PATCH_INDEX_MARKERS: [u32; 2] = [0x43f403, 0x43f407];

/// Bytes per patch-index pair (`u32` marker + `u32` song id).
pub const OJN_PATCH_INDEX_ENTRY_LEN: usize = 8;

/// One `OJNList.dat` patch-index pair: marker word plus song id.
///
/// The real 7.55 file holds 561 pairs (ids 886..100, sparse) in the
/// same order as the patch records (docs/file-formats/ojn.md).
/// Marker semantics: none — the only exe reader is the reader-district
/// pair loop, whose count is the mid-region footer echo word `0` on the
/// shipped file, so the body never executes (dead data, C40,
/// `docs/ojnlist-trailer.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnPatchIndexEntry {
    /// Marker word (`OJN_PATCH_INDEX_MARKERS`).
    pub marker: u32,
    /// Song id this pair belongs to.
    pub song_id: u32,
}

/// One `OJNList.dat` patch record: patch id plus patch date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjnPatchRecord {
    /// Patch id (descending, 886..100 sparse on the real file).
    pub patch_id: u32,
    /// Patch date (`YYYY-MM-DD`).
    pub date: String,
    /// Closing tag (always `OJN_PATCH_TAG` on the real file).
    pub tag: [u8; 5],
}

/// The `OJNList.dat` patch section: 28-byte header, patch records,
/// and the byte length of a truncated trailing record (8 on the real
/// 7.55 file: the 562nd slot holds only magic + const).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjnPatchSection {
    /// Header marker word (same set as index markers).
    pub header_marker: u32,
    /// Declared record-slot count (562: records plus truncated tail).
    pub record_count: u32,
    /// Newest patch id (887: one past the first full record).
    pub top_patch_id: u32,
    /// Newest patch date.
    pub top_date: String,
    /// Newest patch tag.
    pub top_tag: [u8; 5],
    /// Full patch records in file order.
    pub records: Vec<OjnPatchRecord>,
    /// Trailing bytes after the last full record (< 28).
    pub trailing_len: usize,
}

/// Read a `YYYY-MM-DD\0` patch date at `off`; returns the date text.
fn read_patch_date(bytes: &[u8], off: usize) -> Result<String, FileError> {
    let raw = bytes.get(off..off + 11).ok_or(FileError::Truncated {
        need: off + 11,
        have: bytes.len(),
    })?;
    let ascii_date = &raw[..10];
    let is_date = raw[10] == 0 && ascii_date.iter().all(|b| b.is_ascii_digit() || *b == b'-');
    if !is_date {
        return Err(unknown_format(&raw[..4]));
    }
    core::str::from_utf8(ascii_date)
        .map(str::to_owned)
        .map_err(|_| unknown_format(&raw[..4]))
}

/// Read the 5-byte closing tag of one 28-byte patch window.
fn read_patch_tag(bytes: &[u8], off: usize) -> Result<[u8; 5], FileError> {
    let tag: [u8; 5] = bytes
        .get(off + 23..off + OJN_PATCH_RECORD_LEN)
        .and_then(|w| w.try_into().ok())
        .ok_or(FileError::Truncated {
            need: off + OJN_PATCH_RECORD_LEN,
            have: bytes.len(),
        })?;
    if tag != OJN_PATCH_TAG {
        return Err(unknown_format(&bytes[off..off + 4]));
    }
    Ok(tag)
}

/// Parse an `OJNList.dat` patch index: `u32` marker + `u32` song id
/// pairs. The buffer must hold a whole number of pairs.
///
/// # Errors
///
/// `FileError::TooShort` on empty input, `FileError::Truncated` on a
/// partial pair, `FileError::UnknownFormat` on an unknown marker.
pub fn parse_ojn_patch_index(bytes: &[u8]) -> Result<Vec<OjnPatchIndexEntry>, FileError> {
    if bytes.is_empty() {
        return Err(FileError::TooShort(0));
    }
    if !bytes.len().is_multiple_of(OJN_PATCH_INDEX_ENTRY_LEN) {
        let missing = OJN_PATCH_INDEX_ENTRY_LEN - bytes.len() % OJN_PATCH_INDEX_ENTRY_LEN;
        return Err(FileError::Truncated {
            need: bytes.len() + missing,
            have: bytes.len(),
        });
    }
    let mut entries = Vec::with_capacity(bytes.len() / OJN_PATCH_INDEX_ENTRY_LEN);
    let mut off = 0;
    while off < bytes.len() {
        let marker = read_u32le(bytes, off)?;
        if !OJN_PATCH_INDEX_MARKERS.contains(&marker) {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        let song_id = read_u32le(bytes, off + 4)?;
        entries.push(OjnPatchIndexEntry { marker, song_id });
        off += OJN_PATCH_INDEX_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse an `OJNList.dat` patch section: 28-byte header (marker,
/// count, top id, date, tag), 28-byte `;HD` records, and an optional
/// truncated tail shorter than one record.
///
/// The declared count must equal full records plus one slot when a
/// truncated tail is present (562 = 561 + 1 on the real 7.55 file).
///
/// # Errors
///
/// `FileError::Truncated` when the header is incomplete,
/// `FileError::UnknownFormat` on bad magic/words/date/tag or a
/// count that disagrees with the records present.
pub fn parse_ojn_patch_section(bytes: &[u8]) -> Result<OjnPatchSection, FileError> {
    if bytes.len() < OJN_PATCH_RECORD_LEN {
        return Err(FileError::Truncated {
            need: OJN_PATCH_RECORD_LEN,
            have: bytes.len(),
        });
    }
    let header_marker = read_u32le(bytes, 0)?;
    if !OJN_PATCH_INDEX_MARKERS.contains(&header_marker) {
        return Err(unknown_format(&bytes[..4]));
    }
    let record_count = read_u32le(bytes, 4)?;
    let top_patch_id = read_u32le(bytes, 8)?;
    let top_date = read_patch_date(bytes, 12)?;
    let top_tag = read_patch_tag(bytes, 0)?;
    let mut records = Vec::new();
    let mut off = OJN_PATCH_RECORD_LEN;
    while bytes.len() - off >= OJN_PATCH_RECORD_LEN {
        if &bytes[off..off + 4] != OJN_PATCH_MAGIC {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        let word = read_u32le(bytes, off + 4)?;
        if word != OJN_PATCH_CONST {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        let patch_id = read_u32le(bytes, off + 8)?;
        let date = read_patch_date(bytes, off + 12)?;
        let tag = read_patch_tag(bytes, off)?;
        records.push(OjnPatchRecord {
            patch_id,
            date,
            tag,
        });
        off += OJN_PATCH_RECORD_LEN;
    }
    let trailing_len = bytes.len() - off;
    let full_slots = u32::try_from(records.len()).map_err(|_| FileError::Truncated {
        need: usize::MAX,
        have: bytes.len(),
    })?;
    let declared_slots =
        full_slots
            .checked_add(u32::from(trailing_len != 0))
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: bytes.len(),
            })?;
    if declared_slots != record_count {
        return Err(unknown_format(&bytes[..4]));
    }
    Ok(OjnPatchSection {
        header_marker,
        record_count,
        top_patch_id,
        top_date,
        top_tag,
        records,
        trailing_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch_pair(marker: u32, song_id: u32) -> Vec<u8> {
        let mut pair = Vec::with_capacity(OJN_PATCH_INDEX_ENTRY_LEN);
        pair.extend_from_slice(&marker.to_le_bytes());
        pair.extend_from_slice(&song_id.to_le_bytes());
        pair
    }

    fn patch_record_bytes(patch_id: u32, date: &str) -> Vec<u8> {
        let mut record = Vec::with_capacity(OJN_PATCH_RECORD_LEN);
        record.extend_from_slice(OJN_PATCH_MAGIC);
        record.extend_from_slice(&OJN_PATCH_CONST.to_le_bytes());
        record.extend_from_slice(&patch_id.to_le_bytes());
        record.extend_from_slice(date.as_bytes());
        record.push(0);
        record.extend_from_slice(&OJN_PATCH_TAG);
        record
    }

    fn patch_header_bytes(marker: u32, count: u32, top_id: u32, date: &str) -> Vec<u8> {
        let mut header = Vec::with_capacity(OJN_PATCH_RECORD_LEN);
        header.extend_from_slice(&marker.to_le_bytes());
        header.extend_from_slice(&count.to_le_bytes());
        header.extend_from_slice(&top_id.to_le_bytes());
        header.extend_from_slice(date.as_bytes());
        header.push(0);
        header.extend_from_slice(&OJN_PATCH_TAG);
        header
    }

    #[test]
    fn patch_index_parses_markers_and_ids() {
        let mut blob = patch_pair(0x43f403, 886);
        blob.extend_from_slice(&patch_pair(0x43f407, 884));
        let entries = parse_ojn_patch_index(&blob).unwrap();
        assert_eq!(
            entries,
            vec![
                OjnPatchIndexEntry {
                    marker: 0x43f403,
                    song_id: 886
                },
                OjnPatchIndexEntry {
                    marker: 0x43f407,
                    song_id: 884
                },
            ]
        );
    }

    #[test]
    fn patch_index_rejects_unknown_marker() {
        let blob = patch_pair(0x43f400, 886);
        assert!(matches!(
            parse_ojn_patch_index(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn patch_index_rejects_empty_and_partial_input() {
        assert!(matches!(
            parse_ojn_patch_index(&[]),
            Err(FileError::TooShort(_))
        ));
        assert!(matches!(
            parse_ojn_patch_index(&[0x07; 12]),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn patch_section_parses_header_records_and_truncated_tail() {
        let mut blob = patch_header_bytes(0x43f407, 3, 887, "2009-10-29");
        blob.extend_from_slice(&patch_record_bytes(886, "2009-10-29"));
        blob.extend_from_slice(&patch_record_bytes(885, "2005-08-01"));
        blob.extend_from_slice(b";HD\x00\x11\xbd\x00\x00");
        let section = parse_ojn_patch_section(&blob).unwrap();
        assert_eq!(section.header_marker, 0x43f407);
        assert_eq!(section.record_count, 3);
        assert_eq!(section.top_patch_id, 887);
        assert_eq!(section.top_date, "2009-10-29");
        assert_eq!(section.top_tag, OJN_PATCH_TAG);
        assert_eq!(section.records.len(), 2);
        assert_eq!(section.records[0].patch_id, 886);
        assert_eq!(section.records[1].date, "2005-08-01");
        assert_eq!(section.trailing_len, 8);
    }

    #[test]
    fn patch_section_counts_exact_records_without_tail() {
        let mut blob = patch_header_bytes(0x43f403, 1, 101, "2004-04-09");
        blob.extend_from_slice(&patch_record_bytes(100, "2004-04-09"));
        let section = parse_ojn_patch_section(&blob).unwrap();
        assert_eq!(section.records.len(), 1);
        assert_eq!(section.trailing_len, 0);
    }

    #[test]
    fn patch_section_mirrors_real_record_shape() {
        // Real 7.55 shape: 562 declared slots, 561 full records, an
        // 8-byte truncated tail, sparse descending ids.
        let mut blob = patch_header_bytes(0x43f407, 562, 887, "2009-10-29");
        for patch_id in (326..=886u32).rev() {
            blob.extend_from_slice(&patch_record_bytes(patch_id, "2004-04-09"));
        }
        blob.extend_from_slice(b";HD\x00\x11\xbd\x00\x00");
        let section = parse_ojn_patch_section(&blob).unwrap();
        assert_eq!(section.record_count, 562);
        assert_eq!(section.records.len(), 561);
        assert_eq!(section.records[0].patch_id, 886);
        assert_eq!(section.records[560].patch_id, 326);
        assert_eq!(section.trailing_len, 8);
    }

    #[test]
    fn patch_section_rejects_count_mismatch() {
        let mut blob = patch_header_bytes(0x43f407, 9, 887, "2009-10-29");
        blob.extend_from_slice(&patch_record_bytes(886, "2009-10-29"));
        assert!(matches!(
            parse_ojn_patch_section(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn patch_section_rejects_bad_record_magic() {
        let mut blob = patch_header_bytes(0x43f407, 1, 101, "2004-04-09");
        let mut record = patch_record_bytes(100, "2004-04-09");
        record[0..4].copy_from_slice(b"XXXX");
        blob.extend_from_slice(&record);
        assert!(matches!(
            parse_ojn_patch_section(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn patch_section_rejects_bad_const_word() {
        let mut blob = patch_header_bytes(0x43f407, 1, 101, "2004-04-09");
        let mut record = patch_record_bytes(100, "2004-04-09");
        record[4..8].copy_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&record);
        assert!(matches!(
            parse_ojn_patch_section(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn patch_section_rejects_bad_date_and_tag() {
        let mut blob = patch_header_bytes(0x43f407, 1, 101, "2004-04-09");
        let mut record = patch_record_bytes(100, "2004-04-09");
        record[12] = b'?';
        blob.extend_from_slice(&record);
        assert!(matches!(
            parse_ojn_patch_section(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        let mut blob = patch_header_bytes(0x43f407, 1, 101, "2004-04-09");
        let mut record = patch_record_bytes(100, "2004-04-09");
        record[23] = 0x00;
        blob.extend_from_slice(&record);
        assert!(matches!(
            parse_ojn_patch_section(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }
}
