//! Runtime install root for data-bearing tests and the gate probe.
//!
//! Test binaries used to bake `CARGO_MANIFEST_DIR/../../install-root`
//! into the binary, so a stale binary could read another worktree's
//! archives during a hidden-data arm. [`install_root`] reads
//! `O2JAM_INSTALL_ROOT` at runtime and falls back to the checkout root;
//! [`checkout_install_root`] names the fallback alone so the probe can
//! compare both routes (issue #736).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Environment variable naming the runtime data root. `o2jam-dump`
/// reads the same name (scripts/lib/data-root.sh on the gate side).
pub const INSTALL_ROOT_ENV: &str = "O2JAM_INSTALL_ROOT";

/// The root data-dependent code must read: the runtime override when
/// set, otherwise the checkout's git-ignored `install-root`.
#[must_use]
pub fn install_root() -> PathBuf {
    let override_value = std::env::var_os(INSTALL_ROOT_ENV);
    root_from_override(override_value.as_deref())
}

/// The compile-time fallback root, named independently of the runtime
/// override so the hidden-arm probe can see a stale fallback.
#[must_use]
pub fn checkout_install_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

/// Resolve an explicit override without depending on global environment
/// state (the seam the unit tests drive).
#[must_use]
pub fn root_from_override(value: Option<&OsStr>) -> PathBuf {
    value
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(checkout_install_root)
}

/// The 7.55 data folder inside `root`, under either casing.
#[must_use]
pub fn game_data_dir(root: &Path) -> Option<PathBuf> {
    ["O2Jam", "o2jam"]
        .iter()
        .map(|name| root.join(name))
        .find(|path| path.is_dir())
}

/// True when `root` exposes the 7.55 data folder under either casing.
/// Matches the gate's `data_root_has_archives` probe.
#[must_use]
pub fn root_has_game_data(root: &Path) -> bool {
    game_data_dir(root).is_some()
}

/// The SKIP line a real-data test emits when the pack is absent.
#[must_use]
pub fn pack_skip_report(root: &Path, test_name: &str) -> String {
    format!("SKIP {test_name}: install-root absent ({})", root.display())
}

/// Resolve the data folder for a real-data test. An absent pack prints
/// the named SKIP line and yields `None`; a present pack must pass the
/// artifact check, so a gap fails the test instead of skipping (issue
/// #906).
#[must_use]
pub fn pack_dir_for_test(root: &Path, test_name: &str) -> Option<PathBuf> {
    let Some(data_dir) = game_data_dir(root) else {
        println!("{}", pack_skip_report(root, test_name));
        return None;
    };
    require_pack_artifacts(&data_dir, test_name);
    Some(data_dir)
}

/// Fail when a present pack lacks an artifact the file-format guards
/// read. The 14-chart floor is the small-install invariant already
/// pinned by the bit-26 sweep; a wider pack only raises it.
fn require_pack_artifacts(data_dir: &Path, test_name: &str) {
    for name in ["Interface1.opi", "Playing1.opi", "avatar.opa"] {
        let path = data_dir.join("Image").join(name);
        assert!(
            path.is_file(),
            "PACK GAP {test_name}: missing {}",
            path.display()
        );
    }
    let music_dir = data_dir.join("Music");
    assert!(
        music_dir.is_dir(),
        "PACK GAP {test_name}: missing {}",
        music_dir.display()
    );
    let chart_count = std::fs::read_dir(&music_dir)
        .unwrap_or_else(|error| panic!("PACK GAP {test_name}: {music_dir:?}: {error}"))
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "ojn"))
        .count();
    assert!(
        chart_count >= 14,
        "PACK GAP {test_name}: expected at least 14 charts, found {chart_count}"
    );
}

/// True when two root spellings name one directory. An absent root
/// falls back to an absolute spelling, never to the bare relative path.
#[must_use]
pub fn same_root(left: &Path, right: &Path) -> bool {
    resolve_root(left) == resolve_root(right)
}

fn resolve_root(root: &Path) -> PathBuf {
    root.canonicalize()
        .unwrap_or_else(|_| std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_override_wins_over_the_checkout_fallback() {
        let explicit = Path::new("/tmp/o2jam-explicit-root");
        assert_eq!(root_from_override(Some(explicit.as_os_str())), explicit);
    }

    #[test]
    fn empty_override_falls_back_to_the_checkout_root() {
        assert_eq!(
            root_from_override(Some(OsStr::new(""))),
            checkout_install_root()
        );
    }

    #[test]
    fn unset_override_falls_back_to_the_checkout_root() {
        assert_eq!(root_from_override(None), checkout_install_root());
    }

    #[test]
    fn lowercase_data_folder_is_recognised() {
        let root = data_folder_fixture("lowercase", "o2jam");
        assert!(root_has_game_data(&root));
        std::fs::remove_dir_all(&root).expect("remove fixture");
    }

    #[test]
    fn uppercase_data_folder_is_recognised() {
        let root = data_folder_fixture("uppercase", "O2Jam");
        assert!(root_has_game_data(&root));
        std::fs::remove_dir_all(&root).expect("remove fixture");
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn root_without_a_data_folder_has_no_game_data() {
        let root = std::env::temp_dir().join("o2jam-data-root-empty-probe");
        std::fs::create_dir_all(&root).expect("fixture root dir");
        assert!(!root_has_game_data(&root));
        std::fs::remove_dir(&root).expect("remove fixture");
    }

    #[test]
    fn roots_with_a_parent_component_compare_equal_when_present() {
        let root = std::env::temp_dir().join("o2jam-same-root-probe");
        std::fs::create_dir_all(root.join("child")).expect("fixture child dir");
        let indirect = root.join("child/..");
        assert!(same_root(&root, &indirect));
        std::fs::remove_dir_all(&root).expect("remove fixture");
    }

    fn data_folder_fixture(name: &str, folder: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("o2jam-data-root-{name}-probe"));
        std::fs::create_dir_all(root.join(folder)).expect("fixture data dir");
        root
    }
}
