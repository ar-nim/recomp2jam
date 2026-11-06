//! Data tables: avatar outfit sets, avatar shop items, the jam gauge
//! thresholds, the `OJNList.dat` song index (records, middle tables,
//! patch index/records), and the key-name string table.
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

/// Byte length of the `OJNList.dat` middle u32 region
/// (`[168600, 195616)`, between the song records and the patch index).
/// Layout: 8-byte header, table 1, table 2, table 3, table 4, table 5,
/// 40-byte footer — the parts below add up exactly
/// (`8 + 3264 + 11244 + 6516 + 1440 + 4504 + 40`; docs/file-formats/ojn.md).
pub const OJN_MID_REGION_LEN: usize = 27016;

/// Magic heading the middle region (first header word on the real file).
pub const OJN_MID_MAGIC: u32 = 63540;

/// Entries in middle table 1 (second header word on the real file).
pub const OJN_MID_TABLE1_COUNT: usize = 204;

/// Bytes per middle-table-1 entry (`song_id, 0, param, 0`).
pub const OJN_MID_TABLE1_ENTRY_LEN: usize = 16;

/// Song slots in middle tables 2 and 5 (and the footer echo).
pub const OJN_MID_SONG_COUNT: usize = 562;

/// Newest song id (middle-table-2/5 head id and footer echo).
pub const OJN_MID_TOP_SONG_ID: u32 = 887;

/// Bytes per middle-table-2 entry (`song_id, listing, 0, param_b, 0`).
pub const OJN_MID_TABLE2_ENTRY_LEN: usize = 20;

/// Entries in middle table 3.
pub const OJN_MID_TABLE3_COUNT: usize = 407;

/// Bytes per middle-table-3 entry (`song_id, param_a, param_b, anchor`).
pub const OJN_MID_TABLE3_ENTRY_LEN: usize = 16;

/// Constant closing every middle-table-3 entry on the real file
/// (`0x77D2E4B7`, heap-address range — residue candidate, semantics OPEN).
pub const OJN_MID_TABLE3_ANCHOR: u32 = 2010252407;

/// Triples in middle table 4.
pub const OJN_MID_TABLE4_COUNT: usize = 120;

/// Bytes per middle-table-4 triple.
pub const OJN_MID_TABLE4_ENTRY_LEN: usize = 12;

/// Marker heading middle table 5 (word before its song count).
pub const OJN_MID_TABLE5_HEAD: u32 = 1;

/// Bytes per middle-table-5 entry (`song_id, category`).
pub const OJN_MID_TABLE5_ENTRY_LEN: usize = 8;

/// Bytes of the middle-region footer (closes with the
/// `OJN_MID_SONG_COUNT`/`OJN_MID_TOP_SONG_ID` echo).
pub const OJN_MID_FOOTER_LEN: usize = 40;

/// One middle-table-1 entry: song id plus a group word.
///
/// The real 7.55 file holds 204 entries: group 1 covers newer ids
/// (887..774, 108 rows), group 2 older ids (506..100, 96 rows). The
/// loader files the three trailing words into song-record slots
/// `+0x144/+0x148/+0x14c` (stride `0x1b4`; stores at file offsets
/// `0x7a390`/`0x7a3ed`/`0x7a44a`). A full-binary objdump survey finds
/// no song-stride (`imul 0x1b4`) read of these slots — the only indexed
/// accesses are the three stores, and same-displacement hits elsewhere
/// use other strides on unrelated structs — so the group word is
/// write-only through the song vector and its meaning stays OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidGroupEntry {
    /// Song id this entry belongs to.
    pub song_id: u32,
    /// Group word (1 or 2 on the real file).
    pub group: u32,
}

