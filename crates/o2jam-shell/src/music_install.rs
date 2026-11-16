//! The install's `Music` folder: the one place the shell lists it.
//!
//! `docs/music-availability.md` §1: a song is available when its
//! `o2ma<id>.ojn` exists under `<install>\Music\`, and the client
//! learns that from a directory scan. [`o2jam_file::installed_music`]
//! is the pure half (names in, packed seat out); this module is the
//! thin `read_dir` adapter at the shell edge.

use std::path::{Path, PathBuf};

use o2jam_file::{installed_music, InstalledMusic};

/// Folder under the install root the chart files live in (`Music\`,
/// §1).
pub const MUSIC_FOLDER: &str = "O2Jam/Music";

/// The install's `Music` folder.
#[must_use]
pub fn install_music_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root")
        .join(MUSIC_FOLDER)
}

/// Scan the install's `Music` folder into the local music seat (§1).
///
/// `None` when the folder cannot be listed. An install without game
/// data (CI) has no `Music\`, so the client has no local knowledge and
/// callers keep the pre-#276 selectable answer
/// (`screens::side::song_installed`). A name that is not UTF-8 cannot
/// be an `o2ma###.ojn` chart and is skipped.
#[must_use]
pub fn scan_installed_music() -> Option<InstalledMusic> {
    installed_music_in(&install_music_dir())
}

/// Scan one `Music` folder (`None` when it cannot be listed).
#[must_use]
pub fn installed_music_in(dir: &Path) -> Option<InstalledMusic> {
    let names = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    Some(installed_music(names))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway folder named after the test.
    fn fixture_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("recomp2jam-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture dir");
        dir
    }

    #[test]
    fn absent_music_folder_answers_no_local_knowledge() {
        // §1's scan needs a directory. Without one (CI ships no game
        // data) the answer is `None`, never "no songs installed": the
        // caller must not invent a not-downloaded notice for every
        // catalog row.
        let missing = std::env::temp_dir().join("recomp2jam-no-such-music-dir");
        assert_eq!(installed_music_in(&missing), None);
    }

    #[test]
    fn music_folder_listing_packs_the_chart_files_only() {
        // The adapter hands every file name to the pure scan, which
        // keeps the `o2ma###.ojn` charts (and drops the sound
        // containers beside them, §1).
        let dir = fixture_dir("music-install-listing");
        for name in ["o2ma757.ojn", "o2ma758.ojn", "BGM.ojm"] {
            std::fs::write(dir.join(name), b"synthetic").expect("fixture file");
        }
        let seat = installed_music_in(&dir).expect("folder lists");
        // Directory order is the filesystem's (the exe's `FindNextFileA`
        // order too), so compare the set, not the sequence.
        let mut ids: Vec<u16> = seat.songs().iter().map(|song| song.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, [757, 758]);
        assert!(seat.is_installed(757));
        assert!(!seat.is_installed(759));
        std::fs::remove_dir_all(&dir).ok();
    }
}
