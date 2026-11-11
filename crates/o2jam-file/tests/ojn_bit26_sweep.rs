//! Structural bit-26 sweep over the shipped 7.55 `.ojn` charts
//! (claim B50, issue #83).
//!
//! Parser-independent by construction: the only transform applied is
//! the byte-level new-format decrypt ([`o2jam_file::decrypt_ojn_new_format`]);
//! slice offsets (+284/+288/+292/+296), event counts (+64/+68/+72),
//! and the `8 + count * 4` event framing are re-read here as raw
//! little-endian words, so the sweep does not depend on the semantic
//! decoder it is auditing.
//!
//! Skips cleanly when the git-ignored `install-root` is absent (CI).

use std::path::PathBuf;

use o2jam_file::decrypt_ojn_new_format;

/// Repository-local `install-root` (git-ignored game data).
fn music_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root/O2Jam/Music");
    dir.is_dir().then_some(dir)
}

fn u32le(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(bytes[off..off + 4].try_into().expect("u32le"))
}

fn u16le(bytes: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(bytes[off..off + 2].try_into().expect("u16le"))
}

/// One bit-26-set note payload found in a chart.
#[derive(Debug, PartialEq, Eq)]
struct Bit26Hit {
    level: usize,
    measure: i32,
    kind: u16,
    raw: u32,
}

/// Walk every event of every level slice, collecting note payloads
/// (kinds 2+, nonzero low word) with bit 26 (`0x0400_0000`) set.
/// Panics when the manual framing walk does not land exactly on the
/// third slice end — the sweep must run over validated geometry.
fn sweep_chart(bytes: &[u8]) -> Vec<Bit26Hit> {
    const HEADER_LEN: usize = 300;
    let plain = decrypt_ojn_new_format(bytes).expect("decrypt");
    assert!(plain.len() >= HEADER_LEN, "header overrun");
    let mut start = u32le(&plain, 284) as usize;
    let mut hits = Vec::new();
    for level in 0..3 {
        let end = u32le(&plain, 288 + level * 4) as usize;
        let event_count = u32le(&plain, 64 + level * 4);
        let mut off = start;
        for _ in 0..event_count {
            let measure = i32::from_le_bytes(plain[off..off + 4].try_into().expect("measure"));
            let kind = u16le(&plain, off + 4);
            let count = u16le(&plain, off + 6) as usize;
            for i in 0..count {
                let raw = u32le(&plain, off + 8 + i * 4);
                if kind >= 2 && raw != 0 && raw & 0xFFFF != 0 && raw & 0x0400_0000 != 0 {
                    hits.push(Bit26Hit {
                        level,
                        measure,
                        kind,
                        raw,
                    });
                }
            }
            off += 8 + count * 4;
            assert!(off <= end, "event walk overran slice end");
        }
        assert_eq!(off, end, "event walk must land on the slice end");
        start = end;
    }
    hits
}

/// The 13 new-format songs ship zero bit-26 payloads, and every
/// Tutorial hit is a kind-9 BGM record with flags `0x0400` and a
/// contiguous sample id — the B22 autoplay encoding, not a hold flag
/// on a playable lane (docs/ojn-bit26.md).
#[test]
fn shipped_charts_set_bit26_only_on_tutorial_bgm_records() {
    let Some(dir) = music_dir() else {
        return; // no game data: skip cleanly
    };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("read Music dir")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ojn"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 14, "7.55 ships 14 .ojn charts");

    for path in &files {
        let bytes = std::fs::read(path).expect("read chart");
        let hits = sweep_chart(&bytes);
        let name = path.file_name().expect("file name").to_string_lossy();
        if name == "Tutorial.ojn" {
            assert_eq!(hits.len(), 12, "Tutorial BGM sets bit 26 twelve times");
            for (expected_sample, hit) in hits.iter().enumerate() {
                assert_eq!(hit.level, 0, "all hits on the Easy slice");
                assert_eq!(hit.kind, 9, "bit 26 ships only on BGM channel kind 9");
                assert_eq!(
                    hit.raw,
                    0x0400_0000 | (expected_sample as u32 + 1),
                    "flags 0x0400 + sample id {}",
                    expected_sample
                );
            }
        } else {
            assert!(hits.is_empty(), "{name} ships a bit-26 payload: {hits:?}");
        }
    }
}
