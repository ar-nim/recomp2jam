//! File parsing for O2Jam 7.55 assets (`.ojm` music, `.ojn` charts,
//! `OJNList.dat` song index).
//!
//! M30/OMC containers, the OJNList index, and old/new-format `.ojn`
//! note streams are verified against real 7.55 data (see
//! `docs/file-formats/`).

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

/// Signed little-endian word for animation frame origins.
fn read_i16le(bytes: &[u8], off: usize) -> Result<i16, FileError> {
    read_u16le(bytes, off).map(|raw| raw as i16)
}

pub mod control;
pub mod ojm;
pub mod ojn;
pub mod opi;
pub mod sprites;
pub mod tables;

pub use control::*;
pub use ojm::*;
pub use ojn::*;
pub use opi::*;
pub use sprites::*;
pub use tables::*;

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

    #[test]
    fn m30_sample_id_reads_ref_word_only() {
        // Scenario: the id word is `ref` + reserved-high (open2jam
        // `M30_sample_header` split); only the low `ref` word names
        // the strike slot, so a nonzero reserved word is ignored.
        let mut blob = m30_blob();
        // First entry id field at 28 (header) + 44: ref 5, reserved 0xFF.
        blob[28 + 44..28 + 46].copy_from_slice(&5u16.to_le_bytes());
        blob[28 + 46..28 + 48].copy_from_slice(&0xFFu16.to_le_bytes());
        let samples = parse_ojm(&blob).unwrap();
        assert_eq!(samples[0].sample_id, Some(5));
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
        // Synthetic old-format header: id, magic, version, BPM, count.
        let mut blob = vec![0u8; 300];
        blob[0..4].copy_from_slice(&757u32.to_le_bytes());
        blob[4..8].copy_from_slice(b"ojn\0");
        blob[8..12].copy_from_slice(&2.9f32.to_le_bytes());
        blob[16..20].copy_from_slice(&124f32.to_le_bytes());
        blob[64..68].copy_from_slice(&208u32.to_le_bytes());
        let chart = parse_ojn(&blob).unwrap();
        assert_eq!(chart.song_id, 757);
        assert_eq!(chart.format_version, 2.9);
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

    fn mode2_entry(name: &[u8], size: u32, channels: u16, rate: u32) -> Vec<u8> {
        let mut entry = vec![0u8; MODE2_ENTRY_LEN];
        entry[..name.len()].copy_from_slice(name);
        // WAV audio params in open2jam `OMC_WAV_header` order: PCM
        // format, channel count, sample rate, byte rate, alignment,
        // bits per sample. Byte rate and alignment derive from the
        // channel count like the shipped entries.
        entry[MODE2_FORMAT_OFF..MODE2_FORMAT_OFF + 2].copy_from_slice(&1u16.to_le_bytes());
        entry[MODE2_CHANNELS_OFF..MODE2_CHANNELS_OFF + 2].copy_from_slice(&channels.to_le_bytes());
        entry[MODE2_RATE_OFF..MODE2_RATE_OFF + 4].copy_from_slice(&rate.to_le_bytes());
        entry[MODE2_BITRATE_OFF..MODE2_BITRATE_OFF + 4]
            .copy_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        entry[MODE2_ALIGN_OFF..MODE2_ALIGN_OFF + 2].copy_from_slice(&(channels * 2).to_le_bytes());
        entry[MODE2_BITS_OFF..MODE2_BITS_OFF + 2].copy_from_slice(&16u16.to_le_bytes());
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
        blob.extend_from_slice(&mode2_entry(b"s1", 16, 1, 22050));
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
        assert_eq!(tables.mode2[0].audio_format, 1);
        assert_eq!(tables.mode2[0].num_channels, 1);
        assert_eq!(tables.mode2[0].sample_rate, 22050);
        assert_eq!(tables.mode2[0].bit_rate, 44100);
        assert_eq!(tables.mode2[0].block_align, 2);
        assert_eq!(tables.mode2[0].bits_per_sample, 16);
        assert_eq!(tables.mode3[0].audio_format, 0);
        assert_eq!(tables.mode3[0].num_channels, 0);
        assert_eq!(tables.mode3[0].sample_rate, 0);
        assert_eq!(tables.mode3[0].bit_rate, 0);
        assert_eq!(tables.mode3[0].block_align, 0);
        assert_eq!(tables.mode3[0].bits_per_sample, 0);
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
    fn new_variant_entries_parse_stereo_wav_params() {
        // Scenario: a stereo 44100 Hz entry like DialogBGM/bgEffect/
        // Planet bodies (2/44100/176400/4/16) parses to named fields.
        let mut blob = vec![0u8; 20];
        blob[..4].copy_from_slice(b"OMC\0");
        blob[MODE2_COUNT_OFF..MODE2_COUNT_OFF + 2].copy_from_slice(&1u16.to_le_bytes());
        blob[MODE2_ENTRIES_OFF..MODE2_ENTRIES_OFF + 4].copy_from_slice(&20u32.to_le_bytes());
        blob.extend_from_slice(&mode2_entry(b"st", 4, 2, 44100));
        blob.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        let tables = parse_new_variant_entries(&blob).unwrap();
        assert_eq!(tables.mode2.len(), 1);
        let entry = &tables.mode2[0];
        assert_eq!(entry.audio_format, 1);
        assert_eq!(entry.num_channels, 2);
        assert_eq!(entry.sample_rate, 44100);
        assert_eq!(entry.bit_rate, 176400);
        assert_eq!(entry.block_align, 4);
        assert_eq!(entry.bits_per_sample, 16);
        assert_eq!(entry.size, 4);
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

    fn order_tables() -> OjmTables {
        // Scenario: two mode-2 key sounds followed by one mode-3 BGM
        // split, mirroring Tutorial.ojm's 30 + 12 shape at small scale.
        let entry = |name: &str| OjmEntry {
            name: name.to_string(),
            size: 8,
            data_offset: 0,
            audio_format: 1,
            num_channels: 1,
            sample_rate: 44100,
            bit_rate: 88200,
            block_align: 2,
            bits_per_sample: 16,
        };
        OjmTables {
            mode2: vec![entry("apiano1"), entry("apiano2")],
            mode3: vec![entry("bgsplit-01")],
        }
    }

    #[test]
    fn order_slot_maps_mode2_then_mode3_with_high_bank_alias() {
        let tables = order_tables();
        assert_eq!(tables.resolve_order_slot(0).unwrap().name, "apiano1");
        assert_eq!(tables.resolve_order_slot(1).unwrap().name, "apiano2");
        assert_eq!(tables.resolve_order_slot(2).unwrap().name, "bgsplit-01");
        assert!(tables.resolve_order_slot(3).is_none());
        // Bit26-flagged strikes land in the high bank but name the
        // same fetch-order entries (Tutorial.ojn intro: slots 1000+N).
        assert_eq!(
            tables
                .resolve_order_slot(NEW_VARIANT_HIGH_BANK)
                .unwrap()
                .name,
            "apiano1"
        );
        assert_eq!(
            tables
                .resolve_order_slot(NEW_VARIANT_HIGH_BANK + 2)
                .unwrap()
                .name,
            "bgsplit-01"
        );
        assert!(tables
            .resolve_order_slot(NEW_VARIANT_HIGH_BANK + 3)
            .is_none());
    }

    #[test]
    fn entry_sample_bytes_bounds_checks() {
        let bytes = b"0123456789abcdef";
        let ok = OjmEntry {
            name: "s".to_string(),
            size: 4,
            data_offset: 6,
            audio_format: 1,
            num_channels: 1,
            sample_rate: 44100,
            bit_rate: 88200,
            block_align: 2,
            bits_per_sample: 16,
        };
        assert_eq!(ok.sample_bytes(bytes).unwrap(), b"6789");
        let past_end = OjmEntry {
            name: "s".to_string(),
            size: 4,
            data_offset: 14,
            audio_format: 1,
            num_channels: 1,
            sample_rate: 44100,
            bit_rate: 88200,
            block_align: 2,
            bits_per_sample: 16,
        };
        assert!(matches!(
            past_end.sample_bytes(bytes),
            Err(FileError::Truncated { .. })
        ));
    }

    /// Encryption-side mirror of [`not_decrypt`]: same key schedule,
    /// but the 8-byte reset feeds back the ciphertext byte it just
    /// wrote (decryption feeds back the byte it just read). Test-only:
    /// the game never encrypts. Seeded `(0, 0)` like the exe's
    /// mode-2/3 path (only mode-1 forces `0xFF`/`0x00`).
    fn not_encrypt(buf: &mut [u8], mut key1: u8, mut key2: u8) {
        for byte in buf.iter_mut() {
            if (key1 << key2) & 0x80 != 0 {
                *byte = !*byte;
            }
            if key2 >= 7 {
                key2 = 0;
                key1 = *byte;
            } else {
                key2 += 1;
            }
        }
    }

    fn encrypted_body(plain: &[u8], lanes: usize) -> Vec<u8> {
        // Encode mirrors the loader in reverse: NOT-cipher, then
        // scatter into file stripe order.
        let mut cipher = plain.to_vec();
        not_encrypt(&mut cipher, 0x00, 0x00);
        interleave_stripes(&cipher, lanes).unwrap()
    }

    /// Minimal single-page OGG fixture with a valid checksum (header,
    /// segment table, deterministic filler body sized by the table).
    fn ogg_page_fixture(segments: &[u8]) -> Vec<u8> {
        let mut page = Vec::new();
        page.extend_from_slice(b"OggS");
        page.push(0); // version
        page.push(0x02); // header type: beginning of stream
        page.extend_from_slice(&0u64.to_le_bytes()); // granule position
        page.extend_from_slice(&1u32.to_le_bytes()); // bitstream serial
        page.extend_from_slice(&0u32.to_le_bytes()); // page sequence
        page.extend_from_slice(&0u32.to_le_bytes()); // checksum placeholder
        page.push(segments.len() as u8);
        page.extend_from_slice(segments);
        let body_len: usize = segments.iter().map(|&b| b as usize).sum();
        page.extend((0u8..=255).cycle().take(body_len));
        let checksum = crc32_ogg(&page);
        page[22..26].copy_from_slice(&checksum.to_le_bytes());
        page
    }

    #[test]
    fn recover_new_variant_sample_round_trips_both_lane_counts() {
        // Scenario: a checksum-valid synthetic OGG page survives a
        // full encrypt (NOT + 16/17-way scatter) then `recover` cycle.
        // Four 200-byte segments keep every stripe non-empty.
        for lanes in [16, 17] {
            let plain = ogg_page_fixture(&[200, 200, 200, 200]);
            let encoded = encrypted_body(&plain, lanes);
            assert_ne!(encoded, plain);
            assert_eq!(recover_new_variant_sample(&encoded).unwrap(), plain);
        }
    }

    #[test]
    fn descriptor_selects_lanes_and_not_from_version() {
        // Scenario: shipped per-song floats map to the loader's flag
        // arms (17-way + NOT, 16-way + NOT, plain).
        assert_eq!(
            ojm_descriptor_for_version(2.9),
            OjmDescriptor {
                lanes: Some(17),
                not: true
            }
        );
        assert_eq!(
            ojm_descriptor_for_version(2.7),
            OjmDescriptor {
                lanes: Some(16),
                not: true
            }
        );
        assert_eq!(
            ojm_descriptor_for_version(2.4),
            OjmDescriptor {
                lanes: None,
                not: false
            }
        );
    }

    #[test]
    fn decode_new_variant_pcm_round_trips_synthetic_frames() {
        // Scenario: synthetic PCM16 frames survive a full encrypt
        // (NOT + 16-way scatter) then `decode` cycle with the
        // Tutorial descriptor (2.7: 16-way + NOT).
        let descriptor = ojm_descriptor_for_version(2.7);
        let frames: Vec<i16> = (0..2048).map(|i| (i * 13 % 5000) - 2500).collect();
        let mut plain = Vec::with_capacity(frames.len() * 2);
        for frame in &frames {
            plain.extend_from_slice(&frame.to_le_bytes());
        }
        let encoded = encrypted_body(&plain, 16);
        assert_ne!(encoded, plain);
        assert_eq!(
            decode_new_variant_pcm(&encoded, descriptor).unwrap(),
            frames
        );
        // Plain descriptor (2.4 and below) skips both passes.
        let raw_descriptor = ojm_descriptor_for_version(2.4);
        assert_eq!(
            decode_new_variant_pcm(&plain, raw_descriptor).unwrap(),
            frames
        );
        // Odd trailing bytes are not whole frames.
        assert!(matches!(
            decode_new_variant_pcm(&[0x00], descriptor),
            Err(FileError::Truncated { .. })
        ));
    }

    #[test]
    fn recover_new_variant_sample_passes_raw_and_rejects_garbage() {
        // Mode-3 bodies ship raw: a checksum-valid page passes through.
        let plain = ogg_page_fixture(&[8]);
        assert_eq!(recover_new_variant_sample(&plain).unwrap(), plain);
        // A bare `OggS` prefix without a valid page is not enough:
        // wrong-lane decodes can preserve the head while scrambling
        // the page, so they must not validate.
        assert!(matches!(
            recover_new_variant_sample(b"OggSraw-body"),
            Err(FileError::UnknownFormat { .. })
        ));
        // Corrupted pages are unrecoverable, not silent.
        let mut corrupt = plain.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0xFF;
        assert!(matches!(
            recover_new_variant_sample(&corrupt),
            Err(FileError::UnknownFormat { .. })
        ));
        // No lane count yields an OGG page: unrecoverable, not silent.
        assert!(matches!(
            recover_new_variant_sample(&[0xABu8; 64]),
            Err(FileError::UnknownFormat { .. })
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

    fn set_info_row(id: u32, name: &[u8], description: &[u8]) -> Vec<u8> {
        let mut row = Vec::new();
        let mut fixed = [0u32; 17];
        fixed[0] = id;
        fixed[1] = 0x0314_800b;
        fixed[2] = 0x228 + id;
        fixed[7] = 1200;
        for word in fixed {
            row.extend_from_slice(&word.to_le_bytes());
        }
        row.push(0);
        row.extend_from_slice(&(name.len() as u32).to_le_bytes());
        row.extend_from_slice(name);
        row.extend_from_slice(&(description.len() as u32).to_le_bytes());
        row.extend_from_slice(description);
        row
    }

    fn set_info_blob(rows: &[Vec<u8>], trailer: &[u8]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&(rows.len() as u32).to_le_bytes());
        blob.extend_from_slice(&[0u8; 77]);
        for row in rows {
            blob.extend_from_slice(row);
        }
        blob.extend_from_slice(trailer);
        blob
    }

    #[test]
    fn set_info_parses_rows_and_keeps_trailer() {
        let blob = set_info_blob(
            &[
                set_info_row(1, b"set-one", b"first set"),
                set_info_row(2, b"set-two", b"second set"),
            ],
            b"promo",
        );
        let table = parse_set_info_table(&blob).unwrap();
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.trailer_len, 5);
        assert_eq!(
            table.rows[0],
            SetInfoRow {
                set_id: 1,
                group: 0x0314_800b,
                items: [0x229, 0, 0, 0, 0],
                stats: [1200, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                name: "set-one".to_owned(),
                description: "first set".to_owned(),
            }
        );
        assert_eq!(table.rows[1].name, "set-two");
    }

    #[test]
    fn set_info_stops_at_id_break() {
        let blob = set_info_blob(
            &[set_info_row(1, b"a", b"b"), set_info_row(9, b"c", b"d")],
            &[],
        );
        let table = parse_set_info_table(&blob).unwrap();
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.trailer_len, 68 + 1 + (4 + 1) + (4 + 1));
    }

    fn avatar_item_row(id: u32, name: &[u8], sprites: &[&str]) -> Vec<u8> {
        let mut row = Vec::new();
        row.extend_from_slice(&id.to_le_bytes());
        row.extend_from_slice(&0x106u32.to_le_bytes());
        row.extend_from_slice(&0u32.to_le_bytes());
        row.push(1);
        row.extend_from_slice(&1000u32.to_le_bytes());
        row.extend_from_slice(&[0u8; 12]);
        row.push(0);
        row.extend_from_slice(&(name.len() as u32).to_le_bytes());
        row.extend_from_slice(name);
        row.extend_from_slice(&3u32.to_le_bytes());
        row.extend_from_slice(b"dsc");
        for name in sprites {
            row.push(1);
            row.extend_from_slice(&(name.len() as u32).to_le_bytes());
            row.extend_from_slice(name.as_bytes());
        }
        row
    }

    fn avatar_item_blob(count_plus_one: u32, rows: &[Vec<u8>]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&count_plus_one.to_le_bytes());
        blob.extend_from_slice(&[0u8; 80]);
        for row in rows {
            blob.extend_from_slice(row);
        }
        blob
    }

    fn forty_two_sprites() -> Vec<&'static str> {
        let mut slots = vec![""; AVATAR_SPRITE_SLOTS];
        slots[0] = "01F01001.oji";
        slots
    }

    #[test]
    fn avatar_items_parse_with_sprite_slots() {
        let blob = avatar_item_blob(
            3,
            &[
                avatar_item_row(1, b"hair-one", &forty_two_sprites()),
                avatar_item_row(2, b"hair-two", &forty_two_sprites()),
            ],
        );
        let table = parse_avatar_item_table(&blob).unwrap();
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].item_id, 1);
        assert_eq!(table.rows[0].item_type, 0x106);
        assert_eq!(table.rows[0].price, 1000);
        assert_eq!(table.rows[0].name, "hair-one");
        assert_eq!(table.rows[0].description, "dsc");
        assert_eq!(table.rows[0].sprites.len(), AVATAR_SPRITE_SLOTS);
        assert_eq!(table.rows[0].sprites[0], (1, "01F01001.oji".to_owned()));
        assert_eq!(table.rows[1].sprites[0], (1, "01F01001.oji".to_owned()));
    }

    #[test]
    fn avatar_items_reject_count_mismatch() {
        let blob = avatar_item_blob(9, &[avatar_item_row(1, b"a", &forty_two_sprites())]);
        assert!(parse_avatar_item_table(&blob).is_err());
    }

    #[test]
    fn avatar_items_reject_truncated_row() {
        let mut blob = avatar_item_blob(2, &[avatar_item_row(1, b"a", &forty_two_sprites())]);
        blob.truncate(blob.len() - 10);
        assert!(parse_avatar_item_table(&blob).is_err());
    }

    #[test]
    fn set_info_rejects_empty_table() {
        assert!(parse_set_info_table(&[]).is_err());
        assert!(parse_set_info_table(&[1, 0, 0, 0]).is_err());
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

    #[test]
    fn deinterleave_round_trips_synthetic_layouts() {
        for lanes in [16, 17] {
            for extra in [0, 3, 19] {
                let size = lanes * 64 + extra;
                let plain: Vec<u8> = (0..size).map(|i| (i * 7 + 1) as u8).collect();
                let encoded = interleave_stripes(&plain, lanes).unwrap();
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
