//! Pretty printer for `.ojm` music containers.
//!
//! Uses only the verified `o2jam-file` parsers (`parse_m30`,
//! `parse_new_variant_entries`, `parse_omc`); no new parsing logic
//! lives here. Each parser that succeeds contributes one section, so
//! ambiguous files show every coherent view instead of guessing.

use o2jam_file::{OjmSample, OjmTables};

/// Render a full `dump-ojm` report for one file's bytes.
pub fn format_ojm_report(file_label: &str, bytes: &[u8]) -> anyhow::Result<String> {
    let mut lines = vec![format!(
        "{file_label}: OJM container magic=\"{}\" file_bytes={}",
        magic_token(bytes),
        bytes.len()
    )];
    let mut showed_any_section = false;
    if let Ok(samples) = o2jam_file::parse_m30(bytes) {
        lines.extend(m30_section_lines(&samples));
        showed_any_section = true;
    }
    if let Ok(tables) = o2jam_file::parse_new_variant_entries(bytes) {
        if !tables.mode2.is_empty() || !tables.mode3.is_empty() {
            lines.extend(new_variant_section_lines(&tables));
            showed_any_section = true;
        }
    }
    match o2jam_file::parse_omc(bytes) {
        Ok(samples) => {
            if !showed_any_section || !samples.is_empty() {
                lines.extend(old_omc_section_lines(&samples));
                showed_any_section = true;
            }
        }
        Err(parse_error) => {
            if !showed_any_section {
                return Err(anyhow::anyhow!("no OJM view parsed: {parse_error}"));
            }
        }
    }
    if !showed_any_section {
        lines.push("variant: unrecognised (no parser accepted the bytes)".to_string());
    }
    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// First up-to-four bytes as lossy text, for the header line.
/// Trailing NUL padding (`M30\0`, `OMC\0`) is stripped so the
/// label reads as the bare container tag.
fn magic_token(bytes: &[u8]) -> String {
    let end = bytes.len().min(4);
    let prefix = match bytes.get(..end) {
        Some(window) => window,
        None => &[],
    };
    String::from_utf8_lossy(prefix)
        .trim_end_matches('\0')
        .to_owned()
}

/// Section for M30 inline entries with sample ids.
fn m30_section_lines(samples: &[OjmSample<'_>]) -> Vec<String> {
    let mut lines = vec![
        "variant: M30 inline entries (52-byte entries, OGG bodies)".to_string(),
        format!(
            "samples: {} ({}B total)",
            samples.len(),
            stream_total(samples)
        ),
    ];
    for sample in samples {
        lines.push(format!(
            "  id={} size={}B codec=ogg name={}",
            sample_id_token(sample.sample_id),
            sample.data.len(),
            sample.name
        ));
    }
    lines
}

/// Section for new-variant mode-2/mode-3 entry tables.
fn new_variant_section_lines(tables: &OjmTables) -> Vec<String> {
    let mut lines = vec![
        "variant: new-variant tables (mode2 0x38-byte / mode3 0x24-byte)".to_string(),
        format!("mode2 entries: {}", tables.mode2.len()),
    ];
    for (index, entry) in tables.mode2.iter().enumerate() {
        lines.push(format!(
            "  [mode2 {index}] name={} size={}B off={} fmt={} ch={} rate={} bitrate={} align={} bits={} codec=pcm16",
            entry.name,
            entry.size,
            entry.data_offset,
            entry.audio_format,
            entry.num_channels,
            entry.sample_rate,
            entry.bit_rate,
            entry.block_align,
            entry.bits_per_sample
        ));
    }
    lines.push(format!("mode3 entries: {}", tables.mode3.len()));
    for (index, entry) in tables.mode3.iter().enumerate() {
        lines.push(format!(
            "  [mode3 {index}] name={} size={}B off={} codec=ogg",
            entry.name, entry.size, entry.data_offset
        ));
    }
    lines.push(format!(
        "stream inventory: {} entries ({}B declared)",
        tables.mode2.len().saturating_add(tables.mode3.len()),
        declared_total(tables)
    ));
    lines
}

/// Section for old-variant OMC inline entries (no sample ids).
fn old_omc_section_lines(samples: &[OjmSample<'_>]) -> Vec<String> {
    let mut lines = vec![
        "variant: old OMC inline entries (36-byte entries, OGG bodies)".to_string(),
        format!(
            "samples: {} ({}B total)",
            samples.len(),
            stream_total(samples)
        ),
    ];
    for sample in samples {
        lines.push(format!(
            "  size={}B codec=ogg name={}",
            sample.data.len(),
            sample.name
        ));
    }
    lines
}

/// Printable sample id (`-` when the variant carries none).
fn sample_id_token(sample_id: Option<u32>) -> String {
    match sample_id {
        Some(id) => id.to_string(),
        None => "-".to_string(),
    }
}

/// Summed OGG body bytes across parsed samples.
fn stream_total(samples: &[OjmSample<'_>]) -> usize {
    samples.iter().map(|s| s.data.len()).sum()
}

/// Summed declared body bytes across both new-variant tables.
fn declared_total(tables: &OjmTables) -> u64 {
    let mut total: u64 = 0;
    for entry in tables.mode2.iter().chain(tables.mode3.iter()) {
        total = total.saturating_add(u64::from(entry.size));
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m30_entry_bytes(name: &str, size: u32, sample_id: u16) -> Vec<u8> {
        let mut entry = vec![0u8; 52];
        entry[..name.len()].copy_from_slice(name.as_bytes());
        entry[32..36].copy_from_slice(&size.to_le_bytes());
        entry[44..46].copy_from_slice(&sample_id.to_le_bytes());
        entry
    }

    fn m30_fixture() -> Vec<u8> {
        let mut blob = vec![0u8; 28];
        blob[..4].copy_from_slice(b"M30\0");
        blob[12..16].copy_from_slice(&2u32.to_le_bytes());
        blob[16..20].copy_from_slice(&28u32.to_le_bytes());
        blob.extend_from_slice(&m30_entry_bytes("kick", 8, 3));
        blob.extend_from_slice(b"OggSdata");
        blob.extend_from_slice(&m30_entry_bytes("mr1", 8, 980));
        blob.extend_from_slice(b"OggSmore");
        blob
    }

    fn omc_entry_bytes(name: &str, size: u32) -> Vec<u8> {
        let mut entry = vec![0u8; 36];
        entry[..name.len()].copy_from_slice(name.as_bytes());
        entry[32..36].copy_from_slice(&size.to_le_bytes());
        entry
    }

    fn old_omc_fixture() -> Vec<u8> {
        let mut blob = vec![0u8; 16];
        blob[..4].copy_from_slice(b"OMC\0");
        blob[12..16].copy_from_slice(&16u32.to_le_bytes());
        blob.extend_from_slice(&omc_entry_bytes("bgRoom", 8));
        blob.extend_from_slice(b"OggSdata");
        blob.extend_from_slice(&omc_entry_bytes("bgLogo", 8));
        blob.extend_from_slice(b"OggSlogo");
        blob
    }

    fn mode2_entry_bytes(name: &[u8], size: u32, channels: u16, rate: u32) -> Vec<u8> {
        let mut entry = vec![0u8; o2jam_file::MODE2_ENTRY_LEN];
        entry[..name.len()].copy_from_slice(name);
        entry[o2jam_file::MODE2_FORMAT_OFF..o2jam_file::MODE2_FORMAT_OFF + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        entry[o2jam_file::MODE2_CHANNELS_OFF..o2jam_file::MODE2_CHANNELS_OFF + 2]
            .copy_from_slice(&channels.to_le_bytes());
        entry[o2jam_file::MODE2_RATE_OFF..o2jam_file::MODE2_RATE_OFF + 4]
            .copy_from_slice(&rate.to_le_bytes());
        let bitrate = rate
            .checked_mul(u32::from(channels))
            .and_then(|v| v.checked_mul(2))
            .unwrap_or(0);
        entry[o2jam_file::MODE2_BITRATE_OFF..o2jam_file::MODE2_BITRATE_OFF + 4]
            .copy_from_slice(&bitrate.to_le_bytes());
        entry[o2jam_file::MODE2_ALIGN_OFF..o2jam_file::MODE2_ALIGN_OFF + 2]
            .copy_from_slice(&channels.checked_mul(2).unwrap_or(0).to_le_bytes());
        entry[o2jam_file::MODE2_BITS_OFF..o2jam_file::MODE2_BITS_OFF + 2]
            .copy_from_slice(&16u16.to_le_bytes());
        entry[o2jam_file::MODE2_SIZE_OFF..o2jam_file::MODE2_SIZE_OFF + 4]
            .copy_from_slice(&size.to_le_bytes());
        entry
    }

    fn mode3_entry_bytes(name: &[u8], size: u32) -> Vec<u8> {
        let mut entry = vec![0u8; o2jam_file::MODE3_ENTRY_LEN];
        entry[..name.len()].copy_from_slice(name);
        entry[o2jam_file::MODE3_SIZE_OFF..o2jam_file::MODE3_SIZE_OFF + 4]
            .copy_from_slice(&size.to_le_bytes());
        entry
    }

    fn new_variant_fixture() -> Vec<u8> {
        let mut blob = vec![0u8; 20];
        blob[..4].copy_from_slice(b"OMC\0");
        blob[o2jam_file::MODE2_COUNT_OFF..o2jam_file::MODE2_COUNT_OFF + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        blob[o2jam_file::MODE3_COUNT_OFF..o2jam_file::MODE3_COUNT_OFF + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        blob[o2jam_file::MODE2_ENTRIES_OFF..o2jam_file::MODE2_ENTRIES_OFF + 4]
            .copy_from_slice(&20u32.to_le_bytes());
        let mode3_off = 20 + o2jam_file::MODE2_ENTRY_LEN + 16;
        let mode3_off_u32 = u32::try_from(mode3_off).unwrap_or(0);
        blob[o2jam_file::MODE3_ENTRIES_OFF..o2jam_file::MODE3_ENTRIES_OFF + 4]
            .copy_from_slice(&mode3_off_u32.to_le_bytes());
        blob.extend_from_slice(&mode2_entry_bytes(b"s1", 16, 1, 22050));
        blob.extend_from_slice(&[0xAA; 16]);
        blob.extend_from_slice(&mode3_entry_bytes(b"s2", 8));
        blob.extend_from_slice(&[0xBB; 8]);
        blob
    }

    #[test]
    fn m30_fixture_lists_ids_sizes_and_stream_total() {
        let report = format_ojm_report("m30.ojm", &m30_fixture()).unwrap();
        assert!(report.contains("magic=\"M30\""), "{report}");
        assert!(report.contains("variant: M30"), "{report}");
        assert!(report.contains("id=3"), "{report}");
        assert!(report.contains("id=980"), "{report}");
        assert!(report.contains("codec=ogg name=kick"), "{report}");
        assert!(report.contains("samples: 2 (16B total)"), "{report}");
    }

    #[test]
    fn old_omc_fixture_lists_names_without_ids() {
        let report = format_ojm_report("bgm.ojm", &old_omc_fixture()).unwrap();
        assert!(report.contains("magic=\"OMC\""), "{report}");
        assert!(report.contains("variant: old OMC"), "{report}");
        assert!(report.contains("codec=ogg name=bgRoom"), "{report}");
        assert!(report.contains("codec=ogg name=bgLogo"), "{report}");
        assert!(report.contains("samples: 2 (16B total)"), "{report}");
    }

    #[test]
    fn new_variant_fixture_lists_both_tables_with_audio_params() {
        let report = format_ojm_report("new.ojm", &new_variant_fixture()).unwrap();
        assert!(report.contains("variant: new-variant tables"), "{report}");
        assert!(report.contains("mode2 entries: 1"), "{report}");
        assert!(report.contains("mode3 entries: 1"), "{report}");
        assert!(report.contains("rate=22050"), "{report}");
        assert!(report.contains("name=s2"), "{report}");
        assert!(
            report.contains("stream inventory: 2 entries (24B declared)"),
            "{report}"
        );
    }

    #[test]
    fn garbage_bytes_report_an_error() {
        assert!(format_ojm_report("bad.ojm", b"ZZZZ....").is_err());
    }
}
