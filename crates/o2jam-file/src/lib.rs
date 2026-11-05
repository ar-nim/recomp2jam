//! File parsing for O2Jam 7.55 assets (`.ojm` music, `.ojn` charts).
//!
//! Status: day-0 scaffolding. Magic sniffing works; full parsers are
//! documented stubs until verified against installed 7.55 data.
//! See `docs/file-formats/`.

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
    Err(FileError::UnknownFormat {
        magic: bytes[..4.min(bytes.len())].to_vec(),
    })
}

/// One key-sound / BGM stream inside an `.ojm` container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjmSample<'a> {
    /// Sample name (NUL-trimmed ASCII).
    pub name: String,
    /// Numeric sample id (drives chart→audio mapping).
    pub sample_id: u32,
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
            sample_id,
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

/// Parse the OMC variant: 128-byte header (first stream's
/// `name[32]` at 92, `size` at 124), then 36-byte entries
/// (`name[32] + size`) each followed by its OGG. Blank (all-zero)
/// 32-byte names are skipped. Verified byte-exact on real 7.55 files.
pub fn parse_omc(bytes: &[u8]) -> Result<Vec<OjmSample<'_>>, FileError> {
    if bytes.len() < 128 {
        return Err(FileError::TooShort(bytes.len()));
    }
    if !bytes.starts_with(b"OMC") {
        return Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        });
    }
    let mut off = 92usize;
    let mut out = Vec::new();
    loop {
        let name = read_name(bytes, off)?;
        let size = read_u32le(bytes, off + 32)? as usize;
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
        if name.is_empty() && size == 0 {
            // Blank slot: no stream follows; entry was padding.
            off = start;
        } else {
            check_ogg(bytes, start)?;
            out.push(OjmSample {
                name,
                sample_id: u32::MAX,
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

/// Minimal parsed view of an `.ojn` chart file (stub).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjnChartFile {
    /// Raw magic tag seen.
    pub magic: [u8; 4],
}

/// Parse an `.ojm` container into its sample streams.
///
/// Dispatches on magic: `M30` and `OMC` variants are fully implemented;
/// the `OJM` variant stays [`FileError::NotImplemented`] until its
/// encrypted region is understood (docs/file-formats/ojm.md).
pub fn parse_ojm(bytes: &[u8]) -> Result<Vec<OjmSample<'_>>, FileError> {
    match detect_kind(bytes)? {
        AssetKind::Ojm if bytes.starts_with(b"M30") => parse_m30(bytes),
        AssetKind::Ojm if bytes.starts_with(b"OMC") => parse_omc(bytes),
        AssetKind::Ojm => Err(FileError::NotImplemented(
            "OJM variant encrypted region (see docs/file-formats/ojm.md)",
        )),
        AssetKind::Ojn => Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        }),
    }
}

/// Parse an `.ojn` chart file. Same stub contract as [`parse_ojm`].
pub fn parse_ojn(bytes: &[u8]) -> Result<OjnChartFile, FileError> {
    match detect_kind(bytes)? {
        AssetKind::Ojn => Err(FileError::NotImplemented(
            "OJN note streams (see docs/file-formats/ojn.md)",
        )),
        AssetKind::Ojm => Err(FileError::UnknownFormat {
            magic: bytes[..4].to_vec(),
        }),
    }
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
        assert_eq!(samples[0].sample_id, 3);
        assert_eq!(samples[0].data, b"OggSdata");
        assert_eq!(samples[1].name, "mr1");
        assert_eq!(samples[1].sample_id, 980);
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

    fn omc_blob() -> Vec<u8> {
        let mut b = vec![0u8; 128];
        b[..4].copy_from_slice(b"OMC\0");
        b[92..100].copy_from_slice(b"bgRoom.o");
        b[124..128].copy_from_slice(&8u32.to_le_bytes());
        b.extend_from_slice(b"OggSdata");
        let mut e = vec![0u8; 36];
        e[..9].copy_from_slice(b"bgLogo.gg");
        e[32..36].copy_from_slice(&8u32.to_le_bytes());
        b.extend_from_slice(&e);
        b.extend_from_slice(b"OggSlogo");
        b
    }

    #[test]
    fn omc_round_trip_with_first_stream_in_header() {
        let blob = omc_blob();
        let samples = parse_ojm(&blob).unwrap();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].name, "bgRoom.o");
        assert_eq!(samples[0].data, b"OggSdata");
        assert_eq!(samples[1].name, "bgLogo.gg");
        assert_eq!(samples[1].data, b"OggSlogo");
    }

    #[test]
    fn ojm_variant_still_stubbed() {
        assert_eq!(
            parse_ojm(b"OJM\x00...."),
            Err(FileError::NotImplemented(
                "OJM variant encrypted region (see docs/file-formats/ojm.md)"
            ))
        );
        assert_eq!(
            parse_ojn(b"ojnX...."),
            Err(FileError::NotImplemented(
                "OJN note streams (see docs/file-formats/ojn.md)"
            ))
        );
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
}