/// One middle-table-2 entry: song id plus the selection class and a
/// second tag word.
///
/// The real 7.55 file holds 562 entries (ids 887..100, strictly
/// descending — the reversed song-record order): `listing` is 1
/// (545x) or 2 (17x: ids 385..395, 426, 427, 693..700, all absent
/// from table 3), `param_b` is 0 (512x) or 1 (50x).
///
/// The loader files the four words into song-record slots
/// `+0x16c/+0x170/+0x174/+0x178` (stride `0x1b4`; stores at file
/// offsets `0x7a5c9`/`0x7a629`/`0x7a689`/`0x7a6e9`). Exe consumers pin
/// `listing` down: one song filter branches on `listing == 1` versus
/// the rest (file offset `0x3706f`), and several passes skip songs
/// whose word reads zero (e.g. file offset `0xadc68`). The non-1 arm
/// additionally requires `+0x178 != 0` (file offset `0x37261`), which
/// never holds on real data (both reserved words validate zero), so
/// class 2 never passes that selector. `param_b` has no calling read
/// site yet — OPEN. (Static disassembly survey; REA dossier TBD.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidSongEntry {
    /// Song id this entry belongs to.
    pub song_id: u32,
    /// Selection class (1 = standard path, 2 = 17-song special class
    /// routed away from the `listing == 1` selector; see struct docs).
    pub listing: u32,
    /// Second tag word (0 or 1 on the real file).
    pub param_b: u32,
}

/// One middle-table-3 entry: song id plus two param words plus the
/// constant anchor.
///
/// The real 7.55 file holds 407 entries (ids 887..102, sparse,
/// strictly descending): `param_a` reads 0..5000 and `param_b`
/// 0..30000, always in round hundreds. The loader files the three
/// words into song-record slots `+0x150/+0x154/+0x158` (stride `0x1b4`;
/// stores at file offsets `0x7a84b`/`0x7a8ab`/`0x7a90b`). Exe sites gate
/// on `song+0x150 != 0` (e.g. file offsets `0xdd6b6`, `0xe54f0`,
/// `0xeca0c`, `0xecb38`): a nonzero `param_a` enables a per-song
/// feature call, while zero — all 155 row-less songs and rows with
/// `param_a == 0` such as song 887 — skips it. The gated feature's
/// meaning, `param_b`, and the anchor stay OPEN.
/// (Static disassembly survey; REA dossier TBD.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidParamEntry {
    /// Song id this entry belongs to.
    pub song_id: u32,
    /// First param word.
    pub param_a: u32,
    /// Second param word.
    pub param_b: u32,
    /// Closing word (always `OJN_MID_TABLE3_ANCHOR`).
    pub anchor: u32,
}

/// One middle-table-4 triple.
///
/// The real 7.55 file holds 120 triples: triple 0 is the
/// `(120, 246, 2)` header-ish word set, triples 1..119 run in
/// 39/40/40 groups over one shared 40-song id set (mostly
/// `(0/1, id, 0/1/2)` shapes; the head triple's `b = 246` completes the
/// 40-set). The loader files each triple into song-record slots
/// `+0x184/+0x188/+0x18c` (stride `0x1b4`; stores at file offsets
/// `0x7aa6d`/`0x7aacd`/`0x7ab2d`), keyed by song-id match. A full-binary
/// objdump survey finds no song-stride (`imul 0x1b4`) read of these
/// slots — the only indexed accesses are the three stores, and
/// same-displacement hits elsewhere use `shl 4` (x16) strides on
/// unrelated structs — so all three words are write-only through the
/// song vector and their semantics stay OPEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidTriple {
    /// First word.
    pub a: u32,
    /// Second word (a song id outside the head triple).
    pub b: u32,
    /// Third word.
    pub c: u32,
}

