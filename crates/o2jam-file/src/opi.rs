//! OPI/OPA archives: entry tables plus JPEG sniffing.
//!
//! Layout verified on real 7.55 files (docs/file-formats/opi.md).

use super::{read_u32le, FileError};

/// OPI/OPA archive layout, verified on real 7.55 files
/// (`Interface1.opi`, `Playing1.opi`, `avatar.opa`;
/// docs/file-formats/opi.md): 16-byte header (`u32` version +
/// `u32` slot count + `u64` zero), data blobs from +16, and a
/// trailing table of fixed-size entries. Versions observed: 2
/// (`.opi`), 1 (`.opa`); the layout is identical. Public for
/// test-archive builders (real archives stay local-only).
pub const OPI_HEADER_LEN: usize = 16;

pub const OPI_ENTRY_LEN: usize = 152;

pub const OPI_NAME_OFF: usize = 4;

pub const OPI_NAME_LEN: usize = 128;

pub const OPI_OFFSET_OFF: usize = 132;

pub const OPI_SIZE_OFF: usize = 136;

pub const OPI_RESERVED_OFF: usize = 140;

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
    #[must_use]
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
#[must_use]
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
