//! File parsing for O2Jam 7.55 assets (`.ojm` music, `.ojn` charts,
//! `OJNList.dat` song index).
//!
//! M30/OMC containers and the OJNList index are verified against real
//! 7.55 data (see `docs/file-formats/`); `.ojn` note streams stay stubbed
//! until the new-format decrypt lands.

use thiserror::Error;

/// Errors from file identification / parsing.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FileError {
    /// Input too short to even sniff.
    #[error("input too short: {0} bytes")]
    TooShort(usize),
    /// Magic not recognised (or parser not yet implemented for it).
    #[error("unknown format (magic: {magic:02X?})")]
    UnknownFormat {
        /// First bytes seen, for diagnostics.
        magic: Vec<u8>,
    },
    /// Recognised but not yet implemented — tracks RE progress honestly.
    #[error("not yet implemented: {0}")]
    NotImplemented(&'static str),
    /// Recognised header claims a length the buffer cannot satisfy.
    #[error("truncated: need {need} bytes, have {have}")]
    Truncated {
        /// Bytes required.
        need: usize,
        /// Bytes available.
        have: usize,
    },
}

/// Which asset container the bytes look like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    /// Candidate `.ojm` music container.
    Ojm,
    /// Candidate `.ojn` chart.
    Ojn,
    /// Candidate `OJNList.dat` song index.
    OjnList,
}

/// Sniff the first bytes of a buffer.
///
/// This is intentionally conservative: it only reports kinds whose magic
/// we already have reason to believe in, and says `UnknownFormat`
/// otherwise. Real magic tables get pinned down in `docs/file-formats/`.
pub fn detect_kind(bytes: &[u8]) -> Result<AssetKind, FileError> {
    if bytes.len() < 4 {
        return Err(FileError::TooShort(bytes.len()));
    }
    // Verified 7.55 magics (see docs/install-layout.md): M30, OMC, OJM.
    // M32 is community lore, kept as a candidate until observed.
    if bytes.starts_with(b"M30")
        || bytes.starts_with(b"M32")
        || bytes.starts_with(b"OMC")
        || bytes.starts_with(b"OJM")
    {
        return Ok(AssetKind::Ojm);
    }
    if bytes.starts_with(b"ojn") || bytes.starts_with(b"OJN") {
        return Ok(AssetKind::Ojn);
    }
    // Old-format `.ojn`: u32 song id, then `"ojn"` at offset 4
    // (verified: Tutorial.ojn starts `B8 22 00 00 6F 6A 6E 00`).
    if bytes.len() >= 8 && bytes[4..7] == *b"ojn" {
        return Ok(AssetKind::Ojn);
    }
    // `OJNList.dat`: u32 song count, u32 ?, then `"ojn"` at offset 8
    // (verified: starts `32 02 00 00 64 00 00 00 6F 6A 6E 00`).
    if bytes.len() >= 12 && bytes[8..11] == *b"ojn" {
        return Ok(AssetKind::OjnList);
    }
    Err(FileError::UnknownFormat {
        magic: bytes[..4.min(bytes.len())].to_vec(),
    })
}

/// One key-sound / BGM stream inside an `.ojm` container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjmSample<'a> {
    /// Sample name (NUL-trimmed ASCII).
    pub name: String,
    /// Sample id (`None` for variants that do not carry one, e.g. OMC).
    pub sample_id: Option<u32>,
    /// Raw OGG Vorbis bytes.
    pub data: &'a [u8],
}

fn read_u32le(bytes: &[u8], off: usize) -> Result<u32, FileError> {
    bytes
        .get(off..off + 4)
        .and_then(|w| w.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or(FileError::Truncated {
            need: off + 4,
            have: bytes.len(),
        })
}

fn read_name(bytes: &[u8], off: usize) -> Result<String, FileError> {
    let raw = bytes.get(off..off + 32).ok_or(FileError::Truncated {
        need: off + 32,
        have: bytes.len(),
    })?;
    let end = raw.iter().position(|&b| b == 0).unwrap_or(32);
    Ok(String::from_utf8_lossy(&raw[..end]).into_owned())
}

fn check_ogg(data: &[u8], at: usize) -> Result<(), FileError> {
    match data.get(at..at + 4) {
        Some(b"OggS") => Ok(()),
        _ => Err(FileError::UnknownFormat {
            magic: data.get(at..at + 4).unwrap_or_default().to_vec(),
        }),
    }
}

/// OPI/OPA archive layout, verified on real 7.55 files
/// (`Interface1.opi`, `Playing1.opi`, `avatar.opa`;
/// docs/file-formats/opi.md): 16-byte header (`u32` version +
/// `u32` slot count + `u64` zero), data blobs from +16, and a
/// trailing table of fixed-size entries. Versions observed: 2
/// (`.opi`), 1 (`.opa`); the layout is identical.
const OPI_HEADER_LEN: usize = 16;
const OPI_ENTRY_LEN: usize = 152;
const OPI_NAME_OFF: usize = 4;
const OPI_NAME_LEN: usize = 128;
const OPI_OFFSET_OFF: usize = 132;
const OPI_SIZE_OFF: usize = 136;
const OPI_RESERVED_OFF: usize = 140;

/// One file inside an OPI/OPA archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpiEntry {
    /// File name (NUL-trimmed ASCII: `*.ojs`, `*.bnd`, `*.txt`, …).
    pub name: String,
    /// Entry kind from the archive: 1 = file, 0 = free slot.
    pub kind: u32,
    /// Offset of the stored bytes from the archive start.
    pub data_offset: usize,
    /// Stored byte count: the body is always the first `size`
    /// bytes at `data_offset`.
    pub size: u32,
    /// Reserved on-disk stride: the entry's allocation when the
    /// archive was packed. The patcher can shrink `size` for a
    /// replacement payload while `reserved_size` keeps the original
    /// allocation, so slack bytes are stale data, not padding
    /// (docs/file-formats/opi.md). Readers must slice bodies by
    /// `size`, never by `reserved_size`.
    pub reserved_size: u32,
}

fn read_opi_name(bytes: &[u8], off: usize) -> Result<String, FileError> {
    let raw = bytes
        .get(off..off + OPI_NAME_LEN)
        .ok_or(FileError::Truncated {
            need: off + OPI_NAME_LEN,
            have: bytes.len(),
        })?;
    let end = raw.iter().position(|&b| b == 0).unwrap_or(OPI_NAME_LEN);
    Ok(String::from_utf8_lossy(&raw[..end]).into_owned())
}

/// One raw table slot in an OPI/OPA archive, including nameless free
/// slots (deleted-entry reservations: `kind` 0, empty name, `size` 0,
/// nonzero `data_offset`/`reserved_size`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpiSlot {
    /// File name (empty for free slots).
    pub name: String,
    /// Entry kind from the archive: 1 = file, 0 = free slot.
    pub kind: u32,
    /// Offset of the stored bytes (or reservation) from archive start.
    pub data_offset: usize,
    /// Stored byte count (0 for free slots).
    pub size: u32,
    /// Reserved on-disk stride to the next table slot's offset.
    pub reserved_size: u32,
}

impl OpiSlot {
    /// True for nameless deleted-entry reservations.
    pub fn is_free(&self) -> bool {
        self.name.is_empty()
    }
}

fn read_opi_slot(bytes: &[u8], entry_off: usize) -> Result<OpiSlot, FileError> {
    let kind = read_u32le(bytes, entry_off)?;
    let name = read_opi_name(bytes, entry_off + OPI_NAME_OFF)?;
    let data_offset = read_u32le(bytes, entry_off + OPI_OFFSET_OFF)? as usize;
    let size = read_u32le(bytes, entry_off + OPI_SIZE_OFF)?;
    let reserved_size = read_u32le(bytes, entry_off + OPI_RESERVED_OFF)?;
    let body_end = data_offset
        .checked_add(size as usize)
        .ok_or(FileError::Truncated {
            need: usize::MAX,
            have: bytes.len(),
        })?;
    if body_end > bytes.len() {
        return Err(FileError::Truncated {
            need: body_end,
            have: bytes.len(),
        });
    }
    let stride_end =
        data_offset
            .checked_add(reserved_size as usize)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: bytes.len(),
            })?;
    if stride_end > bytes.len() {
        return Err(FileError::Truncated {
            need: stride_end,
            have: bytes.len(),
        });
    }
    Ok(OpiSlot {
        name,
        kind,
        data_offset,
        size,
        reserved_size,
    })
}

/// Parse every OPI/OPA table slot in table order, including nameless
/// free slots. Verified counts on real 7.55 files
/// (docs/file-formats/opi.md): 1596 / 633 / 7149 slots.
/// Stored bytes are NOT validated; readers must slice bodies by
/// `size`, never by `reserved_size`.
pub fn parse_opi_slots(bytes: &[u8]) -> Result<Vec<OpiSlot>, FileError> {
    if bytes.len() < OPI_HEADER_LEN {
        return Err(FileError::TooShort(bytes.len()));
    }
    let slots = read_u32le(bytes, 4)? as usize;
    let table_len = slots
        .checked_mul(OPI_ENTRY_LEN)
        .ok_or(FileError::Truncated {
            need: usize::MAX,
            have: bytes.len(),
        })?;
    if table_len > bytes.len() || OPI_HEADER_LEN > bytes.len() - table_len {
        return Err(FileError::Truncated {
            need: table_len,
            have: bytes.len(),
        });
    }
    let mut out = Vec::with_capacity(slots.min(8192));
    let mut off = bytes.len() - table_len;
    for _ in 0..slots {
        out.push(read_opi_slot(bytes, off)?);
        off += OPI_ENTRY_LEN;
    }
    Ok(out)
}

/// True when the body looks like a complete JPEG (`FF D8 … FF D9`).
/// Verified on real 7.55 files: all 47 `ATI*`/`ati*` album-art `.ojs`
/// bodies in `Interface1.opi` match, and no body in the other two
/// archives starts with `FF D8`.
pub fn is_jpeg_body(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes.starts_with(&[0xFF, 0xD8]) && bytes.ends_with(&[0xFF, 0xD9])
}

/// Parse an OPI/OPA archive directory (trailing table). Returns one
/// entry per named file; nameless free slots are skipped. Stored
/// bytes are NOT validated (script, text, and binary bodies share
/// the container).
/// Verified on real 7.55 files (docs/file-formats/opi.md).
pub fn parse_opi(bytes: &[u8]) -> Result<Vec<OpiEntry>, FileError> {
    Ok(parse_opi_slots(bytes)?
        .into_iter()
        .filter(|slot| !slot.is_free())
        .map(|slot| OpiEntry {
            name: slot.name,
            kind: slot.kind,
            data_offset: slot.data_offset,
            size: slot.size,
            reserved_size: slot.reserved_size,
        })
        .collect())
}

/// Parse the M30 variant: 20-byte header, then inline 52-byte entries
/// (`name[32] + size + _ + _ + id + _`) each followed by its OGG.
/// Verified byte-exact on real 7.55 files (docs/file-formats/ojm.md).
pub fn parse_m30(bytes: &[u8]) -> Result<Vec<OjmSample<'_>>, FileError> {
    if bytes.len() < 20 {
        return Err(FileError::TooShort(bytes.len()));
    }
    if !bytes.starts_with(b"M30") {
        return Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        });
    }
    let count = read_u32le(bytes, 12)? as usize;
    let mut off = read_u32le(bytes, 16)? as usize;
    let mut out = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        let name = read_name(bytes, off)?;
        let size = read_u32le(bytes, off + 32)? as usize;
        let sample_id = read_u32le(bytes, off + 44)?;
        if size == 0 {
            // Degenerate entry: no stream follows, and accepting it
            // would stall the walk (end == start).
            return Err(FileError::Truncated {
                need: off + 52 + 1,
                have: bytes.len().min(off + 52),
            });
        }
        let start = off + 52;
        let end = start.checked_add(size).ok_or(FileError::Truncated {
            need: usize::MAX,
            have: bytes.len(),
        })?;
        if end > bytes.len() {
            return Err(FileError::Truncated {
                need: end,
                have: bytes.len(),
            });
        }
        check_ogg(bytes, start)?;
        out.push(OjmSample {
            name,
            sample_id: Some(sample_id),
            data: &bytes[start..end],
        });
        off = end;
    }
    if off != bytes.len() {
        return Err(FileError::Truncated {
            need: off,
            have: bytes.len(),
        });
    }
    Ok(out)
}

/// New-variant OMC/OJM table layout, verified against the in-exe
/// loader (`FUN_0054e930` reads counts at +4/+6 and seeks the offsets
/// at +8/+12; `FUN_0054eca0` reads `0x38`-byte mode-2 and `0x24`-byte
/// mode-3 entries) and on real 7.55 files (docs/file-formats/ojm.md).
/// Sample bytes follow each entry directly (`[entry, data]` chaining).
const MODE2_ENTRY_LEN: usize = 0x38;
const MODE3_ENTRY_LEN: usize = 0x24;
const MODE2_COUNT_OFF: usize = 4;
const MODE3_COUNT_OFF: usize = 6;
const MODE2_ENTRIES_OFF: usize = 8;
const MODE3_ENTRIES_OFF: usize = 12;
const MODE2_SIZE_OFF: usize = 52;
const MODE3_SIZE_OFF: usize = 32;