/// One middle-table-5 entry: song id plus the label bucket.
///
/// The real 7.55 file holds 562 entries (ids 887..100, strictly
/// descending — the same id set as table 2): `category` runs 1..=5
/// (78x1 GOLD, 82x2 BLACK, 157x3 BLUE, 52x4 RED, 193x5 SILVER;
/// see label order below). The loader stores the pair at song-record
/// `+0x190` (id echo) / `+0x194` (file offsets `0x7ac8f` / `0x7acef`,
/// VAs `0x47ac8f` / `0x47acef`).
///
/// Four exe sites jump-table on `song+0x194 - 1` (range-check `> 4`)
/// and append the song-record pointer into parallel wheel lists
/// (vectors of song pointers, stride `0x1b4` records): file `0xbfa5c`
/// (VA `0x4bfa5c`, table file `0xbff7c`) and VA `0x493409`
/// (file `0x93409`, table VA `0x493a3c`) append via `0x47dd60`
/// into the global heads (VAs `0x77953c..0x77957c` in category order),
/// file `0x16121e` (VA `0x56121e`, table file `0x162e2c`) via
/// `0x454b90`/`0x4552b0`, and VA `0x4cc82f` (file `0x8c82f`, table VA
/// `0x4cc964`) into dialog-relative pointer lists (`+0x1d0`/`+0x1e0`/
/// `+0x1f0` family). `category` is the song-wheel label bucket.
///
/// The select-music dialog filters inline `0x238`-byte entries (NOT the
/// wheel pointers) through chained result lists: master `+0x640`
/// → STEP1 level bands (`+0x212`, VA `0x446950`/file `0x46950`) →
/// `+0x650` → STEP2 labels (`+0x4`, VA `0x447050`/file `0x47050`,
/// 6-arm table VA `0x447718`/file `0x47718`, arms at files
/// `0x472c1`/`0x472fa`/`0x47333`/`0x47369`/`0x4739f` testing
/// `entry+0x4 == 1..=5`) → `+0x660` → STEP3 genre (`+0x214`, VA
/// `0x447730`/file `0x47730`) → `+0x670`. Copies use `0x452850`,
/// clears use `0x452a00`. The STEP2 handler (VA `0x43f7e3`/file
/// `0x3f7e3`) stores `code - 0x2720B021` at dialog `+0x6c0`.
///
/// The entry `+0x4` writer is pinned: the `0x4441f0` entry builder
/// (file `0x441f0`) stores `entry+0x4 = song+0x194` at VA `0x44422e`
/// (file `0x4422e`) from the by-value `0x1b4` song-record copy, then
/// appends the `0x238` entry via `0x452850` into dialog `+0x640` at
/// VA `0x4445cd` (file `0x445cd`) or dialog `+0x680` at VA `0x4445e7`
/// (file `0x445e7`). Dialog `+0x690` producers are five `0x452850`
/// appends in the song-entry builder: files `0x4298d` / `0x429e3` /
/// `0x42c05` / `0x42de8` / `0x42e13` (docs/file-formats/ojn.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidTagEntry {
    /// Song id this entry belongs to.
    pub song_id: u32,
    /// Song-wheel list bucket (1..=5 on the real file; indexes the five
    /// `0x77953c..0x77957c` lists via `category - 1`).
    pub category: u32,
}

/// The decoded `OJNList.dat` middle region: header tables plus footer.
///
/// Resolved against exe consumers (see each entry's docs): table 5 is
/// the song-wheel label bucket (STEP2 label filter), table 2's first
/// word is a selection class, table 3's first word is a nonzero-gated
/// feature switch.
/// Table 1 groups, table 2's second word, table 3's second word and
/// anchor, and all of table 4 stay OPEN (table 1 and table 4 are
/// write-only: stored by the loader, never read back — see entry docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjnMidRegion {
    /// Table 1 group entries in file order.
    pub groups: Vec<OjnMidGroupEntry>,
    /// Table 2 song entries in file order.
    pub songs: Vec<OjnMidSongEntry>,
    /// Table 3 param entries in file order.
    pub params: Vec<OjnMidParamEntry>,
    /// Table 4 triples in file order.
    pub triples: Vec<OjnMidTriple>,
    /// Table 5 bucket entries in file order.
    pub tags: Vec<OjnMidTagEntry>,
    /// Footer words (last two echo `OJN_MID_SONG_COUNT` /
    /// `OJN_MID_TOP_SONG_ID`).
    pub footer: [u32; 10],
}

