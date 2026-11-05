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
const OPI_AUX_SIZE_OFF: usize = 140;

/// One file inside an OPI/OPA archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpiEntry {
    /// File name (NUL-trimmed ASCII: `*.ojs`, `*.bnd`, `*.txt`, …).
    pub name: String,
    /// Entry kind from the archive: 1 = file, 0 = free slot.
    pub kind: u32,
    /// Offset of the stored bytes from the archive start.
    pub data_offset: usize,
    /// Stored byte count.
    pub size: u32,
    /// Second size field (meaning TBD; equals `size` for stored
    /// files, larger for the structured `55 05`-magic bodies).
    pub aux_size: u32,
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

/// Parse an OPI/OPA archive directory (trailing table). Returns one
/// entry per named file; nameless free slots are skipped. Stored
/// bytes are NOT validated (script, text, and binary bodies share
/// the container).
/// Verified on real 7.55 files (docs/file-formats/opi.md).
pub fn parse_opi(bytes: &[u8]) -> Result<Vec<OpiEntry>, FileError> {
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
    let mut out = Vec::new();
    let mut off = bytes.len() - table_len;
    for _ in 0..slots {
        let kind = read_u32le(bytes, off)?;
        let name = read_opi_name(bytes, off + OPI_NAME_OFF)?;
        let data_offset = read_u32le(bytes, off + OPI_OFFSET_OFF)? as usize;
        let size = read_u32le(bytes, off + OPI_SIZE_OFF)?;
        let aux_size = read_u32le(bytes, off + OPI_AUX_SIZE_OFF)?;
        let end = data_offset
            .checked_add(size as usize)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: bytes.len(),
            })?;
        if end > bytes.len() {
            return Err(FileError::Truncated {
                need: end,
                have: bytes.len(),
            });
        }
        if !name.is_empty() {
            out.push(OpiEntry {
                name,
                kind,
                data_offset,
                size,
                aux_size,
            });
        }
        off += OPI_ENTRY_LEN;
    }
    Ok(out)
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
/// Field map verified on real 7.55 data (record 437 = song 757;
/// cross-song BPM check 124/168/103). Offsets are in-record.
#[derive(Debug, Clone, PartialEq)]
pub struct OjnListEntry<'a> {
    /// Record index in the file (record 0 is the header).
    pub index: usize,
    /// Song id (`u32` at +4).
    pub song_id: u32,
    /// Tempo (`f32` at +20; reads 124.0/168.0/103.0 on songs
    /// 757/770/860).
    pub bpm: f32,
    /// Title area (+112, NUL-trimmed, lossy: Korean + tags).
    pub title: String,
    /// Artist area (+176, NUL-trimmed, lossy).
    pub artist: String,
    /// Label (+208, e.g. `NoteFactory`).
    pub label: String,
    /// Container filename (+240, e.g. `o2ma757.ojm`).
    pub ojm_file: String,
    /// Raw record bytes for fields not yet mapped.
    pub raw: &'a [u8],
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
    Ok(String::from_utf8_lossy(raw)
        .trim_matches('\0')
        .trim()
        .to_string())
}

/// Parse an `OJNList.dat` song index: 300-byte records; record 0 is the
/// header (`562`, `100`, `"ojn"`). A trailing partial record (148
/// bytes in 7.55) is ignored; the `562` count semantics are TBD.
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
        if index > 0 {
            out.push(OjnListEntry {
                index,
                song_id: read_u32le(bytes, off + 4)?,
                bpm: f32::from_le_bytes(rec[20..24].try_into().map_err(|_| {
                    FileError::Truncated {
                        need: off + 24,
                        have: bytes.len(),
                    }
                })?),
                title: read_str(bytes, off + 112, 64)?,
                artist: read_str(bytes, off + 176, 32)?,
                label: read_str(bytes, off + 208, 32)?,
                ojm_file: read_str(bytes, off + 240, 32)?,
                raw: rec,
            });
        }
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

    #[test]
    fn ojn_list_round_trip() {
        let mut blob = vec![0u8; 600];
        blob[8..11].copy_from_slice(b"ojn");
        blob[300 + 4..300 + 8].copy_from_slice(&757u32.to_le_bytes());
        blob[300 + 20..300 + 24].copy_from_slice(&124f32.to_le_bytes());
        blob[300 + 112..300 + 117].copy_from_slice(b"Title");
        blob[300 + 240..300 + 250].copy_from_slice(b"o2ma757.oj");
        let entries = parse_ojn_list(&blob).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].index, 1);
        assert_eq!(entries[0].song_id, 757);
        assert_eq!(entries[0].bpm, 124.0);
        assert_eq!(entries[0].title, "Title");
        assert_eq!(entries[0].ojm_file, "o2ma757.oj");
        assert!(matches!(
            parse_ojn_list(b"nope"),
            Err(FileError::UnknownFormat { .. })
        ));
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
        entry[OPI_AUX_SIZE_OFF..OPI_AUX_SIZE_OFF + 4].copy_from_slice(&size.to_le_bytes());
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
}
