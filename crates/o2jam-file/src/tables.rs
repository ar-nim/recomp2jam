//! Data tables: avatar outfit sets, avatar shop items, the jam gauge
//! thresholds, the `OJNList.dat` patch index/records, and the key-name
//! string table.
//!
//! Verified on real 7.55 files (docs/file-formats/opi.md,
//! docs/file-formats/ojn.md).

use super::{decode_control_text, read_u32le, unknown_format, FileError};

/// Item ids per avatar set row.
pub const SET_INFO_ITEMS: usize = 5;

/// Untagged stat `u32`s per avatar set row (prices/bonuses TBD).
pub const SET_INFO_STATS: usize = 10;

/// Fixed `u32` block per avatar set row: id + group + items + stats.
const SET_INFO_FIXED_U32S: usize = 17;

/// Bytes before the first set row: `u32` count + reserved zeros.
const SET_INFO_ROWS_AT: usize = 81;

/// One avatar outfit set (`SetInfodata.ojs` row): set metadata plus
/// Korean name/description. Item ids reference the shop database
/// (`Itemdata_Korea.dat` — join TBD); stat semantics are OPEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetInfoRow {
    /// Set id (1-based on the real file).
    pub set_id: u32,
    /// Group word (high byte varies, low 3 bytes constant-ish; TBD).
    pub group: u32,
    /// Item ids in this set (0 = no item).
    pub items: [u32; SET_INFO_ITEMS],
    /// Untagged stat words.
    pub stats: [u32; SET_INFO_STATS],
    /// Korean set name (EUC-KR).
    pub name: String,
    /// Korean set description (EUC-KR).
    pub description: String,
}

/// A parsed `SetInfodata.ojs` table: set rows plus the unconsumed
/// tail. The real 7.55 file declares count 210 but holds 209 set
/// rows and a 260-byte promo footer of a different shape, so the
/// walk stops at the first row that no longer parses and the rest
/// stays available (docs/file-formats/opi.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetInfoTable {
    /// Set rows in file order.
    pub rows: Vec<SetInfoRow>,
    /// Bytes after the last parsed row.
    pub trailer_len: usize,
}

/// Decode one EUC-KR text field (`u32` byte length + bytes) at `off`;
/// returns the text and the offset after it.
fn read_table_text(bytes: &[u8], off: usize) -> Result<(String, usize), FileError> {
    let len = read_u32le(bytes, off)? as usize;
    let end = off.checked_add(4 + len).ok_or(FileError::Truncated {
        need: usize::MAX,
        have: bytes.len(),
    })?;
    if end > bytes.len() {
        return Err(FileError::Truncated {
            need: end,
            have: bytes.len(),
        });
    }
    let (text, _, had_errors) = encoding_rs::EUC_KR.decode(&bytes[off + 4..end]);
    if had_errors {
        return Err(unknown_format(&bytes[off..end.min(off + 64)]));
    }
    Ok((text.into_owned(), end))
}

/// Parse a `SetInfodata.ojs` avatar-set table with exact row
/// accounting: `u32` count, rows of 17 `u32` + pad byte + two
/// length-prefixed strings, ids expected 1-based in order.
/// Stops (instead of erroring) where the footer starts so the
/// declared-count overstatement stays representable.
///
/// # Errors
///
/// `FileError::TooShort` when no row parses at all, `FileError::UnknownFormat`
/// on undecodable text.
pub fn parse_set_info_table(bytes: &[u8]) -> Result<SetInfoTable, FileError> {
    let count = read_u32le(bytes, 0)? as usize;
    let mut rows = Vec::new();
    let mut off = SET_INFO_ROWS_AT;
    for expected in 1..=count {
        let row_end = off + SET_INFO_FIXED_U32S * 4 + 1;
        if row_end > bytes.len() {
            break;
        }
        let mut fixed = [0u32; SET_INFO_FIXED_U32S];
        for (i, slot) in fixed.iter_mut().enumerate() {
            *slot = read_u32le(bytes, off + i * 4)?;
        }
        if fixed[0] != expected as u32 {
            break;
        }
        let (name, after_name) = read_table_text(bytes, row_end)?;
        let (description, after_desc) = read_table_text(bytes, after_name)?;
        let mut items = [0u32; SET_INFO_ITEMS];
        items.copy_from_slice(&fixed[2..7]);
        let mut stats = [0u32; SET_INFO_STATS];
        stats.copy_from_slice(&fixed[7..17]);
        rows.push(SetInfoRow {
            set_id: fixed[0],
            group: fixed[1],
            items,
            stats,
            name,
            description,
        });
        off = after_desc;
    }
    if rows.is_empty() {
        return Err(FileError::TooShort(bytes.len()));
    }
    Ok(SetInfoTable {
        rows,
        trailer_len: bytes.len() - off,
    })
}

/// Sprite filename slots per avatar item row (variants across
/// genders/parts; per-slot meaning TBD).
pub const AVATAR_SPRITE_SLOTS: usize = 42;

/// Rows start after the header (`u32` count-plus-one + zeros).
const AVATAR_ROWS_AT: usize = 84;