/// One entry of a new-variant OMC/OJM table: name, declared sample
/// size, and file offset of the (possibly still encrypted) bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjmEntry {
    /// Sample name (NUL-trimmed ASCII).
    pub name: String,
    /// Declared sample size in bytes.
    pub size: u32,
    /// File offset where the sample bytes start.
    pub data_offset: usize,
}

/// Both entry tables of a new-variant file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjmTables {
    /// Mode-2 entries (`0x38` bytes each, 32-byte names).
    pub mode2: Vec<OjmEntry>,
    /// Mode-3 entries (`0x24` bytes each, old 36-byte shape).
    pub mode3: Vec<OjmEntry>,
}

fn read_u16le(bytes: &[u8], off: usize) -> Result<u16, FileError> {
    bytes
        .get(off..off + 2)
        .and_then(|w| w.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or(FileError::Truncated {
            need: off + 2,
            have: bytes.len(),
        })
}

fn walk_entries(
    bytes: &[u8],
    mut off: usize,
    count: usize,
    entry_len: usize,
    size_off: usize,
) -> Result<Vec<OjmEntry>, FileError> {
    let mut out = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        let end = off.checked_add(entry_len).ok_or(FileError::Truncated {
            need: usize::MAX,
            have: bytes.len(),
        })?;
        if end > bytes.len() {
            return Err(FileError::Truncated {
                need: end,
                have: bytes.len(),
            });
        }
        let name = read_name(bytes, off)?;
        let size = read_u32le(bytes, off + size_off)?;
        let data_offset = end;
        let next = data_offset
            .checked_add(size as usize)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: bytes.len(),
            })?;
        if next > bytes.len() {
            return Err(FileError::Truncated {
                need: next,
                have: bytes.len(),
            });
        }
        out.push(OjmEntry {
            name,
            size,
            data_offset,
        });
        off = next;
    }
    Ok(out)
}

/// Parse the entry tables of a new-variant OMC/OJM file: counts at
/// +4/+6, table offsets at +8/+12, `[entry, data]` chaining. Sample
/// bytes are NOT validated (bodies may be encrypted); use
/// [`not_decrypt`] plus de-interleaving to recover them.
/// Verified on real 7.55 files (docs/file-formats/ojm.md).
pub fn parse_new_variant_entries(bytes: &[u8]) -> Result<OjmTables, FileError> {
    if bytes.len() < 16 {
        return Err(FileError::TooShort(bytes.len()));
    }
    if !bytes.starts_with(b"OMC") && !bytes.starts_with(b"OJM") {
        return Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        });
    }
    let mode2_count = read_u16le(bytes, MODE2_COUNT_OFF)? as usize;
    let mode3_count = read_u16le(bytes, MODE3_COUNT_OFF)? as usize;
    let mode2_off = read_u32le(bytes, MODE2_ENTRIES_OFF)? as usize;
    let mode3_off = read_u32le(bytes, MODE3_ENTRIES_OFF)? as usize;
    Ok(OjmTables {
        mode2: walk_entries(
            bytes,
            mode2_off,
            mode2_count,
            MODE2_ENTRY_LEN,
            MODE2_SIZE_OFF,
        )?,
        mode3: walk_entries(
            bytes,
            mode3_off,
            mode3_count,
            MODE3_ENTRY_LEN,
            MODE3_SIZE_OFF,
        )?,
    })
}

/// In-place bitwise-NOT stream decipher used for encrypted OJM
/// samples (`FUN_0054f9e0`): each byte is inverted when
/// `(key1 << key2) & 0x80` is set; `key2` cycles 0..7 and `key1`
/// tracks the previous ciphertext byte. The loader seeds both from
/// the stream rewind (`0xFF`, `0x00`).
/// Verified against real 7.55 bodies (docs/file-formats/ojm.md).
pub fn not_decrypt(buf: &mut [u8], mut key1: u8, mut key2: u8) {
    for byte in buf.iter_mut() {
        let cipher = *byte;
        if (key1 << key2) & 0x80 != 0 {
            *byte = !*byte;
        }
        if key2 >= 7 {
            key2 = 0;
            key1 = cipher;
        } else {
            key2 += 1;
        }
    }
}
/// De-interleave map, row-selected by `size % 16`.
/// Copied from the game executable (file offsets 0x27f1d0..+512) as
/// functional data required for interop; see docs/file-formats/ojm.md.
const DEINTERLEAVE_16: [[u16; 16]; 16] = [
    [14, 2, 9, 4, 0, 7, 1, 6, 8, 15, 10, 5, 12, 3, 13, 11], // row 0
    [7, 2, 10, 11, 3, 5, 13, 8, 4, 0, 12, 6, 15, 14, 1, 9], // row 1
    [12, 13, 3, 0, 6, 9, 10, 1, 7, 8, 2, 11, 14, 4, 15, 5], // row 2
    [8, 3, 4, 13, 6, 5, 11, 2, 12, 7, 9, 10, 15, 14, 0, 1], // row 3
    [15, 2, 12, 13, 0, 4, 1, 5, 7, 3, 9, 6, 11, 10, 8, 14], // row 4
    [0, 4, 11, 15, 13, 12, 6, 5, 7, 1, 2, 3, 8, 9, 10, 14], // row 5
    [3, 8, 7, 6, 9, 14, 13, 0, 10, 11, 4, 5, 12, 2, 1, 15], // row 6
    [4, 14, 15, 5, 8, 7, 11, 0, 1, 6, 2, 12, 9, 3, 10, 13], // row 7
    [6, 13, 14, 7, 10, 11, 0, 1, 12, 15, 2, 3, 8, 9, 4, 5], // row 8
    [10, 12, 0, 8, 9, 13, 3, 4, 5, 14, 15, 1, 2, 11, 6, 7], // row 9
    [5, 6, 12, 4, 13, 15, 7, 14, 8, 1, 9, 2, 10, 11, 0, 3], // row 10
    [11, 15, 4, 14, 3, 1, 0, 2, 13, 12, 6, 7, 5, 9, 8, 10], // row 11
    [3, 2, 1, 0, 4, 12, 13, 11, 5, 6, 15, 14, 7, 9, 10, 8], // row 12
    [9, 10, 0, 7, 8, 6, 3, 4, 1, 2, 5, 11, 14, 15, 13, 12], // row 13
    [10, 6, 9, 12, 11, 7, 8, 0, 15, 3, 1, 2, 5, 13, 14, 4], // row 14
    [13, 0, 1, 14, 2, 3, 8, 11, 7, 12, 9, 5, 10, 15, 4, 6], // row 15
];

/// De-interleave map, row-selected by `size % 17`.
/// Copied from the game executable (file offsets 0x27f3d0..+578) as
/// functional data required for interop; see docs/file-formats/ojm.md.
const DEINTERLEAVE_17: [[u16; 17]; 17] = [
    [16, 14, 2, 9, 4, 0, 7, 1, 6, 8, 15, 10, 5, 12, 3, 13, 11], // row 0
    [7, 2, 10, 11, 3, 5, 13, 8, 4, 0, 12, 6, 15, 14, 16, 1, 9], // row 1
    [12, 13, 3, 0, 6, 9, 10, 1, 7, 8, 16, 2, 11, 14, 4, 15, 5], // row 2
    [8, 3, 4, 13, 6, 5, 11, 16, 2, 12, 7, 9, 10, 15, 14, 0, 1], // row 3
    [15, 2, 12, 13, 0, 4, 1, 5, 7, 3, 9, 16, 6, 11, 10, 8, 14], // row 4
    [0, 4, 11, 16, 15, 13, 12, 6, 5, 7, 1, 2, 3, 8, 9, 10, 14], // row 5
    [3, 16, 8, 7, 6, 9, 14, 13, 0, 10, 11, 4, 5, 12, 2, 1, 15], // row 6
    [4, 14, 16, 15, 5, 8, 7, 11, 0, 1, 6, 2, 12, 9, 3, 10, 13], // row 7
    [6, 13, 14, 7, 16, 10, 11, 0, 1, 12, 15, 2, 3, 8, 9, 4, 5], // row 8
    [10, 12, 0, 8, 9, 13, 3, 4, 5, 16, 14, 15, 1, 2, 11, 6, 7], // row 9
    [5, 6, 12, 4, 13, 15, 7, 14, 8, 1, 9, 2, 16, 10, 11, 0, 3], // row 10
    [11, 15, 4, 14, 3, 1, 0, 2, 13, 12, 6, 7, 5, 16, 9, 8, 10], // row 11
    [3, 2, 1, 0, 4, 12, 13, 11, 16, 5, 6, 15, 14, 7, 9, 10, 8], // row 12
    [9, 10, 0, 7, 8, 6, 16, 3, 4, 1, 2, 5, 11, 14, 15, 13, 12], // row 13
    [10, 6, 9, 12, 11, 16, 7, 8, 0, 15, 3, 1, 2, 5, 13, 14, 4], // row 14
    [13, 0, 1, 14, 2, 3, 8, 11, 7, 12, 9, 5, 10, 15, 4, 6, 16], // row 15
    [1, 14, 2, 3, 13, 11, 7, 0, 8, 12, 9, 6, 15, 16, 5, 10, 4], // row 16
];

/// Scatter de-interleave for encrypted OJM samples (`FUN_0054f860`
/// 16-way, `FUN_0054f920` 17-way): the file holds `lanes` stripes of
/// `size / lanes` bytes plus a `size % lanes` tail; stripe `i` lands at
/// plain offset `row[size % lanes][i]`. Only the two lane counts the
/// loader implements are accepted.
/// Verified on real 7.55 bodies (docs/file-formats/ojm.md).
pub fn deinterleave_stripes(data: &[u8], lanes: usize) -> Result<Vec<u8>, FileError> {
    let size = data.len();
    let row: &[u16] = match lanes {
        16 => &DEINTERLEAVE_16[size % 16],
        17 => &DEINTERLEAVE_17[size % 17],
        _ => {
            return Err(FileError::UnknownFormat {
                magic: data[..4.min(size)].to_vec(),
            });
        }
    };
    let quotient = size / lanes;
    let mut out = vec![0u8; size];
    for (stripe, slot) in row.iter().enumerate() {
        let from = stripe * quotient;
        let to = *slot as usize * quotient;
        out[to..to + quotient].copy_from_slice(&data[from..from + quotient]);
    }
    out[lanes * quotient..].copy_from_slice(&data[lanes * quotient..]);
    Ok(out)
}

/// Parse the old-variant OMC/OJM tables: 16-byte header with the
/// first entry offset at +12, then 36-byte entries (`name[32] +
/// size`) each followed by its OGG. Blank (all-zero) entries are
/// skipped. New-variant files (counts at +4/+6, `0x38`-byte
/// entries) need [`parse_new_variant_entries`] instead: their bodies
/// may be encrypted, so no OGG check applies here.
/// Verified byte-exact on real 7.55 files (docs/file-formats/ojm.md).
pub fn parse_omc(bytes: &[u8]) -> Result<Vec<OjmSample<'_>>, FileError> {
    if bytes.len() < 16 {
        return Err(FileError::TooShort(bytes.len()));
    }
    if !bytes.starts_with(b"OMC") && !bytes.starts_with(b"OJM") {
        return Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        });
    }
    let mut off = read_u32le(bytes, 12)? as usize;
    if off < 16 || off > bytes.len() {
        return Err(FileError::Truncated {
            need: off,
            have: bytes.len(),
        });
    }
    let mut out = Vec::new();
    loop {
        if off == bytes.len() {
            // Clean end (also covers fully encrypted bodies with no
            // raw streams: entry offset == file length).
            return Ok(out);
        }
        let name = read_name(bytes, off)?;
        let size = read_u32le(bytes, off + 32)? as usize;
        if name.is_empty() && size == 0 {
            // Blank slot: no stream follows; entry was padding.
            off += 36;
        } else {
            if size == 0 {
                // Degenerate entry: no stream follows, and accepting it
                // would stall the walk (end == start).
                return Err(FileError::Truncated {
                    need: off + 36 + 1,
                    have: bytes.len().min(off + 36),
                });
            }
            let start = off + 36;
            let end = start.checked_add(size).ok_or(FileError::Truncated {
                need: usize::MAX,
                have: bytes.len(),
            })?;
            if end > bytes.len() {
                return Err(FileError::Truncated {
                    need: end,
                    have: bytes.len(),
                });
            }
            check_ogg(bytes, start)?;
            out.push(OjmSample {
                name,
                sample_id: None,
                data: &bytes[start..end],
            });
            off = end;
        }
        if off == bytes.len() {
            return Ok(out);
        }
        if off > bytes.len() {
            return Err(FileError::Truncated {
                need: off,
                have: bytes.len(),
            });
        }
        if out.len() > 100_000 {
            return Err(FileError::UnknownFormat {
                magic: bytes[..4.min(bytes.len())].to_vec(),
            });
        }
        // Next entry header must fit.
        if off + 36 > bytes.len() {
            return Err(FileError::Truncated {
                need: off + 36,
                have: bytes.len(),
            });
        }
    }
}

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

