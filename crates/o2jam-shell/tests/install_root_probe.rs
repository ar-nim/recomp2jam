//! Gate probe: a hidden data arm must not reach archives through the
//! compile-time fallback (issue #736).
//!
//! Live caller chain: `scripts/gate-check.sh --arm hidden` ->
//! `O2JAM_INSTALL_ROOT=<unreachable root>` -> `cargo test --workspace` ->
//! this integration test. The gate also renames the checkout's data
//! symlink before that arm; this test fails if either route still
//! exposes an `O2Jam` folder or decodable archive bytes.

use std::path::{Path, PathBuf};

use o2jam_file::data_root::{
    checkout_install_root, install_root, root_has_game_data, same_root, INSTALL_ROOT_ENV,
};
use o2jam_shell::staging::read_install_archives;

/// Roots from which archive bytes are still reachable.
fn reachable_roots(roots: &[&Path]) -> Vec<PathBuf> {
    roots
        .iter()
        .copied()
        .filter(|root| {
            let archives = read_install_archives(root);
            root_has_game_data(root) || archives.interface.is_some() || archives.playing.is_some()
        })
        .map(Path::to_path_buf)
        .collect()
}

#[test]
fn explicit_data_root_probe_fails_when_a_hidden_arm_can_reach_archives() {
    let has_override = std::env::var_os(INSTALL_ROOT_ENV).is_some_and(|value| !value.is_empty());
    if !has_override {
        return; // no gate root: nothing to probe
    }
    let explicit = install_root();
    let fallback = checkout_install_root();
    if same_root(&explicit, &fallback) {
        return; // present arm: the gate names the fallback itself
    }
    let reachable = reachable_roots(&[&explicit, &fallback]);
    assert!(
        reachable.is_empty(),
        "hidden arm can still reach archives at: {}",
        reachable
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

#[test]
fn reachable_root_probe_names_an_archive_bearing_explicit_root() {
    let explicit = fixture_root("explicit-root-with-data", true);
    let fallback = fixture_root("fallback-root-without-data", false);
    assert_eq!(reachable_roots(&[&explicit, &fallback]), vec![explicit]);
}

#[test]
fn reachable_root_probe_names_an_archive_bearing_fallback() {
    let explicit = fixture_root("explicit-root-without-data", false);
    let fallback = fixture_root("fallback-root-with-data", true);
    assert_eq!(reachable_roots(&[&explicit, &fallback]), vec![fallback]);
}

/// Empty fixture root, optionally shaped like an install with data.
fn fixture_root(name: &str, with_data: bool) -> PathBuf {
    let root = std::env::temp_dir().join(format!("o2jam-probe-{name}"));
    let data = root.join("O2Jam");
    if with_data {
        std::fs::create_dir_all(&data).expect("fixture data dir");
    } else {
        std::fs::create_dir_all(&root).expect("fixture root dir");
        let _ = std::fs::remove_dir_all(&data);
    }
    root
}
