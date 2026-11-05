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

/// Parse the OMC/OJM variants: 16-byte header with the first entry
/// offset at +12, then 36-byte entries (`name[32] + size`) each
/// followed by its OGG. Blank (all-zero) entries are skipped. Files
/// whose entry offset equals the file length (fully encrypted bodies
/// like `DialogBGM.ojm`) yield zero samples; encrypted bodies and
/// garbage name fields are left for the decrypt work.
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
        if i % p == 0 {
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
        }),
        AssetKind::Ojm | AssetKind::OjnList => Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
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
            if i % p == 0 {
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
        let old = parse_ojn(b"ojnX....").unwrap();
        assert_eq!(old.magic, *b"ojnX");
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
        let chart = parse_ojn(b"ojnX....").unwrap();
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
}
