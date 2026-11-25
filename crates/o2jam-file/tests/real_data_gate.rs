//! Pack-presence gate for file-format real-data tests (#906).
//!
//! Synthetic roots exercise both arms without touching game data: an
//! absent root skips with a named report; a present root missing a
//! required artifact fails instead of skipping. No parser runs here.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use o2jam_file::data_root::{pack_dir_for_test, pack_skip_report};

fn fixture_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("o2jam-real-data-gate-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

fn write_pack(root: &Path, archive_names: &[&str]) {
    let image_dir = root.join("O2Jam/Image");
    let music_dir = root.join("O2Jam/Music");
    std::fs::create_dir_all(&image_dir).expect("fixture image dir");
    std::fs::create_dir_all(&music_dir).expect("fixture music dir");
    for name in archive_names {
        std::fs::write(image_dir.join(name), b"synthetic archive").expect("fixture archive");
    }
    for index in 0..14 {
        let name = format!("song{index}.ojn");
        std::fs::write(music_dir.join(name), b"synthetic chart").expect("fixture chart");
    }
}

#[test]
// OPEN: this negative assertion is not yet exe-pinned.
// next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
fn absent_pack_reports_skip_and_returns_no_directory() {
    let root = fixture_root("absent");
    let report = pack_skip_report(&root, "absent probe");
    assert!(
        report.starts_with("SKIP absent probe: install-root absent"),
        "skip line names the test and reason: {report}"
    );
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        pack_dir_for_test(&root, "absent probe")
    }));
    let result = outcome.expect("absent pack must not panic");
    assert!(result.is_none(), "absent pack yields no data directory");
    std::fs::remove_dir_all(&root).expect("remove fixture");
}

#[test]
fn present_pack_with_missing_archive_fails_instead_of_skipping() {
    let root = fixture_root("missing-archive");
    write_pack(&root, &["Interface1.opi", "Playing1.opi"]);
    let outcome = catch_unwind(AssertUnwindSafe(|| pack_dir_for_test(&root, "gap probe")));
    assert!(
        outcome.is_err(),
        "present pack missing avatar.opa must fail loudly"
    );
    std::fs::remove_dir_all(&root).expect("remove fixture");
}

#[test]
fn present_pack_with_all_artifacts_returns_the_data_directory() {
    let root = fixture_root("complete");
    write_pack(&root, &["Interface1.opi", "Playing1.opi", "avatar.opa"]);
    let data_dir = pack_dir_for_test(&root, "complete probe").expect("present pack");
    assert_eq!(data_dir, root.join("O2Jam"));
    std::fs::remove_dir_all(&root).expect("remove fixture");
}

#[test]
fn real_install_root_gate_does_not_skip_when_archives_are_present() {
    let root = o2jam_file::data_root::install_root();
    if o2jam_file::data_root::game_data_dir(&root).is_none() {
        println!("{}", pack_skip_report(&root, "install-root presence probe"));
        return;
    }
    let data_dir = pack_dir_for_test(&root, "install-root presence probe").expect("packed root");
    assert!(
        data_dir.join("Image/Interface1.opi").is_file(),
        "present pack has the interface archive"
    );
}
