//! Avatar outfit sets, avatar shop items, and the jam gauge thresholds.
//!
//! Verified on real 7.55 files (docs/file-formats/opi.md).

use crate::{read_u32le, unknown_format, FileError};

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

impl AvatarItemRow {
    /// Costume bonuses inked by the 7.55 set-item card renderer.
    ///
    /// `FUN_00529230` prints record words `+0x24` and `+0x28` as the
    /// experience and jam-money percentages (REA `ev_ee6fcc7…fb`, file
    /// offsets `0x5294a0`/`0x5294ff`). The parser stores file bytes
    /// `+22` and `+26` inside the legacy `params`/`flags` fields, so
    /// rebuild those two words from the preserved prefix bytes.
    #[must_use]
    pub fn costume_bonus_percent(&self) -> (u32, u32) {
        let experience = (self.params[2] >> 8) | ((self.params[3] & 0xff) << 24);
        let jam_money = (self.params[3] >> 8) | (u32::from(self.flags.1) << 24);
        (experience, jam_money)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn row_with_prefix_bytes(prefix: [u8; 30]) -> AvatarItemRow {
        AvatarItemRow {
            item_id: 1,
            item_type: 0,
            price: 0,
            params: [
                read_u32le(&prefix, 8).unwrap(),
                read_u32le(&prefix, 17).unwrap(),
                read_u32le(&prefix, 21).unwrap(),
                read_u32le(&prefix, 25).unwrap(),
            ],
            flags: (prefix[12], prefix[29]),
            name: String::new(),
            description: String::new(),
            sprites: Vec::new(),
        }
    }

    #[test]
    fn installed_new_arrival_rows_carry_the_exe_card_percentages() {
        let Some(data_dir) = crate::data_root::pack_dir_for_test(
            &crate::data_root::install_root(),
            "new arrival card percentages",
        ) else {
            return; // gate printed SKIP
        };
        let archive_path = data_dir.join("Image/avatar.opa");
        let archive = std::fs::read(&archive_path)
            .unwrap_or_else(|error| panic!("{}: {error}", archive_path.display()));
        let entries = crate::parse_opi(&archive).expect("installed avatar archive parses");
        let table_entry = crate::find_opi_entry(&entries, "Itemdata_Korea.dat")
            .expect("installed avatar archive carries the item table");
        let body = crate::opi_entry_body(&archive, table_entry).expect("item table in bounds");
        let table = parse_avatar_item_table(body).expect("item table parses");
        for item_id in [1535, 1536, 1537, 1539, 1540, 1541] {
            let row = table
                .rows
                .iter()
                .find(|row| row.item_id == item_id)
                .expect("new-arrival row");
            assert_eq!(row.costume_bonus_percent(), (10, 10), "item {item_id}");
        }
    }

    #[test]
    fn costume_bonus_words_read_the_exe_pinned_prefix_offsets() {
        // The exe card renderer reads record `+0x24`/`+0x28`, which are
        // file bytes `+22`/`+26` of the 30-byte item prefix.
        let mut prefix = [0u8; 30];
        prefix[22] = 10; // experience percent
        prefix[26] = 20; // jam-money percent
        let row = row_with_prefix_bytes(prefix);
        assert_eq!(row.costume_bonus_percent(), (10, 20));
    }
}