/// Fixed bytes per avatar item row before the name.
const AVATAR_FIXED_LEN: usize = 30;

/// One avatar shop item (`Itemdata_Korea.dat` row): stats plus
/// Korean name/description plus 42 sprite filename slots. Sprite
/// stems resolve to `.ojs` bodies in `avatar.opa` (the table spells
/// `.oji`); per-slot selection semantics are OPEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarItemRow {
    /// Item id (1-based on the real file).
    pub item_id: u32,
    /// Item type code.
    pub item_type: u32,
    /// Price.
    pub price: u32,
    /// Raw param words (TBD).
    pub params: [u32; 4],
    /// Raw flag bytes (TBD).
    pub flags: (u8, u8),
    /// Korean item name (EUC-KR).
    pub name: String,
    /// Korean item description (EUC-KR).
    pub description: String,
    /// `(flag, filename)` sprite slots in file order.
    pub sprites: Vec<(u8, String)>,
}

/// A parsed `Itemdata_Korea.dat` table: item rows in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarItemTable {
    /// Item rows in file order.
    pub rows: Vec<AvatarItemRow>,
}

/// Parse an `Itemdata_Korea.dat` avatar-item table with exact
/// accounting: header (`u32` rows-plus-one, zeros), then rows of
/// 30 fixed bytes + two length-prefixed strings + 42 counted
/// filename slots, ids 1-based in order, landing exactly at EOF
/// (docs/file-formats/opi.md).
///
/// # Errors
///
/// `FileError::TooShort`/`Truncated` when rows overrun, `UnknownFormat`
/// when the row ids or declared count break sequence.
pub fn parse_avatar_item_table(bytes: &[u8]) -> Result<AvatarItemTable, FileError> {
    let mut rows = Vec::new();
    let mut off = AVATAR_ROWS_AT;
    while off < bytes.len() {
        if off + AVATAR_FIXED_LEN > bytes.len() {
            return Err(FileError::Truncated {
                need: off + AVATAR_FIXED_LEN,
                have: bytes.len(),
            });
        }
        let item_id = read_u32le(bytes, off)?;
        if item_id != rows.len() as u32 + 1 {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        let item_type = read_u32le(bytes, off + 4)?;
        let flag_a = bytes[off + 12];
        let price = read_u32le(bytes, off + 13)?;
        let params = [
            read_u32le(bytes, off + 8)?,
            read_u32le(bytes, off + 17)?,
            read_u32le(bytes, off + 21)?,
            read_u32le(bytes, off + 25)?,
        ];
        let flag_b = bytes[off + 29];
        let (name, after_name) = read_table_text(bytes, off + AVATAR_FIXED_LEN)?;
        let (description, after_desc) = read_table_text(bytes, after_name)?;
        let mut sprites = Vec::with_capacity(AVATAR_SPRITE_SLOTS);
        let mut cursor = after_desc;
        for _ in 0..AVATAR_SPRITE_SLOTS {
            if cursor + 5 > bytes.len() {
                return Err(FileError::Truncated {
                    need: cursor + 5,
                    have: bytes.len(),
                });
            }
            let flag = bytes[cursor];
            let (filename, next) = read_table_text(bytes, cursor + 1)?;
            sprites.push((flag, filename));
            cursor = next;
        }
        rows.push(AvatarItemRow {
            item_id,
            item_type,
            price,
            params,
            flags: (flag_a, flag_b),
            name,
            description,
            sprites,
        });
        off = cursor;
    }
    if rows.is_empty() {
        return Err(FileError::TooShort(bytes.len()));
    }
    let declared = read_u32le(bytes, 0)?;
    if declared != rows.len() as u32 + 1 {
        return Err(unknown_format(&bytes[..4.min(bytes.len())]));
    }
    Ok(AvatarItemTable { rows })
}

/// Parse a `JamTable.dat` gauge table: flat little-endian `u16`
/// combo thresholds (54 entries, `25..4800`, on the real 7.55 file;
/// docs/file-formats/opi.md). Requires a non-empty even length.
///
/// # Errors
///
/// `FileError::TooShort` on empty input, `FileError::UnknownFormat`
/// on odd lengths.
pub fn parse_jam_thresholds(bytes: &[u8]) -> Result<Vec<u16>, FileError> {
    if bytes.len() < 2 {
        return Err(FileError::TooShort(bytes.len()));
    }
    if !bytes.len().is_multiple_of(2) {
        return Err(unknown_format(bytes));
    }
    Ok(bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|w| u16::from_le_bytes(*w))
        .collect())
}

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

/// Key-name table entries in the 7.55 client
/// (exe file `0x2454d8..0x2455dc`).
pub const KEY_NAME_COUNT: usize = 65;

/// Bytes per key-name table entry (one `u32` string VA).
pub const KEY_NAME_ENTRY_LEN: usize = 4;

/// One `OJNList.dat` patch-index pair: marker word plus song id.
///
/// The real 7.55 file holds 561 pairs (ids 886..100, sparse) in the
/// same order as the patch records (docs/file-formats/ojn.md).
/// Marker semantics are OPEN.
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