/// Compare a file `u32` count against an expected table length
/// without narrowing casts (PERF-07).
fn count_matches(actual: u32, expected: usize) -> bool {
    u32::try_from(expected).is_ok_and(|want| actual == want)
}

/// Split `len` bytes off the cursor; errors when they overrun.
fn take_region<'bytes>(
    bytes: &'bytes [u8],
    cursor: &mut usize,
    len: usize,
) -> Result<&'bytes [u8], FileError> {
    let end = cursor.checked_add(len).ok_or(FileError::Truncated {
        need: usize::MAX,
        have: bytes.len(),
    })?;
    let window = bytes.get(*cursor..end).ok_or(FileError::Truncated {
        need: end,
        have: bytes.len(),
    })?;
    *cursor = end;
    Ok(window)
}

/// Parse middle table 1: `OJN_MID_TABLE1_COUNT` entries of
/// (`song_id`, 0, `group`, 0). Requires the exact table length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table, a nonzero reserved word, or a group outside 1..=2.
pub fn parse_ojn_mid_table1(bytes: &[u8]) -> Result<Vec<OjnMidGroupEntry>, FileError> {
    let need = OJN_MID_TABLE1_COUNT * OJN_MID_TABLE1_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let mut entries = Vec::with_capacity(OJN_MID_TABLE1_COUNT);
    let mut off = 0;
    while off < need {
        let song_id = read_u32le(bytes, off)?;
        let reserved_a = read_u32le(bytes, off + 4)?;
        let group = read_u32le(bytes, off + 8)?;
        let reserved_b = read_u32le(bytes, off + 12)?;
        if reserved_a != 0 || reserved_b != 0 || group == 0 || group > 2 {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        entries.push(OjnMidGroupEntry { song_id, group });
        off += OJN_MID_TABLE1_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse middle table 2: `u32` count plus `OJN_MID_SONG_COUNT` entries
/// of (`song_id`, `listing`, 0, `param_b`, 0). Requires the exact
/// table length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table, a wrong count, a nonzero reserved word, or a class
/// outside its observed domain (`listing` 1..=2, `param_b` 0..=1).
pub fn parse_ojn_mid_table2(bytes: &[u8]) -> Result<Vec<OjnMidSongEntry>, FileError> {
    let need = 4 + OJN_MID_SONG_COUNT * OJN_MID_TABLE2_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let count = read_u32le(bytes, 0)?;
    if !count_matches(count, OJN_MID_SONG_COUNT) {
        return Err(unknown_format(&bytes[..4]));
    }
    let mut entries = Vec::with_capacity(OJN_MID_SONG_COUNT);
    let mut off = 4;
    while off < need {
        let song_id = read_u32le(bytes, off)?;
        let listing = read_u32le(bytes, off + 4)?;
        let reserved_a = read_u32le(bytes, off + 8)?;
        let param_b = read_u32le(bytes, off + 12)?;
        let reserved_b = read_u32le(bytes, off + 16)?;
        if reserved_a != 0 || reserved_b != 0 || listing == 0 || listing > 2 || param_b > 1 {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        entries.push(OjnMidSongEntry {
            song_id,
            listing,
            param_b,
        });
        off += OJN_MID_TABLE2_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse middle table 3: `u32` count plus `OJN_MID_TABLE3_COUNT`
/// entries of (`song_id`, `param_a`, `param_b`, `anchor`). Requires
/// the exact table length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table, a wrong count, or an anchor that is not
/// `OJN_MID_TABLE3_ANCHOR`.
pub fn parse_ojn_mid_table3(bytes: &[u8]) -> Result<Vec<OjnMidParamEntry>, FileError> {
    let need = 4 + OJN_MID_TABLE3_COUNT * OJN_MID_TABLE3_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let count = read_u32le(bytes, 0)?;
    if !count_matches(count, OJN_MID_TABLE3_COUNT) {
        return Err(unknown_format(&bytes[..4]));
    }
    let mut entries = Vec::with_capacity(OJN_MID_TABLE3_COUNT);
    let mut off = 4;
    while off < need {
        let song_id = read_u32le(bytes, off)?;
        let param_a = read_u32le(bytes, off + 4)?;
        let param_b = read_u32le(bytes, off + 8)?;
        let anchor = read_u32le(bytes, off + 12)?;
        if anchor != OJN_MID_TABLE3_ANCHOR {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        entries.push(OjnMidParamEntry {
            song_id,
            param_a,
            param_b,
            anchor,
        });
        off += OJN_MID_TABLE3_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse middle table 4: `OJN_MID_TABLE4_COUNT` raw triples.
/// Requires the exact table length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table.
pub fn parse_ojn_mid_table4(bytes: &[u8]) -> Result<Vec<OjnMidTriple>, FileError> {
    let need = OJN_MID_TABLE4_COUNT * OJN_MID_TABLE4_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let mut entries = Vec::with_capacity(OJN_MID_TABLE4_COUNT);
    let mut off = 0;
    while off < need {
        entries.push(OjnMidTriple {
            a: read_u32le(bytes, off)?,
            b: read_u32le(bytes, off + 4)?,
            c: read_u32le(bytes, off + 8)?,
        });
        off += OJN_MID_TABLE4_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse middle table 5: (`OJN_MID_TABLE5_HEAD`, `OJN_MID_SONG_COUNT`)
/// plus entries of (`song_id`, `category`). Requires the exact length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table, a wrong head/count, or a bucket outside 1..=5.
pub fn parse_ojn_mid_table5(bytes: &[u8]) -> Result<Vec<OjnMidTagEntry>, FileError> {
    let need = 8 + OJN_MID_SONG_COUNT * OJN_MID_TABLE5_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let head = read_u32le(bytes, 0)?;
    let count = read_u32le(bytes, 4)?;
    if head != OJN_MID_TABLE5_HEAD || !count_matches(count, OJN_MID_SONG_COUNT) {
        return Err(unknown_format(&bytes[..4]));
    }
    let mut entries = Vec::with_capacity(OJN_MID_SONG_COUNT);
    let mut off = 8;
    while off < need {
        let song_id = read_u32le(bytes, off)?;
        let category = read_u32le(bytes, off + 4)?;
        if category == 0 || category > 5 {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        entries.push(OjnMidTagEntry { song_id, category });
        off += OJN_MID_TABLE5_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse the `OJNList.dat` middle region: magic + table-1 count, the
/// five tables, and the footer. Requires exactly `OJN_MID_REGION_LEN`
/// bytes.
///
/// # Errors
///
/// `FileError::Truncated` on a short region, `FileError::UnknownFormat`
/// on a long region, a wrong magic/count, or any malformed table.
pub fn parse_ojn_mid_region(bytes: &[u8]) -> Result<OjnMidRegion, FileError> {
    if bytes.len() < OJN_MID_REGION_LEN {
        return Err(FileError::Truncated {
            need: OJN_MID_REGION_LEN,
            have: bytes.len(),
        });
    }
    if bytes.len() > OJN_MID_REGION_LEN {
        return Err(unknown_format(bytes));
    }
    let magic = read_u32le(bytes, 0)?;
    let group_count = read_u32le(bytes, 4)?;
    if magic != OJN_MID_MAGIC || !count_matches(group_count, OJN_MID_TABLE1_COUNT) {
        return Err(unknown_format(&bytes[..4]));
    }
    let mut cursor = 8;
    let groups = parse_ojn_mid_table1(take_region(
        bytes,
        &mut cursor,
        OJN_MID_TABLE1_COUNT * OJN_MID_TABLE1_ENTRY_LEN,
    )?)?;
    let songs = parse_ojn_mid_table2(take_region(
        bytes,
        &mut cursor,
        4 + OJN_MID_SONG_COUNT * OJN_MID_TABLE2_ENTRY_LEN,
    )?)?;
    let params = parse_ojn_mid_table3(take_region(
        bytes,
        &mut cursor,
        4 + OJN_MID_TABLE3_COUNT * OJN_MID_TABLE3_ENTRY_LEN,
    )?)?;
    let triples = parse_ojn_mid_table4(take_region(
        bytes,
        &mut cursor,
        OJN_MID_TABLE4_COUNT * OJN_MID_TABLE4_ENTRY_LEN,
    )?)?;
    let tags = parse_ojn_mid_table5(take_region(
        bytes,
        &mut cursor,
        8 + OJN_MID_SONG_COUNT * OJN_MID_TABLE5_ENTRY_LEN,
    )?)?;
    let footer_bytes = take_region(bytes, &mut cursor, OJN_MID_FOOTER_LEN)?;
    let mut footer = [0u32; 10];
    for (slot, word) in footer.iter_mut().enumerate() {
        *word = read_u32le(footer_bytes, slot * 4)?;
    }
    if !count_matches(footer[8], OJN_MID_SONG_COUNT) || footer[9] != OJN_MID_TOP_SONG_ID {
        return Err(unknown_format(&footer_bytes[..4]));
    }
    Ok(OjnMidRegion {
        groups,
        songs,
        params,
        triples,
        tags,
        footer,
    })
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

    fn full_table1_blob(group: u32) -> Vec<u8> {
        // Synthetic ids mirror the real shape (descending); values are
        // arbitrary but in-domain.
        let mut blob = Vec::with_capacity(OJN_MID_TABLE1_COUNT * OJN_MID_TABLE1_ENTRY_LEN);
        for song_id in (684..=887u32).rev() {
            blob.extend_from_slice(&song_id.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
            blob.extend_from_slice(&group.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_table1_parses_ids_and_groups() {
        let blob = full_table1_blob(2);
        let entries = parse_ojn_mid_table1(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_TABLE1_COUNT);
        assert_eq!(
            entries[0],
            OjnMidGroupEntry {
                song_id: 887,
                group: 2
            }
        );
        assert_eq!(entries[OJN_MID_TABLE1_COUNT - 1].group, 2);
    }

    #[test]
    fn mid_table1_rejects_bad_group() {
        let mut blob = full_table1_blob(1);
        blob[8..12].copy_from_slice(&3u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table1(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_table1_rejects_nonzero_reserved() {
        let mut blob = full_table1_blob(1);
        blob[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table1(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_table1_rejects_short_and_long_input() {
        let blob = full_table1_blob(1);
        assert!(matches!(
            parse_ojn_mid_table1(&blob[..blob.len() - 1]),
            Err(FileError::Truncated { .. })
        ));
        let mut long = blob.clone();
        long.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            parse_ojn_mid_table1(&long),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn full_table2_blob(listing: u32, param_b: u32) -> Vec<u8> {
        let mut blob = Vec::with_capacity(4 + OJN_MID_SONG_COUNT * OJN_MID_TABLE2_ENTRY_LEN);
        let count = u32::try_from(OJN_MID_SONG_COUNT).unwrap();
        blob.extend_from_slice(&count.to_le_bytes());
        for song_id in (326..=887u32).rev() {
            blob.extend_from_slice(&song_id.to_le_bytes());
            blob.extend_from_slice(&listing.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
            blob.extend_from_slice(&param_b.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_table2_parses_count_and_classes() {
        let blob = full_table2_blob(2, 1);
        let entries = parse_ojn_mid_table2(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_SONG_COUNT);
        assert_eq!(
            entries[0],
            OjnMidSongEntry {
                song_id: 887,
                listing: 2,
                param_b: 1
            }
        );
    }

    #[test]
    fn mid_table2_rejects_bad_count_and_classes() {
        let mut blob = full_table2_blob(1, 0);
        blob[0..4].copy_from_slice(&561u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table2(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        let mut blob = full_table2_blob(3, 0);
        assert!(matches!(
            parse_ojn_mid_table2(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        blob = full_table2_blob(1, 2);
        assert!(matches!(
            parse_ojn_mid_table2(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn full_table3_blob() -> Vec<u8> {
        let mut blob = Vec::with_capacity(4 + OJN_MID_TABLE3_COUNT * OJN_MID_TABLE3_ENTRY_LEN);
        let count = u32::try_from(OJN_MID_TABLE3_COUNT).unwrap();
        blob.extend_from_slice(&count.to_le_bytes());
        for song_id in (481..=887u32).rev() {
            blob.extend_from_slice(&song_id.to_le_bytes());
            blob.extend_from_slice(&2000u32.to_le_bytes());
            blob.extend_from_slice(&5000u32.to_le_bytes());
            blob.extend_from_slice(&OJN_MID_TABLE3_ANCHOR.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_table3_parses_params_and_anchor() {
        let blob = full_table3_blob();
        let entries = parse_ojn_mid_table3(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_TABLE3_COUNT);
        assert_eq!(
            entries[0],
            OjnMidParamEntry {
                song_id: 887,
                param_a: 2000,
                param_b: 5000,
                anchor: OJN_MID_TABLE3_ANCHOR
            }
        );
    }

    #[test]
    fn mid_table3_rejects_bad_anchor() {
        let mut blob = full_table3_blob();
        let last = blob.len() - 4;
        blob[last..].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table3(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_table4_parses_triples() {
        let mut blob = Vec::with_capacity(OJN_MID_TABLE4_COUNT * OJN_MID_TABLE4_ENTRY_LEN);
        for i in 0..u32::try_from(OJN_MID_TABLE4_COUNT).unwrap() {
            blob.extend_from_slice(&i.to_le_bytes());
            blob.extend_from_slice(&(873 - i).to_le_bytes());
            blob.extend_from_slice(&2u32.to_le_bytes());
        }
        let triples = parse_ojn_mid_table4(&blob).unwrap();
        assert_eq!(triples.len(), OJN_MID_TABLE4_COUNT);
        assert_eq!(triples[0], OjnMidTriple { a: 0, b: 873, c: 2 });
        assert_eq!(
            triples[OJN_MID_TABLE4_COUNT - 1],
            OjnMidTriple {
                a: 119,
                b: 754,
                c: 2
            }
        );
    }

    #[test]
    fn mid_table4_rejects_short_and_long_input() {
        let need = OJN_MID_TABLE4_COUNT * OJN_MID_TABLE4_ENTRY_LEN;
        assert!(matches!(
            parse_ojn_mid_table4(&vec![0u8; need - 4]),
            Err(FileError::Truncated { .. })
        ));
        assert!(matches!(
            parse_ojn_mid_table4(&vec![0u8; need + 4]),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn full_table5_blob(category: u32) -> Vec<u8> {
        let mut blob = Vec::with_capacity(8 + OJN_MID_SONG_COUNT * OJN_MID_TABLE5_ENTRY_LEN);
        blob.extend_from_slice(&OJN_MID_TABLE5_HEAD.to_le_bytes());
        let count = u32::try_from(OJN_MID_SONG_COUNT).unwrap();
        blob.extend_from_slice(&count.to_le_bytes());
        for song_id in (326..=887u32).rev() {
            blob.extend_from_slice(&song_id.to_le_bytes());
            blob.extend_from_slice(&category.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_table5_parses_ids_and_buckets() {
        let blob = full_table5_blob(5);
        let entries = parse_ojn_mid_table5(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_SONG_COUNT);
        assert_eq!(
            entries[0],
            OjnMidTagEntry {
                song_id: 887,
                category: 5
            }
        );
    }

    #[test]
    fn mid_table5_rejects_bad_head_count_and_bucket() {
        let mut blob = full_table5_blob(1);
        blob[0..4].copy_from_slice(&2u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table5(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        let mut blob = full_table5_blob(1);
        blob[8..12].copy_from_slice(&0u32.to_le_bytes());
        blob[12..16].copy_from_slice(&6u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table5(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_table5_category_indexes_five_lists() {
        // The client jump-tables on `category - 1` into five parallel
        // song lists (dispatches at file offsets `0xbfa5c` and
        // `0x16121e`): every stored bucket must index that table, so
        // only 1..=5 parses and each value minus one lands in 0..5.
        let blob = full_table5_blob(5);
        let entries = parse_ojn_mid_table5(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_SONG_COUNT);
        for entry in &entries {
            let bucket = entry.category.checked_sub(1).unwrap();
            assert!(bucket < 5);
        }
    }

    fn full_mid_region_blob() -> Vec<u8> {
        let mut blob = Vec::with_capacity(OJN_MID_REGION_LEN);
        blob.extend_from_slice(&OJN_MID_MAGIC.to_le_bytes());
        let group_count = u32::try_from(OJN_MID_TABLE1_COUNT).unwrap();
        blob.extend_from_slice(&group_count.to_le_bytes());
        blob.extend_from_slice(&full_table1_blob(1));
        blob.extend_from_slice(&full_table2_blob(1, 0));
        blob.extend_from_slice(&full_table3_blob());
        for _ in 0..OJN_MID_TABLE4_COUNT {
            blob.extend_from_slice(&0u32.to_le_bytes());
            blob.extend_from_slice(&246u32.to_le_bytes());
            blob.extend_from_slice(&2u32.to_le_bytes());
        }
        blob.extend_from_slice(&full_table5_blob(1));
        for word in [2u32, 173, 50, 0, 166, 50, 0, 0, 562, 887] {
            blob.extend_from_slice(&word.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_region_parses_all_tables_and_footer() {
        let blob = full_mid_region_blob();
        assert_eq!(blob.len(), OJN_MID_REGION_LEN);
        let region = parse_ojn_mid_region(&blob).unwrap();
        assert_eq!(region.groups.len(), OJN_MID_TABLE1_COUNT);
        assert_eq!(region.songs.len(), OJN_MID_SONG_COUNT);
        assert_eq!(region.params.len(), OJN_MID_TABLE3_COUNT);
        assert_eq!(region.triples.len(), OJN_MID_TABLE4_COUNT);
        assert_eq!(region.tags.len(), OJN_MID_SONG_COUNT);
        assert_eq!(region.groups[0].song_id, 887);
        assert_eq!(region.songs[0].song_id, 887);
        assert_eq!(region.params[0].anchor, OJN_MID_TABLE3_ANCHOR);
        assert_eq!(region.tags[0].category, 1);
        assert_eq!(region.footer[8], u32::try_from(OJN_MID_SONG_COUNT).unwrap());
        assert_eq!(region.footer[9], OJN_MID_TOP_SONG_ID);
    }

    #[test]
    fn mid_region_rejects_bad_magic_and_footer() {
        let mut blob = full_mid_region_blob();
        blob[0..4].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_region(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        let mut blob = full_mid_region_blob();
        let last = blob.len() - 4;
        blob[last..].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_region(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_region_rejects_short_and_long_input() {
        let blob = full_mid_region_blob();
        assert!(matches!(
            parse_ojn_mid_region(&blob[..blob.len() - 4]),
            Err(FileError::Truncated { .. })
        ));
        let mut long = blob.clone();
        long.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            parse_ojn_mid_region(&long),
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