impl<'a> OjnListEntry<'a> {
    /// High word of [`OjnListEntry::key_field`] (tracks the song id).
    pub fn key_hi(&self) -> u16 {
        (self.key_field >> 16) as u16
    }

    /// Low word of [`OjnListEntry::key_field`] (29 / 27).
    pub fn key_lo(&self) -> u16 {
        (self.key_field & 0xFFFF) as u16
    }

    /// Whether this record looks like a real song entry rather than a
    /// dead slot: the filename is an `o2ma###.ojm` token (any ASCII
    /// case; `read_str` already cut it at the first NUL) and the tempo
    /// is sane. Dead slots (157/719 in 7.55) hold heap residue or
    /// zeros and fail one or both checks.
    pub fn is_song(&self) -> bool {
        let lower = self.ojm_file.to_ascii_lowercase();
        let numbered = lower
            .strip_prefix("o2ma")
            .and_then(|rest| rest.strip_suffix(".ojm"))
            .map(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
            .unwrap_or(false);
        numbered && (30.0..300.0).contains(&self.bpm)
    }
}

fn read_f32le(bytes: &[u8], off: usize) -> Result<f32, FileError> {
    bytes
        .get(off..off + 4)
        .and_then(|w| w.try_into().ok())
        .map(f32::from_le_bytes)
        .ok_or(FileError::Truncated {
            need: off + 4,
            have: bytes.len(),
        })
}

fn read_str(bytes: &[u8], off: usize, len: usize) -> Result<String, FileError> {
    let raw = bytes.get(off..off + len).ok_or(FileError::Truncated {
        need: off + len,
        have: bytes.len(),
    })?;
    // C-string semantics: cut at the first NUL (stale tail bytes past
    // it stay visible in `OjnListEntry::raw`).
    let end = raw.iter().position(|&b| b == 0).unwrap_or(len);
    Ok(String::from_utf8_lossy(&raw[..end]).trim().to_string())
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
const OJN_EVENT_COUNT_OFFS: [usize; OJN_LEVEL_COUNT] = [64, 68, 72];
/// Header offsets of the per-level slice end offsets (`u32` each).
const OJN_SLICE_END_OFFS: [usize; OJN_LEVEL_COUNT] = [288, 292, 296];
/// Header offset of the first slice's start (`u32`).
const OJN_SLICES_START_OFF: usize = 284;
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

/// Decode Korean client text (`.ojs` word lists, `ControlList_*.txt`).
///
/// Real 7.55 bodies are CP949 (the MSVC2005 Korean-locale codepage):
/// strict EUC-KR rejects some bytes the client uses, UTF-8 rejects
/// nearly all high bytes, and CP949 decodes every text body without
/// error (docs/file-formats/opi.md). `encoding_rs::EUC_KR` implements
/// the WHATWG euc-kr index, which covers the CP949 repertoire.
/// Malformed input (unpaired lead byte) is an error, not lossy text.
pub fn decode_control_text(bytes: &[u8]) -> Result<String, FileError> {
    let (text, _, had_errors) = encoding_rs::EUC_KR.decode(bytes);
    if had_errors {
        return Err(unknown_format(bytes));
    }
    Ok(text.into_owned())
}

/// Magic of the dominant inner sprite container (`.ojs`, `.ojt`,
/// `.oji` bodies): `tag + 0x0555 + count + colorkey`, then `count`
/// 20-byte image records and one concatenated RGB555 pixel blob.
/// Field layout matches the independent CXO2 spec (their
/// signature/format/frames/colorkey); observed tags 0 and 1 parse
/// identically with exact accounting (CXO2 lists signature 1 as
/// valid — tag 0 is a live 7.55 variant, not corruption).
/// Observed colorkeys: `0x0000`, `0x7D5F`.
/// Verified on all ~9.2k such bodies in the real 7.55 archives
/// (docs/file-formats/opi.md).
pub const SPRITE55_MAGIC: u16 = 0x0555;
/// Sprite55 record length: `x + y + w + h + data_offset + size + 0`.
pub const SPRITE55_RECORD_LEN: usize = 20;
/// Sprite55 header length: `tag + magic + count + colorkey`.
pub const SPRITE55_HEADER_LEN: usize = 8;
/// Magic of the animation container (`.oja` bodies, tag + `0x555A`
/// + count): `count` 20-byte frame records follow the header.
///
/// Frame-data codec TBD; only the header and frame table are verified.
pub const OJA_MAGIC: u16 = 0x555A;
/// Oja header length: `tag + magic + count + reserved`.
pub const OJA_HEADER_LEN: usize = 8;
/// Oja frame-record length: `x + y + w + h + a + b + c`.
pub const OJA_RECORD_LEN: usize = 20;
/// Magic opening every `.bnd` bind section.
pub const BND_MAGIC: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
/// Bnd header length: `magic + count`.
pub const BND_HEADER_LEN: usize = 6;
/// Bnd record length: `(x1, y1, x2, y2)`.
pub const BND_RECORD_LEN: usize = 16;

/// Shorthand for an unrecognised magic / malformed line: the first
/// four bytes as the diagnostic sample.
fn unknown_format(sample: &[u8]) -> FileError {
    FileError::UnknownFormat {
        magic: sample[..4.min(sample.len())].to_vec(),
    }
}

/// Overflow-safe directory end: `header_len + count * record_len`,
/// bounded by the buffer.
fn dir_end(
    count: usize,
    record_len: usize,
    header_len: usize,
    have: usize,
) -> Result<usize, FileError> {
    let need = header_len
        .checked_add(count.checked_mul(record_len).ok_or(FileError::Truncated {
            need: usize::MAX,
            have,
        })?)
        .ok_or(FileError::Truncated {
            need: usize::MAX,
            have,
        })?;
    if need > have {
        return Err(FileError::Truncated { need, have });
    }
    Ok(need)
}

/// One image inside a sprite-55 container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite55Image {
    /// Placement anchor, pixels (often 0,0 for single images).
    pub x: u16,
    /// Placement anchor, pixels.
    pub y: u16,
    /// Width in pixels.
    pub w: u16,
    /// Height in pixels.
    pub h: u16,
    /// Byte offset of this image's data from the pixel-blob start
    /// (end of the record directory). Records chain exactly.
    pub data_offset: usize,
    /// Stored byte count of this image's data.
    pub size: u32,
}

/// A parsed sprite-55 container: header, record directory, and the
/// raw concatenated pixel blob (sliced per image on demand).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite55<'a> {
    /// Container tag (observed: 0, 1; meaning TBD).
    pub tag: u16,
    /// Transparent colour key (`0x0000` or `0x7D5F`).
    pub colorkey: u16,
    /// Image records in stored order.
    pub images: Vec<Sprite55Image>,
    /// Concatenated per-image data (raw RGB555 when
    /// [`Sprite55::is_raw_rgb555`] holds; codec TBD otherwise).
    pub blob: &'a [u8],
}

impl<'a> Sprite55<'a> {
    /// Raw stored bytes of one image.
    pub fn pixel_data(&self, image: &Sprite55Image) -> &'a [u8] {
        &self.blob[image.data_offset..image.data_offset + image.size as usize]
    }
    /// True when the image is stored as plain `w*h` little-endian
    /// RGB555 pixels (`size == w*h*2`; ~99.7% of real bodies).
    /// Outlier classes (`.oji` sub-directories/prefixed frames,
    /// RLE-compressed frames) return false and need their codec
    /// mapped before decoding. Strided combo atlases (see
    /// [`parse_sprite55_strided`]) store raw frames and return true.
    pub fn is_raw_rgb555(&self, image: &Sprite55Image) -> bool {
        image.size as usize == image.w as usize * image.h as usize * 2
    }
}

/// Header + record directory of a sprite-55 container, without the
/// pixel blob: shared contract of the plain and strided layouts.
struct Sprite55Dir {
    tag: u16,
    colorkey: u16,
    images: Vec<Sprite55Image>,
    dir_end: usize,
    stored: usize,
}

/// Read the header and chained record directory; the caller checks
/// the blob accounting of its own layout.
fn parse_sprite55_dir(bytes: &[u8]) -> Result<Sprite55Dir, FileError> {
    if bytes.len() < SPRITE55_HEADER_LEN {
        return Err(FileError::TooShort(bytes.len()));
    }
    let tag = read_u16le(bytes, 0)?;
    if read_u16le(bytes, 2)? != SPRITE55_MAGIC {
        return Err(unknown_format(bytes));
    }
    let count = read_u16le(bytes, 4)? as usize;
    let colorkey = read_u16le(bytes, 6)?;
    let dir_end = dir_end(count, SPRITE55_RECORD_LEN, SPRITE55_HEADER_LEN, bytes.len())?;
    let mut images = Vec::with_capacity(count.min(4096));
    let mut run = 0usize;
    for i in 0..count {
        let off = SPRITE55_HEADER_LEN + i * SPRITE55_RECORD_LEN;
        let image = Sprite55Image {
            x: read_u16le(bytes, off)?,
            y: read_u16le(bytes, off + 2)?,
            w: read_u16le(bytes, off + 4)?,
            h: read_u16le(bytes, off + 6)?,
            data_offset: read_u32le(bytes, off + 8)? as usize,
            size: read_u32le(bytes, off + 12)?,
        };
        if read_u32le(bytes, off + 16)? != 0 || image.data_offset != run {
            return Err(unknown_format(bytes));
        }
        run = run
            .checked_add(image.size as usize)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: bytes.len(),
            })?;
        images.push(image);
    }
    Ok(Sprite55Dir {
        tag,
        colorkey,
        images,
        dir_end,
        stored: run,
    })
}

/// Parse a sprite-55 container: magic, chained record directory,
/// exact total accounting (`8 + 20*count + blob == len`).
/// Verified on all real 7.55 `.ojs`/`.ojt`/`.oji` sprite bodies
/// except the three `Note_Combo*.ojt` strided atlases (see
/// [`parse_sprite55_strided`]).
pub fn parse_sprite55(bytes: &[u8]) -> Result<Sprite55<'_>, FileError> {
    let dir = parse_sprite55_dir(bytes)?;
    if dir.dir_end + dir.stored != bytes.len() {
        return Err(FileError::Truncated {
            need: dir.dir_end + dir.stored,
            have: bytes.len(),
        });
    }
    Ok(Sprite55 {
        tag: dir.tag,
        colorkey: dir.colorkey,
        images: dir.images,
        blob: &bytes[dir.dir_end..],
    })
}

/// A parsed strided sprite-55 atlas: same header and record table
/// as [`Sprite55`], but each image stores `frames_per_image`
/// consecutive `w*h` RGB555 frames (`Note_Combo4/5/6.ojt` hold 4
/// glow-animation frames per 64x64 record; verified by rendering
/// one centred digit band per 8 KiB tile).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite55Strided<'a> {
    /// Container tag (observed: 1).
    pub tag: u16,
    /// Transparent colour key (observed: `0x0000`).
    pub colorkey: u16,
    /// Image records in stored order (`size` = one frame).
    pub images: Vec<Sprite55Image>,
    /// Frames stored per image.
    pub frames_per_image: usize,
    /// Concatenated frame data.
    pub blob: &'a [u8],
}

impl<'a> Sprite55Strided<'a> {
    /// Raw stored bytes of one animation frame.
    pub fn frame_data(&self, image: &Sprite55Image, frame: usize) -> &'a [u8] {
        let start = image.data_offset * self.frames_per_image + frame * image.size as usize;
        &self.blob[start..start + image.size as usize]
    }

    /// True when one frame is stored as plain `w*h` little-endian
    /// RGB555 pixels (`size == w*h*2`).
    pub fn is_raw_rgb555(&self, image: &Sprite55Image) -> bool {
        image.size as usize == image.w as usize * image.h as usize * 2
    }
}

/// Parse a strided sprite-55 atlas: shared header/record contract
/// plus `frames_per_image * stored == blob` accounting. Rejects
/// `frames_per_image == 0`.
pub fn parse_sprite55_strided(
    bytes: &[u8],
    frames_per_image: usize,
) -> Result<Sprite55Strided<'_>, FileError> {
    if frames_per_image == 0 {
        return Err(unknown_format(bytes));
    }
    let dir = parse_sprite55_dir(bytes)?;
    let blob_need = dir
        .stored
        .checked_mul(frames_per_image)
        .ok_or(FileError::Truncated {
            need: usize::MAX,
            have: bytes.len(),
        })?;
    if dir.dir_end + blob_need != bytes.len() {
        return Err(FileError::Truncated {
            need: dir.dir_end + blob_need,
            have: bytes.len(),
        });
    }
    Ok(Sprite55Strided {
        tag: dir.tag,
        colorkey: dir.colorkey,
        images: dir.images,
        frames_per_image,
        blob: &bytes[dir.dir_end..],
    })
}