/// Resolve one key-name VA into a pool offset.
fn va_offset(va: u32, pool_base: u32, have: usize) -> Result<usize, FileError> {
    let relative = va.checked_sub(pool_base).ok_or(FileError::UnknownFormat {
        magic: va.to_le_bytes().to_vec(),
    })?;
    let off = usize::try_from(relative).map_err(|_| FileError::Truncated {
        need: usize::MAX,
        have,
    })?;
    if off >= have {
        return Err(FileError::Truncated {
            need: off + 1,
            have,
        });
    }
    Ok(off)
}

/// Parse the client's key-name table: `KEY_NAME_COUNT` little-endian
/// string VAs plus the pool they point into. Each target is a
/// NUL-terminated CP949 (EUC-KR) name; table order is preserved, so
/// aliased entries decode to equal strings (entries 19/50/57 share
/// one target on the real 7.55 file).
///
/// `pool_base` is the VA of `pool[0]` (image base + file offset).
///
/// # Errors
///
/// `FileError::Truncated` on a short table, a VA outside the pool,
/// or an unterminated name; `FileError::UnknownFormat` on a long
/// table, a VA below the pool, or undecodable text.
pub fn parse_key_name_table(
    table: &[u8],
    pool: &[u8],
    pool_base: u32,
) -> Result<Vec<String>, FileError> {
    let need = KEY_NAME_COUNT * KEY_NAME_ENTRY_LEN;
    if table.len() < need {
        return Err(FileError::Truncated {
            need,
            have: table.len(),
        });
    }
    if table.len() > need {
        return Err(unknown_format(table));
    }
    table
        .as_chunks::<KEY_NAME_ENTRY_LEN>()
        .0
        .iter()
        .map(|entry| {
            let va = u32::from_le_bytes(*entry);
            let off = va_offset(va, pool_base, pool.len())?;
            let end = pool[off..]
                .iter()
                .position(|b| *b == 0)
                .map(|len| off + len)
                .ok_or(FileError::Truncated {
                    need: off + 1,
                    have: pool.len(),
                })?;
            decode_control_text(&pool[off..end])
        })
        .collect()
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

    const KEY_POOL_BASE: u32 = 0x400000;

    fn key_name_fixture() -> (Vec<u8>, Vec<u8>) {
        // Pool: `B`, then CP949 `\xc0\xfc\xc3\xbc` ("all"),
        // then `A`; entries 1 and 2 alias one target like the real
        // entries 19/50/57.
        let pool = b"B\x00\xc0\xfc\xc3\xbc\x00A\x00".to_vec();
        let name_all = KEY_POOL_BASE + 2;
        let mut table = Vec::with_capacity(KEY_NAME_COUNT * KEY_NAME_ENTRY_LEN);
        for index in 0..KEY_NAME_COUNT {
            let va = match index {
                0 => KEY_POOL_BASE,
                1 | 2 => name_all,
                _ => KEY_POOL_BASE + 7,
            };
            table.extend_from_slice(&va.to_le_bytes());
        }
        (table, pool)
    }

    #[test]
    fn key_names_decode_letters_and_keep_aliases() {
        let (table, pool) = key_name_fixture();
        let names = parse_key_name_table(&table, &pool, KEY_POOL_BASE).unwrap();
        assert_eq!(names.len(), KEY_NAME_COUNT);
        assert_eq!(names[0], "B");
        assert_eq!(names[1], names[2]);
        assert_eq!(names[3], "A");
    }

    #[test]
    fn key_names_decode_korean_entry() {
        let (table, pool) = key_name_fixture();
        let names = parse_key_name_table(&table, &pool, KEY_POOL_BASE).unwrap();
        assert_eq!(names[1], "\u{c804}\u{ccb4}");
    }

    #[test]
    fn key_names_reject_bad_table_length() {
        let (table, pool) = key_name_fixture();
        assert!(matches!(
            parse_key_name_table(&table[..table.len() - 1], &pool, KEY_POOL_BASE),
            Err(FileError::Truncated { .. })
        ));
        let mut long = table.clone();
        long.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            parse_key_name_table(&long, &pool, KEY_POOL_BASE),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn key_names_reject_bad_offsets() {
        let (mut table, pool) = key_name_fixture();
        table[0..4].copy_from_slice(&(KEY_POOL_BASE - 1).to_le_bytes());
        assert!(matches!(
            parse_key_name_table(&table, &pool, KEY_POOL_BASE),
            Err(FileError::UnknownFormat { .. })
        ));
        let (mut table, pool) = key_name_fixture();
        let past_end = KEY_POOL_BASE + u32::try_from(pool.len()).unwrap();
        table[0..4].copy_from_slice(&past_end.to_le_bytes());
        assert!(matches!(
            parse_key_name_table(&table, &pool, KEY_POOL_BASE),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn key_names_reject_unterminated_name() {
        let (table, _) = key_name_fixture();
        let pool = b"B\xc0\xfc".to_vec();
        assert!(matches!(
            parse_key_name_table(&table, &pool, KEY_POOL_BASE),
            Err(FileError::Truncated { .. })
        ));
    }
}
