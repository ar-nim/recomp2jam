//! Per-merge sample of the shipped 7.55 chart pack (issue #737).
//!
//! The full 515-chart sweeps run on a seven-day stamp; this guard
//! checks a dozen hash-pinned charts on every suite run, in seconds.
//! The manifest (`fixtures/full-pack-sample.sha256`) stores names and
//! digests only — no copyrighted bytes live in the repository.
//!
//! Live caller chain: `scripts/gate-check.sh --arm quick|full` ->
//! `cargo test --workspace` -> this integration test ->
//! `o2jam_file::data_root::install_root` -> `<root>/O2Jam/Music`.

use std::path::{Path, PathBuf};

use o2jam_file::data_root::{install_root, pack_dir_for_test};
use sha2::{Digest, Sha256};

/// One hash-pinned chart from the committed sample manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PinnedChart {
    /// Chart file name under `O2Jam/Music`.
    name: String,
    /// Expected SHA-256, lowercase hex.
    digest: String,
}

/// Committed manifest, anchored to the checkout rather than the data root.
fn sample_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/full-pack-sample.sha256")
}

/// Parse `name digest` lines, skipping comments and blank lines.
fn read_pinned_charts(manifest: &Path) -> Vec<PinnedChart> {
    std::fs::read_to_string(manifest)
        .unwrap_or_else(|error| panic!("read {}: {error}", manifest.display()))
        .lines()
        .filter(|line| !(line.trim().is_empty() || line.trim_start().starts_with('#')))
        .map(|line| {
            let (name, digest) = line
                .split_once(' ')
                .unwrap_or_else(|| panic!("manifest line has no digest: {line}"));
            PinnedChart {
                name: name.to_owned(),
                digest: digest.to_owned(),
            }
        })
        .collect()
}

/// Lowercase hex SHA-256 of one chart's bytes.
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Compare one chart's bytes with its pinned digest.
fn verify_chart(music_dir: &Path, chart: &PinnedChart) -> Result<(), String> {
    let path = music_dir.join(&chart.name);
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let actual = sha256_hex(&bytes);
    if actual != chart.digest {
        return Err(format!(
            "{}: expected {}, got {actual}",
            chart.name, chart.digest
        ));
    }
    Ok(())
}

/// Compare every pinned chart, naming the first mismatch.
fn verify_charts(music_dir: &Path, charts: &[PinnedChart]) -> Result<usize, String> {
    charts
        .iter()
        .try_for_each(|chart| verify_chart(music_dir, chart))?;
    Ok(charts.len())
}

#[test]
fn full_pack_sample_manifest_pins_twelve_charts() {
    let charts = read_pinned_charts(&sample_manifest());
    assert_eq!(
        charts.len(),
        12,
        "the per-merge sample must cover twelve charts"
    );
}

#[test]
fn sample_manifest_digests_are_full_length_hex() {
    let charts = read_pinned_charts(&sample_manifest());
    assert!(
        charts
            .iter()
            .all(|chart| chart.digest.len() == 64
                && chart.digest.bytes().all(|b| b.is_ascii_hexdigit())),
        "every pinned digest must be 64 hex characters"
    );
}

#[test]
fn full_pack_sample_compares_every_pinned_chart_hash() {
    let Some(data_dir) = pack_dir_for_test(
        &install_root(),
        stringify!(full_pack_sample_compares_every_pinned_chart_hash),
    ) else {
        return; // gate printed SKIP
    };
    let music_dir = data_dir.join("Music");
    let charts = read_pinned_charts(&sample_manifest());
    let verified = verify_charts(&music_dir, &charts)
        .unwrap_or_else(|error| panic!("sample chart check failed: {error}"));
    assert_eq!(verified, charts.len(), "every pinned chart was checked");
}

#[test]
fn full_pack_sample_rejects_a_changed_chart_hash() {
    let root = std::env::temp_dir().join("o2jam-sample-tampered-chart");
    let music_dir = root.join("O2Jam/Music");
    std::fs::create_dir_all(&music_dir).expect("fixture music dir");
    let bytes = b"synthetic chart bytes";
    std::fs::write(music_dir.join("Tutorial.ojn"), bytes).expect("fixture chart");
    let charts = vec![PinnedChart {
        name: "Tutorial.ojn".to_owned(),
        digest: "0".repeat(64),
    }];
    let error = verify_charts(&music_dir, &charts).expect_err("wrong digest is rejected");
    assert!(
        error.contains("Tutorial.ojn"),
        "error names the chart: {error}"
    );
    std::fs::remove_dir_all(&root).expect("remove fixture");
}
