//! Pretty printer for `.ojn` chart files.
//!
//! Uses only the verified `o2jam-file` parsers (`parse_ojn`,
//! `parse_ojn_levels`, `parse_ojn_level_events`) plus the pure
//! value helpers (`decode_note_value`, `ojn_lane_of_kind`,
//! `level_for_difficulty`); no new parsing logic lives here.

use o2jam_file::{OjnChartFile, OjnEvent, OjnLevelSlice};
use std::collections::BTreeMap;

/// Render a full `dump-ojn` report for one file's bytes.
pub fn format_ojn_report(file_label: &str, bytes: &[u8]) -> anyhow::Result<String> {
    let chart = o2jam_file::parse_ojn(bytes)?;
    let slices = o2jam_file::parse_ojn_levels(bytes)?;
    let mut lines = vec![format!(
        "{file_label}: OJN chart song={} file_bytes={}",
        chart.song_id,
        bytes.len()
    )];
    lines.extend(header_map_lines(&chart));
    lines.extend(level_range_lines(&slices));
    for slice in &slices {
        let events = o2jam_file::parse_ojn_level_events(bytes, slice.level)?;
        lines.extend(level_detail_lines(&chart, slice, &events));
    }
    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// Every mapped header field, one line per field group.
fn header_map_lines(chart: &OjnChartFile) -> Vec<String> {
    let magic = String::from_utf8_lossy(&chart.magic).into_owned();
    vec![
        "header:".to_string(),
        format!(
            "  magic={magic:?} song_id={} version={} genre={} bpm={}",
            chart.song_id, chart.format_version, chart.genre, chart.bpm
        ),
        format!(
            "  levels=easy:{} normal:{} hard:{} flags={} stable_minor={}",
            level_token(chart.level_for_difficulty(0)),
            level_token(chart.level_for_difficulty(1)),
            level_token(chart.level_for_difficulty(2)),
            chart.level_flags,
            chart.stable_minor
        ),
        format!(
            "  totals={:?} playable={:?} last_measures={:?} events={:?}",
            chart.total_counts, chart.playable_counts, chart.last_measures, chart.event_counts
        ),
        format!(
            "  key_field={} hi={} lo={} flag96={} const100={} tune={}",
            chart.key_field,
            chart.key_hi(),
            chart.key_lo(),
            chart.flag96,
            chart.const100,
            chart.tune
        ),
        format!(
            "  title={:?} artist={:?} label={:?} ojm={:?}",
            chart.title, chart.artist, chart.label, chart.ojm_file
        ),
        format!(
            "  unknown1={} durations={:?} slice_start={} slice_ends={:?}",
            chart.unknown1, chart.durations, chart.slice_start, chart.slice_ends
        ),
    ]
}

/// Byte range of each difficulty slice.
fn level_range_lines(slices: &[OjnLevelSlice]) -> Vec<String> {
    let mut lines = vec!["levels:".to_string()];
    for slice in slices {
        lines.push(format!(
            "  [level {}] events={} range={}..{} ({}B)",
            slice.level,
            slice.event_count,
            slice.data_start,
            slice.data_end,
            slice.data_end.saturating_sub(slice.data_start)
        ));
    }
    lines
}

/// Per-level kind histogram plus walked note totals.
fn level_detail_lines(
    chart: &OjnChartFile,
    slice: &OjnLevelSlice,
    events: &[OjnEvent],
) -> Vec<String> {
    let mut lines = vec![format!("level {} events by kind:", slice.level)];
    let histogram = count_events_by_kind(events);
    if histogram.is_empty() {
        lines.push("  (no events)".to_string());
    }
    for (kind, count) in &histogram {
        lines.push(format!("  kind={kind}: {count} events"));
    }
    let (walked_total, walked_playable) = count_walked_notes(events);
    let slot = slice.level.min(chart.total_counts.len().saturating_sub(1));
    lines.push(format!(
        "  notes total={walked_total} (header {}) playable={walked_playable} (header {})",
        chart.total_counts[slot], chart.playable_counts[slot]
    ));
    lines
}

/// How many events carry each channel kind.
fn count_events_by_kind(events: &[OjnEvent]) -> BTreeMap<u16, usize> {
    let mut histogram: BTreeMap<u16, usize> = BTreeMap::new();
    for event in events {
        histogram
            .entry(event.kind)
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
    }
    histogram
}

/// Walked note totals: kinds 2+ with a nonzero payload (total) and
/// the lane-kind subset 2..=8 (playable).
fn count_walked_notes(events: &[OjnEvent]) -> (usize, usize) {
    let mut total: usize = 0;
    let mut playable: usize = 0;
    for event in events {
        if event.kind < 2 {
            continue;
        }
        for value in &event.values {
            if o2jam_file::decode_note_value(*value).is_none() {
                continue;
            }
            total = total.saturating_add(1);
            if o2jam_file::ojn_lane_of_kind(event.kind).is_some() {
                playable = playable.saturating_add(1);
            }
        }
    }
    (total, playable)
}

/// Printable difficulty level (`-` when the slot is absent).
fn level_token(level: Option<u32>) -> String {
    match level {
        Some(value) => value.to_string(),
        None => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed_event(measure: i32, kind: u16, payload: &[u32]) -> Vec<u8> {
        let mut event = Vec::new();
        let measure_u32 = u32::try_from(measure).unwrap_or(0);
        event.extend_from_slice(&measure_u32.to_le_bytes());
        event.extend_from_slice(&kind.to_le_bytes());
        let count = u16::try_from(payload.len()).unwrap_or(0);
        event.extend_from_slice(&count.to_le_bytes());
        for unit in payload {
            event.extend_from_slice(&unit.to_le_bytes());
        }
        event
    }

    fn tutorial_shaped_fixture() -> Vec<u8> {
        let level0 = [
            framed_event(0, 9, &vec![0u32; 192]),
            framed_event(12, 2, &[7, 0, 0, 0]),
            framed_event(12, 1, &[150f32.to_bits()]),
            framed_event(17, 0, &[0.75f32.to_bits()]),
            framed_event(20, 5, &[0, 12, 0, 0]),
            framed_event(102, 9, &[0x1F]),
        ]
        .concat();
        let mut blob = vec![0u8; o2jam_file::OJN_HEADER_LEN];
        blob[0..4].copy_from_slice(&8888u32.to_le_bytes());
        blob[4..8].copy_from_slice(b"ojn\0");
        blob[8..12].copy_from_slice(&2.7f32.to_le_bytes());
        blob[12..16].copy_from_slice(&10u32.to_le_bytes());
        blob[16..20].copy_from_slice(&100f32.to_le_bytes());
        blob[20..24].copy_from_slice(&1u32.to_le_bytes());
        blob[28..32].copy_from_slice(&3u32.to_le_bytes());
        blob[40..44].copy_from_slice(&2u32.to_le_bytes());
        blob[52..56].copy_from_slice(&102u32.to_le_bytes());
        blob[64..68].copy_from_slice(&6u32.to_le_bytes());
        blob[76..80].copy_from_slice(&0x22B8_001Bu32.to_le_bytes());
        blob[204..216].copy_from_slice(b"nao.paradigm");
        blob[236..248].copy_from_slice(b"Tutorial.ojm");
        blob[268..272].copy_from_slice(&106954u32.to_le_bytes());
        blob[272..276].copy_from_slice(&247u32.to_le_bytes());
        let header_len_u32 = u32::try_from(o2jam_file::OJN_HEADER_LEN).unwrap_or(0);
        blob[284..288].copy_from_slice(&header_len_u32.to_le_bytes());
        let end0 = o2jam_file::OJN_HEADER_LEN + level0.len();
        let end0_u32 = u32::try_from(end0).unwrap_or(0);
        for off in o2jam_file::OJN_SLICE_END_OFFS {
            blob[off..off + 4].copy_from_slice(&end0_u32.to_le_bytes());
        }
        blob.extend_from_slice(&level0);
        blob
    }

    #[test]
    fn tutorial_fixture_shows_header_map_and_level_ranges() {
        let blob = tutorial_shaped_fixture();
        let report = format_ojn_report("Tutorial.ojn", &blob).unwrap();
        assert!(report.contains("song=8888"), "{report}");
        assert!(report.contains("bpm=100"), "{report}");
        assert!(report.contains("levels:"), "{report}");
        assert!(report.contains("[level 0] events=6"), "{report}");
        assert!(report.contains("range=300.."), "{report}");
        assert!(report.contains("[level 1] events=0"), "{report}");
    }

    #[test]
    fn tutorial_fixture_counts_kinds_and_note_totals() {
        let blob = tutorial_shaped_fixture();
        let report = format_ojn_report("Tutorial.ojn", &blob).unwrap();
        assert!(report.contains("kind=9: 2 events"), "{report}");
        assert!(report.contains("kind=2: 1 events"), "{report}");
        assert!(report.contains("kind=5: 1 events"), "{report}");
        assert!(report.contains("notes total=3 (header 3)"), "{report}");
        assert!(report.contains("playable=2 (header 2)"), "{report}");
    }

    #[test]
    fn garbage_bytes_report_an_error() {
        assert!(format_ojn_report("bad.ojn", b"ZZZZ....").is_err());
    }
}
