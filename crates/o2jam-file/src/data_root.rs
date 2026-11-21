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

/// True when `root` exposes the 7.55 data folder under either casing.
/// Matches the gate's `data_root_has_archives` probe.
#[must_use]
pub fn root_has_game_data(root: &Path) -> bool {
    root.join("O2Jam").is_dir() || root.join("o2jam").is_dir()
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