/// Expand one little-endian RGB555 pixel to 24-bit RGB (replicated
/// low bits, the usual GDI 16-bit expansion; bit 15 unused).
/// Matches the CXO2-documented 16-bit format for magic `0x0555`.
pub fn rgb555_to_rgb888(pixel: u16) -> (u8, u8, u8) {
    let r5 = ((pixel >> 10) & 31) as u8;
    let g5 = ((pixel >> 5) & 31) as u8;
    let b5 = (pixel & 31) as u8;
    (
        (r5 << 3) | (r5 >> 2),
        (g5 << 3) | (g5 >> 2),
        (b5 << 3) | (b5 >> 2),
    )
}

/// One layout rectangle inside a `.bnd` bind section.
///
/// Corners in absolute coordinates: `(x1, y1)` top-left,
/// `(x2, y2)` bottom-right. Record `i` lays out the `i+1`-th
/// `IDC_` entry of the matching `ControlList` block in file order
/// (entry 0 is the dialog/state itself); verified on all 38
/// Interface blocks (docs/file-formats/opi.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BndRect {
    /// Left edge.
    pub x1: u32,
    /// Top edge.
    pub y1: u32,
    /// Right edge (exclusive).
    pub x2: u32,
    /// Bottom edge (exclusive).
    pub y2: u32,
}

/// A parsed `.bnd` bind section: magic + `count` corner rects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bnd {
    /// Layout rectangles in file order.
    pub rects: Vec<BndRect>,
}

/// Parse a `.bnd` bind section: `0xFFFFFFFF` magic, `u16` count,
/// then `count` 16-byte `(x1, y1, x2, y2)` records; the size must
/// match exactly (`6 + 16*count`). Verified on all 53 real 7.55
/// `.bnd` sections.
pub fn parse_bnd(bytes: &[u8]) -> Result<Bnd, FileError> {
    if bytes.len() < BND_HEADER_LEN {
        return Err(FileError::TooShort(bytes.len()));
    }
    if bytes[0..4] != BND_MAGIC {
        return Err(unknown_format(bytes));
    }
    let count = read_u16le(bytes, 4)? as usize;
    let dir_end = dir_end(count, BND_RECORD_LEN, BND_HEADER_LEN, bytes.len())?;
    if dir_end != bytes.len() {
        return Err(FileError::Truncated {
            need: dir_end,
            have: bytes.len(),
        });
    }
    let mut rects = Vec::with_capacity(count.min(4096));
    for i in 0..count {
        let off = BND_HEADER_LEN + i * BND_RECORD_LEN;
        rects.push(BndRect {
            x1: read_u32le(bytes, off)?,
            y1: read_u32le(bytes, off + 4)?,
            x2: read_u32le(bytes, off + 8)?,
            y2: read_u32le(bytes, off + 12)?,
        });
    }
    Ok(Bnd { rects })
}

/// One animation frame header inside an `.oja` container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjaFrame {
    /// Frame origin, pixels (may be negative: centred effects).
    pub x: i16,
    /// Frame origin, pixels.
    pub y: i16,
    /// Frame width, pixels.
    pub w: u16,
    /// Frame height, pixels.
    pub h: u16,
    /// Trailing word 1 (semantics TBD).
    pub a: u32,
    /// Trailing word 2 (semantics TBD).
    pub b: u32,
    /// Trailing word 3 (semantics TBD).
    pub c: u32,
}

/// A parsed `.oja` header + frame table. Only the envelope is
/// verified (magic, count, coherent frame geometry on real 7.55
/// `Note_Click.oja`: shrinking centred frames); the frame-data
/// codec and the `a`/`b`/`c` words are TBD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oja {
    /// Container tag (observed: 1).
    pub tag: u16,
    /// Frame headers in stored order.
    pub frames: Vec<OjaFrame>,
}

/// Parse an `.oja` envelope: magic + `count` 20-byte frame records
/// (`i16 x/y`, `u16 w/h`, three raw `u32` words).
pub fn parse_oja(bytes: &[u8]) -> Result<Oja, FileError> {
    if bytes.len() < OJA_HEADER_LEN {
        return Err(FileError::TooShort(bytes.len()));
    }
    let tag = read_u16le(bytes, 0)?;
    if read_u16le(bytes, 2)? != OJA_MAGIC {
        return Err(unknown_format(bytes));
    }
    let count = read_u16le(bytes, 4)? as usize;
    // Frame data stays unsliced (codec TBD); only the table must fit.
    let _ = dir_end(count, OJA_RECORD_LEN, OJA_HEADER_LEN, bytes.len())?;
    let mut frames = Vec::with_capacity(count.min(1024));
    for i in 0..count {
        let off = OJA_HEADER_LEN + i * OJA_RECORD_LEN;
        frames.push(OjaFrame {
            x: read_i16le(bytes, off)?,
            y: read_i16le(bytes, off + 2)?,
            w: read_u16le(bytes, off + 4)?,
            h: read_u16le(bytes, off + 6)?,
            a: read_u32le(bytes, off + 8)?,
            b: read_u32le(bytes, off + 12)?,
            c: read_u32le(bytes, off + 16)?,
        });
    }
    Ok(Oja { tag, frames })
}

/// Parse a `JamTable.dat` gauge table: flat little-endian `u16`
/// combo thresholds (54 entries, `25..4800`, on the real 7.55 file;
/// docs/file-formats/opi.md). Requires a non-empty even length.
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

/// One bound control entry: `IDC_NAME 0XCODE ["sprite"]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlEntry {
    /// Control id (`IDC_...`).
    pub id: String,
    /// Control code (`0X...`, e.g. `0X84200003`).
    pub code: u32,
    /// Linked sprite file, if the entry names one.
    pub sprite: Option<String>,
}

/// One `SET ... { ... }` subgroup inside a block (frame lists,
/// number sprites, …). Its entries consume `.bnd` rects in file
/// order exactly like top-level entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSet {
    /// Numbers on the `SET` line (e.g. entry count, flags).
    pub params: Vec<u32>,
    /// Member entries in file order.
    pub entries: Vec<ControlEntry>,
}

/// One `STATE_`/`DIALOG_` block of a `ControlList_*.txt` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlBlock {
    /// Block name (`STATE_ROOM`, `DIALOG_EVENT`, …).
    pub name: String,
    /// Declared entry count (first number on the header line).
    pub number: u32,
    /// Extra numbers on the header line, if any.
    pub extra: Vec<u32>,
    /// Sprite-bind file from the `BOUND "..."` line, if present.
    pub bound_file: Option<String>,
    /// Entries before the first `SET` group, in file order
    /// (entry 0 is the block itself).
    pub bound: Vec<ControlEntry>,
    /// `SET` subgroups in file order.
    pub sets: Vec<ControlSet>,
}

impl ControlBlock {
    /// All entries in file order (`bound`, then each set's
    /// entries): `.bnd` record `i` lays out entry `i + 1`.
    pub fn all_entries(&self) -> Vec<&ControlEntry> {
        let mut out = Vec::with_capacity(
            self.bound.len() + self.sets.iter().map(|s| s.entries.len()).sum::<usize>(),
        );
        out.extend(self.bound.iter());
        for set in &self.sets {
            out.extend(set.entries.iter());
        }
        out
    }
}

/// A parsed `ControlList_*.txt` UI definition: state/dialog blocks
/// binding control ids to codes, sprites, and (via the `.bnd` file
/// each block names) screen rects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlList {
    /// Declared `NUMBER_OF_STATE` value, if present.
    pub num_states: Option<u32>,
    /// Declared `NUMBER_OF_DIALOG` value, if present.
    pub num_dialogs: Option<u32>,
    /// `STATE_`/`DIALOG_` blocks in file order (stub blocks with
    /// no body parse to empty entry lists).
    pub blocks: Vec<ControlBlock>,
}

/// Signed little-endian word for animation frame origins.
fn read_i16le(bytes: &[u8], off: usize) -> Result<i16, FileError> {
    read_u16le(bytes, off).map(|raw| raw as i16)
}

fn parse_hex_word(word: &str) -> Result<u32, FileError> {
    let digits = word
        .strip_prefix("0x")
        .or_else(|| word.strip_prefix("0X"))
        .ok_or_else(|| unknown_format(word.as_bytes()))?;
    u32::from_str_radix(digits, 16).map_err(|_| unknown_format(word.as_bytes()))
}

/// First `"quoted"` value on a line, if any.
fn quoted_value(line: &str) -> Option<String> {
    let open = line.find('"')?;
    let close = line[open + 1..].find('"')?;
    Some(line[open + 1..open + 1 + close].to_string())
}

fn parse_control_entry(line: &str) -> Result<ControlEntry, FileError> {
    let mut parts = line.split_whitespace();
    let id = parts
        .next()
        .ok_or_else(|| unknown_format(line.as_bytes()))?;
    if !id.starts_with("IDC_") {
        return Err(unknown_format(id.as_bytes()));
    }
    let code = parts.next().ok_or_else(|| unknown_format(id.as_bytes()))?;
    Ok(ControlEntry {
        id: id.to_string(),
        code: parse_hex_word(code)?,
        sprite: quoted_value(line),
    })
}

/// A top-level `NAME 0xN [0xM]` block header (depth 0 only). Matches
/// by prefix, so the real-world `DAILOG_SELECTMUSIC` typo parses
/// like any other block. Returns `None` for non-header lines.
fn parse_block_header(line: &str) -> Result<Option<ControlBlock>, FileError> {
    let mut words = line.split_whitespace();
    let (Some(name), Some(number)) = (words.next(), words.next()) else {
        return Ok(None);
    };
    let is_block = ["STATE_", "DIALOG_", "DAILOG_"]
        .iter()
        .any(|prefix| name.starts_with(prefix));
    if !is_block || !(number.starts_with("0x") || number.starts_with("0X")) {
        return Ok(None);
    }
    let mut extra = Vec::new();
    for word in words {
        if word.starts_with("//") {
            break;
        }
        extra.push(parse_hex_word(word)?);
    }
    Ok(Some(ControlBlock {
        name: name.to_string(),
        number: parse_hex_word(number)?,
        extra,
        bound_file: None,
        bound: Vec::new(),
        sets: Vec::new(),
    }))
}

/// Incremental `ControlList` builder: one trimmed line at a time.
struct ControlListParser {
    list: ControlList,
    block: Option<ControlBlock>,
    open_set: Option<ControlSet>,
    depth: usize,
}

impl ControlListParser {
    fn new() -> Self {
        Self {
            list: ControlList {
                num_states: None,
                num_dialogs: None,
                blocks: Vec::new(),
            },
            block: None,
            open_set: None,
            depth: 0,
        }
    }

    fn finish(mut self) -> ControlList {
        self.close_block();
        self.list
    }

    fn close_set(&mut self) {
        if let Some(set) = self.open_set.take() {
            if let Some(block) = self.block.as_mut() {
                block.sets.push(set);
            }
        }
    }

    fn close_block(&mut self) {
        self.close_set();
        if let Some(block) = self.block.take() {
            self.list.blocks.push(block);
        }
    }

    fn push_entry(&mut self, line: &str) -> Result<(), FileError> {
        let entry = parse_control_entry(line)?;
        if let Some(set) = self.open_set.as_mut() {
            set.entries.push(entry);
            return Ok(());
        }
        let Some(block) = self.block.as_mut() else {
            return Err(unknown_format(line.as_bytes()));
        };
        if block.sets.is_empty() {
            block.bound.push(entry);
            return Ok(());
        }
        // Continuation run after a SET group (e.g. `STATE_ROOM`'s
        // `EXBUTTON` rows): fold into a trailing parameter-less
        // section so `all_entries` keeps file order.
        match block.sets.last_mut() {
            Some(last) if last.params.is_empty() => last.entries.push(entry),
            _ => block.sets.push(ControlSet {
                params: Vec::new(),
                entries: vec![entry],
            }),
        }
        Ok(())
    }

    fn parse_count(&mut self, line: &str, is_state: bool) -> Result<(), FileError> {
        let word = line
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| unknown_format(line.as_bytes()))?;
        let value = parse_hex_word(word)?;
        if is_state {
            self.list.num_states = Some(value);
        } else {
            self.list.num_dialogs = Some(value);
        }
        Ok(())
    }

    fn parse_line(&mut self, line: &str) -> Result<(), FileError> {
        if line.is_empty() || line.starts_with("//") {
            return Ok(());
        }
        if line == "{" {
            self.depth += 1;
            return Ok(());
        }
        if line == "}" {
            match self.depth {
                2 => self.close_set(),
                1 => self.close_block(),
                _ => return Err(unknown_format(line.as_bytes())),
            }
            self.depth -= 1;
            return Ok(());
        }
        if line.starts_with("NUMBER_OF_STATE") {
            return self.parse_count(line, true);
        }
        if line.starts_with("NUMBER_OF_DIALOG") {
            return self.parse_count(line, false);
        }
        if let Some(rest) = line.strip_prefix("BOUND") {
            let Some(block) = self.block.as_mut() else {
                return Err(unknown_format(line.as_bytes()));
            };
            block.bound_file = quoted_value(rest);
            return Ok(());
        }
        if let Some(rest) = line.strip_prefix("SET") {
            self.close_set();
            let mut params = Vec::new();
            for word in rest.split_whitespace() {
                if word.starts_with("//") {
                    break;
                }
                params.push(parse_hex_word(word)?);
            }
            self.open_set = Some(ControlSet {
                params,
                entries: Vec::new(),
            });
            return Ok(());
        }
        if line.starts_with("IDC_") {
            return self.push_entry(line);
        }
        if self.depth == 0 {
            if let Some(block) = parse_block_header(line)? {
                self.close_block();
                self.block = Some(block);
            }
        }
        Ok(())
    }
}

