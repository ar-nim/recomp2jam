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
    // Community-lore magics, treated as candidates (UNVERIFIED for 7.55):
    // OJM variants "M30\0"?/"M32\0"? and OJN "ojn"-ish headers.
    // We check ASCII-printable tags without committing to offsets.
    if bytes.starts_with(b"M30") || bytes.starts_with(b"M32") {
        return Ok(AssetKind::Ojm);
    }
    if bytes.starts_with(b"ojn") || bytes.starts_with(b"OJN") {
        return Ok(AssetKind::Ojn);
    }
    Err(FileError::UnknownFormat {
        magic: bytes[..4.min(bytes.len())].to_vec(),
    })
}

/// Minimal parsed view of an `.ojm` container (stub).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjmArchive {
    /// Raw magic tag seen.
    pub magic: [u8; 4],
}

/// Minimal parsed view of an `.ojn` chart file (stub).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjnChartFile {
    /// Raw magic tag seen.
    pub magic: [u8; 4],
}

/// Parse an `.ojm` container. Currently verifies the header tag then
/// returns [`FileError::NotImplemented`] so callers can distinguish
/// "recognised, RE pending" from corrupt input.
pub fn parse_ojm(bytes: &[u8]) -> Result<OjmArchive, FileError> {
    match detect_kind(bytes)? {
        AssetKind::Ojm => Err(FileError::NotImplemented(
            "OJM sample table (see docs/file-formats/ojm.md)",
        )),
        AssetKind::Ojn => Err(FileError::UnknownFormat {
            magic: bytes[..4].to_vec(),
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
        assert_eq!(detect_kind(b"ojnXXX...."), Ok(AssetKind::Ojn));
    }

    #[test]
    fn rejects_unknown_magic() {
        let err = detect_kind(b"ZZZZ....").unwrap_err();
        assert!(matches!(err, FileError::UnknownFormat { .. }));
    }

    #[test]
    fn stubs_report_not_implemented() {
        assert_eq!(
            parse_ojm(b"M30\x00...."),
            Err(FileError::NotImplemented(
                "OJM sample table (see docs/file-formats/ojm.md)"
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