/// Parse a decoded `ControlList_*.txt` file (see
/// [`decode_control_text`]): `NUMBER_OF_*` counts plus
/// `STATE_`/`DIALOG_` blocks with `BOUND`/`SET` sections.
/// Comment lines (`//…`) and blank lines are skipped; unknown
/// top-level lines are ignored so future file revisions stay
/// readable, but malformed entries and unbalanced braces are
/// errors.
pub fn parse_control_list(text: &str) -> Result<ControlList, FileError> {
    let mut parser = ControlListParser::new();
    for raw_line in text.lines() {
        parser.parse_line(raw_line.trim())?;
    }
    Ok(parser.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_input() {
        assert_eq!(detect_kind(&[]), Err(FileError::TooShort(0)));
        assert_eq!(detect_kind(b"AB"), Err(FileError::TooShort(2)));
    }

    #[test]
    fn sniffs_candidate_magics() {
        assert_eq!(detect_kind(b"M30\x00...."), Ok(AssetKind::Ojm));
        assert_eq!(detect_kind(b"M32\x00...."), Ok(AssetKind::Ojm));
        // Verified on real 7.55 files (docs/install-layout.md).
        assert_eq!(detect_kind(b"OMC\x00...."), Ok(AssetKind::Ojm));
        assert_eq!(detect_kind(b"OJM\x00...."), Ok(AssetKind::Ojm));
        assert_eq!(detect_kind(b"ojnXXX...."), Ok(AssetKind::Ojn));
        // Old-format `.ojn` (id + "ojn" at offset 4) and OJNList.dat
        // (counts + "ojn" at offset 8), verified on real files.
        assert_eq!(detect_kind(b"\xb8\x22\x00\x00ojn\0..."), Ok(AssetKind::Ojn));
        assert_eq!(
            detect_kind(b"\x32\x02\x00\x00\x64\x00\x00\x00ojn\0"),
            Ok(AssetKind::OjnList)
        );
    }

    #[test]
    fn rejects_unknown_magic() {
        let err = detect_kind(b"ZZZZ....").unwrap_err();
        assert!(matches!(err, FileError::UnknownFormat { .. }));
    }

    fn m30_entry(name: &str, size: u32, id: u32) -> Vec<u8> {
        let mut e = vec![0u8; 52];
        e[..name.len()].copy_from_slice(name.as_bytes());
        e[32..36].copy_from_slice(&size.to_le_bytes());
        e[36..40].copy_from_slice(&0x20005u32.to_le_bytes());
        e[40..44].copy_from_slice(&1u32.to_le_bytes());
        e[44..48].copy_from_slice(&id.to_le_bytes());
        e
    }

    fn m30_blob() -> Vec<u8> {
        // Real M30 header is 28 bytes: magic, ?, ?, count@12, first@16.
        let mut b = vec![0u8; 28];
        b[..4].copy_from_slice(b"M30\0");
        b[12..16].copy_from_slice(&2u32.to_le_bytes());
        b[16..20].copy_from_slice(&28u32.to_le_bytes());
        b.extend_from_slice(&m30_entry("kick", 8, 3));
        b.extend_from_slice(b"OggSdata");
        b.extend_from_slice(&m30_entry("mr1", 8, 980));
        b.extend_from_slice(b"OggSmore");
        b
    }

    #[test]
    fn m30_round_trip() {
        let blob = m30_blob();
        let samples = parse_ojm(&blob).unwrap();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].name, "kick");
        assert_eq!(samples[0].sample_id, Some(3));
        assert_eq!(samples[0].data, b"OggSdata");
        assert_eq!(samples[1].name, "mr1");
        assert_eq!(samples[1].sample_id, Some(980));
        assert_eq!(samples[1].data, b"OggSmore");
    }

    #[test]
    fn m30_rejects_truncation_and_bad_magic() {
        let mut blob = m30_blob();
        blob.truncate(blob.len() - 1);
        assert!(matches!(parse_ojm(&blob), Err(FileError::Truncated { .. })));
        let mut bad = m30_blob();
        bad[80..84].copy_from_slice(b"XXXX");
        assert!(matches!(
            parse_ojm(&bad),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn omc_entry(name: &str, size: u32) -> Vec<u8> {
        let mut e = vec![0u8; 36];
        e[..name.len()].copy_from_slice(name.as_bytes());
        e[32..36].copy_from_slice(&size.to_le_bytes());
        e
    }

    fn omc_blob() -> Vec<u8> {
        // 16-byte header, first entry offset at +12.
        let mut b = vec![0u8; 16];
        b[..4].copy_from_slice(b"OMC\0");
        b[12..16].copy_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&omc_entry("bgRoom.o", 8));
        b.extend_from_slice(b"OggSdata");
        b.extend_from_slice(&omc_entry("bgLogo.gg", 8));
        b.extend_from_slice(b"OggSlogo");
        b
    }

    #[test]
    fn omc_round_trip_with_blank_slots_skipped() {
        let blob = omc_blob();
        let samples = parse_ojm(&blob).unwrap();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].name, "bgRoom.o");
        assert_eq!(samples[0].data, b"OggSdata");
        assert_eq!(samples[1].name, "bgLogo.gg");
        assert_eq!(samples[1].data, b"OggSlogo");
        // Insert a blank 36-byte slot after the header: skipped, and
        // everything downstream still parses (entries are positional).
        let mut with_blank = blob[..16].to_vec();
        with_blank.extend_from_slice(&[0u8; 36]);
        with_blank.extend_from_slice(&blob[16..]);
        let samples = parse_ojm(&with_blank).unwrap();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].name, "bgRoom.o");
        assert_eq!(samples[1].name, "bgLogo.gg");
    }

    fn ojn_list_record(song_id: u32, bpm: f32, title: &str, ojm_file: &str) -> Vec<u8> {
        // Synthetic 300-byte song record with distinctive values at
        // every mapped offset (scenario: full field map parses).
        let mut rec = vec![0u8; 300];
        rec[0..4].copy_from_slice(&113592u32.to_le_bytes());
        rec[4..8].copy_from_slice(&song_id.to_le_bytes());
        rec[8..11].copy_from_slice(b"ojn");
        rec[12..16].copy_from_slice(&2.9f32.to_le_bytes());
        rec[16..20].copy_from_slice(&4u32.to_le_bytes());
        rec[20..24].copy_from_slice(&bpm.to_le_bytes());
        rec[24..28].copy_from_slice(&0x50003u32.to_le_bytes());
        rec[28..32].copy_from_slice(&7u32.to_le_bytes());
        for (slot, base) in [(32, 3594), (44, 312), (56, 69), (68, 806)] {
            for level in 0..3 {
                let off = slot + level * 4;
                rec[off..off + 4].copy_from_slice(&(base + level as u32).to_le_bytes());
            }
        }
        rec[80..84].copy_from_slice(&((song_id << 16) | 29).to_le_bytes());
        rec[100..104].copy_from_slice(&0x01000000u32.to_le_bytes());
        rec[104..108].copy_from_slice(&19256u32.to_le_bytes());
        rec[108..112].copy_from_slice(&0.02f32.to_le_bytes());
        rec[112..112 + title.len()].copy_from_slice(title.as_bytes());
        rec[240..240 + ojm_file.len()].copy_from_slice(ojm_file.as_bytes());
        rec[272..276].copy_from_slice(&165019u32.to_le_bytes());
        for (i, off) in [276, 280, 284].iter().enumerate() {
            rec[*off..*off + 4].copy_from_slice(&(129 + i as u32).to_le_bytes());
        }
        rec[288..292].copy_from_slice(&300u32.to_le_bytes());
        rec[292..296].copy_from_slice(&58376u32.to_le_bytes());
        rec[296..300].copy_from_slice(&124284u32.to_le_bytes());
        rec
    }

    #[test]
    fn ojn_list_round_trip() {
        // Every full 300-byte record parses, including record 0
        // (song 100, not a header); the 148-byte tail is ignored.
        let mut blob = ojn_list_record(100, 130.0, "Title0", "o2ma100.ojm");
        blob.extend_from_slice(&ojn_list_record(757, 124.0, "Title", "o2ma757.ojm"));
        blob.extend_from_slice(&[0xABu8; 148]);
        let entries = parse_ojn_list(&blob).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[0].song_id, 100);
        assert_eq!(entries[1].index, 1);
        assert_eq!(entries[1].song_id, 757);
        assert_eq!(entries[1].bpm, 124.0);
        assert_eq!(entries[1].title, "Title");
        assert_eq!(entries[1].ojm_file, "o2ma757.ojm");
        assert!(matches!(
            parse_ojn_list(b"nope"),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn ojn_list_full_fields_parse() {
        let rec = ojn_list_record(757, 124.0, "Title", "o2ma757.ojm");
        let entries = parse_ojn_list(&rec).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.unknown0, 113592);
        assert_eq!(e.format_version, 2.9);
        assert_eq!(e.genre, 4);
        assert_eq!(e.level_flags, 0x50003);
        assert_eq!(e.stable_minor, 7);
        assert_eq!(e.counts_a, [3594, 3595, 3596]);
        assert_eq!(e.counts_b, [312, 313, 314]);
        assert_eq!(e.measures, [69, 70, 71]);
        assert_eq!(e.events, [806, 807, 808]);
        assert_eq!(e.key_field, (757 << 16) | 29);
        assert_eq!(e.key_hi(), 757);
        assert_eq!(e.key_lo(), 29);
        assert_eq!(e.pad, [0, 0, 0, 0]);
        assert_eq!(e.flag100, 0x01000000);
        assert_eq!(e.const_a, 19256);
        assert_eq!(e.tune, 0.02);
        assert_eq!(e.unknown1, 165019);
        assert_eq!(e.durations, [129, 130, 131]);
        assert_eq!(e.slice_start, 300);
        assert_eq!(e.slice_ends, [58376, 124284]);
        assert!(e.is_song());
    }

    #[test]
    fn ojn_list_is_song_filters_dead_slots() {
        // Zeroed slot: no filename, no tempo.
        let mut dead = vec![0u8; 300];
        dead[8..11].copy_from_slice(b"ojn");
        let dead = parse_ojn_list(&dead).unwrap();
        assert!(!dead[0].is_song());
        // Heap-residue slot: absurd tempo, no filename.
        let mut junk = vec![0x77u8; 300];
        junk[8..11].copy_from_slice(b"ojn");
        junk[20..24].copy_from_slice(&2.6e17f32.to_le_bytes());
        let junk = parse_ojn_list(&junk).unwrap();
        assert!(!junk[0].is_song());
        // Uppercase filename still counts (Windows is case-blind).
        let upper = ojn_list_record(885, 106.0, "T", "O2MA885.ojm");
        assert!(parse_ojn_list(&upper).unwrap()[0].is_song());
    }

    #[test]
    fn ojn_decrypt_round_trip() {
        // Synthetic new-format container mirroring the in-exe encoder
        // contract (FUN_00412180): XOR with (k1,k2,k3) on period P,
        // key slots at buf[3..7], "new" tail, whole buffer reversed.
        let (k1, k2, k3, p): (u8, u8, u8, usize) = (0x97, 0xE0, 0x34, 6);
        let ks = |i: usize| {
            if i.is_multiple_of(p) {
                k1
            } else if i % p == p / 2 {
                k2
            } else {
                k3
            }
        };
        let n = 64usize;
        let mut plain = vec![0u8; n];
        plain[0..4].copy_from_slice(&[0xF5, 0x02, 0x00, 0x00]);
        plain[4..8].copy_from_slice(b"ojn\0");
        for (i, b) in plain.iter_mut().enumerate().take(n).skip(8) {
            *b = (i * 7) as u8;
        }
        let mut buf: Vec<u8> = plain.iter().enumerate().map(|(i, &b)| b ^ ks(i)).collect();
        // Key slots live at plaintext-space tail: rev[n-7..n-4].
        buf[n - 7] = k1;
        buf[n - 6] = k2;
        buf[n - 5] = k3;
        buf[n - 4] = p as u8;
        // Tail bytes become the "new" magic after reversal.
        buf[n - 3] = b'w';
        buf[n - 2] = b'e';
        buf[n - 1] = b'n';
        let enc: Vec<u8> = buf.iter().rev().copied().collect();
        assert!(enc.starts_with(b"new"));
        let dec = decrypt_ojn_new_format(&enc).unwrap();
        // Slots and tail are clobbered by key material; the middle
        // must round-trip exactly, including the magic.
        assert_eq!(&dec[0..n - 7], &plain[0..n - 7]);
        // Old-format input passes through untouched.
        let mut old_raw = vec![0u8; 300];
        old_raw[..8].copy_from_slice(b"ojnX....");
        let old = parse_ojn(&old_raw).unwrap();
        assert_eq!(old.magic, *b"ojnX");
    }

    #[test]
    fn ojn_header_fields_parse() {
        // Synthetic old-format header: id, magic, BPM, count.
        let mut blob = vec![0u8; 300];
        blob[0..4].copy_from_slice(&757u32.to_le_bytes());
        blob[4..8].copy_from_slice(b"ojn\0");
        blob[16..20].copy_from_slice(&124f32.to_le_bytes());
        blob[64..68].copy_from_slice(&208u32.to_le_bytes());
        let chart = parse_ojn(&blob).unwrap();
        assert_eq!(chart.song_id, 757);
        assert_eq!(chart.bpm, 124.0);
        assert_eq!(chart.event_count_hint, 208);
    }

    #[test]
    fn ojm_variant_parses_raw_tail() {
        // OJM magic dispatches to the entry parser: a minimal blob with
        // one entry parses; short input is rejected, not stubbed.
        let mut blob = vec![0u8; 16];
        blob[..4].copy_from_slice(b"OJM\0");
        blob[12..16].copy_from_slice(&16u32.to_le_bytes());
        blob.extend_from_slice(&omc_entry("evt", 8));
        blob.extend_from_slice(b"OggSdata");
        let samples = parse_ojm(&blob).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].name, "evt");
        assert!(matches!(
            parse_ojm(b"OJM\x00...."),
            Err(FileError::TooShort(_))
        ));
        // Old-format chart validates by magic.
        let mut raw = vec![0u8; 300];
        raw[..8].copy_from_slice(b"ojnX....");
        let chart = parse_ojn(&raw).unwrap();
        assert_eq!(chart.magic, *b"ojnX");
    }

    #[test]
    fn cross_kind_is_type_error() {
        assert!(matches!(
            parse_ojm(b"ojnX...."),
            Err(FileError::UnknownFormat { .. })
        ));
        assert!(matches!(
            parse_ojn(b"M30\x00...."),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn mode2_entry(name: &[u8], size: u32) -> Vec<u8> {
        let mut entry = vec![0u8; MODE2_ENTRY_LEN];
        entry[..name.len()].copy_from_slice(name);
        entry[MODE2_SIZE_OFF..MODE2_SIZE_OFF + 4].copy_from_slice(&size.to_le_bytes());
        entry
    }

    fn mode3_entry(name: &[u8], size: u32) -> Vec<u8> {
        let mut entry = vec![0u8; MODE3_ENTRY_LEN];
        entry[..name.len()].copy_from_slice(name);
        entry[MODE3_SIZE_OFF..MODE3_SIZE_OFF + 4].copy_from_slice(&size.to_le_bytes());
        entry
    }

    fn new_variant_blob() -> Vec<u8> {
        let mut blob = vec![0u8; 20];
        blob[..4].copy_from_slice(b"OMC\0");
        blob[MODE2_COUNT_OFF..MODE2_COUNT_OFF + 2].copy_from_slice(&1u16.to_le_bytes());
        blob[MODE3_COUNT_OFF..MODE3_COUNT_OFF + 2].copy_from_slice(&1u16.to_le_bytes());
        blob[MODE2_ENTRIES_OFF..MODE2_ENTRIES_OFF + 4].copy_from_slice(&20u32.to_le_bytes());
        let mode3_off = 20 + MODE2_ENTRY_LEN + 16;
        blob[MODE3_ENTRIES_OFF..MODE3_ENTRIES_OFF + 4]
            .copy_from_slice(&(mode3_off as u32).to_le_bytes());
        blob.extend_from_slice(&mode2_entry(b"s1", 16));
        blob.extend_from_slice(&[0xAA; 16]);
        blob.extend_from_slice(&mode3_entry(b"s2", 8));
        blob.extend_from_slice(&[0xBB; 8]);
        blob
    }

    #[test]
    fn new_variant_entries_walk_chained_layout() {
        let blob = new_variant_blob();
        let tables = parse_new_variant_entries(&blob).unwrap();
        assert_eq!(tables.mode2.len(), 1);
        assert_eq!(tables.mode2[0].name, "s1");
        assert_eq!(tables.mode2[0].size, 16);
        assert_eq!(
            &blob[tables.mode2[0].data_offset..tables.mode2[0].data_offset + 16],
            &[0xAA; 16]
        );
        assert_eq!(tables.mode3.len(), 1);
        assert_eq!(tables.mode3[0].name, "s2");
        assert_eq!(tables.mode3[0].size, 8);
        assert_eq!(
            &blob[tables.mode3[0].data_offset..tables.mode3[0].data_offset + 8],
            &[0xBB; 8]
        );
    }

    #[test]
    fn new_variant_entries_reject_bad_input() {
        assert!(matches!(
            parse_new_variant_entries(b"short"),
            Err(FileError::TooShort(_))
        ));
        let mut blob = new_variant_blob();
        blob[..4].copy_from_slice(b"M30\0");
        assert!(matches!(
            parse_new_variant_entries(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        let truncated = &new_variant_blob()[..32];
        assert!(matches!(
            parse_new_variant_entries(truncated),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn not_decrypt_flips_flagged_bytes() {
        // Hand-traced against `FUN_0054f9e0`: (0xFF << 0..2) & 0x80
        // is set, so every byte inverts; key1 stays 0xFF (no reset
        // before key2 reaches 7).
        let mut buf = [0x41, 0x42, 0x43];
        not_decrypt(&mut buf, 0xFF, 0);
        assert_eq!(buf, [0xBE, 0xBD, 0xBC]);
    }

    #[test]
    fn not_decrypt_resets_key_schedule_every_eight_bytes() {
        // Eight 0x00 bytes flip (key1 0xFF, key2 0..7); at key2 == 7
        // the schedule resets to key1 = last ciphertext (0x00), so
        // the ninth byte is left alone.
        let mut buf = [0x00; 9];
        not_decrypt(&mut buf, 0xFF, 0);
        assert_eq!(buf, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    }

    fn opi_entry(name: &[u8], offset: u32, size: u32) -> Vec<u8> {
        let mut entry = vec![0u8; OPI_ENTRY_LEN];
        entry[..4].copy_from_slice(&1u32.to_le_bytes());
        entry[OPI_NAME_OFF..OPI_NAME_OFF + name.len()].copy_from_slice(name);
        entry[OPI_OFFSET_OFF..OPI_OFFSET_OFF + 4].copy_from_slice(&offset.to_le_bytes());
        entry[OPI_SIZE_OFF..OPI_SIZE_OFF + 4].copy_from_slice(&size.to_le_bytes());
        entry[OPI_RESERVED_OFF..OPI_RESERVED_OFF + 4].copy_from_slice(&size.to_le_bytes());
        entry
    }

    fn opi_blob() -> Vec<u8> {
        let mut blob = vec![0u8; OPI_HEADER_LEN];
        blob[..4].copy_from_slice(&2u32.to_le_bytes());
        blob[4..8].copy_from_slice(&3u32.to_le_bytes());
        let first_data = OPI_HEADER_LEN;
        blob.extend_from_slice(b"hello");
        blob.extend_from_slice(b"world!");
        blob.extend_from_slice(&opi_entry(b"a.ojs", first_data as u32, 5));
        blob.extend_from_slice(&[0u8; OPI_ENTRY_LEN]);
        blob.extend_from_slice(&opi_entry(b"b.txt", (first_data + 5) as u32, 6));
        blob
    }

    #[test]
    fn opi_entries_list_named_files_with_offsets() {
        let blob = opi_blob();
        let entries = parse_opi(&blob).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "a.ojs");
        assert_eq!(entries[0].kind, 1);
        assert_eq!(
            &blob[entries[0].data_offset..entries[0].data_offset + 5],
            b"hello"
        );
        assert_eq!(entries[1].name, "b.txt");
        assert_eq!(
            &blob[entries[1].data_offset..entries[1].data_offset + 6],
            b"world!"
        );
    }

    #[test]
    fn opi_entries_reject_bad_input() {
        assert!(matches!(parse_opi(b"short"), Err(FileError::TooShort(_))));
        let mut oversized = opi_blob();
        let first_size_at = OPI_HEADER_LEN + 5 + 6 + OPI_SIZE_OFF;
        oversized[first_size_at..first_size_at + 4].copy_from_slice(&99999u32.to_le_bytes());
        assert!(matches!(
            parse_opi(&oversized),
            Err(FileError::Truncated { .. })
        ));
        let mut countless = vec![0u8; OPI_HEADER_LEN];
        countless[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            parse_opi(&countless),
            Err(FileError::Truncated { .. })
        ));
    }

    fn opi_blob_with_free_slot() -> Vec<u8> {
        let mut blob = vec![0u8; OPI_HEADER_LEN];
        blob[..4].copy_from_slice(&2u32.to_le_bytes());
        blob[4..8].copy_from_slice(&3u32.to_le_bytes());
        let first_data = OPI_HEADER_LEN;
        blob.extend_from_slice(b"hello");
        blob.extend_from_slice(b"world!");
        blob.extend_from_slice(&opi_entry(b"a.ojs", first_data as u32, 5));
        let mut free = vec![0u8; OPI_ENTRY_LEN];
        free[OPI_OFFSET_OFF..OPI_OFFSET_OFF + 4]
            .copy_from_slice(&(first_data as u32 + 11).to_le_bytes());
        free[OPI_RESERVED_OFF..OPI_RESERVED_OFF + 4].copy_from_slice(&8u32.to_le_bytes());
        blob.extend_from_slice(&free);
        blob.extend_from_slice(&opi_entry(b"b.txt", (first_data + 5) as u32, 6));
        blob
    }

    #[test]
    fn opi_slots_keep_free_slots_in_table_order() {
        let blob = opi_blob_with_free_slot();
        let slots = parse_opi_slots(&blob).unwrap();
        assert_eq!(slots.len(), 3);
        assert_eq!(slots[0].name, "a.ojs");
        assert!(!slots[0].is_free());
        assert!(slots[1].is_free());
        assert_eq!(slots[1].kind, 0);
        assert_eq!(slots[1].size, 0);
        assert_eq!(slots[2].name, "b.txt");
        let files = parse_opi(&blob).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn opi_slots_reject_stride_running_past_file() {
        let mut blob = opi_blob();
        let first_aux_at = OPI_HEADER_LEN + 5 + 6 + OPI_RESERVED_OFF;
        blob[first_aux_at..first_aux_at + 4].copy_from_slice(&99999u32.to_le_bytes());
        assert!(matches!(
            parse_opi_slots(&blob),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn jpeg_body_needs_soi_marker_and_eoi_marker() {
        assert!(is_jpeg_body(&[0xFF, 0xD8, 0xFF, 0xD9]));
        assert!(is_jpeg_body(&[
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0xFF, 0xD9
        ]));
        assert!(!is_jpeg_body(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(!is_jpeg_body(&[0x55, 0x05, 0xFF, 0xD9]));
        assert!(!is_jpeg_body(&[0xFF, 0xD8]));
        assert!(!is_jpeg_body(&[]));
    }

    fn sprite55_image(
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        data: &[u8],
        run: &mut u32,
        out: &mut Vec<u8>,
    ) {
        out.extend_from_slice(&x.to_le_bytes());
        out.extend_from_slice(&y.to_le_bytes());
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&h.to_le_bytes());
        out.extend_from_slice(&run.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        *run += data.len() as u32;
    }

    fn sprite55_blob(tag: u16, colorkey: u16, images: &[(u16, u16, u16, u16, Vec<u8>)]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&tag.to_le_bytes());
        blob.extend_from_slice(&SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&(images.len() as u16).to_le_bytes());
        blob.extend_from_slice(&colorkey.to_le_bytes());
        let mut run = 0u32;
        for (x, y, w, h, data) in images {
            sprite55_image(*x, *y, *w, *h, data, &mut run, &mut blob);
        }
        for (_, _, _, _, data) in images {
            blob.extend_from_slice(data);
        }
        blob
    }

    #[test]
    fn sprite55_parses_chained_images_with_pixel_access() {
        let red_green = vec![0x00, 0xF8, 0xE0, 0x07];
        let blue = vec![0x1F, 0x00];
        let blob = sprite55_blob(
            1,
            0x7D5F,
            &[(0, 0, 2, 1, red_green.clone()), (5, 7, 1, 1, blue.clone())],
        );
        let sprite = parse_sprite55(&blob).unwrap();
        assert_eq!(sprite.tag, 1);
        assert_eq!(sprite.colorkey, 0x7D5F);
        assert_eq!(sprite.images.len(), 2);
        assert_eq!(
            sprite.images[0],
            Sprite55Image {
                x: 0,
                y: 0,
                w: 2,
                h: 1,
                data_offset: 0,
                size: 4
            }
        );
        assert_eq!(sprite.pixel_data(&sprite.images[0]), &red_green);
        assert!(sprite.is_raw_rgb555(&sprite.images[0]));
        assert_eq!(sprite.pixel_data(&sprite.images[1]), &blue);
        assert!(sprite.is_raw_rgb555(&sprite.images[1]));
    }

    #[test]
    fn sprite55_accepts_tag_zero_and_non_raw_sizes() {
        // Compressed-style frame: stored bytes fewer than `w*h*2`.
        let stored = vec![0x56, 0x00, 0x00, 0x00, 0xDD, 0x00];
        let blob = sprite55_blob(0, 0x0000, &[(0, 0, 86, 69, stored.clone())]);
        let sprite = parse_sprite55(&blob).unwrap();
        assert_eq!(sprite.tag, 0);
        assert!(!sprite.is_raw_rgb555(&sprite.images[0]));
        assert_eq!(sprite.pixel_data(&sprite.images[0]), &stored);
    }

    #[test]
    fn sprite55_rejects_broken_directories() {
        let blob = sprite55_blob(1, 0, &[(0, 0, 1, 1, vec![0x00, 0x00])]);
        assert!(matches!(
            parse_sprite55(b"short"),
            Err(FileError::TooShort(_))
        ));
        let mut bad_magic = blob.clone();
        bad_magic[2..4].copy_from_slice(b"ZZ");
        assert!(matches!(
            parse_sprite55(&bad_magic),
            Err(FileError::UnknownFormat { .. })
        ));
        assert!(matches!(
            parse_sprite55(&blob[..blob.len() - 1]),
            Err(FileError::Truncated { .. })
        ));
        // Non-zero pad word breaks the record contract.
        let mut bad_pad = blob.clone();
        let pad_at = SPRITE55_HEADER_LEN + 16;
        bad_pad[pad_at..pad_at + 4].copy_from_slice(&1u32.to_le_bytes());
        assert!(matches!(
            parse_sprite55(&bad_pad),
            Err(FileError::UnknownFormat { .. })
        ));
        // Non-chained offset breaks the record contract.
        let mut bad_chain = blob.clone();
        let off_at = SPRITE55_HEADER_LEN + 8;
        bad_chain[off_at..off_at + 4].copy_from_slice(&7u32.to_le_bytes());
        assert!(matches!(
            parse_sprite55(&bad_chain),
            Err(FileError::UnknownFormat { .. })
        ));
        // Trailing garbage breaks exact accounting.
        let mut trailing = blob.clone();
        trailing.push(0x00);
        assert!(matches!(
            parse_sprite55(&trailing),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn sprite55_strided_slices_animation_frames() {
        // Two 2x1 images, two frames each: record sizes stay in
        // frame units while the blob holds both frames per image.
        let frame = |v: u8| vec![v, 0x00, v, 0x00];
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&2u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        let mut run = 0u32;
        for (x, data_len) in [(0u16, 4u32), (9u16, 4u32)] {
            blob.extend_from_slice(&x.to_le_bytes());
            blob.extend_from_slice(&0u16.to_le_bytes());
            blob.extend_from_slice(&2u16.to_le_bytes());
            blob.extend_from_slice(&1u16.to_le_bytes());
            blob.extend_from_slice(&run.to_le_bytes());
            blob.extend_from_slice(&data_len.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
            run += data_len;
        }
        for v in [0x11u8, 0x22, 0x33, 0x44] {
            blob.extend_from_slice(&frame(v));
        }
        let atlas = parse_sprite55_strided(&blob, 2).unwrap();
        assert_eq!(atlas.frames_per_image, 2);
        assert_eq!(atlas.frame_data(&atlas.images[0], 0), &frame(0x11));
        assert_eq!(atlas.frame_data(&atlas.images[0], 1), &frame(0x22));
        assert_eq!(atlas.frame_data(&atlas.images[1], 0), &frame(0x33));
        assert_eq!(atlas.frame_data(&atlas.images[1], 1), &frame(0x44));
        assert!(matches!(
            parse_sprite55_strided(&blob, 0),
            Err(FileError::UnknownFormat { .. })
        ));
        assert!(matches!(
            parse_sprite55_strided(&blob, 3),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn rgb555_expands_primaries() {
        assert_eq!(rgb555_to_rgb888(0x7C00), (255, 0, 0));
        assert_eq!(rgb555_to_rgb888(0x03E0), (0, 255, 0));
        assert_eq!(rgb555_to_rgb888(0x001F), (0, 0, 255));
        assert_eq!(rgb555_to_rgb888(0x0000), (0, 0, 0));
        assert_eq!(rgb555_to_rgb888(0xFFFF), (255, 255, 255));
    }

    fn bnd_blob(rects: &[(u32, u32, u32, u32)]) -> Vec<u8> {
        let mut blob = Vec::from(BND_MAGIC);
        blob.extend_from_slice(&(rects.len() as u16).to_le_bytes());
        for (x1, y1, x2, y2) in rects {
            blob.extend_from_slice(&x1.to_le_bytes());
            blob.extend_from_slice(&y1.to_le_bytes());
            blob.extend_from_slice(&x2.to_le_bytes());
            blob.extend_from_slice(&y2.to_le_bytes());
        }
        blob
    }

    #[test]
    fn bnd_parses_frame_and_control_rects() {
        let blob = bnd_blob(&[(30, 3, 612, 595), (19, 55, 564, 561), (505, 2, 553, 38)]);
        let bnd = parse_bnd(&blob).unwrap();
        assert_eq!(
            bnd.rects,
            vec![
                BndRect {
                    x1: 30,
                    y1: 3,
                    x2: 612,
                    y2: 595
                },
                BndRect {
                    x1: 19,
                    y1: 55,
                    x2: 564,
                    y2: 561
                },
                BndRect {
                    x1: 505,
                    y1: 2,
                    x2: 553,
                    y2: 38
                },
            ]
        );
    }

    #[test]
    fn bnd_rejects_bad_magic_and_size_mismatch() {
        assert!(matches!(parse_bnd(b"short"), Err(FileError::TooShort(_))));
        let mut bad_magic = bnd_blob(&[(1, 2, 3, 4)]);
        bad_magic[0] = 0x00;
        assert!(matches!(
            parse_bnd(&bad_magic),
            Err(FileError::UnknownFormat { .. })
        ));
        let mut short = bnd_blob(&[(1, 2, 3, 4), (5, 6, 7, 8)]);
        short.truncate(short.len() - 1);
        assert!(matches!(
            parse_bnd(&short),
            Err(FileError::Truncated { .. })
        ));
        let mut trailing = bnd_blob(&[(1, 2, 3, 4)]);
        trailing.push(0x00);
        assert!(matches!(
            parse_bnd(&trailing),
            Err(FileError::Truncated { .. })
        ));
    }

    fn oja_blob(tag: u16, frames: &[(i16, i16, u16, u16, u32, u32, u32)]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&tag.to_le_bytes());
        blob.extend_from_slice(&OJA_MAGIC.to_le_bytes());
        blob.extend_from_slice(&(frames.len() as u16).to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        for (x, y, w, h, a, b, c) in frames {
            blob.extend_from_slice(&x.to_le_bytes());
            blob.extend_from_slice(&y.to_le_bytes());
            blob.extend_from_slice(&w.to_le_bytes());
            blob.extend_from_slice(&h.to_le_bytes());
            blob.extend_from_slice(&a.to_le_bytes());
            blob.extend_from_slice(&b.to_le_bytes());
            blob.extend_from_slice(&c.to_le_bytes());
        }
        blob
    }

    #[test]
    fn oja_parses_frame_table_without_slicing_data() {
        let blob = oja_blob(
            1,
            &[
                (-72, -72, 144, 143, 0, 19446, 8815),
                (-56, -57, 113, 113, 28261, 14038, 6320),
            ],
        );
        let oja = parse_oja(&blob).unwrap();
        assert_eq!(oja.tag, 1);
        assert_eq!(
            oja.frames,
            vec![
                OjaFrame {
                    x: -72,
                    y: -72,
                    w: 144,
                    h: 143,
                    a: 0,
                    b: 19446,
                    c: 8815
                },
                OjaFrame {
                    x: -56,
                    y: -57,
                    w: 113,
                    h: 113,
                    a: 28261,
                    b: 14038,
                    c: 6320
                },
            ]
        );
        assert!(matches!(parse_oja(b"short"), Err(FileError::TooShort(_))));
        let mut bad_magic = blob.clone();
        bad_magic[2..4].copy_from_slice(b"ZZ");
        assert!(matches!(
            parse_oja(&bad_magic),
            Err(FileError::UnknownFormat { .. })
        ));
        assert!(matches!(
            parse_oja(&blob[..blob.len() - 1]),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn jam_thresholds_parse_even_u16_list() {
        let blob: Vec<u8> = [25u16, 50, 75, 100]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        assert_eq!(parse_jam_thresholds(&blob).unwrap(), vec![25, 50, 75, 100]);
        assert!(matches!(
            parse_jam_thresholds(b""),
            Err(FileError::TooShort(_))
        ));
        assert!(matches!(
            parse_jam_thresholds(b"odd"),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn control_text_decodes_cp949_korean() {
        // Synthetic Korean (U+D14C U+C2A4 U+D2B8): high bytes that UTF-8 rejects.
        let korean = [197u8, 215, 189, 186, 198, 174];
        let mut blob = b"Ok 123 ".to_vec();
        blob.extend_from_slice(&korean);
        assert_eq!(
            decode_control_text(&blob).unwrap(),
            "Ok 123 \u{D14C}\u{C2A4}\u{D2B8}"
        );
        assert!(String::from_utf8(blob.clone()).is_err());
    }

    #[test]
    fn control_text_rejects_broken_lead_byte() {
        assert!(matches!(
            decode_control_text(&[0x61, 0x81]),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn control_fixture() -> &'static str {
        "NUMBER_OF_STATE\t\t\t\t0x01\n\
         STATE_FIXTURE\t\t\t\t0x03\t0x01\n\
         {\n\
         \tBOUND\t\t\t\t\"Dialog_Fixture.bnd\"\n\
         \tIDC_FIXTURE_SELF\t\t\t\t0X10000000\n\
         \tIDC_BACK_FIXTURE\t\t\t\t0X10100001\t\"Back.ojs\"\n\
         \tIDC_BUTTON_FIXTURE_CLOSE\t\t0X10200002\t\"Close.ojs\"  // trailing note\n\
         \tSET\t\t\t\t0x01\t0x00020000\n\
         \t{\n\
         \t\tIDC_IMAGE_FIXTURE_NUM\t\t0X10500003\t\"Num.ojs\"\n\
         \t}\n\
         \tIDC_TEXT_FIXTURE_TAIL\t\t0X10400004\n\
         }\n"
    }

    #[test]
    fn control_list_parses_bound_and_set_sections() {
        let list = parse_control_list(control_fixture()).unwrap();
        assert_eq!(list.num_states, Some(1));
        assert_eq!(list.num_dialogs, None);
        assert_eq!(list.blocks.len(), 1);
        let block = &list.blocks[0];
        assert_eq!(block.name, "STATE_FIXTURE");
        assert_eq!(block.number, 3);
        assert_eq!(block.extra, vec![1]);
        assert_eq!(block.bound_file.as_deref(), Some("Dialog_Fixture.bnd"));
        assert_eq!(block.bound.len(), 3);
        assert_eq!(block.bound[1].sprite.as_deref(), Some("Back.ojs"));
        assert_eq!(block.bound[2].code, 0x10200002);
        assert_eq!(block.sets.len(), 2);
        assert_eq!(block.sets[0].params, vec![1, 0x00020000]);
        assert_eq!(block.sets[0].entries[0].id, "IDC_IMAGE_FIXTURE_NUM");
        assert!(block.sets[1].params.is_empty());
        let ids: Vec<&str> = block.all_entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "IDC_FIXTURE_SELF",
                "IDC_BACK_FIXTURE",
                "IDC_BUTTON_FIXTURE_CLOSE",
                "IDC_IMAGE_FIXTURE_NUM",
                "IDC_TEXT_FIXTURE_TAIL",
            ]
        );
    }

    #[test]
    fn control_list_rejects_malformed_input() {
        assert!(matches!(
            parse_control_list("STATE_BROKEN\t0x01\n{\n}\n}\n"),
            Err(FileError::UnknownFormat { .. })
        ));
        assert!(matches!(
            parse_control_list("STATE_BROKEN\t0xZZ\n{\n}\n"),
            Err(FileError::UnknownFormat { .. })
        ));
        assert!(matches!(
            parse_control_list("IDC_ORPHAN\t\t0X10000000\n"),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn control_list_parses_empty_bound_and_typo_header() {
        // Mirrors real 7.55 shapes: `STATE_CHANNEL` carries a bare
        // `BOUND` line with no file, and the select-music dialog
        // header is spelled `DAILOG_` in the file.
        let text = "NUMBER_OF_STATE\t\t0x02\n\
            STATE_FIRST\t\t0x01\n\
            {\n\
            \tBOUND\n\
            \tIDC_FIRST_SELF\t\t0X10000000\n\
            \tIDC_SCROLL_FIRST_LIST\t0X10200001\t\"O2_SBS_VERT\t4\t\t100\"\n\
            }\n\
            DAILOG_TYPO\t\t0x01\n\
            {\n\
            \tBOUND\t\t\"Dialog_Typo.bnd\"\n\
            \tIDC_TYPO_SELF\t\t0X20000000\n\
            //\tIDC_COMMENTED_OUT\t\t0X20000001\t\"Ghost.ojs\"\n\
            }\n";
        let list = parse_control_list(text).unwrap();
        assert_eq!(list.blocks.len(), 2);
        let first = &list.blocks[0];
        assert_eq!(first.name, "STATE_FIRST");
        assert_eq!(first.bound_file, None);
        assert_eq!(first.bound.len(), 2);
        // Scrollbar style strings ride in `sprite` verbatim.
        assert_eq!(
            first.bound[1].sprite.as_deref(),
            Some("O2_SBS_VERT\t4\t\t100")
        );
        let typo = &list.blocks[1];
        assert_eq!(typo.name, "DAILOG_TYPO");
        assert_eq!(typo.bound_file.as_deref(), Some("Dialog_Typo.bnd"));
        // The `//`-commented row stays out of the block.
        assert_eq!(typo.bound.len(), 1);
    }

    #[test]
    fn control_list_parses_stub_and_trailing_comment() {
        // Mirrors real 7.55 shapes: `0x00` stub headers with no body,
        // and entry lines with trailing `//` notes (the Playing
        // file's load-limit comment sits on entry 230 this way).
        let text = "NUMBER_OF_STATE\t\t0x02\n\
            STATE_FULL\t\t0x02\n\
            {\n\
            \tBOUND\t\t\"State_Full.bnd\"\n\
            \tIDC_FULL_SELF\t\t0X10000000\n\
            \tIDC_TEXT_FULL_TAIL\t\t0X1B400011\t\t// loads up to here\n\
            }\n\
            STATE_STUB\t\t0x00\n";
        let list = parse_control_list(text).unwrap();
        assert_eq!(list.num_states, Some(2));
        assert_eq!(list.blocks.len(), 2);
        let full = &list.blocks[0];
        assert_eq!(full.number, 2);
        assert_eq!(full.bound.len(), 2);
        assert_eq!(full.bound[1].code, 0x1B400011);
        assert_eq!(full.bound[1].sprite, None);
        let stub = &list.blocks[1];
        assert_eq!(stub.name, "STATE_STUB");
        assert_eq!(stub.number, 0);
        assert!(stub.bound.is_empty() && stub.sets.is_empty());
    }
    fn ojn_event(tick: u32, kind: u16, payload: &[u32]) -> Vec<u8> {
        let mut event = Vec::with_capacity(8 + payload.len() * 4);
        event.extend_from_slice(&tick.to_le_bytes());
        event.extend_from_slice(&kind.to_le_bytes());
        event.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        for unit in payload {
            event.extend_from_slice(&unit.to_le_bytes());
        }
        event
    }

    fn ojn_levels_blob(counts: [u32; OJN_LEVEL_COUNT], slices: &[Vec<u8>]) -> Vec<u8> {
        let mut blob = vec![0u8; OJN_HEADER_LEN];
        blob[0..4].copy_from_slice(&757u32.to_le_bytes());
        blob[4..8].copy_from_slice(b"ojn\0");
        blob[16..20].copy_from_slice(&124f32.to_le_bytes());
        blob[OJN_SLICES_START_OFF..OJN_SLICES_START_OFF + 4]
            .copy_from_slice(&(OJN_HEADER_LEN as u32).to_le_bytes());
        let mut end = OJN_HEADER_LEN;
        for (level, slice) in slices.iter().enumerate() {
            blob[OJN_EVENT_COUNT_OFFS[level]..OJN_EVENT_COUNT_OFFS[level] + 4]
                .copy_from_slice(&counts[level].to_le_bytes());
            end += slice.len();
            blob[OJN_SLICE_END_OFFS[level]..OJN_SLICE_END_OFFS[level] + 4]
                .copy_from_slice(&(end as u32).to_le_bytes());
        }
        for slice in slices {
            blob.extend_from_slice(slice);
        }
        blob
    }

    #[test]
    fn ojn_levels_split_three_slices_with_exact_walks() {
        // Mirrors the verified 7.55 layout: populated slices followed by
        // an empty trailing slice (Tutorial.ojn levels 1-2 shape).
        let slices = [
            [ojn_event(1, 5, &[13, 0]), ojn_event(2, 2, &[3])].concat(),
            ojn_event(1, 4, &[7]),
            Vec::new(),
        ];
        let blob = ojn_levels_blob([2, 1, 0], &slices);
        let levels = parse_ojn_levels(&blob).unwrap();
        assert_eq!(levels.len(), OJN_LEVEL_COUNT);
        assert_eq!(
            levels[0],
            OjnLevelSlice {
                level: 0,
                event_count: 2,
                data_start: OJN_HEADER_LEN,
                data_end: OJN_HEADER_LEN + slices[0].len(),
            }
        );
        assert_eq!(levels[1].event_count, 1);
        assert_eq!(levels[1].data_start, levels[0].data_end);
        assert_eq!(levels[2].event_count, 0);
        assert_eq!(levels[2].data_start, levels[2].data_end);
        assert_eq!(levels[2].data_end, blob.len());
    }

    #[test]
    fn ojn_levels_reject_count_mismatch() {
        let slices = [ojn_event(1, 5, &[13]), Vec::new(), Vec::new()];
        let blob = ojn_levels_blob([2, 0, 0], &slices);
        assert!(matches!(
            parse_ojn_levels(&blob),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn ojn_levels_reject_trailing_bytes() {
        // One event plus four stray bytes: the walk overshoots the
        // header-declared end.
        let mut first = ojn_event(1, 5, &[13]);
        first.extend_from_slice(&[0u8; 4]);
        let slices = [first, Vec::new(), Vec::new()];
        let blob = ojn_levels_blob([1, 0, 0], &slices);
        assert!(matches!(
            parse_ojn_levels(&blob),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn ojn_levels_reject_bad_input() {
        assert!(matches!(
            parse_ojn_levels(b"short"),
            Err(FileError::TooShort(_))
        ));
        let mut bad_magic = ojn_levels_blob([0, 0, 0], &[Vec::new(), Vec::new(), Vec::new()]);
        bad_magic[4..8].copy_from_slice(b"XXXX");
        assert!(matches!(
            parse_ojn_levels(&bad_magic),
            Err(FileError::UnknownFormat { .. })
        ));
        let mut bad_end = ojn_levels_blob([0, 0, 0], &[Vec::new(), Vec::new(), Vec::new()]);
        bad_end[OJN_SLICE_END_OFFS[0]..OJN_SLICE_END_OFFS[0] + 4]
            .copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            parse_ojn_levels(&bad_end),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn ojn_levels_decrypt_new_format_containers() {
        // Same encoder contract as `ojn_decrypt_round_trip`
        // (`FUN_00412180`): the level parser decrypts first. Key slots
        // live in the last seven plaintext bytes, so pad past the final
        // slice end (the image area in real files) to keep them clear.
        let mut plain =
            ojn_levels_blob([1, 0, 0], &[ojn_event(1, 5, &[13]), Vec::new(), Vec::new()]);
        plain.extend_from_slice(&[0xA5; 64]);
        let (k1, k2, k3, period) = (0x97u8, 0xE0u8, 0x34u8, 6usize);
        let mut buf: Vec<u8> = plain
            .iter()
            .enumerate()
            .map(|(i, &b)| {
                if i.is_multiple_of(period) {
                    b ^ k1
                } else if i % period == period / 2 {
                    b ^ k2
                } else {
                    b ^ k3
                }
            })
            .collect();
        let n = buf.len();
        buf[n - 7] = k1;
        buf[n - 6] = k2;
        buf[n - 5] = k3;
        buf[n - 4] = period as u8;
        buf[n - 3] = b'w';
        buf[n - 2] = b'e';
        buf[n - 1] = b'n';
        let encrypted: Vec<u8> = buf.iter().rev().copied().collect();
        assert!(encrypted.starts_with(b"new"));
        let levels = parse_ojn_levels(&encrypted).unwrap();
        assert_eq!(levels.len(), OJN_LEVEL_COUNT);
        assert_eq!(levels[0].event_count, 1);
        assert_eq!(levels[2].data_end + 64, plain.len());
    }

    #[test]
    fn ojn_lane_of_kind_maps_lanes_two_to_eight() {
        assert_eq!(ojn_lane_of_kind(0), None);
        assert_eq!(ojn_lane_of_kind(1), None);
        for kind in OJN_LANE_KIND_FIRST..=OJN_LANE_KIND_LAST {
            assert_eq!(ojn_lane_of_kind(kind), Some((kind - 2) as u8));
        }
        assert_eq!(ojn_lane_of_kind(9), None);
        assert_eq!(ojn_lane_of_kind(19), None);
    }

    #[test]
    fn decode_note_value_splits_sample_plus_one_and_flags() {
        // Lane tap: sample id 2 stored as 3, no flags.
        assert_eq!(
            decode_note_value(3),
            Some(OjnNoteValue {
                sample_id: 2,
                flags: 0,
            })
        );
        // Flagged autoplay note keeps the raw high word.
        assert_eq!(
            decode_note_value(0x00D0_0003),
            Some(OjnNoteValue {
                sample_id: 2,
                flags: 0xD0,
            })
        );
        assert_eq!(decode_note_value(0), None);
        assert_eq!(decode_note_value(0x0200_0000), None);
    }

    #[test]
    fn ojn_sub_position_places_sixteenths_on_twelve_tick_grid() {
        assert_eq!(ojn_sub_position(0, 16), Some(0));
        assert_eq!(ojn_sub_position(4, 16), Some(48));
        assert_eq!(ojn_sub_position(15, 16), Some(180));
        assert_eq!(ojn_sub_position(189, 192), Some(189));
        assert_eq!(ojn_sub_position(0, 1), Some(0));
        assert_eq!(ojn_sub_position(0, 0), None);
        assert_eq!(ojn_sub_position(1, 7), None);
    }

    #[test]
    fn ojn_level_events_decode_measure_kind_and_values() {
        // Synthetic two-measure level: a lane tap, a BPM float word
        // (kept raw here; the game layer interprets kinds 0/1), and
        // an autoplay note with flags.
        let bpm_bits = 134f32.to_bits();
        let slices = [
            [
                ojn_event(1, 2, &[3, 0, 0, 0]),
                ojn_event(1, 1, &[bpm_bits]),
                ojn_event(2, 10, &[0x00D0_0002]),
            ]
            .concat(),
            Vec::new(),
            Vec::new(),
        ];
        let blob = ojn_levels_blob([3, 0, 0], &slices);
        let events = parse_ojn_level_events(&blob, 0).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].measure, 1);
        assert_eq!(events[0].kind, 2);
        assert_eq!(events[0].values, vec![3, 0, 0, 0]);
        assert_eq!(events[1].kind, 1);
        assert_eq!(events[2].kind, 10);
        assert_eq!(
            decode_note_value(events[2].values[0]),
            Some(OjnNoteValue {
                sample_id: 1,
                flags: 0xD0,
            })
        );
        assert!(parse_ojn_level_events(&blob, 1).unwrap().is_empty());
        assert!(matches!(
            parse_ojn_level_events(&blob, 3),
            Err(FileError::Truncated { .. })
        ));
    }

    fn interleave_stripes(plain: &[u8], lanes: usize) -> Vec<u8> {
        let size = plain.len();
        let quotient = size / lanes;
        let row: &[u16] = match lanes {
            16 => &DEINTERLEAVE_16[size % 16],
            _ => &DEINTERLEAVE_17[size % 17],
        };
        let mut out = vec![0u8; size];
        for (stripe, slot) in row.iter().enumerate() {
            let from = *slot as usize * quotient;
            let to = stripe * quotient;
            out[to..to + quotient].copy_from_slice(&plain[from..from + quotient]);
        }
        out[lanes * quotient..].copy_from_slice(&plain[lanes * quotient..]);
        out
    }

    #[test]
    fn deinterleave_round_trips_synthetic_layouts() {
        for lanes in [16, 17] {
            for extra in [0, 3, 19] {
                let size = lanes * 64 + extra;
                let plain: Vec<u8> = (0..size).map(|i| (i * 7 + 1) as u8).collect();
                let encoded = interleave_stripes(&plain, lanes);
                assert_ne!(encoded, plain);
                assert_eq!(deinterleave_stripes(&encoded, lanes).unwrap(), plain);
            }
        }
    }

    #[test]
    fn deinterleave_rejects_unknown_lane_counts() {
        assert!(matches!(
            deinterleave_stripes(&[0u8; 32], 8),
            Err(FileError::UnknownFormat { .. })
        ));
    }
}
